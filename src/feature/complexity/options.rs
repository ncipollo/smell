//! Options controlling what an analysis visits and counts.

use crate::code::branch::BranchFilter;
use crate::feature::complexity::filter::{FileFilter, TypeFilter};

/// The defaults visit every supported file, analyze every type, count every
/// branch kind, and enforce no complexity limit.
#[derive(Default)]
pub struct AnalysisOptions {
    pub files: FileFilter,
    pub types: TypeFilter,
    pub branches: BranchFilter,
    /// When set, any function whose complexity exceeds this fails the run.
    pub max_complexity: Option<usize>,
    /// When set, any type with more methods than this fails the run.
    pub max_methods: Option<usize>,
    /// When set, any file with more lines than this fails the run.
    pub max_lines: Option<usize>,
}
