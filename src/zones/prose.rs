//! The `prose` dialect — code disguised as English sentences.
//!
//! A prose zone reads like ordinary narrative text, but recognized words drive
//! the shared stack. The model is **postfix**: describe the operands first, then
//! name the action — because the machine needs values on the stack before an
//! operation can consume them. So you write
//!
//! ```text
//! Take six and seven and multiply them, then show the answer.
//! ```
//!
//! which pushes 6, pushes 7, multiplies, and prints 42. Every word that isn't a
//! number or an action word is **narrative filler** and is ignored on purpose —
//! that is exactly what lets the code read like English.
//!
//! Numbers are spelled out (`forty-two`, `one hundred twenty three`), optionally
//! prefixed with `negative`. They do **not** use the word "and" (that stays a
//! plain conjunction), so `six and seven` is two numbers, not one.
//!
//! Action words (case-insensitive), each lowering to one core op:
//! - `add` / `plus`                 -> Add
//! - `subtract` / `minus` / `less`  -> Sub
//! - `multiply` / `times`           -> Mul
//! - `divide`                       -> Div
//! - `duplicate` / `copy`           -> Dup
//! - `drop` / `forget`              -> Drop
//! - `swap` / `exchange`            -> Swap
//! - `print` / `show` / `display`   -> Print (as a number)
//! - `emit` / `say` / `speak`       -> Emit (as a character)
//!
//! Lowering is infallible: any unrecognized word is simply filler. If an action
//! finds too few operands, that surfaces later as a normal runtime stack
//! underflow from the shared VM.

use crate::core::Op;

/// Translate a prose-zone body into core ops.
pub fn lower(body: &str) -> Vec<Op> {
    let tokens = tokenize(body);
    let mut ops = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        // A spelled-out number (possibly several words) pushes a value.
        if let Some((value, consumed)) = parse_number(&tokens[i..]) {
            ops.push(Op::Push(value));
            i += consumed;
            continue;
        }
        // An action word lowers to an op; anything else is narrative filler.
        if let Some(op) = action_word(&tokens[i]) {
            ops.push(op);
        }
        i += 1;
    }

    ops
}

/// Lower-case alphanumeric words, splitting on whitespace and hyphens and
/// dropping all punctuation, so `forty-two,` becomes `forty` and `two`.
fn tokenize(body: &str) -> Vec<String> {
    body.split(|c: char| c.is_whitespace() || c == '-')
        .map(|word| {
            word.chars()
                .filter(|c| c.is_alphanumeric())
                .flat_map(|c| c.to_lowercase())
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect()
}

/// Map an action word to its core op, or `None` if it isn't one.
fn action_word(word: &str) -> Option<Op> {
    Some(match word {
        "add" | "plus" => Op::Add,
        "subtract" | "minus" | "less" => Op::Sub,
        "multiply" | "times" => Op::Mul,
        "divide" => Op::Div,
        "duplicate" | "copy" => Op::Dup,
        "drop" | "forget" => Op::Drop,
        "swap" | "exchange" => Op::Swap,
        "print" | "show" | "display" => Op::Print,
        "emit" | "say" | "speak" => Op::Emit,
        _ => return None,
    })
}

/// The value of a "small" number word (ones, teens, tens), or `None`.
fn small_value(word: &str) -> Option<i64> {
    Some(match word {
        "zero" => 0,
        "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        "eleven" => 11,
        "twelve" => 12,
        "thirteen" => 13,
        "fourteen" => 14,
        "fifteen" => 15,
        "sixteen" => 16,
        "seventeen" => 17,
        "eighteen" => 18,
        "nineteen" => 19,
        "twenty" => 20,
        "thirty" => 30,
        "forty" => 40,
        "fifty" => 50,
        "sixty" => 60,
        "seventy" => 70,
        "eighty" => 80,
        "ninety" => 90,
        _ => return None,
    })
}

/// Parse a spelled-out number at the start of `tokens`.
///
/// Returns the value and how many tokens it consumed, or `None` if the first
/// token doesn't begin a number. Supports an optional leading `negative` and
/// `hundred` / `thousand` groupings.
fn parse_number(tokens: &[String]) -> Option<(i64, usize)> {
    let mut idx = 0;
    let negative = tokens.first().map(String::as_str) == Some("negative");
    if negative {
        idx = 1;
    }

    let mut total: i64 = 0;
    let mut current: i64 = 0;
    let mut saw_number = false;

    while idx < tokens.len() {
        let word = tokens[idx].as_str();
        if let Some(v) = small_value(word) {
            current += v;
        } else if word == "hundred" {
            current = if current == 0 { 1 } else { current } * 100;
        } else if word == "thousand" {
            total += if current == 0 { 1 } else { current } * 1000;
            current = 0;
        } else {
            break;
        }
        saw_number = true;
        idx += 1;
    }

    if !saw_number {
        // A lone "negative" with no number after it is just filler.
        return None;
    }

    let value = total + current;
    Some((if negative { -value } else { value }, idx))
}
