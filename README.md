# smell

CLI tool for static code analysis. Reports cyclomatic complexity per function, grouped by type and file. Each function starts at 1 (the straight-line path); every branch — conditionals, loops, switch arms, catch clauses, short-circuit operators, and hidden branches like Rust `?` or Swift `try?` — adds 1.

## Install

```sh
cargo install smell
```

## Usage

Point `smell` at one or more source files or directories (directories are searched recursively):

```sh
smell src/main.rs           # a single Rust file
smell Sources/Shape.swift   # a single Swift file
smell app/src/main/kotlin   # a directory of Kotlin sources
smell src/main/java         # a directory of Java sources
smell services/api          # a directory of Python sources
smell src lib               # multiple paths, merged into one report
```

Pass `-` to read newline-separated paths from stdin, e.g. for `git diff` or `xargs` workflows:

```sh
git diff --name-only | smell -
git diff --name-only HEAD~1 | smell - --max-complexity 10
```

Unsupported files (like `README.md` in a `git diff` list) are silently skipped rather than erroring, and explicitly-named files are still subject to `--include`/`--exclude`. A path that can't be read is reported to stderr and the run exits non-zero, but the rest of the paths still get analyzed.

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
- JavaScript (`.js`, `.mjs`, `.cjs`)
- Kotlin (`.kt`, `.kts`)
- Python (`.py`)
- Rust (`.rs`)
- Swift (`.swift`)
- TypeScript (`.ts`, `.tsx`, `.mts`, `.cts`)
