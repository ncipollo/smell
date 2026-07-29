//! Options controlling what an analysis visits and counts.

use crate::code::branch::BranchFilter;
use crate::feature::complexity::filter::FileFilter;

/// The defaults visit every supported file and count every branch kind.
#[derive(Default)]
pub struct AnalysisOptions {
    pub files: FileFilter,
    pub branches: BranchFilter,
}
