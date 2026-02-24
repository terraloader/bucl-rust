# BUCL — BatchUp Command Line

[![License: MPL 2.0](https://img.shields.io/badge/License-MPL_2.0-brightgreen.svg)](LICENSE)

BUCL is a simple, text-based scripting language implemented in Rust. It is designed to be easy to learn, with a minimalist syntax built around string variables, indented blocks, and composable built-in functions.

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [WebAssembly / Interactive Demo](#webassembly--interactive-demo)
- [Language Reference](#language-reference)
  - [Variables](#variables)
  - [Assignment](#assignment)
  - [String Interpolation](#string-interpolation)
  - [Comments](#comments)
  - [Output](#output)
  - [Function Calls](#function-calls)
  - [Control Flow](#control-flow)
  - [Loops](#loops)
- [Built-in Functions](#built-in-functions)
- [User-Defined Functions](#user-defined-functions)
- [Examples](#examples)
- [Project Structure](#project-structure)

---

## Installation

Requires [Rust](https://www.rust-lang.org/tools/install) (edition 2021).

```bash
git clone <repo-url>
cd bucl-rust
cargo build --release
# Binary is at: target/release/bucl
```

---

## Quick Start

```bash
# Run a script file
./target/release/bucl examples/hello.bucl

# Run from stdin
echo 'echo "Hello, World!"' | ./target/release/bucl
```

---

## WebAssembly / Interactive Demo

BUCL can be compiled to WebAssembly and run entirely in the browser. Both playgrounds include a code editor, output panel, and six built-in example scripts.

| Demo | Engine | Link |
|------|--------|------|
| **JS demo** | Pure JavaScript port | **[live demo](https://terraloader.github.io/bucl-rust/demo/js/)** |
| **WASM demo** | Prebuilt Rust → WebAssembly | **[live demo](https://terraloader.github.io/bucl-rust/demo/wasm/)** |

The WASM binary is checked into the repo at `docs/demo/wasm/pkg/bucl_wasm.wasm`, so the demo works out of the box — no local Rust toolchain needed.

### Rebuilding the WASM module

After changing the Rust source you can rebuild the `.wasm`:

**Option A — raw `.wasm` (no extra tooling beyond Rust):**

```bash
rustup target add wasm32-unknown-unknown
make wasm-raw          # writes docs/demo/wasm/pkg/bucl_wasm.wasm
make demo              # serves on http://localhost:8000
```

**Option B — wasm-pack (produces optimised JS glue):**

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
make wasm              # release build with wasm-opt
make demo
```

**Development iteration (skips wasm-opt for faster rebuilds):**

```bash
make wasm-dev
make demo
```

### Makefile targets

| Target      | Description                                               |
|-------------|-----------------------------------------------------------|
| `build`     | Native debug binary (`cargo build`)                       |
| `release`   | Native release binary (`cargo build --release`)           |
| `wasm`      | WASM + JS glue via wasm-pack (release, optimised)         |
| `wasm-dev`  | WASM + JS glue via wasm-pack (dev, no wasm-opt)           |
| `wasm-raw`  | Raw `.wasm` via `cargo build` only (no wasm-pack needed)  |
| `demo`      | Serve `docs/demo/` on `http://localhost:8000`             |
| `clean`     | Remove `target/` and `docs/demo/wasm/pkg/`                |

### WASM limitations

- **No filesystem access** — `readfile` and `writefile` are not available in the browser build.
- The standard library functions (`reverse`, `explode`, `implode`, `maxlength`, `slice`) are embedded directly into the WASM binary, so no separate file loading is required.

---

## Language Reference

### Variables

Variables are written as `{name}` — curly braces are part of the syntax, not optional decoration.

```
{greeting}
{user_name}
{count}
```

All variable values are strings. There is no numeric type; arithmetic is done via the `math` function.

### Assignment

Use `=` to assign a value. Multiple arguments are concatenated.

```
{name} = "Alice"
{message} = "Hello, " {name} "!"
```

Every assignment automatically maintains two sub-variables:

| Sub-variable   | Value                                                |
|----------------|------------------------------------------------------|
| `{var/length}` | Character count of the stored value                  |
| `{var/count}`  | Number of arguments (`"1"` for single, `"N"` for multi) |

When a variable holds a **single string**, `{var/N}` returns the character at position N (0-based):

```
{word} = "hello"
echo {word/0}       # h
echo {word/4}       # o
echo {word/length}  # 5
echo {word/count}   # 1
```

When assigned **multiple strings**, each is stored separately as `{var/0}`, `{var/1}`, … and `{var}` holds the concatenation:

```
{parts} = "hello" "world"
echo {parts/0}      # hello
echo {parts/1}      # world
echo {parts}        # helloworld
echo {parts/count}  # 2
```

Variable names can embed other variables using `{var/{i}}` — the inner reference is resolved at runtime:

```
{i} = "1"
echo {parts/{i}}    # world
```

### Array Variable Expansion

A variable that holds **multiple strings** (`{var/count} > 1`) is treated differently depending on where it appears:

**Inside a quoted string** — the elements are joined with a single space and inserted as one value:

```
{colors} = "red" "green" "blue"
echo "I like {colors}"   # I like red green blue
```

**Outside a quoted string (direct argument)** — the variable expands into as many separate arguments as it has elements, exactly as if each element were written individually:

```
{colors} = "red" "green" "blue"

# Both lines below are equivalent — {colors} expands to three arguments:
{joined} implode ", " {colors}
{joined} implode ", " "red" "green" "blue"

# Works with any function that accepts a variable number of arguments:
{e} each {colors}
    echo "color: {e/value}"
```

### String Interpolation

Inside double-quoted strings, variable references are expanded automatically.

```
{name} = "World"
echo "Hello, {name}!"
# prints: Hello, World!
```

### Comments

Lines beginning with `#` are ignored.

```
# This is a comment
{x} = "value"  # inline comments are not supported
```

### Output

Use `echo` to print one or more values to stdout. All arguments are joined with a single space and followed by a newline.

```
echo "Hello!"
echo "x =" {x}
```

### Function Calls

The general syntax for calling a function is:

```
{target} function_name arg1 arg2 ...
```

- `{target}` receives the return value. It can be omitted if the result is not needed.
- Arguments may be quoted strings, variable references, or bare words.
- Some functions accept an indented block (body) on the following lines.

```
{len} length "Hello" "World"
echo "Total length: {len}"
```

### Control Flow

**if / elseif / else**

Conditions compare two values using one of the supported operators. Blocks are indented.

| Operator | Meaning                              |
|----------|--------------------------------------|
| `=`      | String equality                      |
| `!=`     | String inequality                    |
| `>`      | Greater than (numeric if possible)   |
| `<`      | Less than (numeric if possible)      |
| `>=`     | Greater than or equal                |
| `<=`     | Less than or equal                   |

For `>`, `<`, `>=`, `<=`: if both sides parse as numbers the comparison is numeric; otherwise it falls back to lexicographic string comparison.

```
{x} = "b"
if {x} = "a"
    echo "x is a"
elseif {x} = "b"
    echo "x is b"
else
    echo "x is something else"

{n} = "42"
if {n} > "10"
    echo "n is greater than 10"
```

### Loops

**repeat** — execute a block a fixed number of times.

```
{r} repeat 3
    echo "iteration {r/index} of 3"
```

`{r/index}` holds the current iteration number (starting at 1).

**each** — iterate over a list of arguments.

```
{e} each "Alice" "Bob" "Charlie"
    echo "Hello, {e/value}!"
```

`{e/value}` holds the current element.

---

## Built-in Functions

| Function   | Signature                            | Description                                           |
|------------|--------------------------------------|-------------------------------------------------------|
| `=`        | `{target} = val ...`                 | Assign (concatenate args) to variable                 |
| `echo`     | `echo arg ...`                       | Print args (space-joined) to stdout                   |
| `length`   | `{t} length arg ...`                 | Total character length of all arguments               |
| `count`    | `{t} count arg ...`                  | Number of arguments                                   |
| `substr`   | `{t} substr start len str`           | Extract substring at `start` for `len` characters     |
| `strpos`   | `{t} strpos haystack needle`         | Position of `needle` in `haystack` (-1 if not found)  |
| `math`     | `{t} math "expr"`                    | Evaluate arithmetic expression (`+` `-` `*` `/` `%`)  |
| `random`   | `{t} random min max`                 | Random integer in range [min, max]                    |
| `readfile` | `{t} readfile path`                  | Read file contents into variable                      |
| `writefile`| `writefile path content`             | Write content to file                                 |
| `if`       | `if val op val` + block              | Conditional block (`=` `!=` `>` `<` `>=` `<=`)        |
| `repeat`   | `{t} repeat N` + block               | Loop N times                                          |
| `each`     | `{t} each arg ...` + block           | Iterate over arguments                                |

> **`getvar` / `setvar`** exist as low-level built-ins for reading/writing a variable whose name is only known at runtime. They are rarely needed in application scripts: use nested variable references (`{var/{i}}`) for dynamic lookup, and the `{args/N}` convention (see below) inside BUCL functions to access positional arguments by computed index.

---

## User-Defined Functions

Functions can be written in BUCL and placed in a `functions/` directory next to your script (or in the working directory). A file named `functions/foo.bucl` is automatically available as the function `foo`.

Inside a function file, the following variables are available:

| Variable          | Meaning                                              |
|-------------------|------------------------------------------------------|
| `{0}`, `{1}`, …  | Positional arguments (individual variables)          |
| `{args/0}`, `{args/1}`, … | Same arguments accessible as `{args/{i}}` |
| `{args/count}`    | Same as `{argc}`                                     |
| `{argc}`          | Number of arguments                                  |
| `{target}`        | Name of the caller's target variable                 |
| `{return}`        | Set this to return a value                           |

The `{args/N}` variables allow dynamic positional access via `{args/{i}}` without needing `getvar`.

The bundled `functions/` directory includes:

| Function     | Description                                |
|--------------|--------------------------------------------|
| `reverse`    | Reverse a string                           |
| `explode`    | Split a string by a delimiter (returns array) |
| `implode`    | Join arguments with a delimiter            |
| `maxlength`  | Return the length of the longest argument  |
| `slice`      | Extract a slice of arguments               |

---

## Examples

### Hello World

```
echo "Hello, World!"
```

### FizzBuzz (1–15)

```
{i} repeat 15
    {fizz} math "{i/index} % 3"
    {buzz} math "{i/index} % 5"
    {word} = ""
    if {fizz} = "0"
        {word} = "Fizz"
    if {buzz} = "0"
        {word} = "{word}Buzz"
    if {word} = ""
        {word} = "{i/index}"
    echo {word}
```

### String Split and Join

```
# explode returns an array — {parts/count} is the number of elements,
# and each element is accessible as {parts/0}, {parts/1}, …
{parts} explode "-" "one-two-three"
echo "count: {parts/count}"
echo "part 0: {parts/0}"
echo "part 1: {parts/1}"

# The array expands to separate arguments when used directly:
{rejoined} implode " + " {parts}
echo {rejoined}

# Literal items
{joined} implode ", " "alpha" "beta" "gamma"
echo {joined}

# Array variable — expands to separate arguments outside a string
{words} = "alpha" "beta" "gamma"
{joined} implode ", " {words}   # same result as the line above
echo {joined}

# Inside a string the same variable is space-joined into one value
echo "words: {words}"     # words: alpha beta gamma
```

### File I/O

```
writefile "hello.txt" "Hello from BUCL\n"
{contents} readfile "hello.txt"
echo "File says: {contents}"
```

### Dynamic Variable Names

Variable names can embed other variables — the inner part is resolved at runtime:

```
{parts} = "red" "green" "blue"
{i} = "2"
echo "color: {parts/{i}}"   # color: blue

# Write to a computed sub-variable
{parts/{i}} = "purple"
echo "color: {parts/{i}}"   # color: purple
```

---

## Project Structure

```
bucl-rust/
├── src/
│   ├── main.rs          # Entry point; CLI argument handling
│   ├── lib.rs           # WASM entry point (bucl_alloc/bucl_free/bucl_run)
│   ├── lexer.rs         # Tokenizer (variables, strings, bare words)
│   ├── parser.rs        # AST builder (handles indented blocks)
│   ├── ast.rs           # AST node definitions
│   ├── evaluator.rs     # Runtime: variable store, function dispatch, output capture
│   ├── error.rs         # Error types (Parse, Runtime, IO, UnknownFunction)
│   └── functions/       # Built-in function implementations (Rust)
├── functions/           # Standard library functions (BUCL)
│   ├── reverse.bucl
│   ├── explode.bucl
│   ├── implode.bucl
│   ├── maxlength.bucl
│   └── slice.bucl
├── docs/demo/
│   ├── js/
│   │   ├── index.html   # JS Playground (pure JavaScript interpreter)
│   │   └── bucl.js      # JavaScript port of the BUCL engine
│   └── wasm/
│       ├── index.html   # WASM Playground (runs prebuilt Rust via WebAssembly)
│       └── pkg/
│           └── bucl_wasm.wasm  # Prebuilt WASM binary (checked in)
├── examples/
│   ├── hello.bucl
│   └── primitives_test.bucl
├── .cargo/
│   └── config.toml      # wasm32 build flags (opt-level=s, panic=abort)
├── Makefile             # build / release / wasm / wasm-dev / wasm-raw / demo / clean
└── Cargo.toml
```

---

## Error Handling

Errors print to stderr and exit with code 1. Error types:

- **ParseError** — syntax problem, includes line number
- **RuntimeError** — execution failure
- **UnknownFunction** — called a function that doesn't exist
- **IoError** — file read/write failure
