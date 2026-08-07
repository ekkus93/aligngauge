//! Validation-first BAM I/O boundary for `AlignGauge` v0.1.

#![allow(clippy::module_name_repetitions)]

mod header;
mod plan;
mod reader;

pub use header::{
    HeaderIdentity, ReadGroupDeclarationState, ReadGroupDefinition, ReferenceSequence, SortOrder,
    ValidatedHeader,
};
pub use plan::{FieldPlan, RequiredField};
pub use reader::{
    BamReader, CigarFacts, FieldValue, ReadGroupValue, ReaderOptions, RecordCoordinate,
    ValidatedRecord,
};

/// Pinned Rust wrapper version used by the v0.1 BAM boundary.
pub const RUST_HTSLIB_VERSION: &str = "1.0.1";

/// `HTSlib` compatibility line supplied by the pinned rust-htslib release.
pub const HTSLIB_COMPATIBILITY_VERSION: &str = "HTSlib 1.22 series via rust-htslib 1.0.1";
