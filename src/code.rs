//! Code layer. Interaction with tree-sitter libraries, with parser setup
//! organized by language type.

mod collector;
pub mod java;
pub mod kotlin;
pub mod rust;
pub mod swift;

pub struct FunctionComplexity {
    pub name: String,
    /// Cyclomatic complexity: a baseline of 1 plus one per branch.
    pub complexity: usize,
}

/// A class/struct/enum/trait-like declaration and the functions it contains.
pub struct TypeComplexity {
    pub name: String,
    pub functions: Vec<FunctionComplexity>,
}

pub struct FileComplexity {
    /// Top-level functions not contained in any type.
    pub functions: Vec<FunctionComplexity>,
    pub types: Vec<TypeComplexity>,
}

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
                functions: vec![function("area", 1), function("label", 7)],
            }],
        };
        let rollup = complexity.rollup();
        assert_eq!(rollup.total, 12);
        assert_eq!(rollup.max, 7);
        assert_eq!(rollup.average, 4.0);
    }
}
