# smell

CLI tool for static code analysis across various programming languages.

Reports cyclomatic complexity per function, method count per type, line count per file, and declaration count per file, grouped by file. Each function starts at 1; every branch adds 1 (conditionals, loops, switch arms, catch clauses, short-circuit operators, etc).

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

Filter which files and types are analyzed, and optionally fail the run past a complexity limit:

```sh
smell src --include "*.rs" --exclude "**/generated/**"  # only *.rs, skipping generated code
smell src --implements Shape                            # only types implementing/extending Shape
smell src --max-complexity 10                           # exit non-zero if any function exceeds 10
smell src --max-methods 15                              # exit non-zero if any type has more than 15 methods
smell src --max-lines 300                               # exit non-zero if any file has more than 300 lines
smell src --max-declarations 10                         # exit non-zero if any file has more than 10 declarations
```

`--include`/`--exclude` and `--implements` are repeatable. Run `smell --info` for a directory of documentation topics (usage, config, languages, branches, filters, checks), or `smell --info branches` to drill straight into one.

## Configuration

Declare named rules in a `smell.toml` file in the directory you invoke `smell` from:

```toml
[[rule]]
name = "default"
include = ["*.rs"]
exclude = ["**/generated/**"]
implements = ["Shape"]
max_complexity = 10
max_methods = 15
max_lines = 300
max_declarations = 20

[[rule]]
name = "swift"
include = ["*.swift"]
max_complexity = 15
```

The rule named `default` applies automatically; select another with `--rule`:

```sh
smell src --rule swift
```

CLI flags override a matched rule's fields entirely rather than merging with them.

## Supported languages

- C# (`.cs`)
- Java (`.java`)
- JavaScript (`.js`, `.mjs`, `.cjs`)
- Kotlin (`.kt`, `.kts`)
- Python (`.py`)
- Rust (`.rs`)
- Swift (`.swift`)
- TypeScript (`.ts`, `.tsx`, `.mts`, `.cts`)
