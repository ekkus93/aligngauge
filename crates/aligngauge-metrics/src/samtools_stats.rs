use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use aligngauge_core::{AlignGaugeError, ErrorCategory};
use aligngauge_hts::{BamReader, FieldPlan, FieldValue, ReaderOptions, ValidatedRecord};

/// Exact Milestone 10 compatibility profile.
pub const SAMTOOLS_STATS_PROFILE: &str = "samtools-stats-1.24-multiqc-1.35";
/// Pinned `MultiQC` parser version.
pub const MULTIQC_VERSION: &str = "1.35";
const MAX_INSERT_SIZE: u32 = 8_000;
const MAIN_INSERT_BULK: f64 = 0.99;

const FLAG_PAIRED: u16 = 0x1;
const FLAG_PROPER_PAIR: u16 = 0x2;
const FLAG_UNMAPPED: u16 = 0x4;
const FLAG_MATE_UNMAPPED: u16 = 0x8;
const FLAG_REVERSE: u16 = 0x10;
const FLAG_MATE_REVERSE: u16 = 0x20;
const FLAG_READ1: u16 = 0x40;
const FLAG_READ2: u16 = 0x80;
const FLAG_SECONDARY: u16 = 0x100;
const FLAG_QC_FAIL: u16 = 0x200;
const FLAG_DUPLICATE: u16 = 0x400;
const FLAG_SUPPLEMENTARY: u16 = 0x800;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
struct InsertCounts {
    inward: u64,
    outward: u64,
    other: u64,
}

impl InsertCounts {
    fn total(self) -> Result<u64, AlignGaugeError> {
        checked_add(
            checked_add(self.inward, self.outward, "insert_size.total")?,
            self.other,
            "insert_size.total",
        )
    }

    const fn halved(self) -> Self {
        Self {
            inward: self.inward / 2,
            outward: self.outward / 2,
            other: self.other / 2,
        }
    }
}

/// One rendered Samtools `IS` row.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InsertSizeRow {
    pub insert_size: u32,
    pub pairs_total: u64,
    pub inward: u64,
    pub outward: u64,
    pub other: u64,
}

/// Typed canonical state for the Milestone 10 Samtools `SN` + `IS` subset.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SamtoolsStatsReport {
    pub raw_total_sequences: u64,
    pub filtered_sequences: u64,
    pub sequences: u64,
    pub is_sorted: bool,
    pub first_fragments: u64,
    pub last_fragments: u64,
    pub reads_mapped: u64,
    pub reads_mapped_and_paired: u64,
    pub reads_unmapped: u64,
    pub reads_properly_paired: u64,
    pub reads_paired: u64,
    pub reads_duplicated: u64,
    pub reads_mq0: u64,
    pub reads_qc_failed: u64,
    pub non_primary_alignments: u64,
    pub supplementary_alignments: u64,
    pub total_length: u64,
    pub total_first_fragment_length: u64,
    pub total_last_fragment_length: u64,
    pub bases_mapped: u64,
    pub bases_mapped_cigar: u64,
    pub bases_trimmed: u64,
    pub bases_duplicated: u64,
    pub mismatches: u64,
    pub error_rate: String,
    pub average_length: String,
    pub average_first_fragment_length: String,
    pub average_last_fragment_length: String,
    pub maximum_length: u64,
    pub maximum_first_fragment_length: u64,
    pub maximum_last_fragment_length: u64,
    pub average_quality: String,
    pub insert_size_average: String,
    pub insert_size_standard_deviation: String,
    pub inward_oriented_pairs: u64,
    pub outward_oriented_pairs: u64,
    pub pairs_with_other_orientation: u64,
    pub pairs_on_different_chromosomes: u64,
    pub percentage_properly_paired_reads: String,
    pub insert_sizes: Vec<InsertSizeRow>,
}

impl SamtoolsStatsReport {
    /// Render only the compatibility surface frozen by ADR-0007.
    #[must_use]
    pub fn render_samtools_stats(&self) -> String {
        let mut out = String::new();
        Self::render_header(&mut out);
        self.render_summary_counts(&mut out);
        self.render_summary_bases(&mut out);
        self.render_summary_derived(&mut out);
        self.render_insert_sizes(&mut out);
        out
    }

    fn render_header(out: &mut String) {
        writeln!(
            out,
            "# This file was produced by samtools stats (1.24+htslib-1.24) and can be plotted using plot-bamstats"
        )
        .expect("String write cannot fail");
        writeln!(
            out,
            "# AlignGauge compatibility projection: {SAMTOOLS_STATS_PROFILE}; supported sections: SN,IS"
        )
        .expect("String write cannot fail");
        writeln!(
            out,
            "# Summary Numbers. Use `grep ^SN | cut -f 2-` to extract this part."
        )
        .expect("String write cannot fail");
    }

    fn render_summary_counts(&self, out: &mut String) {
        sn(
            out,
            "raw total sequences:",
            self.raw_total_sequences,
            Some("excluding supplementary and secondary reads"),
        );
        sn(out, "filtered sequences:", self.filtered_sequences, None);
        sn(out, "sequences:", self.sequences, None);
        writeln!(
            out,
            "SN\tis sorted:\t{}\t# {} by coordinate",
            u8::from(self.is_sorted),
            if self.is_sorted {
                "sorted"
            } else {
                "not sorted"
            }
        )
        .expect("String write cannot fail");
        sn(out, "1st fragments:", self.first_fragments, None);
        sn(out, "last fragments:", self.last_fragments, None);
        sn(out, "reads mapped:", self.reads_mapped, None);
        sn(
            out,
            "reads mapped and paired:",
            self.reads_mapped_and_paired,
            Some("paired-end technology bit set + both mates mapped"),
        );
        sn(out, "reads unmapped:", self.reads_unmapped, None);
        sn(
            out,
            "reads properly paired:",
            self.reads_properly_paired,
            Some("proper-pair bit set"),
        );
        sn(
            out,
            "reads paired:",
            self.reads_paired,
            Some("paired-end technology bit set"),
        );
        sn(
            out,
            "reads duplicated:",
            self.reads_duplicated,
            Some("PCR or optical duplicate bit set"),
        );
        sn(out, "reads MQ0:", self.reads_mq0, Some("mapped and MQ=0"));
        sn(out, "reads QC failed:", self.reads_qc_failed, None);
        sn(
            out,
            "non-primary alignments:",
            self.non_primary_alignments,
            None,
        );
        sn(
            out,
            "supplementary alignments:",
            self.supplementary_alignments,
            None,
        );
    }

    fn render_summary_bases(&self, out: &mut String) {
        sn(
            out,
            "total length:",
            self.total_length,
            Some("ignores clipping"),
        );
        sn(
            out,
            "total first fragment length:",
            self.total_first_fragment_length,
            Some("ignores clipping"),
        );
        sn(
            out,
            "total last fragment length:",
            self.total_last_fragment_length,
            Some("ignores clipping"),
        );
        sn(
            out,
            "bases mapped:",
            self.bases_mapped,
            Some("ignores clipping"),
        );
        sn(
            out,
            "bases mapped (cigar):",
            self.bases_mapped_cigar,
            Some("more accurate"),
        );
        sn(out, "bases trimmed:", self.bases_trimmed, None);
        sn(out, "bases duplicated:", self.bases_duplicated, None);
        sn(out, "mismatches:", self.mismatches, Some("from NM fields"));
        sn_text(
            out,
            "error rate:",
            &self.error_rate,
            Some("mismatches / bases mapped (cigar)"),
        );
        sn_text(out, "average length:", &self.average_length, None);
        sn_text(
            out,
            "average first fragment length:",
            &self.average_first_fragment_length,
            None,
        );
        sn_text(
            out,
            "average last fragment length:",
            &self.average_last_fragment_length,
            None,
        );
        sn(out, "maximum length:", self.maximum_length, None);
        sn(
            out,
            "maximum first fragment length:",
            self.maximum_first_fragment_length,
            None,
        );
        sn(
            out,
            "maximum last fragment length:",
            self.maximum_last_fragment_length,
            None,
        );
        sn_text(out, "average quality:", &self.average_quality, None);
    }

    fn render_summary_derived(&self, out: &mut String) {
        sn_text(out, "insert size average:", &self.insert_size_average, None);
        sn_text(
            out,
            "insert size standard deviation:",
            &self.insert_size_standard_deviation,
            None,
        );
        sn(
            out,
            "inward oriented pairs:",
            self.inward_oriented_pairs,
            None,
        );
        sn(
            out,
            "outward oriented pairs:",
            self.outward_oriented_pairs,
            None,
        );
        sn(
            out,
            "pairs with other orientation:",
            self.pairs_with_other_orientation,
            None,
        );
        sn(
            out,
            "pairs on different chromosomes:",
            self.pairs_on_different_chromosomes,
            None,
        );
        sn_text(
            out,
            "percentage of properly paired reads (%):",
            &self.percentage_properly_paired_reads,
            None,
        );
    }

    fn render_insert_sizes(&self, out: &mut String) {
        writeln!(out, "# Insert sizes. Use `grep ^IS | cut -f 2-` to extract this part. The columns are: insert size, pairs total, inward oriented pairs, outward oriented pairs, other pairs").expect("String write cannot fail");
        for row in &self.insert_sizes {
            writeln!(
                out,
                "IS\t{}\t{}\t{}\t{}\t{}",
                row.insert_size, row.pairs_total, row.inward, row.outward, row.other
            )
            .expect("String write cannot fail");
        }
    }
}

fn sn(out: &mut String, name: &str, value: u64, comment: Option<&str>) {
    match comment {
        Some(comment) => writeln!(out, "SN\t{name}\t{value}\t# {comment}"),
        None => writeln!(out, "SN\t{name}\t{value}"),
    }
    .expect("String write cannot fail");
}

fn sn_text(out: &mut String, name: &str, value: &str, comment: Option<&str>) {
    match comment {
        Some(comment) => writeln!(out, "SN\t{name}\t{value}\t# {comment}"),
        None => writeln!(out, "SN\t{name}\t{value}"),
    }
    .expect("String write cannot fail");
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum FragmentOrder {
    First,
    Last,
    Other,
}

/// Checked single-pass accumulator for the supported Samtools stats subset.
#[derive(Debug, Default)]
pub struct SamtoolsStatsCollector {
    first_fragments: u64,
    last_fragments: u64,
    other_fragments: u64,
    reads_mapped: u64,
    reads_mapped_and_paired: u64,
    reads_unmapped: u64,
    reads_properly_paired: u64,
    reads_paired: u64,
    reads_duplicated: u64,
    reads_mq0: u64,
    reads_qc_failed: u64,
    non_primary_alignments: u64,
    supplementary_alignments: u64,
    total_length: u64,
    total_first_fragment_length: u64,
    total_last_fragment_length: u64,
    bases_mapped: u64,
    bases_mapped_cigar: u64,
    bases_duplicated: u64,
    mismatches: u64,
    quality_sum: u64,
    maximum_length: u64,
    maximum_first_fragment_length: u64,
    maximum_last_fragment_length: u64,
    different_chromosome_observations: u64,
    insert_observations: BTreeMap<u32, InsertCounts>,
}

impl SamtoolsStatsCollector {
    /// Observe one record from a `FieldPlan::samtools_stats()` reader.
    ///
    /// # Errors
    ///
    /// Returns a typed error when a required planned field is unavailable or checked arithmetic fails.
    pub fn observe(&mut self, record: &ValidatedRecord<'_>) -> Result<(), AlignGaugeError> {
        let flags = record.flags();
        if flags & FLAG_SECONDARY != 0 {
            increment(&mut self.non_primary_alignments, "non_primary_alignments")?;
            return Ok(());
        }
        if flags & FLAG_SUPPLEMENTARY != 0 {
            increment(
                &mut self.supplementary_alignments,
                "supplementary_alignments",
            )?;
        }

        let sequence_length = record.query_length();
        if sequence_length == 0 {
            return Ok(());
        }
        if flags & FLAG_DUPLICATE != 0 {
            increment(&mut self.reads_duplicated, "reads_duplicated")?;
            add_assign(
                &mut self.bases_duplicated,
                sequence_length,
                "bases_duplicated",
            )?;
        }

        let order = fragment_order(flags);
        let unclipped = unclipped_length(record, sequence_length)?;
        self.maximum_length = self.maximum_length.max(unclipped);
        match order {
            FragmentOrder::First => {
                self.maximum_first_fragment_length =
                    self.maximum_first_fragment_length.max(unclipped);
            }
            FragmentOrder::Last => {
                self.maximum_last_fragment_length =
                    self.maximum_last_fragment_length.max(unclipped);
            }
            FragmentOrder::Other => {}
        }

        let original = flags & (FLAG_SECONDARY | FLAG_SUPPLEMENTARY) == 0;
        if original {
            self.observe_original(record, flags, order, sequence_length)?;
        }
        if flags & FLAG_UNMAPPED != 0 {
            return Ok(());
        }
        if original && flags & FLAG_PAIRED != 0 && flags & FLAG_MATE_UNMAPPED == 0 {
            self.observe_insert(record, flags)?;
        }
        match record.edit_distance() {
            FieldValue::Value(value) => add_assign(&mut self.mismatches, *value, "mismatches")?,
            FieldValue::Missing => {}
            FieldValue::NotRequested => return Err(plan_error(record, "NM/edit distance")),
        }
        add_assign(
            &mut self.bases_mapped_cigar,
            mapped_cigar_bases(record)?,
            "bases_mapped_cigar",
        )?;
        Ok(())
    }

    fn observe_original(
        &mut self,
        record: &ValidatedRecord<'_>,
        flags: u16,
        order: FragmentOrder,
        sequence_length: u64,
    ) -> Result<(), AlignGaugeError> {
        add_assign(&mut self.total_length, sequence_length, "total_length")?;
        if flags & FLAG_QC_FAIL != 0 {
            increment(&mut self.reads_qc_failed, "reads_qc_failed")?;
        }
        if flags & FLAG_PAIRED != 0 {
            increment(&mut self.reads_paired, "reads_paired")?;
        }
        match order {
            FragmentOrder::First => {
                increment(&mut self.first_fragments, "first_fragments")?;
                add_assign(
                    &mut self.total_first_fragment_length,
                    sequence_length,
                    "total_first_fragment_length",
                )?;
                self.add_quality_sum(record, sequence_length)?;
            }
            FragmentOrder::Last => {
                increment(&mut self.last_fragments, "last_fragments")?;
                add_assign(
                    &mut self.total_last_fragment_length,
                    sequence_length,
                    "total_last_fragment_length",
                )?;
                self.add_quality_sum(record, sequence_length)?;
            }
            FragmentOrder::Other => increment(&mut self.other_fragments, "other_fragments")?,
        }
        if flags & FLAG_UNMAPPED != 0 {
            increment(&mut self.reads_unmapped, "reads_unmapped")?;
            return Ok(());
        }
        increment(&mut self.reads_mapped, "reads_mapped")?;
        add_assign(&mut self.bases_mapped, sequence_length, "bases_mapped")?;
        match record.mapping_quality() {
            FieldValue::Value(0) => increment(&mut self.reads_mq0, "reads_mq0")?,
            FieldValue::Value(_) => {}
            FieldValue::Missing | FieldValue::NotRequested => {
                return Err(plan_error(record, "mapping quality"));
            }
        }
        if flags & FLAG_PAIRED != 0 && flags & FLAG_MATE_UNMAPPED == 0 {
            increment(&mut self.reads_mapped_and_paired, "reads_mapped_and_paired")?;
            if flags & FLAG_PROPER_PAIR != 0 {
                increment(&mut self.reads_properly_paired, "reads_properly_paired")?;
            }
            let current = record
                .coordinate()
                .ok_or_else(|| plan_error(record, "current mapped coordinate"))?;
            let mate = required_mate(record)?;
            if current.reference_id != mate.reference_id {
                increment(
                    &mut self.different_chromosome_observations,
                    "different_chromosome_observations",
                )?;
            }
        }
        Ok(())
    }

    fn add_quality_sum(
        &mut self,
        record: &ValidatedRecord<'_>,
        sequence_length: u64,
    ) -> Result<(), AlignGaugeError> {
        let qualities = match record.qualities() {
            FieldValue::Value(value) => value,
            FieldValue::Missing | FieldValue::NotRequested => {
                return Err(plan_error(record, "base qualities"));
            }
        };
        let expected = usize::try_from(sequence_length)
            .map_err(|source| plan_error(record, "sequence length").with_source(source))?;
        if qualities.len() != expected {
            return Err(AlignGaugeError::new(
                ErrorCategory::InternalInvariant,
                "validated qualities changed length before Samtools stats collection",
            )
            .with_detail("record_index", record.index()));
        }
        for quality in qualities {
            add_assign(&mut self.quality_sum, u64::from(*quality), "quality_sum")?;
        }
        Ok(())
    }

    fn observe_insert(
        &mut self,
        record: &ValidatedRecord<'_>,
        flags: u16,
    ) -> Result<(), AlignGaugeError> {
        let raw = match record.template_length() {
            FieldValue::Value(value) => *value,
            FieldValue::Missing | FieldValue::NotRequested => {
                return Err(plan_error(record, "template length"));
            }
        };
        let absolute = raw.checked_abs().ok_or_else(|| {
            AlignGaugeError::new(
                ErrorCategory::UnsupportedRecord,
                "template length cannot be represented as a positive Samtools insert size",
            )
            .with_detail("record_index", record.index())
            .with_detail("template_length", i64::from(raw))
        })?;
        let insert_size = u32::try_from(absolute)
            .map_err(|source| plan_error(record, "template length").with_source(source))?
            .min(MAX_INSERT_SIZE);
        let current = record
            .coordinate()
            .ok_or_else(|| plan_error(record, "current mapped coordinate"))?;
        let mate = required_mate(record)?;
        if insert_size == 0 && current.reference_id != mate.reference_id {
            return Ok(());
        }
        let position_delta = mate.position.checked_sub(current.position).ok_or_else(|| {
            AlignGaugeError::new(
                ErrorCategory::InternalInvariant,
                "mate-position delta overflowed",
            )
            .with_detail("record_index", record.index())
        })?;
        let is_first = if flags & FLAG_READ1 != 0 { 1_i8 } else { -1_i8 };
        let is_forward = if flags & FLAG_REVERSE != 0 {
            -1_i8
        } else {
            1_i8
        };
        let mate_forward = if flags & FLAG_MATE_REVERSE != 0 {
            -1_i8
        } else {
            1_i8
        };
        let counts = self.insert_observations.entry(insert_size).or_default();
        let same_orientation = is_forward * mate_forward > 0;
        let signed_position = i64::from(is_first) * position_delta;
        let signed_strand = is_first * is_forward;
        if same_orientation {
            increment(&mut counts.other, "insert.other")?;
        } else if signed_position > 0 {
            if signed_strand > 0 {
                increment(&mut counts.inward, "insert.inward")?;
            } else {
                increment(&mut counts.outward, "insert.outward")?;
            }
        } else if signed_position < 0 {
            if signed_strand > 0 {
                increment(&mut counts.outward, "insert.outward")?;
            } else {
                increment(&mut counts.inward, "insert.inward")?;
            }
        } else {
            increment(&mut counts.inward, "insert.inward")?;
        }
        Ok(())
    }

    /// Finalize the canonical report with the pinned Samtools 1.24 output arithmetic.
    ///
    /// # Errors
    ///
    /// Returns a typed error when checked arithmetic or insert-size reduction cannot be represented.
    pub fn finish(self) -> Result<SamtoolsStatsReport, AlignGaugeError> {
        let sequences = checked_add(
            checked_add(self.first_fragments, self.last_fragments, "sequences")?,
            self.other_fragments,
            "sequences",
        )?;
        let insert = finalize_insert_sizes(&self.insert_observations)?;
        let proper_numerator = self
            .reads_properly_paired
            .checked_mul(100)
            .ok_or_else(|| overflow("percentage_properly_paired_reads"))?;
        Ok(SamtoolsStatsReport {
            raw_total_sequences: sequences,
            filtered_sequences: 0,
            sequences,
            is_sorted: true,
            first_fragments: self.first_fragments,
            last_fragments: self.last_fragments,
            reads_mapped: self.reads_mapped,
            reads_mapped_and_paired: self.reads_mapped_and_paired,
            reads_unmapped: self.reads_unmapped,
            reads_properly_paired: self.reads_properly_paired,
            reads_paired: self.reads_paired,
            reads_duplicated: self.reads_duplicated,
            reads_mq0: self.reads_mq0,
            reads_qc_failed: self.reads_qc_failed,
            non_primary_alignments: self.non_primary_alignments,
            supplementary_alignments: self.supplementary_alignments,
            total_length: self.total_length,
            total_first_fragment_length: self.total_first_fragment_length,
            total_last_fragment_length: self.total_last_fragment_length,
            bases_mapped: self.bases_mapped,
            bases_mapped_cigar: self.bases_mapped_cigar,
            bases_trimmed: 0,
            bases_duplicated: self.bases_duplicated,
            mismatches: self.mismatches,
            error_rate: if self.bases_mapped_cigar == 0 {
                format_scientific(0.0)
            } else {
                format_scientific(u64_to_f32(self.mismatches) / u64_to_f32(self.bases_mapped_cigar))
            },
            average_length: format_zero_decimals(self.total_length, sequences),
            average_first_fragment_length: format_zero_decimals(
                self.total_first_fragment_length,
                self.first_fragments,
            ),
            average_last_fragment_length: format_zero_decimals(
                self.total_last_fragment_length,
                self.last_fragments,
            ),
            maximum_length: self.maximum_length,
            maximum_first_fragment_length: self.maximum_first_fragment_length,
            maximum_last_fragment_length: self.maximum_last_fragment_length,
            average_quality: format_one_decimal(if self.total_length == 0 {
                0.0
            } else {
                u64_to_f64(self.quality_sum) / u64_to_f64(self.total_length)
            }),
            insert_size_average: format_one_decimal(insert.average),
            insert_size_standard_deviation: format_one_decimal(insert.standard_deviation),
            inward_oriented_pairs: insert.inward,
            outward_oriented_pairs: insert.outward,
            pairs_with_other_orientation: insert.other,
            pairs_on_different_chromosomes: self.different_chromosome_observations / 2,
            percentage_properly_paired_reads: format_one_decimal(if sequences == 0 {
                0.0
            } else {
                f64::from(u64_to_f32(proper_numerator) / u64_to_f32(sequences))
            }),
            insert_sizes: insert.rows,
        })
    }
}

struct FinalizedInsertSizes {
    rows: Vec<InsertSizeRow>,
    inward: u64,
    outward: u64,
    other: u64,
    average: f64,
    standard_deviation: f64,
}

fn finalize_insert_sizes(
    raw: &BTreeMap<u32, InsertCounts>,
) -> Result<FinalizedInsertSizes, AlignGaugeError> {
    let mut halved = BTreeMap::new();
    for size in 0..=MAX_INSERT_SIZE {
        halved.insert(size, raw.get(&size).copied().unwrap_or_default().halved());
    }
    let mut inward = 0;
    let mut outward = 0;
    let mut other = 0;
    let mut all_pairs = 0;
    for counts in halved.values().copied() {
        add_assign(&mut inward, counts.inward, "inward_oriented_pairs")?;
        add_assign(&mut outward, counts.outward, "outward_oriented_pairs")?;
        add_assign(&mut other, counts.other, "pairs_with_other_orientation")?;
        add_assign(&mut all_pairs, counts.total()?, "insert_pairs")?;
    }
    if all_pairs == 0 {
        return Ok(FinalizedInsertSizes {
            rows: Vec::new(),
            inward,
            outward,
            other,
            average: 0.0,
            standard_deviation: 0.0,
        });
    }
    let mut bulk = 0_u64;
    let mut weighted = 0.0_f64;
    let mut bulk_end = 0_u32;
    let mut denominator = all_pairs;
    for size in 0..=MAX_INSERT_SIZE {
        let count = halved.get(&size).copied().unwrap_or_default().total()?;
        if count > 0 {
            bulk_end = size
                .checked_add(1)
                .ok_or_else(|| overflow("insert_bulk_end"))?;
        }
        bulk = checked_add(bulk, count, "insert_bulk")?;
        weighted += f64::from(size) * u64_to_f64(count);
        if (u64_to_f64(bulk) / u64_to_f64(all_pairs)) > MAIN_INSERT_BULK {
            bulk_end = size
                .checked_add(1)
                .ok_or_else(|| overflow("insert_bulk_end"))?;
            denominator = bulk;
            break;
        }
    }
    let average = weighted / u64_to_f64(denominator);
    let mut variance = 0.0_f64;
    for size in 1..bulk_end {
        let count = halved.get(&size).copied().unwrap_or_default().total()?;
        let delta = f64::from(size) - average;
        variance += u64_to_f64(count) * delta * delta / u64_to_f64(denominator);
    }
    let mut rows = Vec::with_capacity(usize::try_from(bulk_end).map_err(|source| {
        AlignGaugeError::new(
            ErrorCategory::InternalInvariant,
            "insert-size row count does not fit usize",
        )
        .with_source(source)
    })?);
    for size in 0..bulk_end {
        let counts = halved.get(&size).copied().unwrap_or_default();
        rows.push(InsertSizeRow {
            insert_size: size,
            pairs_total: counts.total()?,
            inward: counts.inward,
            outward: counts.outward,
            other: counts.other,
        });
    }
    Ok(FinalizedInsertSizes {
        rows,
        inward,
        outward,
        other,
        average,
        standard_deviation: variance.sqrt(),
    })
}

fn fragment_order(flags: u16) -> FragmentOrder {
    if flags & FLAG_PAIRED == 0 {
        return FragmentOrder::First;
    }
    match (flags & FLAG_READ1 != 0, flags & FLAG_READ2 != 0) {
        (true, false) => FragmentOrder::First,
        (false, true) => FragmentOrder::Last,
        (false, false) | (true, true) => FragmentOrder::Other,
    }
}

fn unclipped_length(
    record: &ValidatedRecord<'_>,
    sequence_length: u64,
) -> Result<u64, AlignGaugeError> {
    let raw = record
        .raw_cigar()
        .ok_or_else(|| plan_error(record, "CIGAR"))?;
    let mut length = sequence_length;
    for encoded in raw {
        if encoded & 0x0f == 5 {
            add_assign(&mut length, u64::from(encoded >> 4), "unclipped_length")?;
        }
    }
    Ok(length)
}

fn mapped_cigar_bases(record: &ValidatedRecord<'_>) -> Result<u64, AlignGaugeError> {
    let raw = record
        .raw_cigar()
        .ok_or_else(|| plan_error(record, "CIGAR"))?;
    let mut total = 0_u64;
    for encoded in raw {
        if matches!(encoded & 0x0f, 0 | 1 | 7 | 8) {
            add_assign(
                &mut total,
                u64::from(encoded >> 4),
                "record_mapped_cigar_bases",
            )?;
        }
    }
    Ok(total)
}

fn required_mate(
    record: &ValidatedRecord<'_>,
) -> Result<aligngauge_hts::RecordCoordinate, AlignGaugeError> {
    match record.mate_coordinate() {
        FieldValue::Value(Some(value)) => Ok(*value),
        FieldValue::Value(None) | FieldValue::Missing | FieldValue::NotRequested => {
            Err(plan_error(record, "mapped mate coordinate"))
        }
    }
}

fn u64_to_f32(value: u64) -> f32 {
    value
        .to_string()
        .parse::<f32>()
        .expect("every u64 decimal is representable as a finite f32")
}

fn u64_to_f64(value: u64) -> f64 {
    value
        .to_string()
        .parse::<f64>()
        .expect("every u64 decimal is representable as a finite f64")
}

fn format_zero_decimals(numerator: u64, denominator: u64) -> String {
    if denominator == 0 {
        return String::from("0");
    }
    format!("{:.0}", u64_to_f32(numerator) / u64_to_f32(denominator))
}

fn format_one_decimal(value: f64) -> String {
    format!("{value:.1}")
}

fn format_scientific(value: f32) -> String {
    let raw = format!("{value:.6e}");
    let (mantissa, exponent) = raw
        .split_once('e')
        .expect("Rust scientific formatting contains exponent");
    let exponent = exponent
        .parse::<i32>()
        .expect("Rust scientific exponent is numeric");
    format!("{mantissa}e{exponent:+03}")
}

fn increment(value: &mut u64, name: &'static str) -> Result<(), AlignGaugeError> {
    add_assign(value, 1, name)
}
fn add_assign(value: &mut u64, amount: u64, name: &'static str) -> Result<(), AlignGaugeError> {
    *value = checked_add(*value, amount, name)?;
    Ok(())
}
fn checked_add(left: u64, right: u64, name: &'static str) -> Result<u64, AlignGaugeError> {
    left.checked_add(right).ok_or_else(|| overflow(name))
}
fn overflow(name: &'static str) -> AlignGaugeError {
    AlignGaugeError::new(
        ErrorCategory::InternalInvariant,
        format!("Samtools stats accumulator '{name}' overflowed"),
    )
    .with_detail("accumulator", name)
}
fn plan_error(record: &ValidatedRecord<'_>, field: &'static str) -> AlignGaugeError {
    AlignGaugeError::new(
        ErrorCategory::InternalInvariant,
        "Samtools stats field plan did not expose a required validated field",
    )
    .with_detail("field", field)
    .with_detail("record_index", record.index())
}

/// Analyze one BAM with the exact Milestone 10 Samtools stats field plan.
///
/// # Errors
///
/// Returns a typed input, validation, compatibility, or checked-arithmetic error.
pub fn analyze_samtools_stats_bam(
    path: impl AsRef<Path>,
) -> Result<SamtoolsStatsReport, AlignGaugeError> {
    let mut reader = BamReader::open(path, FieldPlan::samtools_stats(), ReaderOptions::default())?;
    let mut collector = SamtoolsStatsCollector::default();
    while let Some(record) = reader.next_record()? {
        collector.observe(&record)?;
    }
    collector.finish()
}

#[cfg(test)]
mod tests {
    use super::{FLAG_PAIRED, FLAG_READ2, FragmentOrder, format_scientific, fragment_order};

    #[test]
    fn scientific_format_matches_c_exponent_width() {
        assert_eq!(format_scientific(0.0), "0.000000e+00");
        assert_eq!(format_scientific(0.001), "1.000000e-03");
    }

    #[test]
    fn paired_record_without_read1_or_read2_is_other() {
        assert_eq!(fragment_order(FLAG_PAIRED), FragmentOrder::Other);
        assert_eq!(
            fragment_order(FLAG_PAIRED | FLAG_READ2),
            FragmentOrder::Last
        );
        assert_eq!(fragment_order(0), FragmentOrder::First);
    }
}
