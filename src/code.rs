//! Code layer. Interaction with tree-sitter libraries, with parser setup
//! organized by language type.

pub mod swift;

pub struct FunctionComplexity {
    pub name: String,
    pub branches: usize,
}
