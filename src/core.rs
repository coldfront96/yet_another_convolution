//! The minimalist core.
//!
//! No matter how baroque a zone's surface syntax becomes, every zone lowers to
//! this tiny instruction set. The whole point of Convolution is that the surface
//! is wild and the soul is small: get this layer right and every future gimmick
//! (2D grids, prose, self-modification, encoding) only has to learn how to emit
//! these handful of ops.

use std::fmt;

/// A core instruction. This is the universal target every zone compiles down to.
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

/// Something went wrong while executing core ops.
#[derive(Debug, PartialEq, Eq)]
pub enum RuntimeError {
    /// An op needed more values than the stack held. Carries the offending word.
    StackUnderflow(&'static str),
    /// Division by zero.
    DivideByZero,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::StackUnderflow(op) => {
                write!(f, "runtime error: stack underflow at `{op}`")
            }
            RuntimeError::DivideByZero => write!(f, "runtime error: division by zero"),
        }
    }
}

/// A tiny stack machine that executes a flat list of core [`Op`]s.
///
/// Output is captured into a buffer rather than written to stdout directly, so
/// the whole pipeline stays testable.
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

    fn pop(&mut self, who: &'static str) -> Result<i64, RuntimeError> {
        self.stack.pop().ok_or(RuntimeError::StackUnderflow(who))
    }

    /// Execute every op in order. Arithmetic wraps rather than panicking, because
    /// a "for fun" language should be hard to crash by accident.
    pub fn run(&mut self, program: &[Op]) -> Result<(), RuntimeError> {
        for op in program {
            match op {
                Op::Push(n) => self.stack.push(*n),
                Op::Add => {
                    let b = self.pop("+")?;
                    let a = self.pop("+")?;
                    self.stack.push(a.wrapping_add(b));
                }
                Op::Sub => {
                    let b = self.pop("-")?;
                    let a = self.pop("-")?;
                    self.stack.push(a.wrapping_sub(b));
                }
                Op::Mul => {
                    let b = self.pop("*")?;
                    let a = self.pop("*")?;
                    self.stack.push(a.wrapping_mul(b));
                }
                Op::Div => {
                    let b = self.pop("/")?;
                    let a = self.pop("/")?;
                    if b == 0 {
                        return Err(RuntimeError::DivideByZero);
                    }
                    self.stack.push(a.wrapping_div(b));
                }
                Op::Dup => {
                    let a = self.pop("dup")?;
                    self.stack.push(a);
                    self.stack.push(a);
                }
                Op::Drop => {
                    self.pop("drop")?;
                }
                Op::Swap => {
                    let b = self.pop("swap")?;
                    let a = self.pop("swap")?;
                    self.stack.push(b);
                    self.stack.push(a);
                }
                Op::Over => {
                    let b = self.pop("over")?;
                    let a = self.pop("over")?;
                    self.stack.push(a);
                    self.stack.push(b);
                    self.stack.push(a);
                }
                Op::Print => {
                    let a = self.pop(".")?;
                    self.output.push_str(&a.to_string());
                    self.output.push('\n');
                }
                Op::Emit => {
                    let a = self.pop(",")?;
                    // Out-of-range code points become the replacement char rather
                    // than aborting the program.
                    let c = u32::try_from(a)
                        .ok()
                        .and_then(char::from_u32)
                        .unwrap_or('\u{FFFD}');
                    self.output.push(c);
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
