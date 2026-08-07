from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:80]!r}")
    p.write_text(text.replace(old, new, 1))

# Field plan: expose sequence/noise only for the explicit Picard alignment profile,
# and CIGAR/mate/TLEN only for the Picard insert profile.
path = "crates/aligngauge-hts/src/plan.rs"
replace_once(path,
"    /// Packed sequence bases. Reserved; not requested by v0.1 plans.\n    Sequence,\n    /// Base qualities.\n",
"    /// Decoded sequence bases. Materialized only by an explicit plan.\n    Sequence,\n    /// Optional Picard `XN` noise tag interpreted as integer one.\n    NoiseTag,\n    /// Base qualities.\n")
replace_once(path, "    pub const ALL: [Self; 11] = [\n", "    pub const ALL: [Self; 12] = [\n")
replace_once(path,
"        Self::MismatchDescriptor,\n        Self::Sequence,\n        Self::Qualities,\n",
"        Self::MismatchDescriptor,\n        Self::Sequence,\n        Self::NoiseTag,\n        Self::Qualities,\n")
replace_once(path,
"            Self::Sequence => \"sequence\",\n            Self::Qualities => \"qualities\",\n",
"            Self::Sequence => \"sequence\",\n            Self::NoiseTag => \"xn_noise_tag\",\n            Self::Qualities => \"qualities\",\n")
replace_once(path,
"    /// Add optional tags used by diagnostic and later metric collectors.\n",
"    /// Build the Picard 3.4.0 reference-independent alignment-summary plan.\n    #[must_use]\n    pub fn picard_alignment_summary() -> Self {\n        Self::from_fields([\n            RequiredField::Flags,\n            RequiredField::Coordinates,\n            RequiredField::MappingQuality,\n            RequiredField::Sequence,\n            RequiredField::NoiseTag,\n        ])\n    }\n\n    /// Build the Picard 3.4.0 default ALL_READS insert-size plan.\n    #[must_use]\n    pub fn picard_insert_size() -> Self {\n        Self::from_fields([\n            RequiredField::Flags,\n            RequiredField::Coordinates,\n            RequiredField::MateCoordinates,\n            RequiredField::Cigar,\n            RequiredField::TemplateLength,\n        ])\n    }\n\n    /// Add optional tags used by diagnostic and later metric collectors.\n")

# Reader: materialize sequence only on request and expose requested XN state.
path = "crates/aligngauge-hts/src/reader.rs"
replace_once(path,
"    query_length: u64,\n    qualities_requested: bool,\n    template_length: FieldValue<i32>,\n",
"    query_length: u64,\n    sequence: FieldValue<Vec<u8>>,\n    qualities_requested: bool,\n    noise_read: FieldValue<bool>,\n    template_length: FieldValue<i32>,\n")
replace_once(path,
"    /// Planned base qualities.\n",
"    /// Planned decoded sequence bases.\n    #[must_use]\n    pub fn sequence(&self) -> FieldValue<&[u8]> {\n        match &self.sequence {\n            FieldValue::NotRequested => FieldValue::NotRequested,\n            FieldValue::Missing => FieldValue::Missing,\n            FieldValue::Value(value) => FieldValue::Value(value.as_slice()),\n        }\n    }\n\n    /// Planned Picard `XN` noise state. Missing is distinct from false.\n    #[must_use]\n    pub const fn noise_read(&self) -> &FieldValue<bool> {\n        &self.noise_read\n    }\n\n    /// Planned base qualities.\n")
replace_once(path,
"            query_length: facts.query_length,\n            qualities_requested: facts.qualities_requested,\n            template_length: facts.template_length,\n",
"            query_length: facts.query_length,\n            sequence: facts.sequence,\n            qualities_requested: facts.qualities_requested,\n            noise_read: facts.noise_read,\n            template_length: facts.template_length,\n")
replace_once(path,
"    query_length: u64,\n    qualities_requested: bool,\n    template_length: FieldValue<i32>,\n",
"    query_length: u64,\n    sequence: FieldValue<Vec<u8>>,\n    qualities_requested: bool,\n    noise_read: FieldValue<bool>,\n    template_length: FieldValue<i32>,\n")
replace_once(path,
"    let query_length = u64_from_usize(layout.sequence_bases)?;\n    let qualities_requested = plan.requires(RequiredField::Qualities);\n",
"    let query_length = u64_from_usize(layout.sequence_bases)?;\n    let sequence = if plan.requires(RequiredField::Sequence) {\n        let decoded = record.seq().as_bytes();\n        if decoded.len() != layout.sequence_bases {\n            return Err(record_error(\n                ErrorCategory::InputCorrupt,\n                \"decoded BAM sequence length differs from the validated record layout\",\n                index,\n                record,\n            ));\n        }\n        FieldValue::Value(decoded)\n    } else {\n        FieldValue::NotRequested\n    };\n    let qualities_requested = plan.requires(RequiredField::Qualities);\n")
replace_once(path,
"        query_length,\n        qualities_requested,\n        template_length,\n",
"        query_length,\n        sequence,\n        qualities_requested,\n        noise_read: tags.noise_read,\n        template_length,\n")
replace_once(path,
"struct TagFacts {\n    edit_distance: FieldValue<u64>,\n",
"struct TagFacts {\n    noise_read: FieldValue<bool>,\n    edit_distance: FieldValue<u64>,\n")
replace_once(path,
"    let mut edit_distance = None;\n    let mut mismatch_descriptor = None;\n",
"    let mut noise_read = None;\n    let mut edit_distance = None;\n    let mut mismatch_descriptor = None;\n")
replace_once(path,
"        match tag {\n            b\"CG\" => {\n",
"        match tag {\n            b\"XN\" if plan.requires(RequiredField::NoiseTag) => {\n                if noise_read.is_some() {\n                    return Err(duplicate_tag_error(\"XN\", index, record));\n                }\n                noise_read = Some(matches!(\n                    value,\n                    Aux::I8(1) | Aux::U8(1) | Aux::I16(1) | Aux::U16(1) | Aux::I32(1) | Aux::U32(1)\n                ));\n            }\n            b\"CG\" => {\n")
replace_once(path,
"    let edit_distance = if plan.requires(RequiredField::EditDistance) {\n",
"    let noise_read = if plan.requires(RequiredField::NoiseTag) {\n        noise_read.map_or(FieldValue::Missing, FieldValue::Value)\n    } else {\n        FieldValue::NotRequested\n    };\n    let edit_distance = if plan.requires(RequiredField::EditDistance) {\n")
replace_once(path,
"    Ok(TagFacts {\n        edit_distance,\n",
"    Ok(TagFacts {\n        noise_read,\n        edit_distance,\n")
replace_once(path,
"fn validate_plan(plan: &FieldPlan) -> Result<(), AlignGaugeError> {\n    if plan.requires(RequiredField::Sequence) {\n        return Err(AlignGaugeError::new(\n            ErrorCategory::UnsupportedRecord,\n            \"reader plan cannot materialize packed sequence bases\",\n        )\n        .with_detail(\"field\", RequiredField::Sequence.as_str()));\n    }\n    if !plan.requires(RequiredField::Flags) || !plan.requires(RequiredField::Coordinates) {\n",
"fn validate_plan(plan: &FieldPlan) -> Result<(), AlignGaugeError> {\n    if !plan.requires(RequiredField::Flags) || !plan.requires(RequiredField::Coordinates) {\n")

# Metrics public module/export.
path = "crates/aligngauge-metrics/src/lib.rs"
replace_once(path, "pub mod samtools_stats;\n", "pub mod picard;\npub mod samtools_stats;\n")
replace_once(path,
"pub use samtools_stats::{\n",
"pub use picard::{\n    PICARD_ALIGNMENT_SUMMARY_PROFILE, PICARD_INSERT_SIZE_PROFILE, PICARD_VERSION,\n    PicardAlignmentCategory, PicardAlignmentSummaryCollector, PicardAlignmentSummaryReport,\n    PicardAlignmentSummaryRow, PicardInsertSizeCollector, PicardInsertSizeReport,\n    PicardInsertSizeRow, PicardPairOrientation, analyze_picard_alignment_summary_bam,\n    analyze_picard_insert_size_bam,\n};\npub use samtools_stats::{\n")

# CLI format options.
path = "crates/aligngauge-cli/src/main.rs"
replace_once(path,
"use aligngauge_metrics::analyze_samtools_stats_bam;\n",
"use aligngauge_metrics::{\n    analyze_picard_alignment_summary_bam, analyze_picard_insert_size_bam,\n    analyze_samtools_stats_bam,\n};\n")
replace_once(path,
"    SamtoolsStats,\n}\n",
"    SamtoolsStats,\n    PicardAlignmentSummary,\n    PicardInsertSize,\n}\n")
replace_once(path,
"fn run_compatibility(input: &Path, format: CompatibilityFormat) -> ExitCode {\n    if format == CompatibilityFormat::SamtoolsStats {\n",
"fn run_compatibility(input: &Path, format: CompatibilityFormat) -> ExitCode {\n    if format == CompatibilityFormat::PicardAlignmentSummary {\n        return match analyze_picard_alignment_summary_bam(input) {\n            Ok(report) => {\n                print!(\"{}\", report.render_picard_metrics());\n                ExitCode::SUCCESS\n            }\n            Err(error) => exit_with_error(&error, LogFormat::Human),\n        };\n    }\n    if format == CompatibilityFormat::PicardInsertSize {\n        return match analyze_picard_insert_size_bam(input) {\n            Ok(report) => {\n                print!(\"{}\", report.render_picard_metrics());\n                ExitCode::SUCCESS\n            }\n            Err(error) => exit_with_error(&error, LogFormat::Human),\n        };\n    }\n    if format == CompatibilityFormat::SamtoolsStats {\n")
replace_once(path,
"                CompatibilityFormat::SamtoolsStats => {\n                    unreachable!(\"handled before counter analysis\")\n                }\n",
"                CompatibilityFormat::SamtoolsStats\n                | CompatibilityFormat::PicardAlignmentSummary\n                | CompatibilityFormat::PicardInsertSize => {\n                    unreachable!(\"handled before counter analysis\")\n                }\n")
replace_once(path,
"        Some(\"samtools-stats\") => Ok(CompatibilityFormat::SamtoolsStats),\n        _ => Err(usage_error(\n",
"        Some(\"samtools-stats\") => Ok(CompatibilityFormat::SamtoolsStats),\n        Some(\"picard-alignment-summary\") => Ok(CompatibilityFormat::PicardAlignmentSummary),\n        Some(\"picard-insert-size\") => Ok(CompatibilityFormat::PicardInsertSize),\n        _ => Err(usage_error(\n")
replace_once(path,
"                \"unsupported --format '{}'; use human, json, samtools-flagstat, samtools-idxstats, or samtools-stats\",\n",
"                \"unsupported --format '{}'; use human, json, samtools-flagstat, samtools-idxstats, samtools-stats, picard-alignment-summary, or picard-insert-size\",\n")
replace_once(path,
"  {0} qc --input <BAM> --format <human|json|samtools-flagstat|samtools-idxstats|samtools-stats>\",\n",
"  {0} qc --input <BAM> --format <human|json|samtools-flagstat|samtools-idxstats|samtools-stats|picard-alignment-summary|picard-insert-size>\",\n")
