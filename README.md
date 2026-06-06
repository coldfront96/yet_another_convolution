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
  -> <dialect>::lower   each zone compiles to core ops
  -> core::Vm::run      execute the flat op stream on a shared stack
  -> output
```

Convoluted to *use*, clean to *build* — each stage is independently testable.

## Run it

```sh
cargo run -- examples/hello.wild     # prints: HI!
cargo run -- examples/arithmetic.wild
cargo test                           # the pipeline test suite
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
| `# ...`             | comment to end of line                   |

```
{stack
  2 3 +   # -> 5
  dup *   # -> 25
  .       # print 25
}
```

### The minimalist core

Everything lowers to ~11 primitive ops (`Push, Add, Sub, Mul, Div, Dup, Drop,
Swap, Over, Print, Emit`). New dialects only need to learn how to emit these.

## Roadmap — the full wild vision

Each item is a layer that slots onto the existing pipeline without reshaping it:

- [x] **Minimalist core** — the tiny op set everything reduces to
- [x] **Polyglot zones** — one program, many borrowed dialects
- [x] **`stack` dialect** — the first surface language
- [ ] **2D / visual `grid` dialect** — a Befunge-style instruction pointer that
      walks a character grid; direction-based control flow
- [ ] **`prose` dialect** — code that reads like English sentences
- [ ] **`lambda` dialect** — a Lisp-like parenthesized surface
- [ ] **Self-modifying zones** — operators that rewrite another zone's source
      before it executes
- [ ] **Encoded outer layer** — a cipher wrapper so a `.wild` file on disk looks
      like garbage and only this tool can decode and run it ("only my repos
      understand it")
- [ ] **Transpiler backend** — a second backend that emits Python/JS from the
      same core ops, instead of interpreting

## Project layout

```
src/
  core.rs        the minimalist core ops + the stack VM
  zone.rs        the polyglot zone splitter
  zones/
    mod.rs       dialect registry
    stack.rs     the `stack` dialect (surface -> core ops)
  lib.rs         the pipeline (compile / run_source)
  main.rs        the `convolution` CLI
examples/        sample .wild programs
tests/           end-to-end pipeline tests
```
