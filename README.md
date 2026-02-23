# BUCL — BatchUp Command Line

[![License: MPL 2.0](https://img.shields.io/badge/License-MPL_2.0-brightgreen.svg)](LICENSE)

BUCL is a simple, text-based scripting language implemented in Rust. It is designed to be easy to learn, with a minimalist syntax built around string variables, indented blocks, and composable built-in functions.

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
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
echo '{output} = "Hello, World!"' | ./target/release/bucl
```

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
{output} = {word/0}       # h
{output} = {word/4}       # o
{output} = {word/length}  # 5
{output} = {word/count}   # 1
```

When assigned **multiple strings**, each is stored separately as `{var/0}`, `{var/1}`, … and `{var}` holds the concatenation:

```
{parts} = "hello" "world"
{output} = {parts/0}      # hello
{output} = {parts/1}      # world
{output} = {parts}        # helloworld
{output} = {parts/count}  # 2
```

Variable names can embed other variables using `{var/{i}}` — the inner reference is resolved at runtime:

```
{i} = "1"
{output} = {parts/{i}}    # world
```

### String Interpolation

Inside double-quoted strings, variable references are expanded automatically.

```
{name} = "World"
{output} = "Hello, {name}!"
# prints: Hello, World!
```

### Comments

Lines beginning with `#` are ignored.

```
# This is a comment
{x} = "value"  # inline comments are not supported
```

### Output

Assigning to the special variable `{output}` prints the value to stdout followed by a newline.

```
{output} = "Hello!"
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
{output} = "Total length: {len}"
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
    {output} = "x is a"
elseif {x} = "b"
    {output} = "x is b"
else
    {output} = "x is something else"

{n} = "42"
if {n} > "10"
    {output} = "n is greater than 10"
```

### Loops

**repeat** — execute a block a fixed number of times.

```
{r} repeat 3
    {output} = "iteration {r/index} of 3"
```

`{r/index}` holds the current iteration number (starting at 1).

**each** — iterate over a list of arguments.

```
{e} each "Alice" "Bob" "Charlie"
    {output} = "Hello, {e/value}!"
```

`{e/value}` holds the current element.

---

## Built-in Functions

| Function   | Signature                            | Description                                           |
|------------|--------------------------------------|-------------------------------------------------------|
| `=`        | `{target} = val ...`                 | Assign (concatenate args) to variable                 |
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

> **`getvar` / `setvar`** exist as low-level built-ins but are rarely needed in application scripts. Use nested variable references (`{var/{i}}`) instead — they resolve at runtime and cover most dynamic-lookup cases. `getvar` and `setvar` are primarily used inside BUCL library functions (e.g. `implode`) that need to walk argument lists by computed index.

---

## User-Defined Functions

Functions can be written in BUCL and placed in a `functions/` directory next to your script (or in the working directory). A file named `functions/foo.bucl` is automatically available as the function `foo`.

Inside a function file:

| Variable     | Meaning                            |
|--------------|------------------------------------|
| `{0}`, `{1}` | Positional arguments               |
| `{argc}`     | Number of arguments                |
| `{target}`   | Name of the caller's target variable |
| `{return}`   | Set this to return a value         |

The bundled `functions/` directory includes:

| Function     | Description                                |
|--------------|--------------------------------------------|
| `reverse`    | Reverse a string                           |
| `explode`    | Split a string by a delimiter              |
| `implode`    | Join arguments with a delimiter            |
| `maxlength`  | Return the length of the longest argument  |
| `slice`      | Extract a slice of arguments               |

---

## Examples

### Hello World

```
{output} = "Hello, World!"
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
    {output} = {word}
```

### String Split and Join

```
{parts} explode "-" "one-two-three"
{output} = "part 0: {parts/0}"
{output} = "part 1: {parts/1}"

{joined} implode ", " "alpha" "beta" "gamma"
{output} = {joined}
```

### File I/O

```
writefile "hello.txt" "Hello from BUCL\n"
{contents} readfile "hello.txt"
{output} = "File says: {contents}"
```

### Dynamic Variable Names

Variable names can embed other variables — the inner part is resolved at runtime:

```
{parts} = "red" "green" "blue"
{i} = "2"
{output} = "color: {parts/{i}}"   # color: blue

# Write to a computed sub-variable
{parts/{i}} = "purple"
{output} = "color: {parts/{i}}"   # color: purple
```

---

## Project Structure

```
bucl-rust/
├── src/
│   ├── main.rs          # Entry point; CLI argument handling
│   ├── lexer.rs         # Tokenizer (variables, strings, bare words)
│   ├── parser.rs        # AST builder (handles indented blocks)
│   ├── ast.rs           # AST node definitions
│   ├── evaluator.rs     # Runtime: variable store, function dispatch
│   ├── error.rs         # Error types (Parse, Runtime, IO, UnknownFunction)
│   └── functions/       # Built-in function implementations (Rust)
├── functions/           # Standard library functions (BUCL)
│   ├── reverse.bucl
│   ├── explode.bucl
│   ├── implode.bucl
│   ├── maxlength.bucl
│   └── slice.bucl
├── examples/
│   ├── hello.bucl
│   └── primitives_test.bucl
└── Cargo.toml
```

---

## Error Handling

Errors print to stderr and exit with code 1. Error types:

- **ParseError** — syntax problem, includes line number
- **RuntimeError** — execution failure
- **UnknownFunction** — called a function that doesn't exist
- **IoError** — file read/write failure
