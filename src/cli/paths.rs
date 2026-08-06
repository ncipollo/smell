//! Expands `-` path arguments into newline-separated paths read from stdin,
//! so pipelines like `git diff --name-only | smell -` work.

use std::io::{self, Read};
use std::path::PathBuf;

/// Reads stdin (only if a `-` entry is present) and expands it in place.
pub fn resolve(paths: Vec<PathBuf>) -> io::Result<Vec<PathBuf>> {
    if !paths.iter().any(|path| path.as_os_str() == "-") {
        return Ok(paths);
    }
    let mut stdin = String::new();
    io::stdin().read_to_string(&mut stdin)?;
    Ok(expand(&paths, &stdin))
}

/// Replaces each `-` entry with the non-blank, trimmed lines of `stdin`.
fn expand(paths: &[PathBuf], stdin: &str) -> Vec<PathBuf> {
    paths
        .iter()
        .flat_map(|path| {
            if path.as_os_str() == "-" {
                stdin_paths(stdin)
            } else {
                vec![path.clone()]
            }
        })
        .collect()
}

fn stdin_paths(stdin: &str) -> Vec<PathBuf> {
    stdin
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_passes_through_literal_paths_unchanged() {
        let paths = vec![PathBuf::from("src"), PathBuf::from("lib.rs")];
        assert_eq!(expand(&paths, ""), paths);
    }

    #[test]
    fn expand_replaces_dash_with_stdin_lines() {
        let paths = vec![PathBuf::from("-")];
        let result = expand(&paths, "a.rs\nb.rs\n");
        assert_eq!(result, vec![PathBuf::from("a.rs"), PathBuf::from("b.rs")]);
    }

    #[test]
    fn expand_skips_blank_lines_and_trims_carriage_returns() {
        let paths = vec![PathBuf::from("-")];
        let result = expand(&paths, "a.rs\r\n\r\n  \nb.rs\r\n");
        assert_eq!(result, vec![PathBuf::from("a.rs"), PathBuf::from("b.rs")]);
    }

    #[test]
    fn expand_mixes_dash_with_literal_paths() {
        let paths = vec![PathBuf::from("src"), PathBuf::from("-")];
        let result = expand(&paths, "a.rs\n");
        assert_eq!(result, vec![PathBuf::from("src"), PathBuf::from("a.rs")]);
    }
}
