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
pub mod transpile;
pub mod zone;
pub mod zones;

use cipher::CipherError;
use core::{Op, RuntimeError, Vm};
use std::fmt;
use std::path::{Path, PathBuf};

use zone::ZoneError;
use zones::grid::Grid;
use zones::lambda::LambdaError;
use zones::stack::StackError;

/// The conventional keyfile name. The CLI discovers it by walking up from a
/// program's directory, so an encoded `.wild` file only runs where this file
/// (carried between your own repos) is present.
pub const KEYFILE_NAME: &str = ".wildkey";

/// Walk up from `start_dir` looking for a [`KEYFILE_NAME`] file, returning the
/// first one found. This is what makes encoded programs "repo-bound": without
/// the keyfile somewhere above the program, the right key can't be found.
pub fn discover_keyfile(start_dir: &Path) -> Option<PathBuf> {
    let mut dir = Some(start_dir);
    while let Some(d) = dir {
        let candidate = d.join(KEYFILE_NAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        dir = d.parent();
    }
    None
}

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
    Lambda(LambdaError),
    Runtime(RuntimeError),
    /// The encoded outer layer failed to decode (e.g. wrong key).
    Cipher(CipherError),
    /// The program uses a feature the transpiler can't statically reproduce
    /// (cross-zone source rewriting). Carries a short reason.
    Untranspilable(&'static str),
}

impl fmt::Display for WildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WildError::Zone(e) => write!(f, "{e}"),
            WildError::UnknownDialect { kind, line } => {
                write!(f, "error: unknown dialect `{kind}` for zone on line {line}")
            }
            WildError::Stack(e) => write!(f, "{e}"),
            WildError::Lambda(e) => write!(f, "{e}"),
            WildError::Runtime(e) => write!(f, "{e}"),
            WildError::Cipher(e) => write!(f, "{e}"),
            WildError::Untranspilable(why) => write!(f, "error: cannot transpile: {why}"),
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
impl From<LambdaError> for WildError {
    fn from(e: LambdaError) -> Self {
        WildError::Lambda(e)
    }
}
impl From<RuntimeError> for WildError {
    fn from(e: RuntimeError) -> Self {
        WildError::Runtime(e)
    }
}

/// Compile one zone's `body` (given its `kind` and source `line`) to a segment.
fn compile_zone(kind: &str, body: &str, line: usize) -> Result<Segment, WildError> {
    Ok(match kind {
        "stack" => Segment::Ops(zones::stack::lower(body, line)?),
        "grid" => Segment::Grid(Grid::parse(body)),
        "prose" => Segment::Ops(zones::prose::lower(body)),
        "lambda" => Segment::Ops(zones::lambda::lower(body)?),
        other => {
            return Err(WildError::UnknownDialect {
                kind: other.to_string(),
                line,
            })
        }
    })
}

/// Statically compile a program into an ordered list of segments, from the
/// original source. This is the view the transpiler uses; note it does **not**
/// reflect any runtime cross-zone rewriting (see [`core::Op::Poke`]).
///
/// Segments execute in source order and share one stack, so values pushed in an
/// earlier zone (even a different dialect) are visible to a later one.
pub fn compile(source: &str) -> Result<Vec<Segment>, WildError> {
    zone::split(source)?
        .into_iter()
        .map(|z| compile_zone(&z.kind, &z.body, z.line))
        .collect()
}

/// Upper bound on total zone executions, so a `warp` loop that never reaches an
/// out-of-range target fails cleanly instead of hanging forever.
const ZONE_STEP_LIMIT: u64 = 1_000_000;

/// Compile and run plaintext source, returning everything it printed.
///
/// Execution is driven by a zone **program counter**, not a straight pass:
/// `warp` makes a chosen zone run next (a computed goto across zones), and a
/// zone is recompiled from its current — possibly `poke`-rewritten — source each
/// time control reaches it. All zones share one [`Vm`] (stack, output, source).
pub fn run_source(source: &str) -> Result<String, WildError> {
    let zones = zone::split(source)?;
    let kinds: Vec<String> = zones.iter().map(|z| z.kind.clone()).collect();
    let lines: Vec<usize> = zones.iter().map(|z| z.line).collect();
    let sources: Vec<Vec<char>> = zones.iter().map(|z| z.body.chars().collect()).collect();

    let mut vm = Vm::new();
    vm.set_sources(sources);
    let count = vm.zone_count();

    let mut pc = 0usize;
    let mut steps = 0u64;
    while pc < count {
        steps += 1;
        if steps > ZONE_STEP_LIMIT {
            return Err(WildError::Runtime(RuntimeError::ZoneStepLimit));
        }
        let body = vm.zone_source(pc);
        match compile_zone(&kinds[pc], &body, lines[pc])? {
            Segment::Ops(ops) => vm.run_ops(&ops)?,
            Segment::Grid(grid) => grid.run(&mut vm)?,
        }
        match vm.take_warp() {
            // A valid target jumps; an out-of-range one halts the program.
            Some(t) if t >= 0 && (t as usize) < count => pc = t as usize,
            Some(_) => break,
            None => pc += 1,
        }
    }
    Ok(vm.output().to_string())
}

/// Peel off the encoded outer layer if present, returning plaintext source.
///
/// If `contents` is wrapped (starts with the cipher [`MAGIC`](cipher::MAGIC)
/// marker) it is decoded with `key`; otherwise it is returned unchanged.
fn unwrap_source(contents: &str, key: &str) -> Result<String, WildError> {
    if cipher::looks_encoded(contents) {
        Ok(cipher::decode(contents, key)?)
    } else {
        Ok(contents.to_string())
    }
}

/// Run file contents, transparently peeling off the encoded outer layer first.
///
/// This is the entry point the CLI uses so encoded and plain `.wild` files Just
/// Work.
pub fn run_program(contents: &str, key: &str) -> Result<String, WildError> {
    run_source(&unwrap_source(contents, key)?)
}

/// Transpile plaintext source to a self-contained Python program.
///
/// Fails with [`WildError::Untranspilable`] if the program rewrites its own
/// source (`poke`), which a static Python program cannot reproduce.
pub fn transpile(source: &str) -> Result<String, WildError> {
    let segments = compile(source)?;
    transpile::to_python(&segments)
}

/// Transpile file contents to Python, peeling off the encoded layer first.
pub fn transpile_program(contents: &str, key: &str) -> Result<String, WildError> {
    transpile(&unwrap_source(contents, key)?)
}
