//! Code layer. Interaction with tree-sitter libraries, with parser setup
//! organized by language type.

pub mod branch;
mod collector;
pub mod csharp;
pub mod java;
pub mod javascript;
pub mod kotlin;
pub mod python;
pub mod rust;
pub mod swift;
pub mod typescript;

#[derive(Debug, Clone)]
pub struct FunctionComplexity {
    pub name: String,
    /// Cyclomatic complexity: a baseline of 1 plus one per branch.
    pub complexity: usize,
}

/// A class/struct/enum/trait-like declaration and the functions it contains.
#[derive(Debug, Clone)]
pub struct TypeComplexity {
    pub name: String,
    /// Raw source text of the type's extends/implements/conformance/trait
    /// clauses, unioned across split declarations (impl blocks, extensions).
    /// Generic arguments are preserved (`Comparable<String>`).
    pub supertypes: Vec<String>,
    pub functions: Vec<FunctionComplexity>,
}

/// A run of comment lines: 1-based and inclusive, so a single-line comment
/// has `start_line == end_line`. Adjacent single-line comments are coalesced
/// into one span, so a run can cover a whole comment paragraph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommentSpan {
    pub start_line: usize,
    pub end_line: usize,
}

impl CommentSpan {
    pub fn lines(&self) -> usize {
        self.end_line - self.start_line + 1
    }
}

#[derive(Debug, Clone, Default)]
pub struct FileComplexity {
    /// Top-level functions not contained in any type.
    pub functions: Vec<FunctionComplexity>,
    pub types: Vec<TypeComplexity>,
    /// Coalesced comment runs, in source order.
    pub comments: Vec<CommentSpan>,
    /// Comment nodes seen before coalescing adjacent runs together. Recorded
    /// separately since coalescing is lossy: a ten-line `//` paragraph is one
    /// span but ten nodes, and a future check may want either unit.
    pub comment_nodes: usize,
}

#[derive(Debug, Clone)]
pub struct ComplexityRollup {
    pub total: usize,
    pub max: usize,
    pub average: f64,
}

impl ComplexityRollup {
    pub fn of(functions: &[FunctionComplexity]) -> Self {
        rollup(functions.iter())
    }
}

impl TypeComplexity {
    pub fn rollup(&self) -> ComplexityRollup {
        rollup(self.functions.iter())
    }
}

impl FileComplexity {
    pub fn rollup(&self) -> ComplexityRollup {
        let type_functions = self.types.iter().flat_map(|t| t.functions.iter());
        rollup(self.functions.iter().chain(type_functions))
    }

    /// Top-level declarations: types plus top-level functions.
    pub fn declarations(&self) -> usize {
        self.types.len() + self.functions.len()
    }

    /// Comment runs: a paragraph of adjacent comment lines counts once.
    pub fn comment_count(&self) -> usize {
        self.comments.len()
    }

    /// Lines occupied by comments, summed across every run.
    pub fn comment_lines(&self) -> usize {
        self.comments.iter().map(CommentSpan::lines).sum()
    }
}

fn rollup<'a>(functions: impl Iterator<Item = &'a FunctionComplexity>) -> ComplexityRollup {
    let complexities: Vec<usize> = functions.map(|function| function.complexity).collect();
    let total: usize = complexities.iter().sum();
    let average = if complexities.is_empty() {
        0.0
    } else {
        total as f64 / complexities.len() as f64
    };
    ComplexityRollup {
        total,
        max: complexities.iter().copied().max().unwrap_or(0),
        average,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn function(name: &str, complexity: usize) -> FunctionComplexity {
        FunctionComplexity {
            name: name.to_string(),
            complexity,
        }
    }

    #[test]
    fn rollup_of_no_functions_is_zero() {
        let complexity = TypeComplexity {
            name: "Empty".to_string(),
            supertypes: Vec::new(),
            functions: Vec::new(),
        };
        let rollup = complexity.rollup();
        assert_eq!(rollup.total, 0);
        assert_eq!(rollup.max, 0);
        assert_eq!(rollup.average, 0.0);
    }

    #[test]
    fn rollup_of_single_function_matches_its_complexity() {
        let complexity = TypeComplexity {
            name: "Single".to_string(),
            supertypes: Vec::new(),
            functions: vec![function("only", 3)],
        };
        let rollup = complexity.rollup();
        assert_eq!(rollup.total, 3);
        assert_eq!(rollup.max, 3);
        assert_eq!(rollup.average, 3.0);
    }

    #[test]
    fn file_rollup_spans_top_level_and_type_functions() {
        let complexity = FileComplexity {
            functions: vec![function("top", 4)],
            types: vec![TypeComplexity {
                name: "Shape".to_string(),
                supertypes: Vec::new(),
                functions: vec![function("area", 1), function("label", 7)],
            }],
            ..Default::default()
        };
        let rollup = complexity.rollup();
        assert_eq!(rollup.total, 12);
        assert_eq!(rollup.max, 7);
        assert_eq!(rollup.average, 4.0);
    }

    #[test]
    fn declarations_counts_types_plus_top_level_functions() {
        let complexity = FileComplexity {
            functions: vec![function("top", 1), function("other", 1)],
            types: vec![TypeComplexity {
                name: "Shape".to_string(),
                supertypes: Vec::new(),
                functions: vec![function("area", 1)],
            }],
            ..Default::default()
        };
        assert_eq!(complexity.declarations(), 3);
    }

    #[test]
    fn declarations_of_empty_file_is_zero() {
        let complexity = FileComplexity::default();
        assert_eq!(complexity.declarations(), 0);
    }

    #[test]
    fn comment_span_lines_counts_inclusively() {
        let span = CommentSpan {
            start_line: 3,
            end_line: 5,
        };
        assert_eq!(span.lines(), 3);
    }

    #[test]
    fn comment_span_lines_of_a_single_line_is_one() {
        let span = CommentSpan {
            start_line: 4,
            end_line: 4,
        };
        assert_eq!(span.lines(), 1);
    }

    #[test]
    fn comment_count_is_zero_for_an_empty_file() {
        assert_eq!(FileComplexity::default().comment_count(), 0);
    }

    #[test]
    fn comment_lines_sums_across_runs() {
        let complexity = FileComplexity {
            comments: vec![
                CommentSpan {
                    start_line: 1,
                    end_line: 1,
                },
                CommentSpan {
                    start_line: 5,
                    end_line: 7,
                },
            ],
            ..Default::default()
        };
        assert_eq!(complexity.comment_count(), 2);
        assert_eq!(complexity.comment_lines(), 4);
    }
}
