//! Testkit-specific failures.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

/// Result type used by the deterministic testkit.
pub type Result<T> = std::result::Result<T, TestkitError>;

/// Failures produced by corpus generation, manifest verification, and comparison.
#[derive(Debug)]
pub enum TestkitError {
    /// Filesystem operation failed.
    Io {
        /// Operation context.
        context: String,
        /// Affected path.
        path: PathBuf,
        /// Underlying error.
        source: std::io::Error,
    },
    /// Manifest text violated the versioned grammar.
    Manifest {
        /// One-based line number, or zero for file-wide failures.
        line: usize,
        /// Failure explanation.
        message: String,
    },
    /// A committed file does not match its recorded digest.
    Checksum {
        /// Affected path.
        path: PathBuf,
        /// Expected lowercase SHA-256.
        expected: String,
        /// Observed lowercase SHA-256.
        actual: String,
    },
    /// Fixture generation violated an internal invariant.
    Generation {
        /// Failure explanation.
        message: String,
    },
    /// A differential input or comparison was invalid.
    Differential {
        /// Failure explanation.
        message: String,
    },
    /// `HTSlib` rejected a generated fixture or index operation.
    Htslib {
        /// Failure explanation.
        message: String,
    },
}

impl TestkitError {
    /// Wrap a filesystem failure with operation context.
    #[must_use]
    pub fn io(
        context: impl Into<String>,
        path: impl Into<PathBuf>,
        source: std::io::Error,
    ) -> Self {
        Self::Io {
            context: context.into(),
            path: path.into(),
            source,
        }
    }

    /// Construct a manifest failure.
    #[must_use]
    pub fn manifest(line: usize, message: impl Into<String>) -> Self {
        Self::Manifest {
            line,
            message: message.into(),
        }
    }

    /// Construct a checksum mismatch.
    #[must_use]
    pub fn checksum(
        path: impl Into<PathBuf>,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self::Checksum {
            path: path.into(),
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Construct a generation failure.
    #[must_use]
    pub fn generation(message: impl Into<String>) -> Self {
        Self::Generation {
            message: message.into(),
        }
    }

    /// Construct a differential failure.
    #[must_use]
    pub fn differential(message: impl Into<String>) -> Self {
        Self::Differential {
            message: message.into(),
        }
    }

    /// Construct an `HTSlib` failure.
    #[must_use]
    pub fn htslib(message: impl Into<String>) -> Self {
        Self::Htslib {
            message: message.into(),
        }
    }
}

impl Display for TestkitError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                context,
                path,
                source,
            } => write!(formatter, "{context} {}: {source}", path.display()),
            Self::Manifest { line: 0, message } => {
                write!(formatter, "manifest error: {message}")
            }
            Self::Manifest { line, message } => {
                write!(formatter, "manifest line {line}: {message}")
            }
            Self::Checksum {
                path,
                expected,
                actual,
            } => write!(
                formatter,
                "checksum mismatch for {}: expected {expected}, observed {actual}",
                path.display()
            ),
            Self::Generation { message } => write!(formatter, "generation error: {message}"),
            Self::Differential { message } => write!(formatter, "differential error: {message}"),
            Self::Htslib { message } => write!(formatter, "HTSlib error: {message}"),
        }
    }
}

impl Error for TestkitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Manifest { .. }
            | Self::Checksum { .. }
            | Self::Generation { .. }
            | Self::Differential { .. }
            | Self::Htslib { .. } => None,
        }
    }
}
