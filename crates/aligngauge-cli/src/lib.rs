//! Minimal BAM counting boundary retained while production collectors are built.

use std::path::Path;

use aligngauge_core::{AlignGaugeError, ErrorCategory};
use rust_htslib::bam::{Read, Reader, Record};

/// Record counts emitted by the Milestone 0.5 walking skeleton.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct BamCounts {
    /// Number of decoded records.
    pub total: u64,
    /// Number of records without the unmapped flag.
    pub mapped: u64,
    /// Number of records carrying the unmapped flag.
    pub unmapped: u64,
}

/// Count total, mapped, and unmapped records in a BAM input.
///
/// A single reusable [`Record`] buffer is used for the entire traversal.
///
/// # Errors
///
/// Returns a typed [`AlignGaugeError`] if `HTSlib` cannot open or decode the input,
/// or if a counter overflows.
pub fn count_bam(path: impl AsRef<Path>) -> Result<BamCounts, AlignGaugeError> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(AlignGaugeError::new(
            ErrorCategory::InputNotFound,
            format!("input BAM '{}' does not exist", path.display()),
        )
        .with_detail("input", path.to_string_lossy().into_owned()));
    }

    let mut reader = Reader::from_path(path).map_err(|source| {
        AlignGaugeError::new(
            ErrorCategory::InputFormat,
            format!("failed to open BAM '{}'", path.display()),
        )
        .with_detail("input", path.to_string_lossy().into_owned())
        .with_source(source)
    })?;
    let mut record = Record::new();
    let mut counts = BamCounts::default();

    while let Some(result) = reader.read(&mut record) {
        result.map_err(|source| {
            AlignGaugeError::new(
                ErrorCategory::InputCorrupt,
                format!("failed to decode BAM record from '{}'", path.display()),
            )
            .with_detail("input", path.to_string_lossy().into_owned())
            .with_source(source)
        })?;

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
