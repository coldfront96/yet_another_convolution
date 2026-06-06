//! The `grid` dialect — Convolution's 2D, visual, self-modifying gimmick.
//!
//! Borrowed from Befunge. The body is a rectangular grid of characters. An
//! instruction pointer (IP) starts at the top-left moving right, and each cell
//! it lands on is an instruction. Crucially, **direction is runtime state** —
//! `> < ^ v` steer the IP and `_ |` steer it based on a popped value — which is
//! exactly why a grid can't be flattened into a linear op list and instead runs
//! its own interpreter against the shared [`Vm`].
//!
//! The `g`/`p` instructions read and write cells *while the program runs*, so a
//! grid is genuinely self-modifying code.
//!
//! Instruction reference:
//! - `0`-`9`      push that digit
//! - `+ - * / %`  arithmetic (pop b, pop a; forgiving: empty pops as 0,
//!   division/mod by zero yields 0)
//! - `!`          logical not (0 -> 1, else 0)
//! - `` ` ``      greater-than (pop b, a; push 1 if a > b else 0)
//! - `> < ^ v`    set IP direction
//! - `_`          pop; go right if 0, else left
//! - `|`          pop; go down if 0, else up
//! - `:`          duplicate top
//! - `\`          swap top two
//! - `$`          drop top
//! - `.`          pop and print as a number followed by a space
//! - `,`          pop and print as a Unicode character
//! - `"`          toggle string mode (push each char's code until the next `"`)
//! - `#`          trampoline: skip the next cell
//! - `g`          pop y, pop x; push the code of the char at (x, y)
//! - `p`          pop y, pop x, pop v; write char v into cell (x, y)
//! - `@`          stop
//! - space        no-op (any unknown char is also a no-op)
//!
//! The IP wraps around the edges (the grid is a torus). `}` cannot appear in a
//! grid body, since it would close the zone.

use crate::core::{RuntimeError, Vm};

/// Generous cap so a runaway grid (one missing its `@`) fails loudly instead of
/// hanging the CLI. Real programs terminate via `@` long before this.
const MAX_STEPS: u64 = 5_000_000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dir {
    Right,
    Left,
    Up,
    Down,
}

/// A parsed grid program. Parsing is infallible: any character is a valid cell.
#[derive(Debug, PartialEq, Eq)]
pub struct Grid {
    cells: Vec<Vec<char>>,
    width: usize,
    height: usize,
}

impl Grid {
    /// Build a grid from a zone body, preserving spaces (they position the IP).
    /// A single leading blank line (the newline right after the `grid` keyword)
    /// and any trailing blank lines are dropped; remaining rows are padded to a
    /// common width with spaces.
    pub fn parse(body: &str) -> Grid {
        let mut lines: Vec<&str> = body.split('\n').collect();
        // Drop one leading empty line (from `{grid\n...`).
        if matches!(lines.first(), Some(l) if l.trim().is_empty()) {
            lines.remove(0);
        }
        // Drop trailing blank lines.
        while matches!(lines.last(), Some(l) if l.trim().is_empty()) {
            lines.pop();
        }

        let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        let cells: Vec<Vec<char>> = lines
            .iter()
            .map(|l| {
                let mut row: Vec<char> = l.chars().collect();
                row.resize(width, ' ');
                row
            })
            .collect();
        let height = cells.len();

        Grid {
            cells,
            width,
            height,
        }
    }

    /// The normalized grid rows (padded to a common width). Used by the
    /// transpiler to embed the grid as data in the generated program.
    pub fn rows(&self) -> Vec<String> {
        self.cells.iter().map(|row| row.iter().collect()).collect()
    }

    /// Run the grid against the shared VM. The grid is copied internally so the
    /// stored program is reusable even though `p` mutates cells at runtime.
    pub fn run(&self, vm: &mut Vm) -> Result<(), RuntimeError> {
        if self.width == 0 || self.height == 0 {
            return Ok(());
        }
        let mut cells = self.cells.clone();
        let w = self.width as i64;
        let h = self.height as i64;

        let mut x: i64 = 0;
        let mut y: i64 = 0;
        let mut dir = Dir::Right;
        let mut string_mode = false;
        let mut steps: u64 = 0;

        loop {
            steps += 1;
            if steps > MAX_STEPS {
                return Err(RuntimeError::StepLimit);
            }

            let c = cells[y as usize][x as usize];

            if string_mode {
                if c == '"' {
                    string_mode = false;
                } else {
                    vm.push(c as i64);
                }
            } else {
                match c {
                    '0'..='9' => vm.push(c as i64 - '0' as i64),
                    '+' => {
                        let b = vm.pop_or_zero();
                        let a = vm.pop_or_zero();
                        vm.push(a.wrapping_add(b));
                    }
                    '-' => {
                        let b = vm.pop_or_zero();
                        let a = vm.pop_or_zero();
                        vm.push(a.wrapping_sub(b));
                    }
                    '*' => {
                        let b = vm.pop_or_zero();
                        let a = vm.pop_or_zero();
                        vm.push(a.wrapping_mul(b));
                    }
                    '/' => {
                        let b = vm.pop_or_zero();
                        let a = vm.pop_or_zero();
                        vm.push(if b == 0 { 0 } else { a.wrapping_div(b) });
                    }
                    '%' => {
                        let b = vm.pop_or_zero();
                        let a = vm.pop_or_zero();
                        vm.push(if b == 0 { 0 } else { a.wrapping_rem(b) });
                    }
                    '!' => {
                        let v = vm.pop_or_zero();
                        vm.push(if v == 0 { 1 } else { 0 });
                    }
                    '`' => {
                        let b = vm.pop_or_zero();
                        let a = vm.pop_or_zero();
                        vm.push(if a > b { 1 } else { 0 });
                    }
                    '>' => dir = Dir::Right,
                    '<' => dir = Dir::Left,
                    '^' => dir = Dir::Up,
                    'v' => dir = Dir::Down,
                    '_' => {
                        let v = vm.pop_or_zero();
                        dir = if v == 0 { Dir::Right } else { Dir::Left };
                    }
                    '|' => {
                        let v = vm.pop_or_zero();
                        dir = if v == 0 { Dir::Down } else { Dir::Up };
                    }
                    ':' => {
                        let v = vm.pop_or_zero();
                        vm.push(v);
                        vm.push(v);
                    }
                    '\\' => {
                        let b = vm.pop_or_zero();
                        let a = vm.pop_or_zero();
                        vm.push(b);
                        vm.push(a);
                    }
                    '$' => {
                        vm.pop_or_zero();
                    }
                    '.' => {
                        let v = vm.pop_or_zero();
                        vm.write_str(&format!("{v} "));
                    }
                    ',' => {
                        let v = vm.pop_or_zero();
                        vm.emit_char(v);
                    }
                    '"' => string_mode = true,
                    '#' => advance(&mut x, &mut y, dir, w, h), // trampoline: extra step
                    'g' => {
                        let yy = vm.pop_or_zero();
                        let xx = vm.pop_or_zero();
                        vm.push(cell_get(&cells, xx, yy, w, h));
                    }
                    'p' => {
                        let yy = vm.pop_or_zero();
                        let xx = vm.pop_or_zero();
                        let v = vm.pop_or_zero();
                        cell_put(&mut cells, xx, yy, w, h, v);
                    }
                    '@' => break,
                    _ => {} // space and anything unknown: no-op
                }
            }

            advance(&mut x, &mut y, dir, w, h);
        }

        Ok(())
    }
}

/// Move the IP one cell in `dir`, wrapping toroidally.
fn advance(x: &mut i64, y: &mut i64, dir: Dir, w: i64, h: i64) {
    match dir {
        Dir::Right => *x = (*x + 1).rem_euclid(w),
        Dir::Left => *x = (*x - 1).rem_euclid(w),
        Dir::Down => *y = (*y + 1).rem_euclid(h),
        Dir::Up => *y = (*y - 1).rem_euclid(h),
    }
}

/// Read the code of the char at (x, y), wrapping coordinates into the grid.
fn cell_get(cells: &[Vec<char>], x: i64, y: i64, w: i64, h: i64) -> i64 {
    let xi = x.rem_euclid(w) as usize;
    let yi = y.rem_euclid(h) as usize;
    cells[yi][xi] as i64
}

/// Write char value `v` into cell (x, y), wrapping coordinates into the grid.
fn cell_put(cells: &mut [Vec<char>], x: i64, y: i64, w: i64, h: i64, v: i64) {
    let xi = x.rem_euclid(w) as usize;
    let yi = y.rem_euclid(h) as usize;
    let ch = u32::try_from(v).ok().and_then(char::from_u32).unwrap_or(' ');
    cells[yi][xi] = ch;
}
