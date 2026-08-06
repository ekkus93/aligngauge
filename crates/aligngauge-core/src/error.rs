//! Stable error categories and renderers.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter, Write as _};

use crate::json::{JsonValue, ToJson};

/// Stable top-level error categories defined by the product specification.
#[derive(Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ErrorCategory {
    /// Invalid command usage.
    Usage,
    /// Invalid or unresolved configuration.
    Configuration,
    /// Requested input does not exist.
    InputNotFound,
    /// Input is not a supported format.
    InputFormat,
    /// Input is truncated or corrupt.
    InputCorrupt,
    /// Input violates required coordinate ordering.
    InputUnsorted,
    /// A record is valid but unsupported by this release.
    UnsupportedRecord,
    /// A reference is required but absent.
    ReferenceRequired,
    /// A supplied reference does not match the input.
    ReferenceMismatch,
    /// Target input is malformed.
    TargetFormat,
    /// Target input names an unsupported contig.
    TargetContig,
    /// A requested plan exceeds a configured resource limit.
    ResourceLimit,
    /// The destination already exists.
    OutputExists,
    /// Output creation, synchronization, or publication failed.
    OutputIo,
    /// A requested compatibility projection is unavailable.
    CompatibilityUnavailable,
    /// An internal invariant was violated.
    InternalInvariant,
}

impl ErrorCategory {
    /// Every category in stable specification order.
    pub const ALL: [Self; 16] = [
        Self::Usage,
        Self::Configuration,
        Self::InputNotFound,
        Self::InputFormat,
        Self::InputCorrupt,
        Self::InputUnsorted,
        Self::UnsupportedRecord,
        Self::ReferenceRequired,
        Self::ReferenceMismatch,
        Self::TargetFormat,
        Self::TargetContig,
        Self::ResourceLimit,
        Self::OutputExists,
        Self::OutputIo,
        Self::CompatibilityUnavailable,
        Self::InternalInvariant,
    ];

    /// Stable machine-readable category name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Usage => "usage",
            Self::Configuration => "configuration",
            Self::InputNotFound => "input_not_found",
            Self::InputFormat => "input_format",
            Self::InputCorrupt => "input_corrupt",
            Self::InputUnsorted => "input_unsorted",
            Self::UnsupportedRecord => "unsupported_record",
            Self::ReferenceRequired => "reference_required",
            Self::ReferenceMismatch => "reference_mismatch",
            Self::TargetFormat => "target_format",
            Self::TargetContig => "target_contig",
            Self::ResourceLimit => "resource_limit",
            Self::OutputExists => "output_exists",
            Self::OutputIo => "output_io",
            Self::CompatibilityUnavailable => "compatibility_unavailable",
            Self::InternalInvariant => "internal_invariant",
        }
    }

    /// Stable nonzero process exit code.
    #[must_use]
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::Usage => 2,
            Self::Configuration => 3,
            Self::InputNotFound => 4,
            Self::InputFormat => 5,
            Self::InputCorrupt => 6,
            Self::InputUnsorted => 7,
            Self::UnsupportedRecord => 8,
            Self::ReferenceRequired => 9,
            Self::ReferenceMismatch => 10,
            Self::TargetFormat => 11,
            Self::TargetContig => 12,
            Self::ResourceLimit => 13,
            Self::OutputExists => 14,
            Self::OutputIo => 15,
            Self::CompatibilityUnavailable => 16,
            Self::InternalInvariant => 70,
        }
    }
}

impl Display for ErrorCategory {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A typed `AlignGauge` failure with public and sensitive diagnostic details.
#[derive(Debug)]
pub struct AlignGaugeError {
    category: ErrorCategory,
    message: String,
    details: BTreeMap<String, JsonValue>,
    sensitive_details: BTreeMap<String, JsonValue>,
    source: Option<Box<dyn Error + Send + Sync>>,
}

impl AlignGaugeError {
    /// Construct a new error.
    #[must_use]
    pub fn new(category: ErrorCategory, message: impl Into<String>) -> Self {
        Self {
            category,
            message: message.into(),
            details: BTreeMap::new(),
            sensitive_details: BTreeMap::new(),
            source: None,
        }
    }

    /// Error category.
    #[must_use]
    pub const fn category(&self) -> ErrorCategory {
        self.category
    }

    /// Stable process exit code.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        self.category.exit_code()
    }

    /// Add a non-sensitive structured detail.
    #[must_use]
    pub fn with_detail(mut self, key: impl Into<String>, value: impl ToJson) -> Self {
        self.details.insert(key.into(), value.to_json());
        self
    }

    /// Add a detail that is redacted unless sensitive diagnostics are requested.
    #[must_use]
    pub fn with_sensitive_detail(mut self, key: impl Into<String>, value: impl ToJson) -> Self {
        self.sensitive_details.insert(key.into(), value.to_json());
        self
    }

    /// Preserve a causal source error.
    #[must_use]
    pub fn with_source<E>(mut self, source: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        self.source = Some(Box::new(source));
        self
    }

    /// Render a human-readable report.
    #[must_use]
    pub fn render_human(&self, include_sensitive: bool) -> String {
        let mut output = format!("[{}] {}", self.category, self.message);
        for (key, value) in self.visible_details(include_sensitive) {
            write!(output, "\n  {key}: {}", value.to_compact_string())
                .expect("writing to String cannot fail");
        }
        for source in self.source_chain() {
            write!(output, "\n  caused by: {source}").expect("writing to String cannot fail");
        }
        output
    }

    /// Render a structured JSON report.
    #[must_use]
    pub fn render_json(&self, include_sensitive: bool) -> String {
        let source_chain = self
            .source_chain()
            .into_iter()
            .map(|source| JsonValue::String(source.to_string()))
            .collect();
        JsonValue::Object(BTreeMap::from([
            (
                String::from("category"),
                JsonValue::String(self.category.as_str().to_owned()),
            ),
            (
                String::from("details"),
                JsonValue::Object(self.visible_details(include_sensitive)),
            ),
            (
                String::from("exit_code"),
                JsonValue::Unsigned(u64::from(self.exit_code())),
            ),
            (
                String::from("message"),
                JsonValue::String(self.message.clone()),
            ),
            (String::from("source_chain"), JsonValue::Array(source_chain)),
        ]))
        .to_pretty_string()
    }

    fn visible_details(&self, include_sensitive: bool) -> BTreeMap<String, JsonValue> {
        let mut details = self.details.clone();
        if include_sensitive {
            details.extend(self.sensitive_details.clone());
        }
        details
    }

    fn source_chain(&self) -> Vec<&(dyn Error + 'static)> {
        let mut chain = Vec::new();
        let mut current = self.source();
        while let Some(source) = current {
            chain.push(source);
            current = source.source();
        }
        chain
    }
}

impl Display for AlignGaugeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "[{}] {}", self.category, self.message)
    }
}

impl Error for AlignGaugeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn Error + 'static))
    }
}

#[cfg(test)]
mod tests {
    use super::{AlignGaugeError, ErrorCategory};
    use std::collections::BTreeSet;
    use std::error::Error as _;
    use std::io;

    #[test]
    fn categories_and_exit_codes_are_stable_and_unique() {
        let names: BTreeSet<_> = ErrorCategory::ALL
            .into_iter()
            .map(ErrorCategory::as_str)
            .collect();
        let codes: BTreeSet<_> = ErrorCategory::ALL
            .into_iter()
            .map(ErrorCategory::exit_code)
            .collect();

        assert_eq!(names.len(), ErrorCategory::ALL.len());
        assert_eq!(codes.len(), ErrorCategory::ALL.len());
        assert!(!codes.contains(&0));
    }

    #[test]
    fn every_category_renders_with_its_stable_identity() {
        for category in ErrorCategory::ALL {
            let error = AlignGaugeError::new(category, "test failure");
            assert_eq!(error.exit_code(), category.exit_code());
            assert!(error.render_human(false).contains(category.as_str()));
            assert!(error.render_json(false).contains(category.as_str()));
        }
    }

    #[test]
    fn sensitive_details_are_redacted_by_default() {
        let error = AlignGaugeError::new(ErrorCategory::InputCorrupt, "record decode failed")
            .with_detail("reference_id", 2_u32)
            .with_sensitive_detail("read_name", "patient-read-42");

        let default_human = error.render_human(false);
        let default_json = error.render_json(false);
        assert!(default_human.contains("reference_id"));
        assert!(!default_human.contains("patient-read-42"));
        assert!(!default_json.contains("patient-read-42"));
        assert!(error.render_json(true).contains("patient-read-42"));
    }

    #[test]
    fn causal_chain_is_preserved() {
        let error = AlignGaugeError::new(ErrorCategory::OutputIo, "write failed")
            .with_source(io::Error::other("disk offline"));

        assert!(error.source().is_some());
        assert!(error.render_human(false).contains("disk offline"));
        assert!(error.render_json(false).contains("disk offline"));
    }
}
