//! Test-only helpers shared across modules.

use std::fs;
use std::path::PathBuf;

use crate::code::FileComplexity;

/// Loads a source file from the `fixtures` directory (organized by language),
/// e.g. `fixture("swift/complexity.swift")`.
pub fn fixture(relative_path: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read fixture {}: {error}", path.display()))
}

/// Summarizes top-level functions as `(name, complexity)` pairs for assertions.
pub fn top_level_summary(complexity: &FileComplexity) -> Vec<(String, usize)> {
    complexity
        .functions
        .iter()
        .map(|function| (function.name.clone(), function.complexity))
        .collect()
}

/// Summarizes each type as `(name, [(function, complexity)])` for assertions.
pub fn type_summary(complexity: &FileComplexity) -> Vec<(String, Vec<(String, usize)>)> {
    complexity
        .types
        .iter()
        .map(|complexity_type| {
            let functions = complexity_type
                .functions
                .iter()
                .map(|function| (function.name.clone(), function.complexity))
                .collect();
            (complexity_type.name.clone(), functions)
        })
        .collect()
}
