//! Production BAM/CRAM streaming, coordinate checks, and record validation.

use std::fs::File;
use std::io::{Read as IoRead, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use aligngauge_core::{AlignGaugeError, ErrorCategory};
use flate2::read::MultiGzDecoder;
use rust_htslib::bam::record::Aux;
use rust_htslib::bam::{Read, Reader, Record};

use crate::header::{ReadGroupDeclarationState, ValidatedHeader};
use crate::plan::{FieldPlan, RequiredField};
use crate::reference::{LocalReferenceIdentity, validate_local_reference};

const MAX_IO_THREADS: usize = 64;
const MAX_RECORD_BYTES: usize = 256 * 1024 * 1024;
const MAX_QUERY_NAME_BYTES: usize = 1024 * 1024;
const MAX_SEQUENCE_BASES: usize = 100_000_000;
const MAX_CIGAR_OPERATIONS: usize = 1_000_000;
const MAX_AUXILIARY_FIELDS: usize = 65_536;
const STANDARD_FLAG_MASK: u16 = 0x0fff;
const LONG_CIGAR_LIMIT: usize = 65_535;

/// Physical alignment container detected before `HTSlib` traversal.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AlignmentFormat {
    /// BGZF-compressed BAM.
    Bam,
    /// CRAM with an explicit local reference.
    Cram,
}

impl AlignmentFormat {
    /// Stable lower-case format name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bam => "bam",
            Self::Cram => "cram",
        }
    }
}

/// Reader resource controls resolved before traversal.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ReaderOptions {
    /// Number of `HTSlib` background decode threads. `1` keeps decoding serial.
    pub io_threads: usize,
}

impl Default for ReaderOptions {
    fn default() -> Self {
        Self { io_threads: 1 }
    }
}

/// A validated zero-based BAM coordinate.
#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub struct RecordCoordinate {
    /// Reference table index.
    pub reference_id: i32,
    /// Zero-based position.
    pub position: i64,
}

/// A requested field that may be absent in a valid record.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FieldValue<T> {
    /// The field was not part of the resolved plan.
    NotRequested,
    /// The field was requested but is absent from this record.
    Missing,
    /// The field was requested and validated.
    Value(T),
}

/// Explicit read-group resolution without invented groups.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ReadGroupValue {
    /// The `RG` tag was not requested.
    NotRequested,
    /// No `RG` tag is present.
    Missing,
    /// Exactly one matching header declaration exists.
    Known(String),
    /// No matching header declaration exists.
    Unknown(String),
    /// Multiple matching header declarations exist.
    Ambiguous(String),
}

/// Validated CIGAR arithmetic for one record.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CigarFacts {
    /// Decoded operation count.
    pub operation_count: usize,
    /// Query-consuming length.
    pub query_span: u64,
    /// Reference-consuming length.
    pub reference_span: u64,
    /// Whether `HTSlib` expanded a BAM long-CIGAR representation.
    pub long_cigar_expanded: bool,
}

/// One validated record borrowed until the next reader call.
#[derive(Debug)]
pub struct ValidatedRecord<'a> {
    record: &'a Record,
    index: u64,
    flags: u16,
    coordinate: Option<RecordCoordinate>,
    mate_coordinate: FieldValue<Option<RecordCoordinate>>,
    mapping_quality: FieldValue<u8>,
    cigar: FieldValue<CigarFacts>,
    edit_distance: FieldValue<u64>,
    mismatch_descriptor: FieldValue<String>,
    read_group: ReadGroupValue,
}

impl ValidatedRecord<'_> {
    /// One-based record index in traversal order.
    #[must_use]
    pub const fn index(&self) -> u64 {
        self.index
    }

    /// Validated SAM flags.
    #[must_use]
    pub const fn flags(&self) -> u16 {
        self.flags
    }

    /// Whether the SAM unmapped flag is set.
    #[must_use]
    pub const fn is_unmapped(&self) -> bool {
        self.flags & 0x4 != 0
    }

    /// Coordinate, or `None` for a no-coordinate tail record.
    #[must_use]
    pub const fn coordinate(&self) -> Option<RecordCoordinate> {
        self.coordinate
    }

    /// Planned mate coordinate. `Value(None)` means unpaired or mate-unmapped.
    #[must_use]
    pub const fn mate_coordinate(&self) -> &FieldValue<Option<RecordCoordinate>> {
        &self.mate_coordinate
    }

    /// Planned mapping quality.
    #[must_use]
    pub const fn mapping_quality(&self) -> &FieldValue<u8> {
        &self.mapping_quality
    }

    /// Planned and validated CIGAR facts.
    #[must_use]
    pub const fn cigar(&self) -> &FieldValue<CigarFacts> {
        &self.cigar
    }

    /// Raw CIGAR operations only when the plan requested CIGAR access.
    #[must_use]
    pub fn raw_cigar(&self) -> Option<&[u32]> {
        matches!(&self.cigar, FieldValue::Value(_)).then(|| self.record.raw_cigar())
    }

    /// Optional `NM` value. Missing is never represented as zero.
    #[must_use]
    pub const fn edit_distance(&self) -> &FieldValue<u64> {
        &self.edit_distance
    }

    /// Optional `MD` value. Missing is never represented as an empty string.
    #[must_use]
    pub const fn mismatch_descriptor(&self) -> &FieldValue<String> {
        &self.mismatch_descriptor
    }

    /// Explicit read-group declaration state.
    #[must_use]
    pub const fn read_group(&self) -> &ReadGroupValue {
        &self.read_group
    }
}

/// Single-pass alignment reader with strict header, record, order, and CRAM reference validation.
pub struct BamReader {
    input: PathBuf,
    input_format: AlignmentFormat,
    reference_identity: Option<LocalReferenceIdentity>,
    reader: Reader,
    record: Record,
    header: ValidatedHeader,
    field_plan: FieldPlan,
    record_index: u64,
    previous_coordinate: Option<RecordCoordinate>,
    no_coordinate_tail_started: bool,
}

impl BamReader {
    /// Open and validate a local BAM input.
    ///
    /// # Errors
    /// Returns a typed error for missing/non-BAM input, invalid options, `HTSlib` failures, or
    /// malformed header content.
    pub fn open(
        input: impl AsRef<Path>,
        field_plan: FieldPlan,
        options: ReaderOptions,
    ) -> Result<Self, AlignGaugeError> {
        validate_plan(&field_plan)?;
        validate_options(options)?;
        let input = input.as_ref().to_path_buf();
        verify_bam_signature(&input)?;
        let reader = open_htslib_reader(&input, options, AlignmentFormat::Bam)?;
        Self::finish_open(input, AlignmentFormat::Bam, None, reader, field_plan)
    }

    /// Open a local CRAM only after validating an explicit local FASTA.
    ///
    /// Network transports are absent from the pinned `HTSlib` build. The supplied FASTA is checked
    /// against CRAM `@SQ` SN/LN/M5 before `set_reference` is called, so a mismatch cannot fall
    /// through to inherited `REF_CACHE`, `REF_PATH`, or `UR` reference providers.
    ///
    /// # Errors
    /// Returns a typed error for missing/corrupt CRAM, invalid options, missing/mismatched FASTA,
    /// `HTSlib` reference configuration failures, or malformed header content.
    pub fn open_cram(
        input: impl AsRef<Path>,
        reference: impl AsRef<Path>,
        field_plan: FieldPlan,
        options: ReaderOptions,
    ) -> Result<Self, AlignGaugeError> {
        validate_plan(&field_plan)?;
        validate_options(options)?;
        let input = input.as_ref().to_path_buf();
        let reference = reference.as_ref();
        verify_cram_signature(&input)?;
        let mut reader = open_htslib_reader(&input, options, AlignmentFormat::Cram)?;
        let reference_identity = validate_local_reference(reader.header(), reference)?;
        reader.set_reference(reference).map_err(|source| {
            AlignGaugeError::new(
                ErrorCategory::ReferenceMismatch,
                "HTSlib rejected the validated local CRAM reference",
            )
            .with_detail("reference", reference.to_string_lossy().into_owned())
            .with_source(source)
        })?;
        Self::finish_open(
            input,
            AlignmentFormat::Cram,
            Some(reference_identity),
            reader,
            field_plan,
        )
    }

    fn finish_open(
        input: PathBuf,
        input_format: AlignmentFormat,
        reference_identity: Option<LocalReferenceIdentity>,
        reader: Reader,
        field_plan: FieldPlan,
    ) -> Result<Self, AlignGaugeError> {
        let header = ValidatedHeader::from_view(reader.header())?;
        Ok(Self {
            input,
            input_format,
            reference_identity,
            reader,
            record: Record::new(),
            header,
            field_plan,
            record_index: 0,
            previous_coordinate: None,
            no_coordinate_tail_started: false,
        })
    }

    /// Detected physical alignment format.
    #[must_use]
    pub const fn input_format(&self) -> AlignmentFormat {
        self.input_format
    }

    /// Validated local reference identity for CRAM, or `None` for BAM.
    #[must_use]
    pub const fn reference_identity(&self) -> Option<&LocalReferenceIdentity> {
        self.reference_identity.as_ref()
    }

    /// Validated input header.
    #[must_use]
    pub const fn header(&self) -> &ValidatedHeader {
        &self.header
    }

    /// Resolved required-field plan.
    #[must_use]
    pub const fn field_plan(&self) -> &FieldPlan {
        &self.field_plan
    }

    /// Read, validate, and borrow the next record.
    ///
    /// # Errors
    ///
    /// Returns a typed failure for decode corruption, unsupported records,
    /// invalid coordinates/CIGAR/tags, or coordinate-order regressions.
    pub fn next_record(&mut self) -> Result<Option<ValidatedRecord<'_>>, AlignGaugeError> {
        let Some(result) = self.reader.read(&mut self.record) else {
            return Ok(None);
        };
        result.map_err(|source| {
            AlignGaugeError::new(
                ErrorCategory::InputCorrupt,
                format!(
                    "failed to decode alignment record from '{}'",
                    self.input.display()
                ),
            )
            .with_detail("input", self.input.to_string_lossy().into_owned())
            .with_detail("next_record_index", self.record_index.saturating_add(1))
            .with_source(source)
        })?;

        self.record_index = self.record_index.checked_add(1).ok_or_else(|| {
            AlignGaugeError::new(
                ErrorCategory::InternalInvariant,
                "alignment traversal record index overflowed",
            )
        })?;
        let facts = validate_record(
            &self.record,
            &self.header,
            &self.field_plan,
            self.record_index,
        )?;
        self.validate_coordinate_order(facts.coordinate)?;

        Ok(Some(ValidatedRecord {
            record: &self.record,
            index: self.record_index,
            flags: facts.flags,
            coordinate: facts.coordinate,
            mate_coordinate: facts.mate_coordinate,
            mapping_quality: facts.mapping_quality,
            cigar: facts.cigar,
            edit_distance: facts.edit_distance,
            mismatch_descriptor: facts.mismatch_descriptor,
            read_group: facts.read_group,
        }))
    }

    fn validate_coordinate_order(
        &mut self,
        coordinate: Option<RecordCoordinate>,
    ) -> Result<(), AlignGaugeError> {
        let Some(current) = coordinate else {
            self.no_coordinate_tail_started = true;
            return Ok(());
        };

        if self.no_coordinate_tail_started {
            return Err(order_error(
                "coordinate-bearing record appears after the no-coordinate tail",
                self.record_index,
                self.previous_coordinate,
                current,
                &self.record,
            ));
        }
        if self
            .previous_coordinate
            .is_some_and(|previous| current < previous)
        {
            return Err(order_error(
                "alignment record coordinates regress",
                self.record_index,
                self.previous_coordinate,
                current,
                &self.record,
            ));
        }
        self.previous_coordinate = Some(current);
        Ok(())
    }
}

struct RecordLayout {
    query_name_bytes: usize,
    sequence_bases: usize,
}

fn validate_record_layout(record: &Record, index: u64) -> Result<RecordLayout, AlignGaugeError> {
    let inner = record.inner();
    let data_len = usize::try_from(inner.l_data).map_err(|source| {
        record_error_without_name(
            ErrorCategory::InputCorrupt,
            "BAM record has a negative or unrepresentable data length",
            index,
        )
        .with_source(source)
    })?;
    if data_len > MAX_RECORD_BYTES {
        return Err(record_error_without_name(
            ErrorCategory::ResourceLimit,
            "BAM record exceeds the supported byte limit",
            index,
        )
        .with_detail("record_bytes", u64_from_usize(data_len)?)
        .with_detail("maximum_record_bytes", u64_from_usize(MAX_RECORD_BYTES)?));
    }

    let query_name_bytes = usize::from(inner.core.l_qname);
    let extra_nul_bytes = usize::from(inner.core.l_extranul);
    if query_name_bytes == 0 || extra_nul_bytes > 3 || extra_nul_bytes >= query_name_bytes {
        return Err(record_error_without_name(
            ErrorCategory::InputCorrupt,
            "BAM record has an invalid query-name layout",
            index,
        )
        .with_detail(
            "query_name_storage_bytes",
            u64_from_usize(query_name_bytes)?,
        )
        .with_detail("extra_nul_bytes", u64_from_usize(extra_nul_bytes)?));
    }

    let cigar_operations = usize::try_from(inner.core.n_cigar).map_err(|source| {
        record_error_without_name(
            ErrorCategory::InputCorrupt,
            "BAM CIGAR operation count does not fit usize",
            index,
        )
        .with_source(source)
    })?;
    let cigar_bytes = cigar_operations.checked_mul(4).ok_or_else(|| {
        record_error_without_name(
            ErrorCategory::InputCorrupt,
            "BAM CIGAR storage length overflows usize",
            index,
        )
    })?;
    let sequence_bases = usize::try_from(inner.core.l_qseq).map_err(|source| {
        record_error_without_name(
            ErrorCategory::InputCorrupt,
            "BAM record has a negative or unrepresentable sequence length",
            index,
        )
        .with_source(source)
    })?;
    let packed_sequence_bytes = sequence_bases
        .checked_add(1)
        .and_then(|value| value.checked_div(2))
        .ok_or_else(|| {
            record_error_without_name(
                ErrorCategory::InputCorrupt,
                "BAM packed-sequence storage length overflows usize",
                index,
            )
        })?;
    let minimum_data_bytes = query_name_bytes
        .checked_add(cigar_bytes)
        .and_then(|value| value.checked_add(packed_sequence_bytes))
        .and_then(|value| value.checked_add(sequence_bases))
        .ok_or_else(|| {
            record_error_without_name(
                ErrorCategory::InputCorrupt,
                "BAM variable-data layout overflows usize",
                index,
            )
        })?;
    if minimum_data_bytes > data_len {
        return Err(record_error_without_name(
            ErrorCategory::InputCorrupt,
            "BAM variable-data fields exceed the decoded record buffer",
            index,
        )
        .with_detail("record_bytes", u64_from_usize(data_len)?)
        .with_detail("minimum_record_bytes", u64_from_usize(minimum_data_bytes)?));
    }

    Ok(RecordLayout {
        query_name_bytes,
        sequence_bases,
    })
}

struct RecordFacts {
    flags: u16,
    coordinate: Option<RecordCoordinate>,
    mate_coordinate: FieldValue<Option<RecordCoordinate>>,
    mapping_quality: FieldValue<u8>,
    cigar: FieldValue<CigarFacts>,
    edit_distance: FieldValue<u64>,
    mismatch_descriptor: FieldValue<String>,
    read_group: ReadGroupValue,
}

#[allow(clippy::too_many_lines)]
fn validate_record(
    record: &Record,
    header: &ValidatedHeader,
    plan: &FieldPlan,
    index: u64,
) -> Result<RecordFacts, AlignGaugeError> {
    let layout = validate_record_layout(record, index)?;
    if layout.query_name_bytes > MAX_QUERY_NAME_BYTES {
        return Err(record_error(
            ErrorCategory::ResourceLimit,
            "BAM query name exceeds the supported byte limit",
            index,
            record,
        ));
    }
    if layout.sequence_bases > MAX_SEQUENCE_BASES {
        return Err(record_error(
            ErrorCategory::ResourceLimit,
            "BAM sequence exceeds the supported base limit",
            index,
            record,
        )
        .with_detail("sequence_bases", u64_from_usize(layout.sequence_bases)?));
    }

    let flags = record.flags();
    validate_flags(flags, index, record)?;
    let coordinate = validate_coordinate(record, header, index)?;
    let mate_coordinate = validate_mate_coordinate(record, header, plan, index)?;
    let mapping_quality = if plan.requires(RequiredField::MappingQuality) {
        FieldValue::Value(record.mapq())
    } else {
        FieldValue::NotRequested
    };
    let cigar_facts = validate_cigar(record, header, coordinate, index)?;
    let tags = validate_auxiliary(record, header, plan, index, cigar_facts.operation_count)?;

    Ok(RecordFacts {
        flags,
        coordinate,
        mate_coordinate,
        mapping_quality,
        cigar: if plan.requires(RequiredField::Cigar) {
            FieldValue::Value(cigar_facts)
        } else {
            FieldValue::NotRequested
        },
        edit_distance: tags.edit_distance,
        mismatch_descriptor: tags.mismatch_descriptor,
        read_group: tags.read_group,
    })
}

fn validate_flags(flags: u16, index: u64, record: &Record) -> Result<(), AlignGaugeError> {
    if flags & !STANDARD_FLAG_MASK != 0 {
        return Err(record_error(
            ErrorCategory::UnsupportedRecord,
            "BAM record uses reserved flag bits",
            index,
            record,
        )
        .with_detail("flags", u64::from(flags)));
    }
    let paired = flags & 0x1 != 0;
    let proper_pair = flags & 0x2 != 0;
    let read1 = flags & 0x40 != 0;
    let read2 = flags & 0x80 != 0;
    if proper_pair && !paired {
        return Err(record_error(
            ErrorCategory::InputCorrupt,
            "proper-pair flag is set on an unpaired record",
            index,
            record,
        ));
    }
    if (read1 || read2) && !paired {
        return Err(record_error(
            ErrorCategory::InputCorrupt,
            "read1/read2 flag is set on an unpaired record",
            index,
            record,
        ));
    }
    if read1 && read2 {
        return Err(record_error(
            ErrorCategory::InputCorrupt,
            "read1 and read2 flags are both set",
            index,
            record,
        ));
    }
    Ok(())
}

fn validate_coordinate(
    record: &Record,
    header: &ValidatedHeader,
    index: u64,
) -> Result<Option<RecordCoordinate>, AlignGaugeError> {
    let target_id = record.tid();
    let position = record.pos();
    if target_id < -1 || position < -1 {
        return Err(record_error(
            ErrorCategory::InputCorrupt,
            "BAM record has a coordinate below the no-coordinate sentinel",
            index,
            record,
        )
        .with_detail("reference_id", i64::from(target_id))
        .with_detail("position", position));
    }
    if (target_id == -1) != (position == -1) {
        return Err(record_error(
            ErrorCategory::InputCorrupt,
            "BAM record has only one no-coordinate sentinel",
            index,
            record,
        )
        .with_detail("reference_id", i64::from(target_id))
        .with_detail("position", position));
    }
    if target_id == -1 {
        if !record.is_unmapped() {
            return Err(record_error(
                ErrorCategory::InputCorrupt,
                "mapped BAM record has no coordinate",
                index,
                record,
            ));
        }
        return Ok(None);
    }
    if header.reference(target_id).is_none() {
        return Err(record_error(
            ErrorCategory::InputCorrupt,
            "BAM record references an unknown target ID",
            index,
            record,
        )
        .with_detail("reference_id", i64::from(target_id)));
    }
    Ok(Some(RecordCoordinate {
        reference_id: target_id,
        position,
    }))
}

fn validate_mate_coordinate(
    record: &Record,
    header: &ValidatedHeader,
    plan: &FieldPlan,
    index: u64,
) -> Result<FieldValue<Option<RecordCoordinate>>, AlignGaugeError> {
    if !plan.requires(RequiredField::MateCoordinates) {
        return Ok(FieldValue::NotRequested);
    }

    let flags = record.flags();
    if flags & 0x1 == 0 {
        return Ok(FieldValue::Value(None));
    }

    let target_id = record.mtid();
    let position = record.mpos();
    if target_id < -1 || position < -1 {
        return Err(record_error(
            ErrorCategory::InputCorrupt,
            "BAM mate coordinate is below the no-coordinate sentinel",
            index,
            record,
        )
        .with_detail("mate_reference_id", i64::from(target_id))
        .with_detail("mate_position", position));
    }

    let mate_unmapped = flags & 0x8 != 0;
    if mate_unmapped && target_id == -1 && position == -1 {
        return Ok(FieldValue::Value(None));
    }
    if (target_id == -1) != (position == -1) {
        return Err(record_error(
            ErrorCategory::InputCorrupt,
            "BAM mate has only one no-coordinate sentinel",
            index,
            record,
        )
        .with_detail("mate_reference_id", i64::from(target_id))
        .with_detail("mate_position", position));
    }
    if target_id == -1 {
        return Err(record_error(
            ErrorCategory::InputCorrupt,
            "mapped mate has no coordinate",
            index,
            record,
        ));
    }

    let reference = header.reference(target_id).ok_or_else(|| {
        record_error(
            ErrorCategory::InputCorrupt,
            "BAM mate references an unknown target ID",
            index,
            record,
        )
        .with_detail("mate_reference_id", i64::from(target_id))
    })?;
    let position_u64 = u64::try_from(position).map_err(|source| {
        record_error(
            ErrorCategory::InputCorrupt,
            "BAM mate position does not fit u64",
            index,
            record,
        )
        .with_source(source)
    })?;
    if position_u64 >= reference.length() {
        return Err(record_error(
            ErrorCategory::InputCorrupt,
            "BAM mate position lies outside the declared reference",
            index,
            record,
        )
        .with_detail("mate_reference_id", i64::from(target_id))
        .with_detail("mate_position", position)
        .with_detail("reference_length", reference.length()));
    }

    if mate_unmapped {
        Ok(FieldValue::Value(None))
    } else {
        Ok(FieldValue::Value(Some(RecordCoordinate {
            reference_id: target_id,
            position,
        })))
    }
}

#[allow(clippy::too_many_lines)]
fn validate_cigar(
    record: &Record,
    header: &ValidatedHeader,
    coordinate: Option<RecordCoordinate>,
    index: u64,
) -> Result<CigarFacts, AlignGaugeError> {
    let raw = record.raw_cigar();
    if raw.len() > MAX_CIGAR_OPERATIONS {
        return Err(record_error(
            ErrorCategory::ResourceLimit,
            "BAM CIGAR exceeds the supported operation limit",
            index,
            record,
        )
        .with_detail("cigar_operations", u64_from_usize(raw.len())?));
    }

    let mut query_span = 0_u64;
    let mut reference_span = 0_u64;
    for encoded in raw {
        let length = u64::from(encoded >> 4);
        let operation = encoded & 0x0f;
        if length == 0 {
            return Err(record_error(
                ErrorCategory::InputCorrupt,
                "BAM CIGAR contains a zero-length operation",
                index,
                record,
            ));
        }
        match operation {
            0 | 7 | 8 => {
                query_span = checked_span_add(query_span, length, "query", index, record)?;
                reference_span =
                    checked_span_add(reference_span, length, "reference", index, record)?;
            }
            1 | 4 => {
                query_span = checked_span_add(query_span, length, "query", index, record)?;
            }
            2 | 3 => {
                reference_span =
                    checked_span_add(reference_span, length, "reference", index, record)?;
            }
            5 | 6 => {}
            _ => {
                return Err(record_error(
                    ErrorCategory::InputCorrupt,
                    "BAM CIGAR contains an unknown operation code",
                    index,
                    record,
                )
                .with_detail("cigar_operation_code", u64::from(operation)));
            }
        }
    }

    if !raw.is_empty() && query_span != u64_from_usize(record.seq_len())? {
        return Err(record_error(
            ErrorCategory::InputCorrupt,
            "BAM CIGAR query span does not match sequence length",
            index,
            record,
        )
        .with_detail("cigar_query_span", query_span)
        .with_detail("sequence_bases", u64_from_usize(record.seq_len())?));
    }
    if !record.is_unmapped() && raw.is_empty() {
        return Err(record_error(
            ErrorCategory::InputCorrupt,
            "mapped BAM record has an empty CIGAR",
            index,
            record,
        ));
    }

    if let Some(coordinate) = coordinate {
        let reference = header.reference(coordinate.reference_id).ok_or_else(|| {
            AlignGaugeError::new(
                ErrorCategory::InternalInvariant,
                "validated coordinate lost its reference",
            )
        })?;
        let start = u64::try_from(coordinate.position).map_err(|source| {
            record_error(
                ErrorCategory::InputCorrupt,
                "BAM position does not fit u64",
                index,
                record,
            )
            .with_source(source)
        })?;
        let end = start.checked_add(reference_span).ok_or_else(|| {
            record_error(
                ErrorCategory::InputCorrupt,
                "BAM reference span overflows u64",
                index,
                record,
            )
        })?;
        if end > reference.length() {
            return Err(record_error(
                ErrorCategory::InputCorrupt,
                "BAM CIGAR extends beyond the declared reference length",
                index,
                record,
            )
            .with_detail("reference_id", i64::from(coordinate.reference_id))
            .with_detail("position", coordinate.position)
            .with_detail("reference_span", reference_span)
            .with_detail("reference_length", reference.length()));
        }
    }

    Ok(CigarFacts {
        operation_count: raw.len(),
        query_span,
        reference_span,
        long_cigar_expanded: raw.len() > LONG_CIGAR_LIMIT,
    })
}

fn checked_span_add(
    current: u64,
    length: u64,
    span_name: &'static str,
    index: u64,
    record: &Record,
) -> Result<u64, AlignGaugeError> {
    current.checked_add(length).ok_or_else(|| {
        record_error(
            ErrorCategory::InputCorrupt,
            format!("BAM CIGAR {span_name} span overflows u64"),
            index,
            record,
        )
    })
}

struct TagFacts {
    edit_distance: FieldValue<u64>,
    mismatch_descriptor: FieldValue<String>,
    read_group: ReadGroupValue,
}

#[allow(clippy::too_many_lines)]
fn validate_auxiliary(
    record: &Record,
    header: &ValidatedHeader,
    plan: &FieldPlan,
    index: u64,
    cigar_operation_count: usize,
) -> Result<TagFacts, AlignGaugeError> {
    let mut field_count = 0_usize;
    let mut edit_distance = None;
    let mut mismatch_descriptor = None;
    let mut read_group = None;
    let mut saw_cg = false;

    for item in record.aux_iter() {
        field_count = field_count.checked_add(1).ok_or_else(|| {
            AlignGaugeError::new(
                ErrorCategory::InternalInvariant,
                "auxiliary field count overflowed",
            )
        })?;
        if field_count > MAX_AUXILIARY_FIELDS {
            return Err(record_error(
                ErrorCategory::ResourceLimit,
                "BAM record has too many auxiliary fields",
                index,
                record,
            ));
        }
        let (tag, value) = item.map_err(|source| {
            record_error(
                ErrorCategory::InputCorrupt,
                "BAM record contains malformed auxiliary data",
                index,
                record,
            )
            .with_source(source)
        })?;
        if tag.len() != 2 {
            return Err(record_error(
                ErrorCategory::InputCorrupt,
                "BAM auxiliary tag is not two bytes",
                index,
                record,
            ));
        }

        match tag {
            b"CG" => {
                saw_cg = true;
                if !matches!(value, Aux::ArrayU32(_) | Aux::ArrayI32(_)) {
                    return Err(record_error(
                        ErrorCategory::UnsupportedRecord,
                        "CG tag is not a supported long-CIGAR integer array",
                        index,
                        record,
                    ));
                }
            }
            b"NM" if plan.requires(RequiredField::EditDistance) => {
                if edit_distance.is_some() {
                    return Err(duplicate_tag_error("NM", index, record));
                }
                edit_distance = Some(parse_nonnegative_integer(&value, "NM", index, record)?);
            }
            b"MD" if plan.requires(RequiredField::MismatchDescriptor) => {
                if mismatch_descriptor.is_some() {
                    return Err(duplicate_tag_error("MD", index, record));
                }
                mismatch_descriptor = Some(match value {
                    Aux::String(value) => value.to_owned(),
                    _ => {
                        return Err(record_error(
                            ErrorCategory::InputCorrupt,
                            "MD tag is not a string",
                            index,
                            record,
                        ));
                    }
                });
            }
            b"RG" if plan.requires(RequiredField::ReadGroup) => {
                if read_group.is_some() {
                    return Err(duplicate_tag_error("RG", index, record));
                }
                read_group = Some(match value {
                    Aux::String(value) => value.to_owned(),
                    _ => {
                        return Err(record_error(
                            ErrorCategory::InputCorrupt,
                            "RG tag is not a string",
                            index,
                            record,
                        ));
                    }
                });
            }
            _ => {}
        }
    }

    if saw_cg && cigar_operation_count <= LONG_CIGAR_LIMIT {
        return Err(record_error(
            ErrorCategory::UnsupportedRecord,
            "CG long-CIGAR tag was not expanded by the pinned backend",
            index,
            record,
        ));
    }

    let edit_distance = if plan.requires(RequiredField::EditDistance) {
        edit_distance.map_or(FieldValue::Missing, FieldValue::Value)
    } else {
        FieldValue::NotRequested
    };
    let mismatch_descriptor = if plan.requires(RequiredField::MismatchDescriptor) {
        mismatch_descriptor.map_or(FieldValue::Missing, FieldValue::Value)
    } else {
        FieldValue::NotRequested
    };
    let read_group = if plan.requires(RequiredField::ReadGroup) {
        read_group.map_or(ReadGroupValue::Missing, |id| {
            match header.read_group_state(&id) {
                ReadGroupDeclarationState::Known => ReadGroupValue::Known(id),
                ReadGroupDeclarationState::Unknown => ReadGroupValue::Unknown(id),
                ReadGroupDeclarationState::Ambiguous => ReadGroupValue::Ambiguous(id),
            }
        })
    } else {
        ReadGroupValue::NotRequested
    };

    Ok(TagFacts {
        edit_distance,
        mismatch_descriptor,
        read_group,
    })
}

fn parse_nonnegative_integer(
    value: &Aux<'_>,
    tag: &'static str,
    index: u64,
    record: &Record,
) -> Result<u64, AlignGaugeError> {
    let value = match value {
        Aux::I8(value) => i64::from(*value),
        Aux::U8(value) => i64::from(*value),
        Aux::I16(value) => i64::from(*value),
        Aux::U16(value) => i64::from(*value),
        Aux::I32(value) => i64::from(*value),
        Aux::U32(value) => return Ok(u64::from(*value)),
        _ => {
            return Err(record_error(
                ErrorCategory::InputCorrupt,
                format!("{tag} tag is not an integer"),
                index,
                record,
            ));
        }
    };
    u64::try_from(value).map_err(|source| {
        record_error(
            ErrorCategory::InputCorrupt,
            format!("{tag} tag is negative"),
            index,
            record,
        )
        .with_source(source)
    })
}

fn duplicate_tag_error(tag: &'static str, index: u64, record: &Record) -> AlignGaugeError {
    record_error(
        ErrorCategory::InputCorrupt,
        "BAM record repeats an auxiliary tag",
        index,
        record,
    )
    .with_detail("tag", tag)
}

fn validate_plan(plan: &FieldPlan) -> Result<(), AlignGaugeError> {
    for unsupported in [RequiredField::Sequence, RequiredField::Qualities] {
        if plan.requires(unsupported) {
            return Err(AlignGaugeError::new(
                ErrorCategory::UnsupportedRecord,
                "v0.1 reader plan cannot materialize sequence or quality fields",
            )
            .with_detail("field", unsupported.as_str()));
        }
    }
    if !plan.requires(RequiredField::Flags) || !plan.requires(RequiredField::Coordinates) {
        return Err(AlignGaugeError::new(
            ErrorCategory::InternalInvariant,
            "v0.1 reader plan must include flags and coordinates",
        ));
    }
    Ok(())
}

fn validate_options(options: ReaderOptions) -> Result<(), AlignGaugeError> {
    if options.io_threads == 0 || options.io_threads > MAX_IO_THREADS {
        return Err(AlignGaugeError::new(
            ErrorCategory::Configuration,
            "io_threads must be between 1 and 64",
        )
        .with_detail("io_threads", u64_from_usize(options.io_threads)?));
    }
    Ok(())
}

fn open_htslib_reader(
    input: &Path,
    options: ReaderOptions,
    format: AlignmentFormat,
) -> Result<Reader, AlignGaugeError> {
    let mut reader = Reader::from_path(input).map_err(|source| {
        AlignGaugeError::new(
            ErrorCategory::InputCorrupt,
            format!("failed to open {} '{}'", format.as_str(), input.display()),
        )
        .with_detail("input", input.to_string_lossy().into_owned())
        .with_source(source)
    })?;
    if options.io_threads > 1 {
        reader.set_threads(options.io_threads).map_err(|source| {
            AlignGaugeError::new(
                ErrorCategory::ResourceLimit,
                "failed to configure HTSlib alignment decode threads",
            )
            .with_detail(
                "io_threads",
                u64_from_usize(options.io_threads).unwrap_or(u64::MAX),
            )
            .with_source(source)
        })?;
    }
    Ok(reader)
}

/// Detect BAM versus CRAM from local file magic without asking `HTSlib` to resolve references.
///
/// # Errors
/// Returns typed missing, corrupt, or unsupported-format errors.
pub fn detect_alignment_format(path: impl AsRef<Path>) -> Result<AlignmentFormat, AlignGaugeError> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(AlignGaugeError::new(
            ErrorCategory::InputNotFound,
            format!("alignment input '{}' does not exist", path.display()),
        )
        .with_detail("input", path.to_string_lossy().into_owned()));
    }
    let mut file = File::open(path).map_err(|source| {
        AlignGaugeError::new(
            ErrorCategory::InputFormat,
            format!("failed to open alignment input '{}'", path.display()),
        )
        .with_source(source)
    })?;
    let mut prefix = [0_u8; 4];
    file.read_exact(&mut prefix).map_err(|source| {
        AlignGaugeError::new(
            ErrorCategory::InputCorrupt,
            "alignment input is too short to contain BAM or CRAM magic",
        )
        .with_detail("input", path.to_string_lossy().into_owned())
        .with_source(source)
    })?;
    if prefix == *b"CRAM" {
        return Ok(AlignmentFormat::Cram);
    }
    if prefix[..2] == [0x1f, 0x8b] {
        verify_bam_signature(path)?;
        return Ok(AlignmentFormat::Bam);
    }
    Err(AlignGaugeError::new(
        ErrorCategory::InputFormat,
        "input is neither BGZF-compressed BAM nor CRAM",
    )
    .with_detail("input", path.to_string_lossy().into_owned()))
}

fn verify_cram_signature(path: &Path) -> Result<(), AlignGaugeError> {
    if !path.exists() {
        return Err(AlignGaugeError::new(
            ErrorCategory::InputNotFound,
            format!("input CRAM '{}' does not exist", path.display()),
        )
        .with_detail("input", path.to_string_lossy().into_owned()));
    }
    let mut file = File::open(path).map_err(|source| {
        AlignGaugeError::new(
            ErrorCategory::InputFormat,
            format!("failed to open CRAM '{}'", path.display()),
        )
        .with_source(source)
    })?;
    let mut magic = [0_u8; 4];
    file.read_exact(&mut magic).map_err(|source| {
        AlignGaugeError::new(
            ErrorCategory::InputCorrupt,
            "input is too short to contain a CRAM stream",
        )
        .with_source(source)
    })?;
    if magic != *b"CRAM" {
        return Err(AlignGaugeError::new(
            ErrorCategory::InputFormat,
            "input does not begin with CRAM magic",
        )
        .with_detail("input", path.to_string_lossy().into_owned()));
    }
    Ok(())
}

fn verify_bam_signature(path: &Path) -> Result<(), AlignGaugeError> {
    if !path.exists() {
        return Err(AlignGaugeError::new(
            ErrorCategory::InputNotFound,
            format!("input BAM '{}' does not exist", path.display()),
        )
        .with_detail("input", path.to_string_lossy().into_owned()));
    }

    let mut file = File::open(path).map_err(|source| {
        AlignGaugeError::new(
            ErrorCategory::InputFormat,
            format!("failed to open input '{}'", path.display()),
        )
        .with_detail("input", path.to_string_lossy().into_owned())
        .with_source(source)
    })?;
    let mut compressed_prefix = [0_u8; 4];
    file.read_exact(&mut compressed_prefix).map_err(|source| {
        AlignGaugeError::new(
            ErrorCategory::InputCorrupt,
            "input is too short to contain a BAM stream",
        )
        .with_detail("input", path.to_string_lossy().into_owned())
        .with_source(source)
    })?;
    if compressed_prefix[..2] != [0x1f, 0x8b] {
        return Err(AlignGaugeError::new(
            ErrorCategory::InputFormat,
            "v0.1 accepts BGZF-compressed BAM input only",
        )
        .with_detail("input", path.to_string_lossy().into_owned()));
    }
    file.seek(SeekFrom::Start(0)).map_err(|source| {
        AlignGaugeError::new(ErrorCategory::InputFormat, "failed to rewind BAM input")
            .with_source(source)
    })?;
    let mut decoder = MultiGzDecoder::new(file);
    let mut bam_magic = [0_u8; 4];
    decoder.read_exact(&mut bam_magic).map_err(|source| {
        AlignGaugeError::new(
            ErrorCategory::InputCorrupt,
            "failed to decode the first BAM BGZF member",
        )
        .with_detail("input", path.to_string_lossy().into_owned())
        .with_source(source)
    })?;
    if bam_magic != *b"BAM\x01" {
        return Err(AlignGaugeError::new(
            ErrorCategory::InputFormat,
            "decompressed input does not begin with BAM magic",
        )
        .with_detail("input", path.to_string_lossy().into_owned()));
    }
    Ok(())
}

fn order_error(
    message: impl Into<String>,
    index: u64,
    previous: Option<RecordCoordinate>,
    current: RecordCoordinate,
    record: &Record,
) -> AlignGaugeError {
    let mut error = record_error(ErrorCategory::InputUnsorted, message, index, record)
        .with_detail("current_reference_id", i64::from(current.reference_id))
        .with_detail("current_position", current.position);
    if let Some(previous) = previous {
        error = error
            .with_detail("previous_reference_id", i64::from(previous.reference_id))
            .with_detail("previous_position", previous.position);
    }
    error
}

fn record_error_without_name(
    category: ErrorCategory,
    message: impl Into<String>,
    index: u64,
) -> AlignGaugeError {
    AlignGaugeError::new(category, message).with_detail("record_index", index)
}

fn record_error(
    category: ErrorCategory,
    message: impl Into<String>,
    index: u64,
    record: &Record,
) -> AlignGaugeError {
    AlignGaugeError::new(category, message)
        .with_detail("record_index", index)
        .with_sensitive_detail(
            "read_name",
            String::from_utf8_lossy(record.qname()).into_owned(),
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
