from pathlib import Path


def replace_exact(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if old not in text:
        raise SystemExit(f"expected text not found in {path}: {old!r}")
    path.write_text(text.replace(old, new), encoding="utf-8")


reader = Path("crates/aligngauge-hts/src/reader.rs")
replace_exact(reader, "Number of HTSlib background decode threads", "Number of `HTSlib` background decode threads")
replace_exact(reader, "Whether HTSlib expanded a BAM long-CIGAR representation", "Whether `HTSlib` expanded a BAM long-CIGAR representation")
replace_exact(reader, "invalid options, HTSlib\n", "invalid options, `HTSlib`\n")
replace_exact(
    reader,
    """                ErrorCategory::InputFormat,
                format!("failed to open BAM '{}'", input.display()),
""",
    """                ErrorCategory::InputCorrupt,
                format!("failed to open BAM '{}'", input.display()),
""",
)
replace_exact(
    reader,
    "self.record_index.checked_add(1).unwrap_or(u64::MAX)",
    "self.record_index.saturating_add(1)",
)
replace_exact(
    reader,
    "fn validate_cigar(\n",
    "#[allow(clippy::too_many_lines)]\nfn validate_cigar(\n",
)
replace_exact(
    reader,
    "fn validate_auxiliary(\n",
    "#[allow(clippy::too_many_lines)]\nfn validate_auxiliary(\n",
)
replace_exact(
    reader,
    "parse_nonnegative_integer(value, \"NM\", index, record)?",
    "parse_nonnegative_integer(&value, \"NM\", index, record)?",
)
replace_exact(reader, "    value: Aux<'_>,\n", "    value: &Aux<'_>,\n")
replace_exact(
    reader,
    "    let value = match value {\n        Aux::I8(value) => i64::from(value),\n        Aux::U8(value) => i64::from(value),\n        Aux::I16(value) => i64::from(value),\n        Aux::U16(value) => i64::from(value),\n        Aux::I32(value) => i64::from(value),\n        Aux::U32(value) => return Ok(u64::from(value)),\n",
    "    let value = match value {\n        Aux::I8(value) => i64::from(*value),\n        Aux::U8(value) => i64::from(*value),\n        Aux::I16(value) => i64::from(*value),\n        Aux::U16(value) => i64::from(*value),\n        Aux::I32(value) => i64::from(*value),\n        Aux::U32(value) => return Ok(u64::from(*value)),\n",
)

replace_exact(
    reader,
    "struct RecordFacts {\n",
    """struct RecordLayout {
    query_name_bytes: usize,
    sequence_bases: usize,
}

fn validate_record_layout(
    record: &Record,
    index: u64,
) -> Result<RecordLayout, AlignGaugeError> {
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
        .with_detail("query_name_storage_bytes", u64_from_usize(query_name_bytes)?)
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
""",
)
replace_exact(
    reader,
    """    let data_len = usize::try_from(record.inner().l_data).map_err(|source| {
        record_error(
            ErrorCategory::InputCorrupt,
            "BAM record has a negative or unrepresentable data length",
            index,
            record,
        )
        .with_source(source)
    })?;
    if data_len > MAX_RECORD_BYTES {
        return Err(record_error(
            ErrorCategory::ResourceLimit,
            "BAM record exceeds the supported byte limit",
            index,
            record,
        )
        .with_detail("record_bytes", u64_from_usize(data_len)?)
        .with_detail("maximum_record_bytes", u64_from_usize(MAX_RECORD_BYTES)?));
    }
    if record.qname().len() > MAX_QUERY_NAME_BYTES {
""",
    """    let layout = validate_record_layout(record, index)?;
    if layout.query_name_bytes > MAX_QUERY_NAME_BYTES {
""",
)
replace_exact(
    reader,
    """    if record.seq_len() > MAX_SEQUENCE_BASES {
        return Err(record_error(
            ErrorCategory::ResourceLimit,
            "BAM sequence exceeds the supported base limit",
            index,
            record,
        )
        .with_detail("sequence_bases", u64_from_usize(record.seq_len())?));
    }
""",
    """    if layout.sequence_bases > MAX_SEQUENCE_BASES {
        return Err(record_error(
            ErrorCategory::ResourceLimit,
            "BAM sequence exceeds the supported base limit",
            index,
            record,
        )
        .with_detail("sequence_bases", u64_from_usize(layout.sequence_bases)?));
    }
""",
)
replace_exact(
    reader,
    "fn record_error(\n",
    """fn record_error_without_name(
    category: ErrorCategory,
    message: impl Into<String>,
    index: u64,
) -> AlignGaugeError {
    AlignGaugeError::new(category, message).with_detail("record_index", index)
}

fn record_error(
""",
)

lib = Path("crates/aligngauge-hts/src/lib.rs")
replace_exact(
    lib,
    "/// HTSlib compatibility line supplied by the pinned rust-htslib release.",
    "/// `HTSlib` compatibility line supplied by the pinned rust-htslib release.",
)
