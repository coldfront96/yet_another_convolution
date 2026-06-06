//! The `stack` dialect.
//!
//! A Forth/Brainfuck-flavored surface where everything is a whitespace-separated
//! word that manipulates a shared stack. This is the first dialect Convolution
//! ships with; it lowers directly to core [`Op`]s.
//!
//! Words:
//! - integer literal  -> push it
//! - `+ - * /`        -> arithmetic
//! - `dup drop swap over` -> stack shuffles
//! - `.`              -> print top as a number
//! - `,`              -> emit top as a character
//! - `poke`           -> pop `z i c`; rewrite char `i` of a later zone `z` to `c`
//! - `#` starts a comment that runs to end of line

use crate::core::Op;
use std::fmt;

/// A problem found while lowering a stack-zone body.
#[derive(Debug, PartialEq, Eq)]
pub enum StackError {
    /// A word that isn't a known op and isn't a valid integer.
    UnknownWord { word: String, line: usize },
}

impl fmt::Display for StackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StackError::UnknownWord { word, line } => {
                write!(f, "stack error: unknown word `{word}` on line {line}")
            }
        }
    }
}

/// Translate a stack-zone body into core ops.
///
/// `line_base` is the source line on which the zone opened, so error messages
/// point back into the original file rather than into the extracted body.
pub fn lower(body: &str, line_base: usize) -> Result<Vec<Op>, StackError> {
    let mut ops = Vec::new();

    for (offset, raw_line) in body.lines().enumerate() {
        let line = line_base + offset;
        // Strip line comments.
        let code = match raw_line.split_once('#') {
            Some((before, _)) => before,
            None => raw_line,
        };

        for word in code.split_whitespace() {
            let op = match word {
                "+" => Op::Add,
                "-" => Op::Sub,
                "*" => Op::Mul,
                "/" => Op::Div,
                "dup" => Op::Dup,
                "drop" => Op::Drop,
                "swap" => Op::Swap,
                "over" => Op::Over,
                "." => Op::Print,
                "," => Op::Emit,
                "poke" => Op::Poke,
                _ => match word.parse::<i64>() {
                    Ok(n) => Op::Push(n),
                    Err(_) => {
                        return Err(StackError::UnknownWord {
                            word: word.to_string(),
                            line,
                        })
                    }
                },
            };
            ops.push(op);
        }
    }

    Ok(ops)
}
