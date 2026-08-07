//! Fail-closed local FASTA validation for CRAM input.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use aligngauge_core::{AlignGaugeError, ErrorCategory, JsonValue, ToJson};
use md5::Md5;
use rust_htslib::bam::HeaderView;
use sha2::{Digest, Sha256};

const MAX_FASTA_HEADER_BYTES: usize = 1024 * 1024;
const MAX_REFERENCE_COUNT: usize = 1_000_000;

/// One CRAM header reference requirement.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReferenceRequirement {
    name: String,
    length: u64,
    md5: Option<String>,
}

impl ReferenceRequirement {
    /// Reference name exactly as declared by `@SQ SN`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Reference length declared by `@SQ LN`.
    #[must_use]
    pub const fn length(&self) -> u64 {
        self.length
    }

    /// Lower-case `@SQ M5` value when present.
    #[must_use]
    pub fn md5(&self) -> Option<&str> {
        self.md5.as_deref()
    }
}

/// Identity of one sequence actually read from the supplied local FASTA.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReferenceContigIdentity {
    name: String,
    length: u64,
    md5: String,
}

impl ReferenceContigIdentity {
    /// FASTA sequence name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Normalized sequence length used for CRAM reference validation.
    #[must_use]
    pub const fn length(&self) -> u64 {
        self.length
    }

    /// SAM-specification M5 digest of the normalized sequence.
    #[must_use]
    pub fn md5(&self) -> &str {
        &self.md5
    }
}

impl ToJson for ReferenceContigIdentity {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (String::from("length"), self.length.to_json()),
            (String::from("md5"), self.md5.to_json()),
            (String::from("name"), self.name.to_json()),
        ]))
    }
}

/// Reproducible identity of the explicit local FASTA used for CRAM decoding.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LocalReferenceIdentity {
    path: PathBuf,
    size_bytes: u64,
    sha256: String,
    contigs: Vec<ReferenceContigIdentity>,
}

impl LocalReferenceIdentity {
    /// User-supplied FASTA path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Exact FASTA file size.
    #[must_use]
    pub const fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    /// SHA-256 of the exact FASTA file bytes.
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Required contigs validated against the CRAM header, in CRAM header order.
    #[must_use]
    pub fn contigs(&self) -> &[ReferenceContigIdentity] {
        &self.contigs
    }
}

impl ToJson for LocalReferenceIdentity {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (String::from("contigs"), self.contigs.to_json()),
            (
                String::from("path"),
                self.path.to_string_lossy().into_owned().to_json(),
            ),
            (String::from("sha256"), self.sha256.to_json()),
            (String::from("size_bytes"), self.size_bytes.to_json()),
        ]))
    }
}

/// Parse CRAM `@SQ` requirements and validate an explicit local FASTA before decoding records.
///
/// The FASTA M5 calculation follows SAM v1.6+ semantics: sequence characters outside printable
/// ASCII `!` through `~` are removed and lower-case ASCII is converted to upper-case before MD5.
/// Extra FASTA contigs are allowed but are not candidates for fallback; every CRAM-declared contig
/// must be present exactly once and match `LN` plus `M5` when `M5` is present.
///
/// # Errors
/// Returns `reference_required` when the FASTA cannot be read or a CRAM contig is missing, and
/// `reference_mismatch` for duplicate FASTA contigs, length mismatches, or M5 mismatches.
pub fn validate_local_reference(
    header: &HeaderView,
    fasta: impl AsRef<Path>,
) -> Result<LocalReferenceIdentity, AlignGaugeError> {
    let requirements = parse_reference_requirements(header)?;
    let fasta = fasta.as_ref();
    if !fasta.is_file() {
        return Err(AlignGaugeError::new(
            ErrorCategory::ReferenceRequired,
            format!(
                "local reference FASTA '{}' is not a readable file",
                fasta.display()
            ),
        )
        .with_detail("reference", fasta.to_string_lossy().into_owned()));
    }

    let parsed = parse_fasta(fasta)?;
    let mut validated = Vec::with_capacity(requirements.len());
    for requirement in &requirements {
        let actual = parsed.contigs.get(requirement.name()).ok_or_else(|| {
            let mut error = AlignGaugeError::new(
                ErrorCategory::ReferenceRequired,
                "required CRAM reference sequence is missing from the supplied local FASTA",
            )
            .with_detail("reference_name", requirement.name().to_owned())
            .with_detail("expected_length", requirement.length());
            if let Some(md5) = requirement.md5() {
                error = error.with_detail("expected_md5", md5.to_owned());
            }
            error
        })?;

        if actual.length != requirement.length() {
            return Err(AlignGaugeError::new(
                ErrorCategory::ReferenceMismatch,
                "supplied local FASTA reference length does not match the CRAM header",
            )
            .with_detail("reference_name", requirement.name().to_owned())
            .with_detail("expected_length", requirement.length())
            .with_detail("actual_length", actual.length));
        }
        if let Some(expected_md5) = requirement.md5()
            && actual.md5 != expected_md5
        {
            return Err(AlignGaugeError::new(
                ErrorCategory::ReferenceMismatch,
                "supplied local FASTA reference MD5 does not match the CRAM header",
            )
            .with_detail("reference_name", requirement.name().to_owned())
            .with_detail("expected_md5", expected_md5.to_owned())
            .with_detail("actual_md5", actual.md5.clone()));
        }
        validated.push(actual.clone());
    }

    Ok(LocalReferenceIdentity {
        path: fasta.to_path_buf(),
        size_bytes: parsed.size_bytes,
        sha256: parsed.sha256,
        contigs: validated,
    })
}

/// Parse the reference requirements carried by the alignment header.
///
/// # Errors
/// Returns `input_format` for malformed or contradictory `@SQ` fields.
pub fn parse_reference_requirements(
    header: &HeaderView,
) -> Result<Vec<ReferenceRequirement>, AlignGaugeError> {
    let text = std::str::from_utf8(header.as_bytes()).map_err(|source| {
        AlignGaugeError::new(
            ErrorCategory::InputFormat,
            "alignment header is not valid UTF-8",
        )
        .with_source(source)
    })?;
    let mut requirements = Vec::new();
    let mut names = BTreeSet::new();

    for (line_index, line) in text.split_terminator('\n').enumerate() {
        let line_number = line_index.saturating_add(1);
        let line = line.strip_suffix('\r').unwrap_or(line);
        if !line.starts_with("@SQ\t") {
            continue;
        }
        if requirements.len() >= MAX_REFERENCE_COUNT {
            return Err(AlignGaugeError::new(
                ErrorCategory::ResourceLimit,
                "CRAM reference count exceeds the supported limit",
            ));
        }
        let fields = parse_sq_fields(line, line_number)?;
        let name = fields
            .get("SN")
            .filter(|value| !value.is_empty())
            .ok_or_else(|| reference_header_error(line_number, "@SQ record is missing SN"))?;
        if !names.insert(name.clone()) {
            return Err(reference_header_error(
                line_number,
                "CRAM header repeats an @SQ reference name",
            )
            .with_detail("reference_name", name.clone()));
        }
        let length_text = fields
            .get("LN")
            .filter(|value| !value.is_empty())
            .ok_or_else(|| reference_header_error(line_number, "@SQ record is missing LN"))?;
        let length = length_text.parse::<u64>().map_err(|source| {
            reference_header_error(line_number, "@SQ LN is not a valid u64")
                .with_detail("reference_name", name.clone())
                .with_source(source)
        })?;
        let md5 = fields.get("M5").map(|value| value.to_ascii_lowercase());
        if let Some(value) = &md5
            && (value.len() != 32 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        {
            return Err(
                reference_header_error(line_number, "@SQ M5 is not a 32-digit MD5")
                    .with_detail("reference_name", name.clone())
                    .with_detail("md5", value.clone()),
            );
        }
        requirements.push(ReferenceRequirement {
            name: name.clone(),
            length,
            md5,
        });
    }

    if requirements.len()
        != usize::try_from(header.target_count()).map_err(|source| {
            AlignGaugeError::new(
                ErrorCategory::InternalInvariant,
                "alignment target count does not fit usize",
            )
            .with_source(source)
        })?
    {
        return Err(AlignGaugeError::new(
            ErrorCategory::InputFormat,
            "textual @SQ declarations do not match the alignment target table",
        ));
    }
    Ok(requirements)
}

fn parse_sq_fields(
    line: &str,
    line_number: usize,
) -> Result<BTreeMap<String, String>, AlignGaugeError> {
    let mut fields = BTreeMap::new();
    for field in line.split('\t').skip(1) {
        let (tag, value) = field
            .split_once(':')
            .ok_or_else(|| reference_header_error(line_number, "@SQ field does not contain ':'"))?;
        if tag.len() != 2 || !tag.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
            return Err(reference_header_error(
                line_number,
                "@SQ field tag is invalid",
            ));
        }
        if fields.insert(tag.to_owned(), value.to_owned()).is_some() {
            return Err(
                reference_header_error(line_number, "@SQ repeats a field tag")
                    .with_detail("tag", tag.to_owned()),
            );
        }
    }
    Ok(fields)
}

struct ParsedFasta {
    size_bytes: u64,
    sha256: String,
    contigs: BTreeMap<String, ReferenceContigIdentity>,
}

struct ActiveContig {
    name: String,
    length: u64,
    md5: Md5,
}

fn parse_fasta(path: &Path) -> Result<ParsedFasta, AlignGaugeError> {
    let file = File::open(path).map_err(|source| {
        AlignGaugeError::new(
            ErrorCategory::ReferenceRequired,
            format!("failed to open local reference FASTA '{}'", path.display()),
        )
        .with_detail("reference", path.to_string_lossy().into_owned())
        .with_source(source)
    })?;
    let metadata = file.metadata().map_err(|source| {
        AlignGaugeError::new(
            ErrorCategory::ReferenceRequired,
            "failed to read local reference FASTA metadata",
        )
        .with_source(source)
    })?;
    let mut reader = BufReader::new(file);
    let mut file_sha256 = Sha256::new();
    let mut line = Vec::new();
    let mut active: Option<ActiveContig> = None;
    let mut contigs = BTreeMap::new();

    loop {
        line.clear();
        let count = reader.read_until(b'\n', &mut line).map_err(|source| {
            AlignGaugeError::new(
                ErrorCategory::ReferenceRequired,
                "failed while reading local reference FASTA",
            )
            .with_source(source)
        })?;
        if count == 0 {
            break;
        }
        file_sha256.update(&line);
        let logical = trim_line_ending(&line);
        if let Some(rest) = logical.strip_prefix(b">") {
            finish_contig(&mut contigs, active.take())?;
            if rest.len() > MAX_FASTA_HEADER_BYTES {
                return Err(AlignGaugeError::new(
                    ErrorCategory::ResourceLimit,
                    "reference FASTA header exceeds the supported byte limit",
                ));
            }
            let token = rest
                .split(u8::is_ascii_whitespace)
                .next()
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    AlignGaugeError::new(
                        ErrorCategory::ReferenceMismatch,
                        "reference FASTA contains an empty sequence name",
                    )
                })?;
            let name = std::str::from_utf8(token).map_err(|source| {
                AlignGaugeError::new(
                    ErrorCategory::ReferenceMismatch,
                    "reference FASTA sequence name is not valid UTF-8",
                )
                .with_source(source)
            })?;
            active = Some(ActiveContig {
                name: name.to_owned(),
                length: 0,
                md5: Md5::new(),
            });
            continue;
        }

        let current = active.as_mut().ok_or_else(|| {
            AlignGaugeError::new(
                ErrorCategory::ReferenceMismatch,
                "reference FASTA contains sequence data before the first header",
            )
        })?;
        for &byte in logical {
            if !(33..=126).contains(&byte) {
                continue;
            }
            let normalized = byte.to_ascii_uppercase();
            current.md5.update([normalized]);
            current.length = current.length.checked_add(1).ok_or_else(|| {
                AlignGaugeError::new(
                    ErrorCategory::ResourceLimit,
                    "reference FASTA sequence length overflows u64",
                )
                .with_detail("reference_name", current.name.clone())
            })?;
        }
    }
    finish_contig(&mut contigs, active)?;
    if contigs.is_empty() {
        return Err(AlignGaugeError::new(
            ErrorCategory::ReferenceRequired,
            "local reference FASTA contains no sequences",
        ));
    }

    Ok(ParsedFasta {
        size_bytes: metadata.len(),
        sha256: lower_hex(&file_sha256.finalize()),
        contigs,
    })
}

fn finish_contig(
    contigs: &mut BTreeMap<String, ReferenceContigIdentity>,
    active: Option<ActiveContig>,
) -> Result<(), AlignGaugeError> {
    let Some(active) = active else {
        return Ok(());
    };
    let identity = ReferenceContigIdentity {
        name: active.name.clone(),
        length: active.length,
        md5: lower_hex(&active.md5.finalize()),
    };
    if contigs.insert(active.name.clone(), identity).is_some() {
        return Err(AlignGaugeError::new(
            ErrorCategory::ReferenceMismatch,
            "reference FASTA repeats a sequence name",
        )
        .with_detail("reference_name", active.name));
    }
    Ok(())
}

fn trim_line_ending(mut line: &[u8]) -> &[u8] {
    if let Some(stripped) = line.strip_suffix(b"\n") {
        line = stripped;
    }
    line.strip_suffix(b"\r").unwrap_or(line)
}

fn lower_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        write!(output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn reference_header_error(line_number: usize, message: impl Into<String>) -> AlignGaugeError {
    AlignGaugeError::new(ErrorCategory::InputFormat, message).with_detail(
        "header_line",
        u64::try_from(line_number).unwrap_or(u64::MAX),
    )
}
