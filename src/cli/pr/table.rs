use comfy_table::Table;

use crate::feature::diff::{DiffStatus, FileDiff, FlatActivity, FunctionDiff};

/// Renders one table per file, omitting unchanged functions — the
/// mechanical-vs-real noise a diff is meant to filter out — with a trailing
/// count of how many were omitted.
pub(super) fn format_files(files: &[FileDiff]) -> String {
    files.iter().map(format_file).collect()
}

fn format_file(file: &FileDiff) -> String {
    let mut text = format!("{}{}\n", file.path.display(), file_status_suffix(file));
    let mut table = Table::new();
    table.set_header(["Function", "Status", "Before", "After", "Δ"]);
    let mut omitted = 0;
    for function in &file.functions {
        match row(function) {
            Some(cells) => {
                table.add_row(cells);
            }
            None => omitted += 1,
        }
    }
    text.push_str(&format!("{table}\n"));
    if omitted > 0 {
        text.push_str(&format!("{omitted} unchanged function(s) omitted\n"));
    }
    if let Some(summary) = whole_file_summary(file) {
        text.push_str(&summary);
    }
    text.push('\n');
    text
}

/// `" (new file)"`/`" (removed)"` next to the path, so a reviewer sees at a
/// glance that every row below belongs to a file that didn't exist before
/// (or doesn't exist any more) rather than a function added to/removed from
/// an existing file.
fn file_status_suffix(file: &FileDiff) -> &'static str {
    if file.is_new() {
        " (new file)"
    } else if file.is_removed() {
        " (removed)"
    } else {
        ""
    }
}

/// A whole-file complexity total for a new or removed file — the per-row
/// table already lists each function as `added`/`removed`, but not the
/// file's overall size, which is what a reviewer actually needs to gauge
/// how much review attention a brand new file deserves.
fn whole_file_summary(file: &FileDiff) -> Option<String> {
    if !file.is_new() && !file.is_removed() {
        return None;
    }
    let count = file.functions.len();
    let noun = if count == 1 { "function" } else { "functions" };
    Some(format!(
        "{count} {noun}, total complexity {}\n",
        file.total_complexity()
    ))
}

fn name(function: &FunctionDiff) -> String {
    match &function.type_name {
        Some(type_name) => format!("{type_name}.{}", function.name),
        None => function.name.clone(),
    }
}

/// `None` for an unchanged function with no flagged activity, which the
/// table omits entirely.
fn row(function: &FunctionDiff) -> Option<[String; 5]> {
    let (status, before, after, delta) = match function.status {
        DiffStatus::Added { complexity } => (
            "added",
            "-".to_string(),
            complexity.to_string(),
            String::new(),
        ),
        DiffStatus::Removed { complexity } => (
            "removed",
            complexity.to_string(),
            "-".to_string(),
            String::new(),
        ),
        DiffStatus::Changed { before, after } => (
            "changed",
            before.to_string(),
            after.to_string(),
            format!("{:+}", after as i64 - before as i64),
        ),
        DiffStatus::Unchanged { complexity } => return flat_active_row(function, complexity),
    };
    Some([name(function), status.to_string(), before, after, delta])
}

/// A flagged "flat but active" row: complexity is honestly `"unchanged"`
/// (that status didn't move), but the Δ column explains why the row wasn't
/// dropped — a call or mutation count moved that complexity can't see. Rows
/// with no flagged activity stay `None`, omitted exactly as before.
fn flat_active_row(function: &FunctionDiff, complexity: usize) -> Option<[String; 5]> {
    let activity = function.flat_active?;
    Some([
        name(function),
        "unchanged".to_string(),
        complexity.to_string(),
        complexity.to_string(),
        activity_note(&activity),
    ])
}

/// e.g. `"calls +2, mutations +1"`, omitting either clause whose delta is
/// zero (one of the two is always nonzero, so this is never empty).
fn activity_note(activity: &FlatActivity) -> String {
    let mut parts = Vec::new();
    if activity.calls_delta() != 0 {
        parts.push(format!("calls {:+}", activity.calls_delta()));
    }
    if activity.mutations_delta() != 0 {
        parts.push(format!("mutations {:+}", activity.mutations_delta()));
    }
    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature::diff::Snapshot;
    use std::path::PathBuf;

    fn function(type_name: Option<&str>, name: &str, status: DiffStatus) -> FunctionDiff {
        FunctionDiff {
            type_name: type_name.map(str::to_string),
            name: name.to_string(),
            status,
            flat_active: None,
        }
    }

    fn flat_active_function(name: &str, complexity: usize, activity: FlatActivity) -> FunctionDiff {
        FunctionDiff {
            type_name: None,
            name: name.to_string(),
            status: DiffStatus::Unchanged { complexity },
            flat_active: Some(activity),
        }
    }

    fn file_diff(path: &str, functions: Vec<FunctionDiff>) -> FileDiff {
        FileDiff {
            path: PathBuf::from(path),
            functions,
            old: None,
            new: None,
        }
    }

    fn snapshot() -> Snapshot {
        Snapshot {
            lines: 1,
            complexity: crate::code::FileComplexity::default(),
        }
    }

    fn new_file_diff(path: &str, functions: Vec<FunctionDiff>) -> FileDiff {
        FileDiff {
            path: PathBuf::from(path),
            functions,
            old: None,
            new: Some(snapshot()),
        }
    }

    fn removed_file_diff(path: &str, functions: Vec<FunctionDiff>) -> FileDiff {
        FileDiff {
            path: PathBuf::from(path),
            functions,
            old: Some(snapshot()),
            new: None,
        }
    }

    #[test]
    fn format_files_includes_the_file_path() {
        let files = [file_diff(
            "src/a.rs",
            vec![function(None, "a", DiffStatus::Added { complexity: 1 })],
        )];
        assert!(format_files(&files).contains("src/a.rs"));
    }

    #[test]
    fn format_files_marks_a_brand_new_file_and_sums_its_complexity() {
        let files = [new_file_diff(
            "src/brandnew.rs",
            vec![
                function(None, "a", DiffStatus::Added { complexity: 3 }),
                function(None, "b", DiffStatus::Added { complexity: 4 }),
            ],
        )];
        let text = format_files(&files);
        assert!(text.contains("src/brandnew.rs (new file)"));
        assert!(text.contains("2 functions, total complexity 7"));
    }

    #[test]
    fn format_files_marks_a_wholly_removed_file_and_sums_its_complexity() {
        let files = [removed_file_diff(
            "src/gone.rs",
            vec![function(None, "a", DiffStatus::Removed { complexity: 5 })],
        )];
        let text = format_files(&files);
        assert!(text.contains("src/gone.rs (removed)"));
        assert!(text.contains("1 function, total complexity 5"));
    }

    #[test]
    fn format_files_has_no_new_or_removed_suffix_for_an_existing_file() {
        let files = [file_diff(
            "src/a.rs",
            vec![function(
                None,
                "a",
                DiffStatus::Changed {
                    before: 1,
                    after: 2,
                },
            )],
        )];
        let text = format_files(&files);
        assert!(text.contains("src/a.rs\n"));
        assert!(!text.contains("new file"));
        assert!(!text.contains("(removed)"));
        assert!(!text.contains("total complexity"));
    }

    #[test]
    fn format_files_omits_unchanged_functions_and_counts_them() {
        let files = [file_diff(
            "src/a.rs",
            vec![
                function(None, "a", DiffStatus::Unchanged { complexity: 1 }),
                function(None, "b", DiffStatus::Added { complexity: 2 }),
            ],
        )];
        let text = format_files(&files);
        assert!(!text.contains(" a "));
        assert!(text.contains("b"));
        assert!(text.contains("1 unchanged function(s) omitted"));
    }

    #[test]
    fn format_files_has_no_omitted_line_when_nothing_is_unchanged() {
        let files = [file_diff(
            "src/a.rs",
            vec![function(None, "a", DiffStatus::Added { complexity: 1 })],
        )];
        assert!(!format_files(&files).contains("omitted"));
    }

    #[test]
    fn row_prefixes_the_enclosing_type_name() {
        let diff = function(Some("Shape"), "area", DiffStatus::Added { complexity: 1 });
        let cells = row(&diff).unwrap();
        assert_eq!(cells[0], "Shape.area");
    }

    #[test]
    fn row_shows_a_signed_delta_for_changed_functions() {
        let diff = function(
            None,
            "a",
            DiffStatus::Changed {
                before: 3,
                after: 7,
            },
        );
        let cells = row(&diff).unwrap();
        assert_eq!(
            cells,
            [
                "a".to_string(),
                "changed".to_string(),
                "3".to_string(),
                "7".to_string(),
                "+4".to_string()
            ]
        );
    }

    #[test]
    fn row_is_none_for_unchanged() {
        let diff = function(None, "a", DiffStatus::Unchanged { complexity: 1 });
        assert!(row(&diff).is_none());
    }

    #[test]
    fn row_shows_a_flat_active_function_instead_of_omitting_it() {
        let diff = flat_active_function(
            "a",
            1,
            FlatActivity {
                calls_before: 2,
                calls_after: 4,
                mutations_before: 0,
                mutations_after: 0,
            },
        );
        let cells = row(&diff).unwrap();
        assert_eq!(
            cells,
            [
                "a".to_string(),
                "unchanged".to_string(),
                "1".to_string(),
                "1".to_string(),
                "calls +2".to_string()
            ]
        );
    }

    #[test]
    fn format_files_renders_flat_active_functions_instead_of_omitting_them() {
        let files = [file_diff(
            "src/a.rs",
            vec![flat_active_function(
                "a",
                1,
                FlatActivity {
                    calls_before: 0,
                    calls_after: 0,
                    mutations_before: 1,
                    mutations_after: 2,
                },
            )],
        )];
        let text = format_files(&files);
        assert!(text.contains("mutations +1"));
        assert!(!text.contains("omitted"));
    }
}
