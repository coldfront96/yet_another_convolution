//! The minimalist core.
//!
//! No matter how baroque a dialect's surface syntax becomes, it ultimately
//! drives this tiny machine. Linear dialects (like `stack`) compile to a flat
//! list of [`Op`]s; richer dialects (like the 2D `grid`) run their own
//! interpreter but operate on this same [`Vm`] — the same shared stack and the
//! same output buffer. That shared substrate is what makes the language truly
//! polyglot instead of a pile of unrelated mini-languages.

use std::fmt;

/// A core instruction — the universal target the linear dialects compile to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// Push a literal integer onto the stack.
    Push(i64),
    Add,
    Sub,
    Mul,
    Div,
    /// Duplicate the top of the stack: `a -> a a`.
    Dup,
    /// Discard the top of the stack: `a ->`.
    Drop,
    /// Swap the top two values: `a b -> b a`.
    Swap,
    /// Copy the second value over the top: `a b -> a b a`.
    Over,
    /// Pop and print the top as a decimal number followed by a newline.
    Print,
    /// Pop and print the top as a single Unicode character.
    Emit,
}

/// Something went wrong while executing.
#[derive(Debug, PartialEq, Eq)]
pub enum RuntimeError {
    /// A linear op needed more values than the stack held. Carries the word.
    StackUnderflow(&'static str),
    /// Division by zero in a linear op.
    DivideByZero,
    /// A 2D dialect ran past its step budget (likely a missing `@`).
    StepLimit,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::StackUnderflow(op) => {
                write!(f, "runtime error: stack underflow at `{op}`")
            }
            RuntimeError::DivideByZero => write!(f, "runtime error: division by zero"),
            RuntimeError::StepLimit => {
                write!(f, "runtime error: step limit exceeded (is a grid missing `@`?)")
            }
        }
    }
}

/// The shared machine: one stack, one output buffer.
///
/// Output is captured into a buffer rather than written to stdout directly, so
/// the whole pipeline stays testable. Every dialect, linear or 2D, pushes and
/// pops through these methods.
pub struct Vm {
    stack: Vec<i64>,
    output: String,
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

impl Vm {
    pub fn new() -> Self {
        Vm {
            stack: Vec::new(),
            output: String::new(),
        }
    }

    // --- Shared primitives, used by every dialect -------------------------

    /// Push a value.
    pub fn push(&mut self, n: i64) {
        self.stack.push(n);
    }

    /// Pop a value, or `0` if the stack is empty. This is the forgiving,
    /// Befunge-style pop that 2D dialects use.
    pub fn pop_or_zero(&mut self) -> i64 {
        self.stack.pop().unwrap_or(0)
    }

    /// Append a number followed by a newline to the output (the `stack`
    /// dialect's print convention).
    pub fn print_line(&mut self, n: i64) {
        self.output.push_str(&n.to_string());
        self.output.push('\n');
    }

    /// Append raw text to the output (used by dialects with their own print
    /// formatting, e.g. the grid's number-then-space convention).
    pub fn write_str(&mut self, s: &str) {
        self.output.push_str(s);
    }

    /// Append `n` interpreted as a single Unicode character. Out-of-range code
    /// points become the replacement char rather than aborting.
    pub fn emit_char(&mut self, n: i64) {
        let c = u32::try_from(n)
            .ok()
            .and_then(char::from_u32)
            .unwrap_or('\u{FFFD}');
        self.output.push(c);
    }

    // --- Linear op execution ---------------------------------------------

    fn pop_checked(&mut self, who: &'static str) -> Result<i64, RuntimeError> {
        self.stack.pop().ok_or(RuntimeError::StackUnderflow(who))
    }

    /// Execute a flat list of core ops in order. Arithmetic wraps rather than
    /// panicking, because a "for fun" language should be hard to crash.
    pub fn run_ops(&mut self, program: &[Op]) -> Result<(), RuntimeError> {
        for op in program {
            match op {
                Op::Push(n) => self.push(*n),
                Op::Add => {
                    let b = self.pop_checked("+")?;
                    let a = self.pop_checked("+")?;
                    self.push(a.wrapping_add(b));
                }
                Op::Sub => {
                    let b = self.pop_checked("-")?;
                    let a = self.pop_checked("-")?;
                    self.push(a.wrapping_sub(b));
                }
                Op::Mul => {
                    let b = self.pop_checked("*")?;
                    let a = self.pop_checked("*")?;
                    self.push(a.wrapping_mul(b));
                }
                Op::Div => {
                    let b = self.pop_checked("/")?;
                    let a = self.pop_checked("/")?;
                    if b == 0 {
                        return Err(RuntimeError::DivideByZero);
                    }
                    self.push(a.wrapping_div(b));
                }
                Op::Dup => {
                    let a = self.pop_checked("dup")?;
                    self.push(a);
                    self.push(a);
                }
                Op::Drop => {
                    self.pop_checked("drop")?;
                }
                Op::Swap => {
                    let b = self.pop_checked("swap")?;
                    let a = self.pop_checked("swap")?;
                    self.push(b);
                    self.push(a);
                }
                Op::Over => {
                    let b = self.pop_checked("over")?;
                    let a = self.pop_checked("over")?;
                    self.push(a);
                    self.push(b);
                    self.push(a);
                }
                Op::Print => {
                    let a = self.pop_checked(".")?;
                    self.print_line(a);
                }
                Op::Emit => {
                    let a = self.pop_checked(",")?;
                    self.emit_char(a);
                }
            }
        }
        Ok(())
    }

    /// Everything the program printed.
    pub fn output(&self) -> &str {
        &self.output
    }

    /// The values left on the stack when execution finished (useful for tests).
    pub fn stack(&self) -> &[i64] {
        &self.stack
    }
}
