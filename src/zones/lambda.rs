//! The `lambda` dialect — a Lisp-like, parenthesized surface.
//!
//! A lambda zone is a sequence of S-expressions. Evaluating an expression tree
//! in post-order is naturally postfix, which is exactly what the stack machine
//! wants, so the whole dialect lowers cleanly to core ops.
//!
//! ```text
//! (print (* (+ 1 2) (- 10 3)))   ; prints 21
//! ```
//!
//! Forms:
//! - integer literals push themselves
//! - `(+ a b ...)` sum   (no args -> 0)
//! - `(* a b ...)` product (no args -> 1)
//! - `(- a b ...)` left fold; `(- a)` negates
//! - `(/ a b ...)` left fold (needs at least two args)
//! - `(print x)` print `x` as a number
//! - `(emit x)`  print `x` as a character
//! - `;` starts a comment that runs to end of line
//!
//! A lambda zone may also leave a value on the stack (no `print`) for a later
//! zone in another dialect to pick up.

use crate::core::Op;
use std::fmt;

/// A problem found while parsing or lowering a lambda zone.
#[derive(Debug, PartialEq, Eq)]
pub enum LambdaError {
    /// A `)` appeared with no matching `(`.
    UnexpectedClose,
    /// A `(` was never closed.
    UnclosedParen,
    /// An empty application `()`.
    EmptyApplication,
    /// The head of a list wasn't an operator (e.g. `(1 2)`).
    NotCallable,
    /// A symbol used on its own, not as a list head (e.g. a bare `+`).
    BareSymbol(String),
    /// A list head that isn't a known operator.
    UnknownOperator(String),
    /// An operator got the wrong number of arguments.
    BadArity(String),
}

impl fmt::Display for LambdaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LambdaError::UnexpectedClose => write!(f, "lambda error: unexpected `)`"),
            LambdaError::UnclosedParen => write!(f, "lambda error: unclosed `(`"),
            LambdaError::EmptyApplication => write!(f, "lambda error: empty application `()`"),
            LambdaError::NotCallable => {
                write!(f, "lambda error: list head is not an operator")
            }
            LambdaError::BareSymbol(s) => {
                write!(f, "lambda error: symbol `{s}` used outside a list")
            }
            LambdaError::UnknownOperator(s) => {
                write!(f, "lambda error: unknown operator `{s}`")
            }
            LambdaError::BadArity(op) => {
                write!(f, "lambda error: wrong number of arguments to `{op}`")
            }
        }
    }
}

/// Parse a lambda-zone body and lower every top-level expression to core ops.
pub fn lower(body: &str) -> Result<Vec<Op>, LambdaError> {
    let tokens = tokenize(body);
    let exprs = parse_program(&tokens)?;
    let mut ops = Vec::new();
    for expr in &exprs {
        emit(expr, &mut ops)?;
    }
    Ok(ops)
}

// --- syntax tree --------------------------------------------------------

#[derive(Debug)]
enum Expr {
    Num(i64),
    Sym(String),
    List(Vec<Expr>),
}

enum Tok {
    Open,
    Close,
    Atom(String),
}

fn tokenize(body: &str) -> Vec<Tok> {
    let mut tokens = Vec::new();
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '(' => tokens.push(Tok::Open),
            ')' => tokens.push(Tok::Close),
            ';' => {
                // Comment to end of line.
                while let Some(&n) = chars.peek() {
                    if n == '\n' {
                        break;
                    }
                    chars.next();
                }
            }
            c if c.is_whitespace() => {}
            _ => {
                let mut atom = String::from(c);
                while let Some(&n) = chars.peek() {
                    if n == '(' || n == ')' || n == ';' || n.is_whitespace() {
                        break;
                    }
                    atom.push(n);
                    chars.next();
                }
                tokens.push(Tok::Atom(atom));
            }
        }
    }
    tokens
}

fn parse_program(tokens: &[Tok]) -> Result<Vec<Expr>, LambdaError> {
    let mut pos = 0;
    let mut exprs = Vec::new();
    while pos < tokens.len() {
        exprs.push(parse_expr(tokens, &mut pos)?);
    }
    Ok(exprs)
}

fn parse_expr(tokens: &[Tok], pos: &mut usize) -> Result<Expr, LambdaError> {
    match &tokens[*pos] {
        Tok::Open => {
            *pos += 1;
            let mut items = Vec::new();
            loop {
                match tokens.get(*pos) {
                    None => return Err(LambdaError::UnclosedParen),
                    Some(Tok::Close) => {
                        *pos += 1;
                        break;
                    }
                    Some(_) => items.push(parse_expr(tokens, pos)?),
                }
            }
            Ok(Expr::List(items))
        }
        Tok::Close => Err(LambdaError::UnexpectedClose),
        Tok::Atom(a) => {
            *pos += 1;
            Ok(match a.parse::<i64>() {
                Ok(n) => Expr::Num(n),
                Err(_) => Expr::Sym(a.clone()),
            })
        }
    }
}

// --- lowering -----------------------------------------------------------

fn emit(expr: &Expr, ops: &mut Vec<Op>) -> Result<(), LambdaError> {
    match expr {
        Expr::Num(n) => {
            ops.push(Op::Push(*n));
            Ok(())
        }
        Expr::Sym(s) => Err(LambdaError::BareSymbol(s.clone())),
        Expr::List(items) => {
            let (head, args) = items.split_first().ok_or(LambdaError::EmptyApplication)?;
            match head {
                Expr::Sym(op) => apply(op, args, ops),
                _ => Err(LambdaError::NotCallable),
            }
        }
    }
}

fn apply(op: &str, args: &[Expr], ops: &mut Vec<Op>) -> Result<(), LambdaError> {
    match op {
        "+" => fold_with_identity(args, ops, Op::Add, 0),
        "*" => fold_with_identity(args, ops, Op::Mul, 1),
        "-" => {
            if args.is_empty() {
                return Err(LambdaError::BadArity("-".into()));
            }
            if args.len() == 1 {
                // Unary minus: 0 - x.
                ops.push(Op::Push(0));
                emit(&args[0], ops)?;
                ops.push(Op::Sub);
                Ok(())
            } else {
                fold(args, ops, Op::Sub)
            }
        }
        "/" => {
            if args.len() < 2 {
                return Err(LambdaError::BadArity("/".into()));
            }
            fold(args, ops, Op::Div)
        }
        "print" => unary(args, ops, Op::Print, "print"),
        "emit" => unary(args, ops, Op::Emit, "emit"),
        _ => Err(LambdaError::UnknownOperator(op.to_string())),
    }
}

/// Variadic fold with an identity for the zero-argument case.
fn fold_with_identity(
    args: &[Expr],
    ops: &mut Vec<Op>,
    op: Op,
    identity: i64,
) -> Result<(), LambdaError> {
    if args.is_empty() {
        ops.push(Op::Push(identity));
        Ok(())
    } else {
        fold(args, ops, op)
    }
}

/// Left fold a binary op over `args` (which must be non-empty).
fn fold(args: &[Expr], ops: &mut Vec<Op>, op: Op) -> Result<(), LambdaError> {
    emit(&args[0], ops)?;
    for arg in &args[1..] {
        emit(arg, ops)?;
        ops.push(op.clone());
    }
    Ok(())
}

/// A one-argument form: evaluate the argument, then apply `op`.
fn unary(args: &[Expr], ops: &mut Vec<Op>, op: Op, name: &str) -> Result<(), LambdaError> {
    if args.len() != 1 {
        return Err(LambdaError::BadArity(name.to_string()));
    }
    emit(&args[0], ops)?;
    ops.push(op);
    Ok(())
}
