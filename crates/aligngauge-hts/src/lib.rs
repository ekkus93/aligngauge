//! Validation-first BAM/CRAM I/O boundary for `AlignGauge`.

#![allow(clippy::module_name_repetitions)]

mod header;
mod plan;
mod reader;
mod reference;

pub use header::{
    HeaderIdentity, ReadGroupDeclarationState, ReadGroupDefinition, ReferenceSequence, SortOrder,
    ValidatedHeader,
};
pub use plan::{FieldPlan, RequiredField};
pub use reader::{
    AlignmentFormat, BamReader, CigarFacts, FieldValue, ReadGroupValue, ReaderOptions,
    RecordCoordinate, ValidatedRecord, detect_alignment_format,
};
pub use reference::{
    LocalReferenceIdentity, ReferenceContigIdentity, ReferenceRequirement,
    parse_reference_requirements, validate_local_reference,
};

/// Pinned Rust wrapper version used by the production alignment boundary.
pub const RUST_HTSLIB_VERSION: &str = "1.0.1";

/// Exact `hts-sys` crate selected in `Cargo.lock`.
pub const HTS_SYS_VERSION: &str = "2.2.1";

/// Exact vendored `HTSlib` release carried by the pinned `hts-sys` package.
pub const HTSLIB_COMPATIBILITY_VERSION: &str = "1.19.1";

/// Network transport is intentionally unavailable in the production `HTSlib` build.
pub const HTSLIB_NETWORK_TRANSPORT_ENABLED: bool = false;
