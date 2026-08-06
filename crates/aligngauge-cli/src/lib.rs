//! Command-line orchestration over the production BAM reader and checked counters.

use std::path::Path;

use aligngauge_core::AlignGaugeError;
use aligngauge_metrics::analyze_bam as analyze_metrics_bam;

pub use aligngauge_metrics::CounterReport;

/// Legacy three-counter projection retained for the walking-skeleton contract.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct BamCounts {
    /// Number of decoded and validated records.
    pub total: u64,
    /// Number of records without the unmapped flag.
    pub mapped: u64,
    /// Number of records carrying the unmapped flag.
    pub unmapped: u64,
}

/// Analyze a BAM with all Milestone 4 counters.
///
/// # Errors
/// Returns a typed reader failure or checked-counter overflow.
pub fn analyze_bam(path: impl AsRef<Path>) -> Result<CounterReport, AlignGaugeError> {
    analyze_metrics_bam(path)
}

/// Validate a BAM and return the original three-counter projection.
///
/// # Errors
/// Returns a typed reader failure or checked-counter overflow.
pub fn count_bam(path: impl AsRef<Path>) -> Result<BamCounts, AlignGaugeError> {
    let report = analyze_bam(path)?;
    let counters = report.alignment_counters();
    Ok(BamCounts {
        total: counters.total,
        mapped: counters.mapped,
        unmapped: counters.unmapped,
    })
}
