//! Strict BAM header validation and provenance identity.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use aligngauge_core::{AlignGaugeError, ErrorCategory};
use rust_htslib::bam::HeaderView;
use sha2::{Digest, Sha256};

const MAX_HEADER_BYTES: usize = 16 * 1024 * 1024;
const MAX_HEADER_FIELDS: usize = 256;
const MAX_HEADER_FIELD_BYTES: usize = 64 * 1024;
const MAX_REFERENCE_COUNT: usize = 1_000_000;
const MAX_REFERENCE_NAME_BYTES: usize = 1024;
const MAX_READ_GROUP_COUNT: usize = 1_000_000;

/// One validated BAM reference declaration.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReferenceSequence {
    name: String,
    length: u64,
}

impl ReferenceSequence {
    /// Reference name exactly as declared by the BAM header.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Reference length in bases.
    #[must_use]
    pub const fn length(&self) -> u64 {
        self.length
    }
}

/// Parsed SAM header sort-order declaration.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SortOrder {
    /// No `SO` declaration was present.
    Absent,
    /// `SO:coordinate`.
    Coordinate,
    /// `SO:queryname`.
    QueryName,
    /// `SO:unsorted`.
    Unsorted,
    /// `SO:unknown`.
    Unknown,
    /// A syntactically valid value not understood by this release.
    Other(String),
}

/// Provenance identity for the validated header and binary reference table.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HeaderIdentity {
    sha256: String,
    raw_header_bytes: usize,
    reference_count: usize,
}

impl HeaderIdentity {
    /// SHA-256 of a domain-separated canonical header representation.
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Raw SAM header byte length.
    #[must_use]
    pub const fn raw_header_bytes(&self) -> usize {
        self.raw_header_bytes
    }

    /// Number of binary BAM reference entries.
    #[must_use]
    pub const fn reference_count(&self) -> usize {
        self.reference_count
    }
}

/// Read-group declaration state for a record's `RG` tag.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ReadGroupDeclarationState {
    /// Exactly one header declaration has this ID.
    Known,
    /// No header declaration has this ID.
    Unknown,
    /// More than one declaration has this ID, so its metadata is not trusted.
    Ambiguous,
}

/// One parsed read-group declaration.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReadGroupDefinition {
    id: String,
    fields: BTreeMap<String, String>,
}

impl ReadGroupDefinition {
    /// Read-group identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Untrusted declaration fields, retained only for later explicit consumers.
    #[must_use]
    pub const fn fields(&self) -> &BTreeMap<String, String> {
        &self.fields
    }
}

/// Header accepted by the v0.1 production BAM boundary.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ValidatedHeader {
    references: Vec<ReferenceSequence>,
    sort_order: SortOrder,
    read_groups: BTreeMap<String, Vec<ReadGroupDefinition>>,
    identity: HeaderIdentity,
}

impl ValidatedHeader {
    /// Validate a rust-htslib header view.
    ///
    /// # Errors
    ///
    /// Returns `input_format` for malformed, contradictory, oversized, or
    /// binary/text-inconsistent header data.
    pub fn from_view(view: &HeaderView) -> Result<Self, AlignGaugeError> {
        let raw = view.as_bytes();
        if raw.len() > MAX_HEADER_BYTES {
            return Err(header_error("BAM header exceeds the supported byte limit")
                .with_detail("header_bytes", u64_from_usize(raw.len())?)
                .with_detail("maximum_header_bytes", u64_from_usize(MAX_HEADER_BYTES)?));
        }

        let text = std::str::from_utf8(raw).map_err(|source| {
            header_error("BAM header is not valid UTF-8").with_source(source)
        })?;
        let parsed = ParsedTextHeader::parse(text)?;
        let references = validate_binary_references(view, &parsed.references)?;
        let identity = build_identity(raw, &references)?;

        Ok(Self {
            references,
            sort_order: parsed.sort_order,
            read_groups: parsed.read_groups,
            identity,
        })
    }

    /// Validated references in binary target-table order.
    #[must_use]
    pub fn references(&self) -> &[ReferenceSequence] {
        &self.references
    }

    /// Reference at a validated nonnegative target ID.
    #[must_use]
    pub fn reference(&self, target_id: i32) -> Option<&ReferenceSequence> {
        usize::try_from(target_id)
            .ok()
            .and_then(|index| self.references.get(index))
    }

    /// Header sort-order declaration. Actual coordinates are still validated.
    #[must_use]
    pub const fn sort_order(&self) -> &SortOrder {
        &self.sort_order
    }

    /// Stable header provenance identity.
    #[must_use]
    pub const fn identity(&self) -> &HeaderIdentity {
        &self.identity
    }

    /// Parsed read-group declarations, grouped by ID.
    #[must_use]
    pub const fn read_groups(&self) -> &BTreeMap<String, Vec<ReadGroupDefinition>> {
        &self.read_groups
    }

    /// Resolve an `RG` tag without inventing a replacement group.
    #[must_use]
    pub fn read_group_state(&self, id: &str) -> ReadGroupDeclarationState {
        match self.read_groups.get(id).map(Vec::len) {
            Some(1) => ReadGroupDeclarationState::Known,
            Some(_) => ReadGroupDeclarationState::Ambiguous,
            None => ReadGroupDeclarationState::Unknown,
        }
    }
}

struct ParsedTextHeader {
    references: Vec<ReferenceSequence>,
    sort_order: SortOrder,
    read_groups: BTreeMap<String, Vec<ReadGroupDefinition>>,
}

impl ParsedTextHeader {
    #[allow(clippy::too_many_lines)]
    fn parse(text: &str) -> Result<Self, AlignGaugeError> {
        let mut references = Vec::new();
        let mut reference_names = BTreeSet::new();
        let mut read_groups: BTreeMap<String, Vec<ReadGroupDefinition>> = BTreeMap::new();
        let mut sort_order = SortOrder::Absent;
        let mut saw_hd = false;
        let mut read_group_count = 0_usize;

        for (line_index, line) in text.split_terminator('\n').enumerate() {
            let line_number = line_index.checked_add(1).ok_or_else(|| {
                AlignGaugeError::new(
                    ErrorCategory::InternalInvariant,
                    "header line-number arithmetic overflowed",
                )
            })?;
            let line = line.strip_suffix('\r').unwrap_or(line);
            if line.is_empty() {
                continue;
            }
            if !line.starts_with('@') {
                return Err(header_line_error(
                    line_number,
                    "header line does not begin with '@'",
                ));
            }
            if line.starts_with("@CO") {
                continue;
            }

            let mut columns = line.split('\t');
            let record_type = columns.next().unwrap_or_default();
            let fields = parse_fields(columns, line_number)?;

            match record_type {
                "@HD" => {
                    if saw_hd {
                        return Err(header_line_error(
                            line_number,
                            "header contains more than one @HD record",
                        ));
                    }
                    saw_hd = true;
                    sort_order = fields.get("SO").map_or(SortOrder::Absent, |value| {
                        match value.as_str() {
                            "coordinate" => SortOrder::Coordinate,
                            "queryname" => SortOrder::QueryName,
                            "unsorted" => SortOrder::Unsorted,
                            "unknown" => SortOrder::Unknown,
                            other => SortOrder::Other(other.to_owned()),
                        }
                    });
                }
                "@SQ" => {
                    if references.len() >= MAX_REFERENCE_COUNT {
                        return Err(header_line_error(
                            line_number,
                            "header reference count exceeds the supported limit",
                        ));
                    }
                    let name = required_field(&fields, "SN", line_number)?;
                    if name.len() > MAX_REFERENCE_NAME_BYTES {
                        return Err(header_line_error(
                            line_number,
                            "reference name exceeds the supported byte limit",
                        )
                        .with_detail("reference_name_bytes", u64_from_usize(name.len())?));
                    }
                    if !reference_names.insert(name.to_owned()) {
                        return Err(header_line_error(
                            line_number,
                            "duplicate or contradictory @SQ reference declaration",
                        )
                        .with_detail("reference_name", name.to_owned()));
                    }
                    let length_text = required_field(&fields, "LN", line_number)?;
                    let length = length_text.parse::<u64>().map_err(|source| {
                        header_line_error(line_number, "reference length is not a valid u64")
                            .with_detail("reference_name", name.to_owned())
                            .with_detail("reference_length", length_text.to_owned())
                            .with_source(source)
                    })?;
                    references.push(ReferenceSequence {
                        name: name.to_owned(),
                        length,
                    });
                }
                "@RG" => {
                    read_group_count = read_group_count.checked_add(1).ok_or_else(|| {
                        AlignGaugeError::new(
                            ErrorCategory::InternalInvariant,
                            "read-group count overflowed",
                        )
                    })?;
                    if read_group_count > MAX_READ_GROUP_COUNT {
                        return Err(header_line_error(
                            line_number,
                            "read-group count exceeds the supported limit",
                        ));
                    }
                    let id = required_field(&fields, "ID", line_number)?.to_owned();
                    read_groups
                        .entry(id.clone())
                        .or_default()
                        .push(ReadGroupDefinition { id, fields });
                }
                _ => {}
            }
        }

        Ok(Self {
            references,
            sort_order,
            read_groups,
        })
    }
}

fn parse_fields<'a>(
    columns: impl Iterator<Item = &'a str>,
    line_number: usize,
) -> Result<BTreeMap<String, String>, AlignGaugeError> {
    let mut fields = BTreeMap::new();
    for (field_index, field) in columns.enumerate() {
        if field_index >= MAX_HEADER_FIELDS {
            return Err(header_line_error(
                line_number,
                "header record has too many fields",
            ));
        }
        if field.len() > MAX_HEADER_FIELD_BYTES {
            return Err(header_line_error(
                line_number,
                "header field exceeds the supported byte limit",
            ));
        }
        let (tag, value) = field.split_once(':').ok_or_else(|| {
            header_line_error(line_number, "header field does not contain ':'")
        })?;
        if tag.len() != 2 || !tag.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
            return Err(header_line_error(line_number, "header tag is invalid"));
        }
        if fields.insert(tag.to_owned(), value.to_owned()).is_some() {
            return Err(header_line_error(
                line_number,
                "header record repeats a field tag",
            )
            .with_detail("tag", tag.to_owned()));
        }
    }
    Ok(fields)
}

fn required_field<'a>(
    fields: &'a BTreeMap<String, String>,
    tag: &str,
    line_number: usize,
) -> Result<&'a str, AlignGaugeError> {
    fields
        .get(tag)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            header_line_error(line_number, "header record is missing a required field")
                .with_detail("required_tag", tag.to_owned())
        })
}

fn validate_binary_references(
    view: &HeaderView,
    textual: &[ReferenceSequence],
) -> Result<Vec<ReferenceSequence>, AlignGaugeError> {
    let count = usize::try_from(view.target_count()).map_err(|source| {
        header_error("binary reference count does not fit usize").with_source(source)
    })?;
    if count > MAX_REFERENCE_COUNT {
        return Err(header_error("binary reference count exceeds the supported limit")
            .with_detail("reference_count", u64_from_usize(count)?));
    }
    if count != textual.len() {
        return Err(header_error(
            "textual @SQ declarations do not match the binary reference table",
        )
        .with_detail("text_reference_count", u64_from_usize(textual.len())?)
        .with_detail("binary_reference_count", u64_from_usize(count)?));
    }

    let names = view.target_names();
    let mut references = Vec::with_capacity(count);
    for (index, expected) in textual.iter().enumerate() {
        let index_value = u64_from_usize(index)?;
        let binary_name = names.get(index).ok_or_else(|| {
            header_error("binary reference name table ended unexpectedly")
                .with_detail("reference_index", index_value)
        })?;
        let binary_name = std::str::from_utf8(binary_name).map_err(|source| {
            header_error("binary reference name is not valid UTF-8")
                .with_detail("reference_index", index_value)
                .with_source(source)
        })?;
        let target_id = u32::try_from(index).map_err(|source| {
            header_error("reference index does not fit u32").with_source(source)
        })?;
        let binary_length = view.target_len(target_id).ok_or_else(|| {
            header_error("binary reference length is unavailable")
                .with_detail("reference_index", index_value)
        })?;

        if expected.name != binary_name || expected.length != binary_length {
            return Err(header_error(
                "textual @SQ declaration contradicts the binary reference table",
            )
            .with_detail("reference_index", index_value)
            .with_detail("text_name", expected.name.clone())
            .with_detail("binary_name", binary_name.to_owned())
            .with_detail("text_length", expected.length)
            .with_detail("binary_length", binary_length));
        }
        references.push(expected.clone());
    }
    Ok(references)
}

fn build_identity(
    raw_header: &[u8],
    references: &[ReferenceSequence],
) -> Result<HeaderIdentity, AlignGaugeError> {
    let mut hasher = Sha256::new();
    hasher.update(b"aligngauge-header-v1\0");
    hasher.update(
        u64::try_from(raw_header.len())
            .map_err(|source| {
                AlignGaugeError::new(
                    ErrorCategory::InternalInvariant,
                    "header length does not fit u64",
                )
                .with_source(source)
            })?
            .to_le_bytes(),
    );
    hasher.update(raw_header);
    for reference in references {
        hasher.update(
            u64::try_from(reference.name.len())
                .map_err(|source| {
                    AlignGaugeError::new(
                        ErrorCategory::InternalInvariant,
                        "reference-name length does not fit u64",
                    )
                    .with_source(source)
                })?
                .to_le_bytes(),
        );
        hasher.update(reference.name.as_bytes());
        hasher.update(reference.length.to_le_bytes());
    }

    let digest = hasher.finalize();
    let mut sha256 = String::with_capacity(64);
    for byte in digest {
        write!(sha256, "{byte:02x}").expect("writing to String cannot fail");
    }

    Ok(HeaderIdentity {
        sha256,
        raw_header_bytes: raw_header.len(),
        reference_count: references.len(),
    })
}

fn header_error(message: impl Into<String>) -> AlignGaugeError {
    AlignGaugeError::new(ErrorCategory::InputFormat, message)
}

fn header_line_error(line_number: usize, message: impl Into<String>) -> AlignGaugeError {
    header_error(message).with_detail(
        "header_line",
        u64_from_usize(line_number).unwrap_or(u64::MAX),
    )
}

fn u64_from_usize(value: usize) -> Result<u64, AlignGaugeError> {
    u64::try_from(value).map_err(|source| {
        AlignGaugeError::new(
            ErrorCategory::InternalInvariant,
            "usize value does not fit u64",
        )
        .with_source(source)
    })
}
