//! Convolution: a deliberately baroque, polyglot esolang.
//!
//! The pipeline is one straight line, each stage independently testable:
//!
//! ```text
//! source (.wild)
//!   -> zone::split        carve into polyglot zones
//!   -> <dialect> compile  each zone -> a Segment (linear ops or a grid)
//!   -> core::Vm           run every segment on one shared stack + output
//!   -> output string
//! ```
//!
//! The surface is convoluted on purpose; the core it all reduces to is tiny.
//! Future gimmicks (a `prose` dialect, a `lambda` dialect, an outer encoding
//! layer) bolt onto this same pipeline without changing its shape.

pub mod cipher;
pub mod core;
pub mod zone;
pub mod zones;

use cipher::CipherError;
use core::{Op, RuntimeError, Vm};
use std::fmt;

use zone::ZoneError;
use zones::grid::Grid;
use zones::stack::StackError;

/// One compiled zone. Linear dialects produce [`Segment::Ops`]; the 2D `grid`
/// dialect produces [`Segment::Grid`], which carries its own interpreter. Both
/// run against the same shared [`Vm`], in source order.
#[derive(Debug, PartialEq, Eq)]
pub enum Segment {
    Ops(Vec<Op>),
    Grid(Grid),
}

/// Any failure across the whole pipeline.
#[derive(Debug, PartialEq, Eq)]
pub enum WildError {
    Zone(ZoneError),
    /// A zone named a dialect we don't have.
    UnknownDialect { kind: String, line: usize },
    Stack(StackError),
    Runtime(RuntimeError),
    /// The encoded outer layer failed to decode (e.g. wrong key).
    Cipher(CipherError),
}

impl fmt::Display for WildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WildError::Zone(e) => write!(f, "{e}"),
            WildError::UnknownDialect { kind, line } => {
                write!(f, "error: unknown dialect `{kind}` for zone on line {line}")
            }
            WildError::Stack(e) => write!(f, "{e}"),
            WildError::Runtime(e) => write!(f, "{e}"),
            WildError::Cipher(e) => write!(f, "{e}"),
        }
    }
}

impl From<ZoneError> for WildError {
    fn from(e: ZoneError) -> Self {
        WildError::Zone(e)
    }
}
impl From<CipherError> for WildError {
    fn from(e: CipherError) -> Self {
        WildError::Cipher(e)
    }
}
impl From<StackError> for WildError {
    fn from(e: StackError) -> Self {
        WildError::Stack(e)
    }
}
impl From<RuntimeError> for WildError {
    fn from(e: RuntimeError) -> Self {
        WildError::Runtime(e)
    }
}

/// Compile a Convolution program into an ordered list of segments.
///
/// Segments execute in source order and share one stack, so values pushed in an
/// earlier zone (even a different dialect) are visible to a later one — that
/// cross-zone flow is what lets the dialects genuinely talk to each other.
pub fn compile(source: &str) -> Result<Vec<Segment>, WildError> {
    let zones = zone::split(source)?;
    let mut segments = Vec::new();
    for z in zones {
        let segment = match z.kind.as_str() {
            "stack" => Segment::Ops(zones::stack::lower(&z.body, z.line)?),
            "grid" => Segment::Grid(Grid::parse(&z.body)),
            "prose" => Segment::Ops(zones::prose::lower(&z.body)),
            other => {
                return Err(WildError::UnknownDialect {
                    kind: other.to_string(),
                    line: z.line,
                })
            }
        };
        segments.push(segment);
    }
    Ok(segments)
}

/// Compile and run plaintext source, returning everything it printed.
pub fn run_source(source: &str) -> Result<String, WildError> {
    let segments = compile(source)?;
    let mut vm = Vm::new();
    for segment in &segments {
        match segment {
            Segment::Ops(ops) => vm.run_ops(ops)?,
            Segment::Grid(grid) => grid.run(&mut vm)?,
        }
    }
    Ok(vm.output().to_string())
}

/// Run file contents, transparently peeling off the encoded outer layer first.
///
/// If `contents` is wrapped (starts with the cipher [`MAGIC`](cipher::MAGIC)
/// marker) it is decoded with `key`; otherwise it runs as plaintext. This is the
/// entry point the CLI uses so encoded and plain `.wild` files Just Work.
pub fn run_program(contents: &str, key: &str) -> Result<String, WildError> {
    let source = if cipher::looks_encoded(contents) {
        cipher::decode(contents, key)?
    } else {
        contents.to_string()
    };
    run_source(&source)
}
