//! Versioned test-data manifest parsing and local verification.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Result, TestkitError};
use crate::hash::{validate_sha256, verify_sha256};

/// Manifest schema identifier.
pub const MANIFEST_SCHEMA: &str = "aligngauge-testdata-v1";

const HEADER: &str = "schema\tid\tkind\tpath\tsha256\tindex_path\tindex_sha256\tsource_url\tsource_checksum\treference_build\tgeneration\tlicense\texpected_validity\texpected_error\texpected_metrics";

/// Storage policy for a manifest entry.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ArtifactKind {
    /// Small deterministic artifact committed to Git.
    Committed,
    /// Large external artifact prepared only by an explicit user command.
    External,
}

impl ArtifactKind {
    fn parse(line: usize, value: &str) -> Result<Self> {
        match value {
            "committed" => Ok(Self::Committed),
            "external" => Ok(Self::External),
            _ => Err(TestkitError::manifest(
                line,
                format!("unsupported artifact kind {value:?}"),
            )),
        }
    }
}

/// Expected validity of an artifact.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ExpectedValidity {
    /// Artifact is structurally valid.
    Valid,
    /// Artifact must fail with the named stable category.
    Error(String),
}

impl ExpectedValidity {
    fn parse(line: usize, validity: &str, error: &str) -> Result<Self> {
        match (validity, error) {
            ("valid", "-") => Ok(Self::Valid),
            ("error", category) if category != "-" && !category.is_empty() => {
                Ok(Self::Error(category.to_owned()))
            }
            ("valid", _) => Err(TestkitError::manifest(
                line,
                "valid entry must use '-' for expected_error",
            )),
            ("error", _) => Err(TestkitError::manifest(
                line,
                "error entry requires a stable expected_error category",
            )),
            _ => Err(TestkitError::manifest(
                line,
                format!("unsupported expected_validity {validity:?}"),
            )),
        }
    }
}

/// One versioned test-data entry.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ManifestEntry {
    /// Stable fixture identifier.
    pub id: String,
    /// Whether the artifact is committed or explicitly prepared.
    pub kind: ArtifactKind,
    /// Repository-relative local path, or `None` for an unprepared external entry.
    pub path: Option<PathBuf>,
    /// Lowercase SHA-256 for a committed local artifact.
    pub sha256: Option<String>,
    /// Optional repository-relative index path.
    pub index_path: Option<PathBuf>,
    /// Optional lowercase SHA-256 for the index.
    pub index_sha256: Option<String>,
    /// Immutable or authoritative source URL.
    pub source_url: String,
    /// Source checksum in `algorithm:value` form, or `-` for generated artifacts.
    pub source_checksum: Option<String>,
    /// Named reference build.
    pub reference_build: String,
    /// Reproduction command.
    pub generation: String,
    /// Redistribution or source license statement.
    pub license: String,
    /// Expected structural validity.
    pub expected_validity: ExpectedValidity,
    /// Optional expected-metrics file.
    pub expected_metrics: Option<PathBuf>,
}

impl ManifestEntry {
    /// Verify all committed local files without performing network access.
    ///
    /// # Errors
    /// Returns an error for an incomplete identity, missing local file, checksum
    /// mismatch, or missing expected-metrics file.
    pub fn verify_local(&self, repository_root: &Path) -> Result<()> {
        if self.kind == ArtifactKind::External {
            return Ok(());
        }

        let path = required_path(self.path.as_deref(), "path", &self.id)?;
        let digest = required_text(self.sha256.as_deref(), "sha256", &self.id)?;
        verify_sha256(&repository_root.join(path), digest)?;

        match (&self.index_path, &self.index_sha256) {
            (Some(index_path), Some(index_digest)) => {
                verify_sha256(&repository_root.join(index_path), index_digest)?;
            }
            (None, None) => {}
            _ => {
                return Err(TestkitError::manifest(
                    0,
                    format!(
                        "entry {} must provide both index_path and index_sha256",
                        self.id
                    ),
                ));
            }
        }

        if let Some(metrics) = &self.expected_metrics {
            let metrics_path = repository_root.join(metrics);
            fs::metadata(&metrics_path).map_err(|source| {
                TestkitError::io("read expected metrics", metrics_path, source)
            })?;
        }

        Ok(())
    }
}

/// Parsed versioned manifest.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TestDataManifest {
    /// Ordered entries.
    pub entries: Vec<ManifestEntry>,
}

impl TestDataManifest {
    /// Parse strict UTF-8 TSV text.
    ///
    /// # Errors
    /// Returns an error for any schema, field-count, identity, checksum-text,
    /// duplicate-key, or expected-validity contract violation.
    pub fn parse(text: &str) -> Result<Self> {
        let normalized = text.replace("\r\n", "\n");
        let mut lines = normalized.lines();
        let header = lines
            .next()
            .ok_or_else(|| TestkitError::manifest(0, "manifest is empty"))?;
        if header != HEADER {
            return Err(TestkitError::manifest(
                1,
                format!("unexpected header; expected {HEADER:?}"),
            ));
        }

        let mut ids = BTreeSet::new();
        let mut entries = Vec::new();

        for (zero_index, raw_line) in lines.enumerate() {
            let line_number = zero_index + 2;
            if raw_line.is_empty() {
                return Err(TestkitError::manifest(
                    line_number,
                    "blank lines are not allowed",
                ));
            }
            let fields: Vec<&str> = raw_line.split('\t').collect();
            if fields.len() != 15 {
                return Err(TestkitError::manifest(
                    line_number,
                    format!("expected 15 fields, found {}", fields.len()),
                ));
            }
            if fields[0] != MANIFEST_SCHEMA {
                return Err(TestkitError::manifest(
                    line_number,
                    format!("unsupported schema {:?}", fields[0]),
                ));
            }
            let id = required_field(line_number, "id", fields[1])?.to_owned();
            if !ids.insert(id.clone()) {
                return Err(TestkitError::manifest(
                    line_number,
                    format!("duplicate id {id:?}"),
                ));
            }

            let kind = ArtifactKind::parse(line_number, fields[2])?;
            let path = optional_path(fields[3]);
            let sha256 = optional_text(fields[4]);
            let index_path = optional_path(fields[5]);
            let index_sha256 = optional_text(fields[6]);
            validate_local_identity(line_number, kind, path.as_deref(), sha256.as_deref())?;
            if let Some(digest) = &sha256 {
                validate_sha256(digest)?;
            }
            if let Some(digest) = &index_sha256 {
                validate_sha256(digest)?;
            }
            if index_path.is_some() != index_sha256.is_some() {
                return Err(TestkitError::manifest(
                    line_number,
                    "index_path and index_sha256 must be supplied together",
                ));
            }

            let source_url = required_field(line_number, "source_url", fields[7])?.to_owned();
            let source_checksum = optional_text(fields[8]);
            validate_source_checksum(line_number, source_checksum.as_deref())?;
            let reference_build =
                required_field(line_number, "reference_build", fields[9])?.to_owned();
            let generation = required_field(line_number, "generation", fields[10])?.to_owned();
            let license = required_field(line_number, "license", fields[11])?.to_owned();
            let expected_validity = ExpectedValidity::parse(line_number, fields[12], fields[13])?;
            let expected_metrics = optional_path(fields[14]);

            entries.push(ManifestEntry {
                id,
                kind,
                path,
                sha256,
                index_path,
                index_sha256,
                source_url,
                source_checksum,
                reference_build,
                generation,
                license,
                expected_validity,
                expected_metrics,
            });
        }

        if entries.is_empty() {
            return Err(TestkitError::manifest(0, "manifest has no entries"));
        }

        Ok(Self { entries })
    }

    /// Load and parse a manifest from a local path.
    ///
    /// # Errors
    /// Returns local read failures or any manifest parsing error.
    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .map_err(|source| TestkitError::io("read manifest", path, source))?;
        Self::parse(&text)
    }

    /// Verify every committed artifact using local filesystem reads only.
    ///
    /// # Errors
    /// Returns the first committed-entry identity, local I/O, or checksum error.
    pub fn verify_local(&self, repository_root: &Path) -> Result<()> {
        for entry in &self.entries {
            entry.verify_local(repository_root)?;
        }
        Ok(())
    }

    /// Return the explicitly prepared external entries.
    #[must_use]
    pub fn external_entries(&self) -> Vec<&ManifestEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.kind == ArtifactKind::External)
            .collect()
    }
}

fn required_field<'a>(line: usize, name: &str, value: &'a str) -> Result<&'a str> {
    if value.is_empty() || value == "-" {
        Err(TestkitError::manifest(line, format!("{name} is required")))
    } else {
        Ok(value)
    }
}

fn optional_text(value: &str) -> Option<String> {
    (value != "-").then(|| value.to_owned())
}

fn optional_path(value: &str) -> Option<PathBuf> {
    (value != "-").then(|| PathBuf::from(value))
}

fn required_path<'a>(value: Option<&'a Path>, name: &str, id: &str) -> Result<&'a Path> {
    value
        .ok_or_else(|| TestkitError::manifest(0, format!("committed entry {id} is missing {name}")))
}

fn required_text<'a>(value: Option<&'a str>, name: &str, id: &str) -> Result<&'a str> {
    value
        .ok_or_else(|| TestkitError::manifest(0, format!("committed entry {id} is missing {name}")))
}

fn validate_local_identity(
    line: usize,
    kind: ArtifactKind,
    path: Option<&Path>,
    sha256: Option<&str>,
) -> Result<()> {
    match kind {
        ArtifactKind::Committed if path.is_none() || sha256.is_none() => Err(
            TestkitError::manifest(line, "committed entry requires path and sha256"),
        ),
        ArtifactKind::External if path.is_some() || sha256.is_some() => {
            Err(TestkitError::manifest(
                line,
                "external entry must not claim an unprepared local path or SHA-256",
            ))
        }
        ArtifactKind::Committed | ArtifactKind::External => Ok(()),
    }
}

fn validate_source_checksum(line: usize, value: Option<&str>) -> Result<()> {
    let Some(value) = value else {
        return Ok(());
    };
    let Some((algorithm, digest)) = value.split_once(':') else {
        return Err(TestkitError::manifest(
            line,
            "source_checksum must use algorithm:value form",
        ));
    };
    if algorithm.is_empty() || digest.is_empty() {
        return Err(TestkitError::manifest(
            line,
            "source_checksum algorithm and value must be nonempty",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = concat!(
        "schema\tid\tkind\tpath\tsha256\tindex_path\tindex_sha256\tsource_url\t",
        "source_checksum\treference_build\tgeneration\tlicense\texpected_validity\t",
        "expected_error\texpected_metrics\n",
        "aligngauge-testdata-v1\ttiny\tcommitted\tfixtures/tiny.bam\t",
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\t",
        "-\t-\tgenerated:aligngauge-testkit\t-\tsynthetic-v1\tgenerate\tApache-2.0\t",
        "valid\t-\t-\n",
        "aligngauge-testdata-v1\thg002\texternal\t-\t-\t-\t-\thttps://example.invalid/hg002.bam\t",
        "md5:0123456789abcdef0123456789abcdef\tGRCh38-GIABv3\tprepare-hg002\tpublic-data\t",
        "valid\t-\t-\n"
    );

    #[test]
    fn parses_versioned_manifest_without_resolving_urls() {
        let manifest = TestDataManifest::parse(VALID).expect("parse manifest");
        assert_eq!(manifest.entries.len(), 2);
        assert_eq!(manifest.external_entries().len(), 1);
        assert_eq!(
            manifest.external_entries()[0].source_url,
            "https://example.invalid/hg002.bam"
        );
    }

    #[test]
    fn rejects_duplicate_identifiers() {
        let duplicate = format!("{VALID}{}", VALID.lines().nth(1).expect("entry"));
        let error = TestDataManifest::parse(&duplicate).expect_err("duplicate must fail");
        assert!(error.to_string().contains("duplicate id"));
    }

    #[test]
    fn rejects_local_identity_for_external_artifact() {
        let invalid = VALID.replace(
            "\thg002\texternal\t-\t-\t",
            "\thg002\texternal\tfixtures/hg002.bam\tba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\t",
        );
        let error = TestDataManifest::parse(&invalid).expect_err("external path must fail");
        assert!(error.to_string().contains("must not claim"));
    }
}
