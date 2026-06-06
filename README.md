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

### The minimalist core

Everything lowers to ~11 primitive ops (`Push, Add, Sub, Mul, Div, Dup, Drop,
Swap, Over, Print, Emit`). New dialects only need to learn how to emit these.

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

The key comes from `--key <KEY>`, else `$CONVOLUTION_KEY`, else a built-in
default. **Set your own key** to make a program only your tooling (and repos)
can read:

```sh
CONVOLUTION_KEY="my-private-key" cargo run -- encode prog.wild > prog.enc
CONVOLUTION_KEY="my-private-key" cargo run -- run prog.enc
```

A wrong key fails loudly (an inner tag is checked) rather than running garbage.

> This is *obfuscation*, not cryptography — a keystream XOR is exactly as strong
> as keeping the key secret and no stronger. Perfect for a "for fun" esolang;
> don't protect anything that actually matters with it.

## Roadmap — the full wild vision

Each item is a layer that slots onto the existing pipeline without reshaping it:

- [x] **Minimalist core** — the tiny op set everything reduces to
- [x] **Polyglot zones** — one program, many borrowed dialects
- [x] **`stack` dialect** — the first surface language
- [x] **2D / visual `grid` dialect** — a Befunge-style instruction pointer that
      walks a character grid; direction-based control flow
- [x] **Self-modifying code** — the grid's `g`/`p` read and write cells at
      runtime
- [x] **Encoded outer layer** — a keyed cipher wrapper so a `.wild` file on disk
      looks like garbage and only this tool (with the key) can decode and run it
- [x] **`prose` dialect** — code that reads like English sentences
- [ ] **`lambda` dialect** — a Lisp-like parenthesized surface
- [ ] **Transpiler backend** — a second backend that emits Python/JS from the
      same core ops, instead of interpreting

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
  cipher.rs      the encoded outer layer (keyed XOR + base64, no deps)
  lib.rs         the pipeline (Segment, compile / run_source / run_program)
  main.rs        the `convolution` CLI (run / encode / decode)
examples/        sample .wild programs (+ secret.wild.enc, encoded)
tests/           end-to-end pipeline tests
```
