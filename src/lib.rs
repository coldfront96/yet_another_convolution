//! Convolution: a deliberately baroque, polyglot esolang.
//!
//! The pipeline is one straight line, each stage independently testable:
//!
//! ```text
//! source (.wild)
//!   -> zone::split        carve into polyglot zones
//!   -> <dialect>::lower   each zone -> core ops
//!   -> core::Vm::run      execute the flat op stream
//!   -> output string
//! ```
//!
//! The surface is convoluted on purpose; the core it all reduces to is tiny.
//! Future gimmicks (a 2D `grid` dialect, a `prose` dialect, self-modifying
//! zones, an outer encoding layer) bolt onto this same pipeline without
//! changing its shape.

pub mod core;
pub mod zone;
pub mod zones;

use core::{RuntimeError, Vm};
use std::fmt;

use zone::ZoneError;
use zones::stack::StackError;

/// Any failure across the whole pipeline.
#[derive(Debug, PartialEq, Eq)]
pub enum WildError {
    Zone(ZoneError),
    /// A zone named a dialect we don't have.
    UnknownDialect { kind: String, line: usize },
    Stack(StackError),
    Runtime(RuntimeError),
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
        }
    }
}

impl From<ZoneError> for WildError {
    fn from(e: ZoneError) -> Self {
        WildError::Zone(e)
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

/// Compile a Convolution program down to a flat stream of core ops.
///
/// Zones execute in source order and share one stack, so values pushed in an
/// earlier zone are visible to a later one — that cross-zone flow is what will
/// eventually let different dialects talk to each other.
pub fn compile(source: &str) -> Result<Vec<core::Op>, WildError> {
    let zones = zone::split(source)?;
    let mut program = Vec::new();
    for z in zones {
        match z.kind.as_str() {
            "stack" => program.extend(zones::stack::lower(&z.body, z.line)?),
            other => {
                return Err(WildError::UnknownDialect {
                    kind: other.to_string(),
                    line: z.line,
                })
            }
        }
    }
    Ok(program)
}

/// Compile and run a program, returning everything it printed.
pub fn run_source(source: &str) -> Result<String, WildError> {
    let program = compile(source)?;
    let mut vm = Vm::new();
    vm.run(&program)?;
    Ok(vm.output().to_string())
}
