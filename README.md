# smell

CLI tool for static code analysis. Reports branch complexity per function, grouped by type and file.

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
| Function    | Complexity (branches)       |
+===========================================+
| Shape       | total 2 · max 1 · avg 1.0   |
|-------------+-----------------------------|
|   area      | 1                           |
|-------------+-----------------------------|
|   fmt       | 1                           |
|-------------+-----------------------------|
| (top-level) | total 13 · max 12 · avg 2.6 |
|-------------+-----------------------------|
|   simple    | 0                           |
|-------------+-----------------------------|
|   branchy   | 12                          |
|-------------+-----------------------------|
|   fallible  | 1                           |
|-------------+-----------------------------|
| file        | total 15 · max 12 · avg 2.1 |
+-------------+-----------------------------+
```

## Supported languages

- Java (`.java`)
- Kotlin (`.kt`, `.kts`)
- Rust (`.rs`)
- Swift (`.swift`)
