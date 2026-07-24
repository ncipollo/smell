use std::path::Path;

use crate::code::FileComplexity;
use crate::code::{java, kotlin, rust, swift};

const SUPPORTED_EXTENSIONS: &[&str] = &["java", "kt", "kts", "rs", "swift"];

pub fn is_supported(path: &Path) -> bool {
    extension(path).is_some_and(|extension| SUPPORTED_EXTENSIONS.contains(&extension.as_str()))
}

/// Routes the source to the language parser matching the file extension.
/// Returns `None` for unsupported file types.
pub fn file_complexity(path: &Path, source: &str) -> Option<FileComplexity> {
    match extension(path)?.as_str() {
        "java" => Some(java::file_complexity(source)),
        "kt" | "kts" => Some(kotlin::file_complexity(source)),
        "rs" => Some(rust::file_complexity(source)),
        "swift" => Some(swift::file_complexity(source)),
        _ => None,
    }
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .map(|extension| extension.to_string_lossy().to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_supported_accepts_each_supported_extension() {
        for extension in SUPPORTED_EXTENSIONS {
            assert!(
                is_supported(Path::new(&format!("src/file.{extension}"))),
                "expected .{extension} to be supported"
            );
        }
    }

    #[test]
    fn is_supported_rejects_other_files() {
        assert!(!is_supported(Path::new("notes.md")));
        assert!(!is_supported(Path::new("no_extension")));
    }

    #[test]
    fn file_complexity_routes_swift() {
        let complexity =
            file_complexity(Path::new("file.swift"), "func simple() {}").expect("swift routed");
        assert_eq!(complexity.functions.len(), 1);
    }

    #[test]
    fn file_complexity_rejects_unsupported_extension() {
        assert!(file_complexity(Path::new("notes.md"), "").is_none());
    }
}
