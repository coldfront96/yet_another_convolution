# Convolution

A deliberately baroque, polyglot, multi-zone esoteric programming language —
built for fun. The **surface is convoluted on purpose; the core it reduces to is
tiny.** Source files use the `.wild` extension.

This is the first vertical slice: a working interpreter for the `stack` dialect
running on the minimalist core. Every wild idea in the roadmap below bolts onto
this same pipeline.

## The pipeline

```text
source (.wild)
  -> zone::split        carve the program into polyglot zones
  -> <dialect> compile  each zone -> a Segment (linear ops, or a 2D grid)
  -> core::Vm           run every segment on one shared stack + output
  -> output
```

Convoluted to *use*, clean to *build* — each stage is independently testable.
Linear dialects (`stack`) compile to a flat list of core ops; richer dialects
(`grid`) carry their own interpreter — but every dialect drives the **same
shared stack and output buffer**, which is what makes the language genuinely
polyglot instead of a pile of unrelated mini-languages.

## Run it

```sh
cargo run -- examples/hello.wild        # prints: HI!
cargo run -- examples/arithmetic.wild
cargo run -- examples/hello_grid.wild   # prints: Hello, World!
cargo run -- examples/grid2d.wild       # a 2D path computing 12
cargo run -- examples/selfmod.wild      # self-inspecting grid
cargo run -- examples/secret.wild.enc   # an encoded program, run directly
cargo run -- examples/prose.wild        # an English sentence that prints 42
cargo run -- examples/prose_hello.wild  # a short story that prints HI!
cargo run -- examples/lambda.wild       # nested S-expressions -> 21
cargo run -- examples/lambda_hello.wild # Lisp that prints HI!
cargo run -- examples/selfrewrite.wild  # a zone rewrites a later zone -> 42
cargo run -- transpile examples/lambda.wild | python3   # transpile, then run
cargo test                              # the pipeline test suite
```

## The language today

A program is a sequence of **zones**. A zone is `{kind ...body... }`, where
`kind` picks the dialect that parses the body. Zones execute in source order and
**share one stack**, so a value pushed in one zone is visible to the next.

Currently the only dialect is `stack` — a Forth/Brainfuck-flavored surface:

| Word                | Effect                                   |
| ------------------- | ---------------------------------------- |
| integer             | push it onto the stack                   |
| `+` `-` `*` `/`     | pop two, push the result                 |
| `dup`               | `a -> a a`                               |
| `drop`              | `a ->`                                   |
| `swap`              | `a b -> b a`                             |
| `over`              | `a b -> a b a`                           |
| `.`                 | pop and print as a decimal number        |
| `,`                 | pop and print as a Unicode character     |
| `poke`              | pop `z i c`; rewrite a *later* zone's source (see below) |
| `# ...`             | comment to end of line                   |

```
{stack
  2 3 +   # -> 5
  dup *   # -> 25
  .       # print 25
}
```

### The `grid` dialect (2D / visual / self-modifying)

A Befunge-flavored 2D dialect. The body is a grid of characters; an instruction
pointer starts top-left moving right, and each cell it lands on is an
instruction. `> < ^ v` steer it, `_ |` steer it based on a popped value, and the
IP wraps around the edges. Because direction is *runtime* state, a grid can't be
flattened to linear ops — it runs its own interpreter.

| Cell            | Effect                                              |
| --------------- | --------------------------------------------------- |
| `0`-`9`         | push that digit                                     |
| `+ - * / %`     | arithmetic (empty pops as 0; ÷/% by 0 gives 0)      |
| `!`             | logical not                                         |
| `` ` ``         | greater-than: `a b -> (a>b)`                         |
| `> < ^ v`       | set direction                                       |
| `_`             | pop; right if 0, else left                          |
| `\|`            | pop; down if 0, else up                              |
| `: \ $`         | dup / swap / drop                                   |
| `.`             | pop and print as a number + space                   |
| `,`             | pop and print as a character                        |
| `"`             | toggle string mode (push char codes)                |
| `#`             | trampoline (skip next cell)                          |
| `g` / `p`       | get / put a cell at runtime — **self-modifying**    |
| `@`             | stop                                                |

```
{grid
>34*v        the IP walks right (3*4=12), turns down at v,
    .        prints 12, then stops
    @
}
```

The `g`/`p` instructions read and write the grid *while it runs*, so grids are
genuinely self-modifying code. (`}` can't appear in a grid body — it would close
the zone.)

### The `prose` dialect (code disguised as English)

A prose zone reads like narrative text, but recognized words drive the same
stack. It's **postfix** — describe the operands, *then* name the action, because
the machine needs values before an operation can consume them:

```
{prose
  Take six and seven and multiply them, then show the answer.   # prints 42
}
```

Numbers are spelled out (`forty-two`, `one hundred twenty three`, optionally
`negative ...`) and don't use the word "and". Action words: `add`/`plus`,
`subtract`/`minus`/`less`, `multiply`/`times`, `divide`, `duplicate`/`copy`,
`drop`/`forget`, `swap`/`exchange`, `print`/`show`/`display`,
`emit`/`say`/`speak`. **Every other word is narrative filler, ignored on
purpose** — that's what lets the code read like English.

### The `lambda` dialect (Lisp-like S-expressions)

A parenthesized, nestable surface. Evaluating an expression tree in post-order
is naturally postfix, so it lowers straight to core ops:

```
{lambda
  (print (* (+ 1 2) (- 10 3)))   ; (1+2) * (10-3) = 21
}
```

Forms: integer literals push themselves; `(+ ...)` / `(* ...)` / `(- ...)` /
`(/ ...)` are variadic folds (`(- x)` negates); `(print x)` and `(emit x)`
output a number or a character; `(poke z i c)` rewrites a later zone's source
(see below); `;` starts a comment. A lambda zone can also leave a value on the
stack (with no `print`) for a later zone to pick up.

### Cross-zone source rewriting (`poke`)

The grid's `g`/`p` let a zone rewrite *itself*. `poke` goes further: it lets one
zone reach across and rewrite the **source text of a later zone before it runs**.

`poke` pops three values — a zone index `z`, a character offset `i`, and a
character code `c` — and sets character `i` of zone `z`'s body to `c`. Because
edits must land *before* the target compiles, the interpreter compiles and runs
zones **one at a time** rather than all up front.

```
{stack 1 5 42 poke}   # write '*' (code 42) over offset 5 of zone 1's body
{stack 6 7 + . }      # LOOKS like 6 + 7 = 13... but prints 42
```

The second zone reads like it adds, but the first zone rewrote its `+` into a
`*` first. Edits to an already-run zone, or to an out-of-range zone, are
silently ignored — only zones not yet compiled can be changed. (`poke` is the
one *meta*-op; it manipulates source, not the stack's values.)

### The minimalist core

Everything lowers to ~12 primitive ops (`Push, Add, Sub, Mul, Div, Dup, Drop,
Swap, Over, Print, Emit`, plus the meta-op `Poke`). New dialects only need to
learn how to emit these.

## The encoded outer layer ("only my repos understand it")

A program can be wrapped so that on disk it's opaque base64 garbage. The CLI
auto-detects the wrapper and decodes it with a key before running, so encoded
and plain files Just Work:

```sh
# Wrap a program (writes the encoded form to a .enc file)
cargo run -- encode examples/hello_grid.wild > examples/secret.wild.enc

# Run it directly -- the CLI peels off the encoding first
cargo run -- run examples/secret.wild.enc        # -> Hello, World!

# Reveal the original source
cargo run -- decode examples/secret.wild.enc
```

A wrong key fails loudly (an inner tag is checked) rather than running garbage.

### Repo-bound: a keyfile you carry between *your* repos

The point isn't one secret key memorized in your head — it's a **keyfile you
own** that travels with you. Generate one, and encoded programs only run where
that keyfile is present:

```sh
cargo run -- keygen                       # writes a random .wildkey (gitignored)
cargo run -- encode prog.wild > prog.enc  # encodes with the discovered .wildkey
cargo run -- run prog.enc                 # finds .wildkey, decodes, runs
```

Drop the same `.wildkey` into another repo of yours and the encoded programs
work there too. Anyone who clones a repo **without** your keyfile just gets a
`wrong key` error — the language "breaks" for them, by design. `.wildkey` is
gitignored so you never commit it by accident, and `keygen` refuses to clobber
an existing one.

The key is resolved in this order (first match wins):

1. `--key <KEY>` — an explicit key on the command line
2. `--keyfile <PATH>` — read the key from a specific file
3. `$CONVOLUTION_KEY` — an environment variable
4. a `.wildkey` discovered by walking up from the program's directory
5. the built-in default (what the shipped `secret.wild.enc` example uses)

> This is *obfuscation*, not cryptography — a keystream XOR is exactly as strong
> as keeping the key secret and no stronger. Perfect for a "for fun" esolang;
> don't protect anything that actually matters with it.

## The transpiler backend (compile to Python)

The same compiled segments the VM runs can instead be emitted as a
self-contained Python program that produces identical output:

```sh
cargo run -- transpile examples/arithmetic.wild        # prints Python to stdout
cargo run -- transpile examples/arithmetic.wild | python3   # ...and run it
```

Linear dialects (`stack`/`prose`/`lambda`) become straight-line Python stack
operations; `grid` zones embed their data and call a faithful Python port of the
2D interpreter (transpiling dynamic Befunge to static code is impractical, so the
runtime is embedded). The generated program opens with a small fixed runtime, and
the grid half of that runtime is only included when the program actually uses a
grid.

Faithfulness isn't assumed — the test suite transpiles every example, runs the
output through `python3`, and asserts it matches the interpreter.

One honest boundary: a program that rewrites its own source with `poke` has no
static Python equivalent, so the transpiler **refuses it** with a clear error
rather than emitting code that would silently diverge from the interpreter.

## Roadmap — the full wild vision

Each item is a layer that slots onto the existing pipeline without reshaping it:

- [x] **Minimalist core** — the tiny op set everything reduces to
- [x] **Polyglot zones** — one program, many borrowed dialects
- [x] **`stack` dialect** — the first surface language
- [x] **2D / visual `grid` dialect** — a Befunge-style instruction pointer that
      walks a character grid; direction-based control flow
- [x] **Self-modifying code** — the grid's `g`/`p` read and write cells at
      runtime
- [x] **Cross-zone source rewriting** — `poke` lets one zone rewrite a later
      zone's source before it runs
- [x] **Encoded outer layer** — a keyed cipher wrapper so a `.wild` file on disk
      looks like garbage and only this tool (with the key) can decode and run it
- [x] **Repo-bound keyfile** — a private `.wildkey` you carry between your own
      repos; without it, encoded programs won't decode
- [x] **`prose` dialect** — code that reads like English sentences
- [x] **`lambda` dialect** — a Lisp-like parenthesized surface
- [x] **Transpiler backend** — emits a self-contained Python program from the
      same compiled segments, instead of interpreting (output verified against
      the interpreter in CI tests)

## Project layout

```
src/
  core.rs        the minimalist core ops + the shared VM (stack + output)
  zone.rs        the polyglot zone splitter
  zones/
    mod.rs       dialect registry
    stack.rs     the `stack` dialect (surface -> core ops)
    grid.rs      the 2D `grid` dialect (its own interpreter)
    prose.rs     the `prose` dialect (English sentences -> core ops)
    lambda.rs    the `lambda` dialect (Lisp S-expressions -> core ops)
  cipher.rs      the encoded outer layer (keyed XOR + base64, no deps)
  transpile.rs   the Python backend (segments -> a runnable Python program)
  lib.rs         the pipeline (Segment, compile / run / transpile / run_program,
                 per-zone execution for poke, keyfile discovery)
  main.rs        the `convolution` CLI (run / encode / decode / transpile / keygen)
examples/        sample .wild programs (+ secret.wild.enc, encoded)
tests/
  pipeline.rs    end-to-end interpreter tests
  transpile.rs   transpile-then-run-in-python equivalence tests
  cli.rs         CLI tests, incl. the repo-bound keyfile round-trip
```
