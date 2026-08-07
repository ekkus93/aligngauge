//! Exact, memory-planned v0.1 coverage accumulation.

use std::path::Path;

use aligngauge_core::AlignGaugeError;
use aligngauge_hts::{BamReader, FieldPlan, ReaderOptions};

mod accumulator;
mod cigar;
mod plan;
mod report;
mod util;

pub use accumulator::CoverageCollector;
pub use cigar::{CoverageBlock, cigar_to_coverage_blocks};
pub use plan::{CoverageMemoryPlan, CoverageOptions};
pub use report::{CoverageReport, PerReferenceCoverage};

/// Stable canonical coverage profile name.
pub const COVERAGE_PROFILE: &str = "aligngauge-v0.1";
/// One exact implementation shared by every chunk size.
pub const COVERAGE_STRATEGY: &str = "parameterized-chunked-delta-v1";

/// Analyze one local BAM with the canonical v0.1 coverage profile.
///
/// The memory plan is computed before the reader is opened, so an impossible plan fails before
/// BAM traversal.
///
/// # Errors
/// Returns typed configuration, resource, reader-validation, or checked-arithmetic failures.
pub fn analyze_bam(
    path: impl AsRef<Path>,
    options: CoverageOptions,
) -> Result<CoverageReport, AlignGaugeError> {
    let plan =
        CoverageMemoryPlan::plan(options.memory_limit_bytes, 1, options.chunk_size_override)?;
    let mut reader = BamReader::open(path, FieldPlan::coverage(), ReaderOptions::default())?;
    let mut collector = CoverageCollector::new(reader.header(), options.thresholds, plan)?;
    while let Some(record) = reader.next_record()? {
        collector.observe(&record)?;
    }
    collector.finish()
}

#[cfg(test)]
mod tests;
