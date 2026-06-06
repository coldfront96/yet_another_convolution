//! The polyglot zone splitter.
//!
//! A Convolution program is a sequence of zones, each potentially written in a
//! different borrowed dialect. A zone is written `{kind  ...body... }`, where
//! `kind` selects which dialect parses the body. Today only the `stack` dialect
//! exists, but everything here is built so new kinds (`grid`, `prose`, `lambda`)
//! slot in without the splitter caring.

use std::fmt;

/// One parsed-out region of source, not yet lowered to core ops.
#[derive(Debug, PartialEq, Eq)]
pub struct Zone {
    /// The dialect keyword that followed the opening brace (e.g. `"stack"`).
    pub kind: String,
    /// The raw body between the kind keyword and the matching close brace.
    pub body: String,
    /// 1-based line where this zone opened, for friendlier errors.
    pub line: usize,
}

/// Something went wrong while carving the source into zones.
#[derive(Debug, PartialEq, Eq)]
pub enum ZoneError {
    /// A `{` was opened but never closed.
    Unterminated { line: usize },
    /// A `{` was not followed by a dialect keyword.
    MissingKind { line: usize },
    /// A `}` appeared with no matching open zone.
    StrayClose { line: usize },
    /// Text appeared between zones that isn't inside any zone.
    StrayText { line: usize },
}

impl fmt::Display for ZoneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZoneError::Unterminated { line } => {
                write!(f, "zone error: unterminated zone opened on line {line}")
            }
            ZoneError::MissingKind { line } => {
                write!(f, "zone error: `{{` on line {line} is missing a dialect keyword")
            }
            ZoneError::StrayClose { line } => {
                write!(f, "zone error: stray `}}` on line {line} with no open zone")
            }
            ZoneError::StrayText { line } => {
                write!(f, "zone error: text on line {line} is outside any zone")
            }
        }
    }
}

/// Split source into its top-level zones.
///
/// Braces nest: a zone body may itself contain `{...}` (future zones will use
/// this for nested control structures), so we match braces by depth rather than
/// stopping at the first `}`.
pub fn split(source: &str) -> Result<Vec<Zone>, ZoneError> {
    let mut zones = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    let mut line = 1;

    while i < chars.len() {
        let c = chars[i];
        match c {
            '\n' => {
                line += 1;
                i += 1;
            }
            c if c.is_whitespace() => {
                i += 1;
            }
            '{' => {
                let zone_line = line;
                i += 1; // consume '{'

                // Read the dialect keyword.
                while i < chars.len() && chars[i].is_whitespace() {
                    if chars[i] == '\n' {
                        line += 1;
                    }
                    i += 1;
                }
                let kind_start = i;
                while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '}' {
                    i += 1;
                }
                let kind: String = chars[kind_start..i].iter().collect();
                if kind.is_empty() {
                    return Err(ZoneError::MissingKind { line: zone_line });
                }

                // Capture the body up to the matching close brace, tracking depth.
                let body_start = i;
                let mut depth = 1;
                while i < chars.len() && depth > 0 {
                    match chars[i] {
                        '{' => depth += 1,
                        '}' => depth -= 1,
                        '\n' => line += 1,
                        _ => {}
                    }
                    if depth == 0 {
                        break;
                    }
                    i += 1;
                }
                if depth != 0 {
                    return Err(ZoneError::Unterminated { line: zone_line });
                }
                let body: String = chars[body_start..i].iter().collect();
                i += 1; // consume the closing '}'

                zones.push(Zone {
                    kind,
                    body,
                    line: zone_line,
                });
            }
            '}' => return Err(ZoneError::StrayClose { line }),
            '#' => {
                // A comment between zones: skip to end of line.
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            _ => return Err(ZoneError::StrayText { line }),
        }
    }

    Ok(zones)
}
