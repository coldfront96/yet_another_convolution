//! The transpiler backend — a second backend that emits Python instead of
//! interpreting.
//!
//! The same compiled [`Segment`](crate::Segment)s that the VM runs are turned
//! into a self-contained Python program that, when executed, produces identical
//! output. Linear segments become straight-line Python stack operations. Grid
//! segments embed their data and call a faithful Python port of the 2D
//! interpreter — transpiling dynamic, direction-driven Befunge to static code
//! is impractical, so the runtime is embedded instead.
//!
//! The generated program opens with a small fixed runtime (the prelude); the
//! grid half of the runtime is only included when the program actually contains
//! a grid, so nothing unused is emitted.

use crate::core::Op;
use crate::zones::grid::Grid;
use crate::Segment;

/// Emit a complete, runnable Python program for the given segments.
pub fn to_python(segments: &[Segment]) -> String {
    let needs_grid = segments.iter().any(|s| matches!(s, Segment::Grid(_)));

    let mut out = String::new();
    out.push_str(BASE_PRELUDE);
    if needs_grid {
        out.push_str(GRID_PRELUDE);
    }
    out.push('\n');

    for (i, segment) in segments.iter().enumerate() {
        out.push_str(&format!("# --- segment {} ---\n", i + 1));
        match segment {
            Segment::Ops(ops) => {
                for op in ops {
                    for line in op_lines(op) {
                        out.push_str(&line);
                        out.push('\n');
                    }
                }
            }
            Segment::Grid(grid) => {
                out.push_str(&grid_call(grid));
                out.push('\n');
            }
        }
    }

    out
}

/// The Python lines for one linear op. Temporaries `a`/`b` are reused module
/// globals — harmless, since every op fully consumes them.
fn op_lines(op: &Op) -> Vec<String> {
    let lines: &[&str] = match op {
        Op::Push(n) => return vec![format!("_push({n})")],
        Op::Add => &["b = _pop()", "a = _pop()", "_push(_w(a + b))"],
        Op::Sub => &["b = _pop()", "a = _pop()", "_push(_w(a - b))"],
        Op::Mul => &["b = _pop()", "a = _pop()", "_push(_w(a * b))"],
        Op::Div => &[
            "b = _pop()",
            "a = _pop()",
            "if b == 0:",
            "    sys.stderr.write('runtime error: division by zero\\n'); sys.exit(1)",
            "_push(_w(_trunc_div(a, b)))",
        ],
        Op::Dup => &["a = _pop()", "_push(a)", "_push(a)"],
        Op::Drop => &["_pop()"],
        Op::Swap => &["b = _pop()", "a = _pop()", "_push(b)", "_push(a)"],
        Op::Over => &["b = _pop()", "a = _pop()", "_push(a)", "_push(b)", "_push(a)"],
        Op::Print => &["_printnum(_pop())"],
        Op::Emit => &["_emit(_pop())"],
    };
    lines.iter().map(|s| s.to_string()).collect()
}

/// A call that runs an embedded grid as data.
fn grid_call(grid: &Grid) -> String {
    let rows: Vec<String> = grid.rows().iter().map(|r| py_str(r)).collect();
    format!("_run_grid([{}])", rows.join(", "))
}

/// Encode `s` as a safe double-quoted Python string literal.
fn py_str(s: &str) -> String {
    let mut out = String::from('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The base runtime: a stack, 64-bit wrapping, checked pop, output helpers.
const BASE_PRELUDE: &str = r##"import sys

_stack = []

def _w(n):
    n &= (1 << 64) - 1
    return n - (1 << 64) if n >= (1 << 63) else n

def _push(n):
    _stack.append(n)

def _pop():
    if not _stack:
        sys.stderr.write("runtime error: stack underflow\n")
        sys.exit(1)
    return _stack.pop()

def _trunc_div(a, b):
    q = abs(a) // abs(b)
    return -q if (a < 0) != (b < 0) else q

def _emit(n):
    if 0 <= n <= 0x10FFFF and not (0xD800 <= n <= 0xDFFF):
        sys.stdout.write(chr(n))
    else:
        sys.stdout.write("�")

def _printnum(n):
    sys.stdout.write(str(n) + "\n")
"##;

/// The grid half of the runtime: forgiving pop, truncating remainder, and a
/// faithful port of the 2D interpreter from `zones/grid.rs`.
const GRID_PRELUDE: &str = r##"
def _popz():
    return _stack.pop() if _stack else 0

def _trunc_rem(a, b):
    return a - b * _trunc_div(a, b)

def _run_grid(rows):
    h = len(rows)
    if h == 0:
        return
    w = len(rows[0])
    if w == 0:
        return
    cells = [list(r) for r in rows]
    x = y = 0
    dx, dy = 1, 0
    string_mode = False
    steps = 0
    while True:
        steps += 1
        if steps > 5000000:
            sys.stderr.write("runtime error: step limit exceeded (is a grid missing `@`?)\n")
            sys.exit(1)
        c = cells[y][x]
        if string_mode:
            if c == '"':
                string_mode = False
            else:
                _push(ord(c))
        elif '0' <= c <= '9':
            _push(ord(c) - ord('0'))
        elif c == '+':
            b = _popz(); a = _popz(); _push(_w(a + b))
        elif c == '-':
            b = _popz(); a = _popz(); _push(_w(a - b))
        elif c == '*':
            b = _popz(); a = _popz(); _push(_w(a * b))
        elif c == '/':
            b = _popz(); a = _popz(); _push(0 if b == 0 else _w(_trunc_div(a, b)))
        elif c == '%':
            b = _popz(); a = _popz(); _push(0 if b == 0 else _w(_trunc_rem(a, b)))
        elif c == '!':
            _push(1 if _popz() == 0 else 0)
        elif c == '`':
            b = _popz(); a = _popz(); _push(1 if a > b else 0)
        elif c == '>':
            dx, dy = 1, 0
        elif c == '<':
            dx, dy = -1, 0
        elif c == '^':
            dx, dy = 0, -1
        elif c == 'v':
            dx, dy = 0, 1
        elif c == '_':
            dx, dy = (1, 0) if _popz() == 0 else (-1, 0)
        elif c == '|':
            dx, dy = (0, 1) if _popz() == 0 else (0, -1)
        elif c == ':':
            v = _popz(); _push(v); _push(v)
        elif c == '\\':
            b = _popz(); a = _popz(); _push(b); _push(a)
        elif c == '$':
            _popz()
        elif c == '.':
            sys.stdout.write(str(_popz()) + " ")
        elif c == ',':
            _emit(_popz())
        elif c == '"':
            string_mode = True
        elif c == '#':
            x = (x + dx) % w; y = (y + dy) % h
        elif c == 'g':
            yy = _popz(); xx = _popz(); _push(ord(cells[yy % h][xx % w]))
        elif c == 'p':
            yy = _popz(); xx = _popz(); val = _popz()
            cells[yy % h][xx % w] = chr(val) if 0 <= val <= 0x10FFFF and not (0xD800 <= val <= 0xDFFF) else ' '
        elif c == '@':
            return
        x = (x + dx) % w; y = (y + dy) % h
"##;
