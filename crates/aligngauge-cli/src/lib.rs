//! Minimal BAM counting boundary for the AlignGauge walking skeleton.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};

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

/// Failure to open, decode, or count a BAM input.
#[derive(Debug)]
pub enum BamCountError {
    /// HTSlib could not open the requested path as an alignment input.
    Open {
        /// Input path.
        path: PathBuf,
        /// Original HTSlib error.
        source: rust_htslib::errors::Error,
    },
    /// HTSlib reported a record-decoding failure after opening the input.
    Read {
        /// Input path.
        path: PathBuf,
        /// Original HTSlib error.
        source: rust_htslib::errors::Error,
    },
    /// A counter exceeded the representable `u64` range.
    CounterOverflow {
        /// Counter that overflowed.
        field: &'static str,
    },
}

impl Display for BamCountError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open { path, source } => {
                write!(
                    formatter,
                    "failed to open BAM '{}': {source}",
                    path.display()
                )
            }
            Self::Read { path, source } => {
                write!(
                    formatter,
                    "failed to read BAM '{}': {source}",
                    path.display()
                )
            }
            Self::CounterOverflow { field } => {
                write!(formatter, "BAM record counter '{field}' overflowed")
            }
        }
    }
}

impl Error for BamCountError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Open { source, .. } | Self::Read { source, .. } => Some(source),
            Self::CounterOverflow { .. } => None,
        }
    }
}

/// Count total, mapped, and unmapped records in a BAM input.
///
/// A single reusable `Record` buffer is used for the entire traversal.
///
/// # Errors
///
/// Returns [`BamCountError`] if HTSlib cannot open or decode the input, or if a
/// counter overflows.
pub fn count_bam(path: impl AsRef<Path>) -> Result<BamCounts, BamCountError> {
    let path = path.as_ref();
    let mut reader = Reader::from_path(path).map_err(|source| BamCountError::Open {
        path: path.to_path_buf(),
        source,
    })?;
    let mut record = Record::new();
    let mut counts = BamCounts::default();

    while let Some(result) = reader.read(&mut record) {
        result.map_err(|source| BamCountError::Read {
            path: path.to_path_buf(),
            source,
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

fn increment(counter: &mut u64, field: &'static str) -> Result<(), BamCountError> {
    *counter = counter
        .checked_add(1)
        .ok_or(BamCountError::CounterOverflow { field })?;
    Ok(())
}
