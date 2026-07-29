//! Options controlling what an analysis visits and counts.

use crate::code::branch::BranchFilter;
use crate::feature::complexity::filter::{FileFilter, TypeFilter};

/// The defaults visit every supported file, analyze every type, and count
/// every branch kind.
#[derive(Default)]
pub struct AnalysisOptions {
    pub files: FileFilter,
    pub types: TypeFilter,
    pub branches: BranchFilter,
}
