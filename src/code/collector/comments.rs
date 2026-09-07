//! Comment collection: which nodes are comments, the line span each covers,
//! and the merge of neighboring comments into runs. Split out of the walker
//! because one grammar (Rust doc comments) reports an end position on the
//! row after the comment, which the span extraction has to correct for.

use std::mem;

use tree_sitter::{Node, Point};

use crate::code::{CommentSpan, FileComplexity};

/// Whether the node is one of the language's comment kinds. The walk must not
/// descend into one: a Rust `///` line carries a nested `doc_comment` child.
pub fn is_comment(node: Node, kinds: &[&str]) -> bool {
    kinds.contains(&node.kind())
}

/// Records one comment node's span, plus a tally of nodes seen, which the
/// later merge into runs would otherwise lose.
pub fn record(node: Node, file: &mut FileComplexity) {
    file.comments.push(span(node));
    file.comment_nodes += 1;
}

/// Merges neighboring comments into runs, in place, after the walk.
pub fn coalesce(file: &mut FileComplexity) {
    file.comments = merge(mem::take(&mut file.comments));
}

fn span(node: Node) -> CommentSpan {
    let start = node.start_position();
    CommentSpan {
        start_line: start.row + 1,
        end_line: end_row(start.row, node.end_position()) + 1,
    }
}

/// A comment whose end position sits at column 0 got there by consuming the
/// line's newline (Rust doc comments do this: the grammar keeps the newline
/// in the doc content), so the last line it actually covers is the row
/// before. A comment node is always at least two characters wide, so an end
/// at column 0 on the same row it started is impossible.
fn end_row(start_row: usize, end: Point) -> usize {
    if end.column == 0 && end.row > start_row {
        end.row - 1
    } else {
        end.row
    }
}

/// A span whose start line is exactly one past the previous span's end line
/// merges into it, so a paragraph of adjacent `//` lines - separate nodes in
/// every supported grammar - becomes one run.
fn merge(spans: Vec<CommentSpan>) -> Vec<CommentSpan> {
    let mut runs: Vec<CommentSpan> = Vec::new();
    for span in spans {
        match runs.last_mut() {
            Some(previous) if span.start_line == previous.end_line + 1 => {
                previous.end_line = span.end_line;
            }
            _ => runs.push(span),
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(row: usize, column: usize) -> Point {
        Point { row, column }
    }

    fn span_of(start_line: usize, end_line: usize) -> CommentSpan {
        CommentSpan {
            start_line,
            end_line,
        }
    }

    #[test]
    fn end_row_clamps_a_column_zero_end_on_a_later_row() {
        assert_eq!(end_row(9, point(10, 0)), 9);
    }

    #[test]
    fn end_row_does_not_clamp_mid_line_ends() {
        assert_eq!(end_row(7, point(8, 5)), 8);
    }

    #[test]
    fn end_row_does_not_clamp_a_single_row_span() {
        assert_eq!(end_row(3, point(3, 20)), 3);
    }

    #[test]
    fn merge_joins_adjacent_lines_into_one_run() {
        let spans = vec![span_of(4, 4), span_of(5, 5), span_of(6, 6)];
        assert_eq!(merge(spans), vec![span_of(4, 6)]);
    }

    #[test]
    fn merge_keeps_a_blank_line_gap_as_two_runs() {
        let spans = vec![span_of(1, 1), span_of(3, 3)];
        assert_eq!(merge(spans), vec![span_of(1, 1), span_of(3, 3)]);
    }

    #[test]
    fn merge_joins_a_trailing_comment_with_a_full_line_comment_after_it() {
        let spans = vec![span_of(13, 13), span_of(14, 14)];
        assert_eq!(merge(spans), vec![span_of(13, 14)]);
    }

    #[test]
    fn merge_of_no_spans_is_empty() {
        assert!(merge(Vec::new()).is_empty());
    }
}
