# smell

CLI tool for static code analysis. Reports cyclomatic complexity per function, grouped by type and file. Each function starts at 1 (the straight-line path); every branch — conditionals, loops, switch arms, catch clauses, short-circuit operators, and hidden branches like Rust `?` or Swift `try?` — adds 1.

## Install

```sh
cargo install smell
```

## Usage

Point `smell complexity` at a source file or a directory (searched recursively):

```sh
smell complexity src/main.rs           # a single Rust file
smell complexity Sources/Shape.swift   # a single Swift file
smell complexity app/src/main/kotlin   # a directory of Kotlin sources
smell complexity src/main/java         # a directory of Java sources
```

## Example output

```
src/shape.rs
+-------------+-----------------------------+
| Function    | Complexity                  |
+===========================================+
| Shape       | total 4 · max 2 · avg 2.0   |
|-------------+-----------------------------|
|   area      | 2                           |
|-------------+-----------------------------|
|   fmt       | 2                           |
|-------------+-----------------------------|
| (top-level) | total 18 · max 15 · avg 6.0 |
|-------------+-----------------------------|
|   simple    | 1                           |
|-------------+-----------------------------|
|   branchy   | 15                          |
|-------------+-----------------------------|
|   fallible  | 2                           |
|-------------+-----------------------------|
| file        | total 22 · max 15 · avg 4.4 |
+-------------+-----------------------------+
```

## Supported languages

- Java (`.java`)
- Kotlin (`.kt`, `.kts`)
- Rust (`.rs`)
- Swift (`.swift`)
