use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

use aligngauge_core::{AlignGaugeError, ErrorCategory};
use aligngauge_hts::{BamReader, FieldPlan, FieldValue, ReaderOptions, ValidatedRecord};

/// Pinned Picard executable version for Milestone 11.
pub const PICARD_VERSION: &str = "3.4.0";
/// Exact reference-independent alignment-summary compatibility profile.
pub const PICARD_ALIGNMENT_SUMMARY_PROFILE: &str =
    "picard-alignment-summary-3.4.0-all-reads-subset-v1";
/// Exact default `ALL_READS` insert-size compatibility profile.
pub const PICARD_INSERT_SIZE_PROFILE: &str = "picard-insert-size-3.4.0-all-reads-v1";

const FLAG_PAIRED: u16 = 0x1;
const FLAG_UNMAPPED: u16 = 0x4;
const FLAG_MATE_UNMAPPED: u16 = 0x8;
const FLAG_REVERSE: u16 = 0x10;
const FLAG_MATE_REVERSE: u16 = 0x20;
const FLAG_READ1: u16 = 0x40;
const FLAG_SECONDARY: u16 = 0x100;
const FLAG_QC_FAIL: u16 = 0x200;
const FLAG_DUPLICATE: u16 = 0x400;
const FLAG_SUPPLEMENTARY: u16 = 0x800;

const ADAPTER_MATCH_LENGTH: usize = 16;
const MAX_ADAPTER_ERRORS: usize = 1;
const MINIMUM_ORIENTATION_PCT: f64 = 0.05;
const INSERT_DEVIATIONS: f64 = 10.0;

const DEFAULT_ADAPTERS: [&str; 6] = [
    "AATGATACGGCGACCACCGAGATCTACACTCTTTCCCTACACGACGCTCTTCCGATCT",
    "AGATCGGAAGAGCTCGTATGCCGTCTTCTGCTTG",
    "AATGATACGGCGACCACCGAGATCTACACTCTTTCCCTACACGACGCTCTTCCGATCT",
    "AGATCGGAAGAGCGGTTCAGCAGGAATGCCGAGACCGATCTCGTATGCCGTCTTCTGCTTG",
    "AATGATACGGCGACCACCGAGATCTACACTCTTTCCCTACACGACGCTCTTCCGATCT",
    "AGATCGGAAGAGCACACGTCTGAACTCCAGTCACNNNNNNNNATCTCGTATGCCGTCTTCTGCTTG",
];

/// Picard alignment-summary category emitted by the `ALL_READS` collector.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PicardAlignmentCategory {
    /// Unpaired reads or Picard's required empty-file row.
    Unpaired,
    /// First reads of paired templates.
    FirstOfPair,
    /// Second reads of paired templates.
    SecondOfPair,
    /// Aggregate of first and second reads.
    Pair,
}

impl PicardAlignmentCategory {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Unpaired => "UNPAIRED",
            Self::FirstOfPair => "FIRST_OF_PAIR",
            Self::SecondOfPair => "SECOND_OF_PAIR",
            Self::Pair => "PAIR",
        }
    }
}

/// One exact row of the Milestone 11 reference-independent alignment-summary subset.
#[derive(Debug, Clone, PartialEq)]
pub struct PicardAlignmentSummaryRow {
    pub category: PicardAlignmentCategory,
    pub total_reads: u64,
    pub pf_reads: u64,
    pub pct_pf_reads: f64,
    pub pf_noise_reads: u64,
    pub pct_adapter: f64,
    pub mean_read_length: f64,
    pub sd_read_length: f64,
    pub median_read_length: f64,
    pub mad_read_length: f64,
    pub min_read_length: u64,
    pub max_read_length: u64,
    pub bad_cycles: u64,
}

/// Typed completed alignment-summary subset.
#[derive(Debug, Clone, PartialEq)]
pub struct PicardAlignmentSummaryReport {
    pub rows: Vec<PicardAlignmentSummaryRow>,
}

impl PicardAlignmentSummaryReport {
    /// Render a Picard metrics-file compatible subset. Unsupported reference-dependent
    /// columns are absent rather than synthesized as zero.
    #[must_use]
    pub fn render_picard_metrics(&self) -> String {
        let mut out = String::new();
        writeln!(out, "## htsjdk.samtools.metrics.StringHeader").expect("String write cannot fail");
        writeln!(
            out,
            "# AlignGauge exact compatibility projection: {PICARD_ALIGNMENT_SUMMARY_PROFILE}"
        )
        .expect("String write cannot fail");
        writeln!(out).expect("String write cannot fail");
        writeln!(
            out,
            "## METRICS CLASS\tpicard.analysis.AlignmentSummaryMetrics"
        )
        .expect("String write cannot fail");
        writeln!(out, "CATEGORY\tTOTAL_READS\tPF_READS\tPCT_PF_READS\tPF_NOISE_READS\tPCT_ADAPTER\tMEAN_READ_LENGTH\tSD_READ_LENGTH\tMEDIAN_READ_LENGTH\tMAD_READ_LENGTH\tMIN_READ_LENGTH\tMAX_READ_LENGTH\tBAD_CYCLES").expect("String write cannot fail");
        for row in &self.rows {
            writeln!(
                out,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                row.category.as_str(),
                row.total_reads,
                row.pf_reads,
                format_picard_float(row.pct_pf_reads),
                row.pf_noise_reads,
                format_picard_float(row.pct_adapter),
                format_picard_float(row.mean_read_length),
                format_picard_float(row.sd_read_length),
                format_picard_float(row.median_read_length),
                format_picard_float(row.mad_read_length),
                row.min_read_length,
                row.max_read_length,
                row.bad_cycles,
            )
            .expect("String write cannot fail");
        }
        out
    }
}

#[derive(Debug, Default)]
struct IntegerHistogram {
    bins: BTreeMap<u32, u64>,
    count: u64,
}

impl IntegerHistogram {
    fn increment(&mut self, key: u32) -> Result<(), AlignGaugeError> {
        let value = self.bins.entry(key).or_default();
        *value = checked_add(*value, 1, "histogram_bin")?;
        self.count = checked_add(self.count, 1, "histogram_count")?;
        Ok(())
    }

    fn value(&self, key: u32) -> u64 {
        self.bins.get(&key).copied().unwrap_or(0)
    }

    fn min(&self) -> Option<u32> {
        self.bins.first_key_value().map(|(key, _)| *key)
    }

    fn max(&self) -> Option<u32> {
        self.bins.last_key_value().map(|(key, _)| *key)
    }

    fn mean(&self) -> f64 {
        if self.count == 0 {
            return f64::NAN;
        }
        let weighted = self.bins.iter().fold(0.0, |total, (key, count)| {
            total + f64::from(*key) * u64_to_f64(*count)
        });
        weighted / u64_to_f64(self.count)
    }

    fn standard_deviation(&self) -> f64 {
        let mean = self.mean();
        let total = self.bins.iter().fold(0.0, |sum, (key, count)| {
            let delta = f64::from(*key) - mean;
            sum + u64_to_f64(*count) * delta * delta
        });
        (total / (u64_to_f64(self.count) - 1.0)).sqrt()
    }

    fn median(&self) -> f64 {
        weighted_median_u32(&self.bins, self.count)
    }

    fn median_absolute_deviation(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        let median = self.median();
        let mut deviations: BTreeMap<OrderedF64, u64> = BTreeMap::new();
        for (key, count) in &self.bins {
            let deviation = OrderedF64((f64::from(*key) - median).abs());
            let value = deviations.entry(deviation).or_default();
            *value = value
                .checked_add(*count)
                .expect("source histogram count already fits u64");
        }
        weighted_median_f64(&deviations, self.count)
    }

    fn mode(&self) -> f64 {
        self.bins
            .iter()
            .max_by(|left, right| left.1.cmp(right.1).then_with(|| right.0.cmp(left.0)))
            .map_or(0.0, |(key, _)| f64::from(*key))
    }

    fn trimmed(&self, width: u32) -> Self {
        let mut output = Self::default();
        for (key, count) in self.bins.range(..=width) {
            output.bins.insert(*key, *count);
            output.count = output
                .count
                .checked_add(*count)
                .expect("trimmed count cannot exceed source count");
        }
        output
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct OrderedF64(f64);

impl Eq for OrderedF64 {}
impl PartialOrd for OrderedF64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for OrderedF64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

#[derive(Debug, Default)]
struct AlignmentCategoryAccumulator {
    total_reads: u64,
    pf_reads: u64,
    pf_noise_reads: u64,
    adapter_reads: u64,
    read_lengths: IntegerHistogram,
    bad_cycles: BTreeMap<u32, u64>,
}

impl AlignmentCategoryAccumulator {
    fn observe(
        &mut self,
        record: &ValidatedRecord<'_>,
        adapter_kmers: &[Vec<u8>],
    ) -> Result<(), AlignGaugeError> {
        let flags = record.flags();
        let sequence = required_sequence(record)?;

        if flags & FLAG_SUPPLEMENTARY == 0 {
            increment(&mut self.total_reads, "alignment.total_reads")?;
            if flags & FLAG_QC_FAIL == 0 {
                increment(&mut self.pf_reads, "alignment.pf_reads")?;
                if required_noise_flag(record)? {
                    increment(&mut self.pf_noise_reads, "alignment.pf_noise_reads")?;
                }
                let length = u32::try_from(sequence.len()).map_err(|source| {
                    plan_error(record, "sequence length for Picard alignment summary")
                        .with_source(source)
                })?;
                self.read_lengths.increment(length)?;
                if is_adapter(record, sequence, adapter_kmers)? {
                    increment(&mut self.adapter_reads, "alignment.adapter_reads")?;
                }
            }
        }

        for (index, base) in sequence.iter().copied().enumerate() {
            if is_no_call(base) {
                let cycle = if flags & FLAG_REVERSE != 0 {
                    sequence.len().checked_sub(index).ok_or_else(|| {
                        AlignGaugeError::new(
                            ErrorCategory::InternalInvariant,
                            "reverse read cycle underflowed",
                        )
                    })?
                } else {
                    index
                        .checked_add(1)
                        .ok_or_else(|| overflow("alignment.read_cycle"))?
                };
                let cycle = u32::try_from(cycle).map_err(|source| {
                    plan_error(record, "Picard bad-cycle index").with_source(source)
                })?;
                let count = self.bad_cycles.entry(cycle).or_default();
                *count = checked_add(*count, 1, "alignment.bad_cycle_count")?;
            }
        }
        Ok(())
    }

    fn finish(&self, category: PicardAlignmentCategory) -> PicardAlignmentSummaryRow {
        if self.pf_reads == 0 {
            return PicardAlignmentSummaryRow {
                category,
                total_reads: self.total_reads,
                pf_reads: 0,
                pct_pf_reads: 0.0,
                pf_noise_reads: self.pf_noise_reads,
                pct_adapter: 0.0,
                mean_read_length: 0.0,
                sd_read_length: 0.0,
                median_read_length: 0.0,
                mad_read_length: 0.0,
                min_read_length: 0,
                max_read_length: 0,
                bad_cycles: 0,
            };
        }
        let bad_cycles = self.bad_cycles.values().fold(0_u64, |total, count| {
            if u64_to_f64(*count) / u64_to_f64(self.total_reads) >= 0.8 {
                total + 1
            } else {
                total
            }
        });
        PicardAlignmentSummaryRow {
            category,
            total_reads: self.total_reads,
            pf_reads: self.pf_reads,
            pct_pf_reads: u64_to_f64(self.pf_reads) / u64_to_f64(self.total_reads),
            pf_noise_reads: self.pf_noise_reads,
            pct_adapter: u64_to_f64(self.adapter_reads) / u64_to_f64(self.pf_reads),
            mean_read_length: self.read_lengths.mean(),
            sd_read_length: self.read_lengths.standard_deviation(),
            median_read_length: self.read_lengths.median(),
            mad_read_length: self.read_lengths.median_absolute_deviation(),
            min_read_length: u64::from(self.read_lengths.min().unwrap_or(0)),
            max_read_length: u64::from(self.read_lengths.max().unwrap_or(0)),
            bad_cycles,
        }
    }
}

/// Checked single-pass collector for the M11 alignment-summary subset.
#[derive(Debug)]
pub struct PicardAlignmentSummaryCollector {
    unpaired: AlignmentCategoryAccumulator,
    first: AlignmentCategoryAccumulator,
    second: AlignmentCategoryAccumulator,
    pair: AlignmentCategoryAccumulator,
    adapter_kmers: Vec<Vec<u8>>,
}

impl Default for PicardAlignmentSummaryCollector {
    fn default() -> Self {
        Self {
            unpaired: AlignmentCategoryAccumulator::default(),
            first: AlignmentCategoryAccumulator::default(),
            second: AlignmentCategoryAccumulator::default(),
            pair: AlignmentCategoryAccumulator::default(),
            adapter_kmers: default_adapter_kmers(),
        }
    }
}

impl PicardAlignmentSummaryCollector {
    /// Observe one record from `FieldPlan::picard_alignment_summary()`.
    ///
    /// # Errors
    /// Returns a typed error for missing planned fields or checked arithmetic failure.
    pub fn observe(&mut self, record: &ValidatedRecord<'_>) -> Result<(), AlignGaugeError> {
        if record.flags() & FLAG_SECONDARY != 0 {
            return Ok(());
        }
        if record.flags() & FLAG_PAIRED != 0 {
            self.pair.observe(record, &self.adapter_kmers)?;
            if record.flags() & FLAG_READ1 != 0 {
                self.first.observe(record, &self.adapter_kmers)?;
            } else {
                self.second.observe(record, &self.adapter_kmers)?;
            }
        } else {
            self.unpaired.observe(record, &self.adapter_kmers)?;
        }
        Ok(())
    }

    /// Finalize category ordering and Picard's paired `BAD_CYCLES` override.
    ///
    /// # Errors
    /// Returns a typed error if combining the paired bad-cycle counts overflows.
    pub fn finish(self) -> Result<PicardAlignmentSummaryReport, AlignGaugeError> {
        let first = self.first.finish(PicardAlignmentCategory::FirstOfPair);
        let second = self.second.finish(PicardAlignmentCategory::SecondOfPair);
        let mut pair = self.pair.finish(PicardAlignmentCategory::Pair);
        pair.bad_cycles = checked_add(
            first.bad_cycles,
            second.bad_cycles,
            "alignment.pair_bad_cycles",
        )?;
        let unpaired = self.unpaired.finish(PicardAlignmentCategory::Unpaired);

        let mut rows = Vec::with_capacity(4);
        if first.total_reads > 0 {
            rows.push(first);
            rows.push(second);
            rows.push(pair);
        }
        if unpaired.total_reads > 0 || rows.is_empty() {
            rows.push(unpaired);
        }
        Ok(PicardAlignmentSummaryReport { rows })
    }
}

/// Picard/HTSJDK pair orientation.
#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub enum PicardPairOrientation {
    Fr,
    Rf,
    Tandem,
}

impl PicardPairOrientation {
    const ALL: [Self; 3] = [Self::Fr, Self::Rf, Self::Tandem];

    const fn as_str(self) -> &'static str {
        match self {
            Self::Fr => "FR",
            Self::Rf => "RF",
            Self::Tandem => "TANDEM",
        }
    }

    const fn histogram_label(self) -> &'static str {
        match self {
            Self::Fr => "All_Reads.fr_count",
            Self::Rf => "All_Reads.rf_count",
            Self::Tandem => "All_Reads.tandem_count",
        }
    }
}

/// One Picard `InsertSizeMetrics` `ALL_READS` row plus its trimmed orientation histogram.
#[derive(Debug, Clone, PartialEq)]
pub struct PicardInsertSizeRow {
    pub median_insert_size: f64,
    pub mode_insert_size: f64,
    pub median_absolute_deviation: f64,
    pub min_insert_size: u32,
    pub max_insert_size: u32,
    pub mean_insert_size: f64,
    pub standard_deviation: f64,
    pub read_pairs: u64,
    pub pair_orientation: PicardPairOrientation,
    pub width_of_10_percent: u64,
    pub width_of_20_percent: u64,
    pub width_of_30_percent: u64,
    pub width_of_40_percent: u64,
    pub width_of_50_percent: u64,
    pub width_of_60_percent: u64,
    pub width_of_70_percent: u64,
    pub width_of_80_percent: u64,
    pub width_of_90_percent: u64,
    pub width_of_95_percent: u64,
    pub width_of_99_percent: u64,
    pub histogram: BTreeMap<u32, u64>,
}

/// Typed completed Picard insert-size report.
#[derive(Debug, Clone, PartialEq)]
pub struct PicardInsertSizeReport {
    pub rows: Vec<PicardInsertSizeRow>,
}

impl PicardInsertSizeReport {
    /// Render the default `ALL_READS` `InsertSizeMetrics` table and trimmed histogram surface.
    #[must_use]
    pub fn render_picard_metrics(&self) -> String {
        let mut out = String::new();
        writeln!(out, "## htsjdk.samtools.metrics.StringHeader").expect("String write cannot fail");
        writeln!(
            out,
            "# AlignGauge exact compatibility projection: {PICARD_INSERT_SIZE_PROFILE}"
        )
        .expect("String write cannot fail");
        writeln!(out).expect("String write cannot fail");
        writeln!(out, "## METRICS CLASS\tpicard.analysis.InsertSizeMetrics")
            .expect("String write cannot fail");
        writeln!(out, "MEDIAN_INSERT_SIZE\tMODE_INSERT_SIZE\tMEDIAN_ABSOLUTE_DEVIATION\tMIN_INSERT_SIZE\tMAX_INSERT_SIZE\tMEAN_INSERT_SIZE\tSTANDARD_DEVIATION\tREAD_PAIRS\tPAIR_ORIENTATION\tWIDTH_OF_10_PERCENT\tWIDTH_OF_20_PERCENT\tWIDTH_OF_30_PERCENT\tWIDTH_OF_40_PERCENT\tWIDTH_OF_50_PERCENT\tWIDTH_OF_60_PERCENT\tWIDTH_OF_70_PERCENT\tWIDTH_OF_80_PERCENT\tWIDTH_OF_90_PERCENT\tWIDTH_OF_95_PERCENT\tWIDTH_OF_99_PERCENT\tSAMPLE\tLIBRARY\tREAD_GROUP").expect("String write cannot fail");
        for row in &self.rows {
            writeln!(out, "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t\t\t",
                format_picard_float(row.median_insert_size),
                format_picard_float(row.mode_insert_size),
                format_picard_float(row.median_absolute_deviation),
                row.min_insert_size,
                row.max_insert_size,
                format_picard_float(row.mean_insert_size),
                format_picard_float(row.standard_deviation),
                row.read_pairs,
                row.pair_orientation.as_str(),
                row.width_of_10_percent,
                row.width_of_20_percent,
                row.width_of_30_percent,
                row.width_of_40_percent,
                row.width_of_50_percent,
                row.width_of_60_percent,
                row.width_of_70_percent,
                row.width_of_80_percent,
                row.width_of_90_percent,
                row.width_of_95_percent,
                row.width_of_99_percent,
            ).expect("String write cannot fail");
        }
        if !self.rows.is_empty() {
            writeln!(out).expect("String write cannot fail");
            writeln!(out, "## HISTOGRAM\tjava.lang.Integer").expect("String write cannot fail");
            write!(out, "insert_size").expect("String write cannot fail");
            for row in &self.rows {
                write!(out, "\t{}", row.pair_orientation.histogram_label())
                    .expect("String write cannot fail");
            }
            writeln!(out).expect("String write cannot fail");
            let max = self
                .rows
                .iter()
                .filter_map(|row| row.histogram.last_key_value().map(|(key, _)| *key))
                .max()
                .unwrap_or(0);
            for size in 0..=max {
                if self
                    .rows
                    .iter()
                    .all(|row| !row.histogram.contains_key(&size))
                {
                    continue;
                }
                write!(out, "{size}").expect("String write cannot fail");
                for row in &self.rows {
                    if let Some(value) = row.histogram.get(&size) {
                        write!(out, "\t{value}").expect("String write cannot fail");
                    } else {
                        write!(out, "\t").expect("String write cannot fail");
                    }
                }
                writeln!(out).expect("String write cannot fail");
            }
        }
        out
    }
}

/// Checked single-pass collector for Picard 3.4.0 default `ALL_READS` insert-size metrics.
#[derive(Debug, Default)]
pub struct PicardInsertSizeCollector {
    histograms: BTreeMap<PicardPairOrientation, IntegerHistogram>,
}

impl PicardInsertSizeCollector {
    /// Observe one record from `FieldPlan::picard_insert_size()`.
    ///
    /// # Errors
    /// Returns a typed error for missing planned fields, invalid same-contig pair state,
    /// TLEN absolute-value overflow, or checked accumulation failure.
    pub fn observe(&mut self, record: &ValidatedRecord<'_>) -> Result<(), AlignGaugeError> {
        let flags = record.flags();
        if flags & FLAG_PAIRED == 0
            || flags & FLAG_UNMAPPED != 0
            || flags & FLAG_MATE_UNMAPPED != 0
            || flags & FLAG_READ1 != 0
            || flags & (FLAG_SECONDARY | FLAG_SUPPLEMENTARY) != 0
            || flags & FLAG_DUPLICATE != 0
        {
            return Ok(());
        }
        let template_length = required_template_length(record)?;
        if template_length == 0 {
            return Ok(());
        }
        let insert_size = template_length.checked_abs().ok_or_else(|| {
            AlignGaugeError::new(
                ErrorCategory::UnsupportedRecord,
                "Picard insert size cannot represent abs(TLEN)",
            )
            .with_detail("record_index", record.index())
            .with_detail("template_length", i64::from(template_length))
        })?;
        let insert_size = u32::try_from(insert_size)
            .map_err(|source| plan_error(record, "Picard insert size").with_source(source))?;
        let orientation = pair_orientation(record, flags, template_length)?;
        self.histograms
            .entry(orientation)
            .or_default()
            .increment(insert_size)
    }

    /// Finalize Picard orientation filtering, robust trimming, moments, and width thresholds.
    ///
    /// # Errors
    /// Returns a typed error when reduction arithmetic cannot be represented.
    pub fn finish(self) -> Result<PicardInsertSizeReport, AlignGaugeError> {
        let total_inserts = self
            .histograms
            .values()
            .try_fold(0_u64, |total, histogram| {
                checked_add(total, histogram.count, "insert.total_inserts")
            })?;
        if total_inserts == 0 {
            return Ok(PicardInsertSizeReport { rows: Vec::new() });
        }

        let mut rows = Vec::with_capacity(3);
        for orientation in PicardPairOrientation::ALL {
            let histogram = self.histograms.get(&orientation);
            let count = histogram.map_or(0, |value| value.count);
            if u64_to_f64(count) < u64_to_f64(total_inserts) * MINIMUM_ORIENTATION_PCT {
                continue;
            }
            let histogram = histogram.ok_or_else(|| {
                AlignGaugeError::new(
                    ErrorCategory::InternalInvariant,
                    "Picard orientation passed minimum percentage without a histogram",
                )
            })?;
            rows.push(finalize_insert_row(orientation, histogram)?);
        }
        Ok(PicardInsertSizeReport { rows })
    }
}

fn finalize_insert_row(
    orientation: PicardPairOrientation,
    histogram: &IntegerHistogram,
) -> Result<PicardInsertSizeRow, AlignGaugeError> {
    let median = histogram.median();
    let mad = histogram.median_absolute_deviation();
    let width_float = median + INSERT_DEVIATIONS * mad;
    if !(0.0..=f64::from(u32::MAX)).contains(&width_float) {
        return Err(overflow("insert.histogram_width"));
    }
    let trim_width =
        truncating_f64_to_u32(width_float).ok_or_else(|| overflow("insert.histogram_width"))?;
    let trimmed = histogram.trimmed(trim_width);
    let widths = centered_widths(histogram)?;
    Ok(PicardInsertSizeRow {
        median_insert_size: median,
        mode_insert_size: histogram.mode(),
        median_absolute_deviation: mad,
        min_insert_size: histogram.min().unwrap_or(0),
        max_insert_size: histogram.max().unwrap_or(0),
        mean_insert_size: trimmed.mean(),
        standard_deviation: trimmed.standard_deviation(),
        read_pairs: histogram.count,
        pair_orientation: orientation,
        width_of_10_percent: widths[0],
        width_of_20_percent: widths[1],
        width_of_30_percent: widths[2],
        width_of_40_percent: widths[3],
        width_of_50_percent: widths[4],
        width_of_60_percent: widths[5],
        width_of_70_percent: widths[6],
        width_of_80_percent: widths[7],
        width_of_90_percent: widths[8],
        width_of_95_percent: widths[9],
        width_of_99_percent: widths[10],
        histogram: trimmed.bins,
    })
}

fn centered_widths(histogram: &IntegerHistogram) -> Result<[u64; 11], AlignGaugeError> {
    let thresholds = [0.1_f64, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.95, 0.99];
    let min = histogram.min().unwrap_or(0);
    let max = histogram.max().unwrap_or(0);
    let median = histogram.median();
    let mut low = median;
    let mut high = median;
    let mut covered = 0_u64;
    let mut result = [0_u64; 11];

    while low >= f64::from(min) - 1.0 || high <= f64::from(max) + 1.0 {
        let low_key = java_double_to_i32(low);
        if let Ok(low_key) = u32::try_from(low_key) {
            covered = checked_add(covered, histogram.value(low_key), "insert.width_covered")?;
        }
        if low.to_bits() != high.to_bits() {
            let high_key = java_double_to_i32(high);
            if let Ok(high_key) = u32::try_from(high_key) {
                covered = checked_add(covered, histogram.value(high_key), "insert.width_covered")?;
            }
        }
        let fraction = u64_to_f64(covered) / u64_to_f64(histogram.count);
        let distance = java_double_to_i32(high - low)
            .checked_add(1)
            .ok_or_else(|| overflow("insert.width_distance"))?;
        let distance = u64::try_from(distance).map_err(|source| {
            AlignGaugeError::new(
                ErrorCategory::InternalInvariant,
                "Picard centered width became negative",
            )
            .with_source(source)
        })?;
        for (slot, threshold) in result.iter_mut().zip(thresholds) {
            if *slot == 0 && fraction >= threshold {
                *slot = distance;
            }
        }
        low -= 1.0;
        high += 1.0;
    }
    Ok(result)
}

fn pair_orientation(
    record: &ValidatedRecord<'_>,
    flags: u16,
    template_length: i32,
) -> Result<PicardPairOrientation, AlignGaugeError> {
    let current = record
        .coordinate()
        .ok_or_else(|| plan_error(record, "mapped current coordinate"))?;
    let mate = required_mate(record)?;
    if current.reference_id != mate.reference_id {
        return Err(AlignGaugeError::new(
            ErrorCategory::UnsupportedRecord,
            "Picard pair orientation requires both reads on the same reference",
        )
        .with_detail("record_index", record.index())
        .with_detail("reference_id", i64::from(current.reference_id))
        .with_detail("mate_reference_id", i64::from(mate.reference_id)));
    }
    let read_reverse = flags & FLAG_REVERSE != 0;
    let mate_reverse = flags & FLAG_MATE_REVERSE != 0;
    if read_reverse == mate_reverse {
        return Ok(PicardPairOrientation::Tandem);
    }

    let current_start = current
        .position
        .checked_add(1)
        .ok_or_else(|| overflow("insert.current_start"))?;
    let mate_start = mate
        .position
        .checked_add(1)
        .ok_or_else(|| overflow("insert.mate_start"))?;
    let positive_five_prime = if read_reverse {
        mate_start
    } else {
        current_start
    };
    let negative_five_prime =
        if read_reverse {
            let reference_span = match record.cigar() {
                FieldValue::Value(value) => value.reference_span,
                FieldValue::Missing | FieldValue::NotRequested => {
                    return Err(plan_error(record, "CIGAR reference span"));
                }
            };
            current
                .position
                .checked_add(i64::try_from(reference_span).map_err(|source| {
                    plan_error(record, "CIGAR reference span").with_source(source)
                })?)
                .ok_or_else(|| overflow("insert.alignment_end"))?
        } else {
            current_start
                .checked_add(i64::from(template_length))
                .ok_or_else(|| overflow("insert.mate_five_prime"))?
        };
    Ok(if positive_five_prime < negative_five_prime {
        PicardPairOrientation::Fr
    } else {
        PicardPairOrientation::Rf
    })
}

fn is_adapter(
    record: &ValidatedRecord<'_>,
    sequence: &[u8],
    adapter_kmers: &[Vec<u8>],
) -> Result<bool, AlignGaugeError> {
    if sequence.len() < ADAPTER_MATCH_LENGTH {
        return Ok(false);
    }
    if record.flags() & FLAG_UNMAPPED == 0 {
        let mapq = match record.mapping_quality() {
            FieldValue::Value(value) => *value,
            FieldValue::Missing | FieldValue::NotRequested => {
                return Err(plan_error(record, "mapping quality"));
            }
        };
        if mapq != 0 {
            return Ok(false);
        }
    }
    let reverse = record.flags() & FLAG_UNMAPPED == 0 && record.flags() & FLAG_REVERSE != 0;
    for adapter in adapter_kmers {
        let mut errors = 0_usize;
        for (index, expected) in adapter.iter().copied().enumerate() {
            let observed = if reverse {
                complement(sequence[sequence.len() - index - 1])
            } else {
                sequence[index]
            };
            if !bases_equal(observed, expected) {
                errors += 1;
                if errors > MAX_ADAPTER_ERRORS {
                    break;
                }
            }
        }
        if errors <= MAX_ADAPTER_ERRORS {
            return Ok(true);
        }
    }
    Ok(false)
}

fn default_adapter_kmers() -> Vec<Vec<u8>> {
    let mut kmers = BTreeSet::new();
    for sequence in DEFAULT_ADAPTERS {
        let bytes = sequence.as_bytes();
        for window in bytes.windows(ADAPTER_MATCH_LENGTH) {
            let ambiguous = window
                .iter()
                .fold(0_usize, |count, base| count + usize::from(*base == b'N'));
            if ambiguous <= MAX_ADAPTER_ERRORS {
                let upper = window
                    .iter()
                    .map(u8::to_ascii_uppercase)
                    .collect::<Vec<_>>();
                kmers.insert(upper.clone());
                kmers.insert(reverse_complement(&upper));
            }
        }
    }
    kmers.into_iter().collect()
}

fn reverse_complement(sequence: &[u8]) -> Vec<u8> {
    sequence.iter().rev().copied().map(complement).collect()
}

fn complement(base: u8) -> u8 {
    match base.to_ascii_uppercase() {
        b'A' => b'T',
        b'C' => b'G',
        b'G' => b'C',
        b'T' => b'A',
        b'M' => b'K',
        b'R' => b'Y',
        b'W' => b'W',
        b'S' => b'S',
        b'Y' => b'R',
        b'K' => b'M',
        b'V' => b'B',
        b'H' => b'D',
        b'D' => b'H',
        b'B' => b'V',
        b'N' | b'.' => b'N',
        other => other,
    }
}

fn bases_equal(left: u8, right: u8) -> bool {
    base_mask(left) != 0 && base_mask(left) == base_mask(right)
}

fn base_mask(base: u8) -> u8 {
    match base.to_ascii_uppercase() {
        b'A' => 1,
        b'C' => 2,
        b'G' => 4,
        b'T' => 8,
        b'M' => 3,
        b'R' => 5,
        b'W' => 9,
        b'S' => 6,
        b'Y' => 10,
        b'K' => 12,
        b'V' => 7,
        b'H' => 11,
        b'D' => 13,
        b'B' => 14,
        b'N' | b'.' => 15,
        _ => 0,
    }
}

fn is_no_call(base: u8) -> bool {
    matches!(base, b'N' | b'n' | b'.')
}

fn weighted_median_u32(bins: &BTreeMap<u32, u64>, count: u64) -> f64 {
    if count == 0 {
        return 0.0;
    }
    if count == 1 {
        return bins
            .first_key_value()
            .map_or(0.0, |(key, _)| f64::from(*key));
    }
    let (low_rank, high_rank) = median_ranks(count);
    let mut seen = 0_u64;
    let mut low = None;
    let mut high = None;
    for (key, value) in bins {
        seen = seen
            .checked_add(*value)
            .expect("cumulative histogram count cannot exceed the checked source count");
        if low.is_none() && seen >= low_rank {
            low = Some(f64::from(*key));
        }
        if high.is_none() && seen >= high_rank {
            high = Some(f64::from(*key));
            break;
        }
    }
    f64::midpoint(low.unwrap_or(0.0), high.unwrap_or(0.0))
}

fn weighted_median_f64(bins: &BTreeMap<OrderedF64, u64>, count: u64) -> f64 {
    let (low_rank, high_rank) = median_ranks(count);
    let mut seen = 0_u64;
    let mut low = None;
    let mut high = None;
    for (key, value) in bins {
        seen = seen
            .checked_add(*value)
            .expect("cumulative histogram count cannot exceed the checked source count");
        if low.is_none() && seen >= low_rank {
            low = Some(key.0);
        }
        if high.is_none() && seen >= high_rank {
            high = Some(key.0);
            break;
        }
    }
    f64::midpoint(low.unwrap_or(0.0), high.unwrap_or(0.0))
}

const fn median_ranks(count: u64) -> (u64, u64) {
    if count.is_multiple_of(2) {
        (count / 2, count / 2 + 1)
    } else {
        let rank = count / 2 + 1;
        (rank, rank)
    }
}

fn java_double_to_i32(value: f64) -> i32 {
    if value.is_nan() {
        0
    } else if value >= f64::from(i32::MAX) {
        i32::MAX
    } else if value <= f64::from(i32::MIN) {
        i32::MIN
    } else {
        value
            .trunc()
            .to_string()
            .parse::<i32>()
            .unwrap_or_else(|_| {
                if value.is_sign_negative() {
                    i32::MIN
                } else {
                    i32::MAX
                }
            })
    }
}

fn truncating_f64_to_u32(value: f64) -> Option<u32> {
    if !value.is_finite() || value < 0.0 || value > f64::from(u32::MAX) {
        return None;
    }
    value.trunc().to_string().parse::<u32>().ok()
}

fn required_sequence<'a>(record: &'a ValidatedRecord<'_>) -> Result<&'a [u8], AlignGaugeError> {
    match record.sequence() {
        FieldValue::Value(value) => Ok(value),
        FieldValue::Missing | FieldValue::NotRequested => Err(plan_error(record, "sequence")),
    }
}

fn required_noise_flag(record: &ValidatedRecord<'_>) -> Result<bool, AlignGaugeError> {
    match record.noise_read() {
        FieldValue::Value(value) => Ok(*value),
        FieldValue::Missing => Ok(false),
        FieldValue::NotRequested => Err(plan_error(record, "XN noise tag")),
    }
}

fn required_template_length(record: &ValidatedRecord<'_>) -> Result<i32, AlignGaugeError> {
    match record.template_length() {
        FieldValue::Value(value) => Ok(*value),
        FieldValue::Missing | FieldValue::NotRequested => {
            Err(plan_error(record, "template length"))
        }
    }
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

fn format_picard_float(value: f64) -> String {
    if !value.is_finite() {
        return String::from("?");
    }
    let mut text = format!("{value:.6}");
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    if text == "-0" {
        String::from("0")
    } else {
        text
    }
}

fn u64_to_f64(value: u64) -> f64 {
    value
        .to_string()
        .parse::<f64>()
        .expect("every u64 decimal is representable as finite f64")
}

fn increment(value: &mut u64, name: &'static str) -> Result<(), AlignGaugeError> {
    *value = checked_add(*value, 1, name)?;
    Ok(())
}

fn checked_add(left: u64, right: u64, name: &'static str) -> Result<u64, AlignGaugeError> {
    left.checked_add(right).ok_or_else(|| overflow(name))
}

fn overflow(name: &'static str) -> AlignGaugeError {
    AlignGaugeError::new(
        ErrorCategory::InternalInvariant,
        format!("Picard compatibility accumulator '{name}' overflowed"),
    )
    .with_detail("accumulator", name)
}

fn plan_error(record: &ValidatedRecord<'_>, field: &'static str) -> AlignGaugeError {
    AlignGaugeError::new(
        ErrorCategory::InternalInvariant,
        "Picard compatibility field plan did not expose a required validated field",
    )
    .with_detail("field", field)
    .with_detail("record_index", record.index())
}

/// Analyze one BAM with the M11 reference-independent alignment-summary field plan.
///
/// # Errors
/// Returns typed BAM validation, field-plan, or checked-arithmetic failures.
pub fn analyze_picard_alignment_summary_bam(
    path: impl AsRef<Path>,
) -> Result<PicardAlignmentSummaryReport, AlignGaugeError> {
    let mut reader = BamReader::open(
        path,
        FieldPlan::picard_alignment_summary(),
        ReaderOptions::default(),
    )?;
    let mut collector = PicardAlignmentSummaryCollector::default();
    while let Some(record) = reader.next_record()? {
        collector.observe(&record)?;
    }
    collector.finish()
}

/// Analyze one BAM with the exact default `ALL_READS` Picard insert-size field plan.
///
/// # Errors
/// Returns typed BAM validation, field-plan, orientation, or checked-arithmetic failures.
pub fn analyze_picard_insert_size_bam(
    path: impl AsRef<Path>,
) -> Result<PicardInsertSizeReport, AlignGaugeError> {
    let mut reader = BamReader::open(
        path,
        FieldPlan::picard_insert_size(),
        ReaderOptions::default(),
    )?;
    let mut collector = PicardInsertSizeCollector::default();
    while let Some(record) = reader.next_record()? {
        collector.observe(&record)?;
    }
    collector.finish()
}

#[cfg(test)]
mod tests {
    use super::{IntegerHistogram, PicardPairOrientation, centered_widths, default_adapter_kmers};

    #[test]
    fn mode_ties_choose_the_smallest_insert_size() {
        let mut histogram = IntegerHistogram::default();
        histogram.increment(20).unwrap();
        histogram.increment(10).unwrap();
        assert_eq!(histogram.mode().to_bits(), 10.0_f64.to_bits());
    }

    #[test]
    fn even_median_and_mad_preserve_half_values() {
        let mut histogram = IntegerHistogram::default();
        for value in [10, 11, 20, 21] {
            histogram.increment(value).unwrap();
        }
        assert_eq!(histogram.median().to_bits(), 15.5_f64.to_bits());
        assert_eq!(
            histogram.median_absolute_deviation().to_bits(),
            5.0_f64.to_bits()
        );
    }

    #[test]
    fn centered_widths_follow_integer_bin_expansion() {
        let mut histogram = IntegerHistogram::default();
        for value in [10, 10, 10, 11, 12, 20] {
            histogram.increment(value).unwrap();
        }
        let widths = centered_widths(&histogram).unwrap();
        assert!(widths.iter().all(|width| *width > 0));
        assert!(widths.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn adapter_kmers_are_deduplicated_and_sixteen_bases() {
        let kmers = default_adapter_kmers();
        assert!(!kmers.is_empty());
        assert!(kmers.iter().all(|kmer| kmer.len() == 16));
    }

    #[test]
    fn orientation_order_is_picard_enum_order() {
        assert_eq!(
            PicardPairOrientation::ALL,
            [
                PicardPairOrientation::Fr,
                PicardPairOrientation::Rf,
                PicardPairOrientation::Tandem
            ]
        );
    }
}
