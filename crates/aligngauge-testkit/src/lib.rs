//! Deterministic test-data and differential-validation support for `AlignGauge`.
//!
//! Ordinary library and test operations perform local filesystem access only.
//! External datasets are prepared exclusively by explicitly invoked scripts.

pub mod bam;
pub mod corpus;
pub mod differential;
pub mod error;
pub mod hash;
pub mod manifest;

pub use corpus::generate_corpus;
pub use differential::{DifferentialReport, compare_files, parse_actual, parse_expected};
pub use error::{Result, TestkitError};
pub use manifest::{ArtifactKind, ExpectedValidity, ManifestEntry, TestDataManifest};
