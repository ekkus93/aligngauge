//! Minimal counting CLI over the production BAM validation boundary.

use std::path::Path;

use aligngauge_core::{AlignGaugeError, ErrorCategory};
use aligngauge_hts::{BamReader, FieldPlan, ReaderOptions};

/// Record counts emitted while Milestone 4 collectors are built.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct BamCounts {
    /// Number of decoded and validated records.
    pub total: u64,
    /// Number of records without the unmapped flag.
    pub mapped: u64,
    /// Number of records carrying the unmapped flag.
    pub unmapped: u64,
}

/// Validate a BAM stream and count total, mapped, and unmapped records.
///
/// The production reader reuses one rust-htslib record buffer. Counts are
/// returned only after the entire stream passes header, record, tag, reference,
/// and coordinate-order validation.
///
/// # Errors
///
/// Returns a typed [`AlignGaugeError`] for any reader validation failure or
/// checked-counter overflow.
pub fn count_bam(path: impl AsRef<Path>) -> Result<BamCounts, AlignGaugeError> {
    let mut reader = BamReader::open(
        path,
        FieldPlan::counters(),
        ReaderOptions::default(),
    )?;
    let mut counts = BamCounts::default();

    while let Some(record) = reader.next_record()? {
        increment(&mut counts.total, "total")?;
        if record.is_unmapped() {
            increment(&mut counts.unmapped, "unmapped")?;
        } else {
            increment(&mut counts.mapped, "mapped")?;
        }
    }

    Ok(counts)
}

fn increment(counter: &mut u64, field: &'static str) -> Result<(), AlignGaugeError> {
    *counter = counter.checked_add(1).ok_or_else(|| {
        AlignGaugeError::new(
            ErrorCategory::InternalInvariant,
            format!("BAM record counter '{field}' overflowed"),
        )
        .with_detail("counter", field)
    })?;
    Ok(())
}
