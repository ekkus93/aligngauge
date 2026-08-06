//! Checked v0.1 alignment counters and Samtools 1.24 compatibility projections.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use aligngauge_core::{
    AlignGaugeError, AlignmentCounters, Availability, BuildInfo, ErrorCategory, JsonValue,
    MetricDefinition, PerReferenceCounters, Provenance, Summary,
};
use aligngauge_hts::{
    BamReader, FieldPlan, FieldValue, ReaderOptions, RecordCoordinate, ReferenceSequence,
    ValidatedHeader, ValidatedRecord,
};

/// Canonical counter semantics selected for v0.1.
pub const COUNTER_PROFILE: &str = "aligngauge-v0.1";
/// Pinned compatibility tool version.
pub const SAMTOOLS_VERSION: &str = "1.24";
/// Exact flagstat compatibility profile.
pub const SAMTOOLS_FLAGSTAT_PROFILE: &str = "samtools-flagstat-1.24";
/// Exact idxstats compatibility profile.
pub const SAMTOOLS_IDXSTATS_PROFILE: &str = "samtools-idxstats-1.24";

const FLAG_PAIRED: u16 = 0x1;
const FLAG_PROPER_PAIR: u16 = 0x2;
const FLAG_UNMAPPED: u16 = 0x4;
const FLAG_MATE_UNMAPPED: u16 = 0x8;
const FLAG_READ1: u16 = 0x40;
const FLAG_READ2: u16 = 0x80;
const FLAG_SECONDARY: u16 = 0x100;
const FLAG_QC_FAIL: u16 = 0x200;
const FLAG_DUPLICATE: u16 = 0x400;
const FLAG_SUPPLEMENTARY: u16 = 0x800;

/// One mutually exclusive top-level alignment class.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RecordClass {
    /// Neither secondary nor supplementary.
    Primary,
    /// Secondary; this takes priority when both bits are set.
    Secondary,
    /// Supplementary without the secondary bit.
    Supplementary,
}

impl RecordClass {
    /// Classify with the pinned Samtools 1.24 priority.
    #[must_use]
    pub const fn from_flags(flags: u16) -> Self {
        if flags & FLAG_SECONDARY != 0 {
            Self::Secondary
        } else if flags & FLAG_SUPPLEMENTARY != 0 {
            Self::Supplementary
        } else {
            Self::Primary
        }
    }
}

/// Checked counters for one vendor-QC partition.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct CounterPartition {
    /// Total records.
    pub total: u64,
    /// Primary records.
    pub primary: u64,
    /// Secondary records.
    pub secondary: u64,
    /// Supplementary records.
    pub supplementary: u64,
    /// All mapped records.
    pub mapped: u64,
    /// All unmapped records.
    pub unmapped: u64,
    /// All duplicate records.
    pub duplicate: u64,
    /// Primary mapped records.
    pub primary_mapped: u64,
    /// Primary duplicate records.
    pub primary_duplicate: u64,
    /// Primary paired records.
    pub paired: u64,
    /// Primary mapped proper-pair records.
    pub proper_pair: u64,
    /// Primary read-one records.
    pub read1: u64,
    /// Primary read-two records.
    pub read2: u64,
    /// Primary paired records whose mate is mapped.
    pub mate_mapped: u64,
    /// Primary paired records whose mate is unmapped.
    pub mate_unmapped: u64,
    /// Primary pairs where both current record and mate are mapped.
    pub both_mapped: u64,
    /// Primary paired mapped records whose mate is unmapped.
    pub singleton: u64,
    /// Primary pairs mapped to different references.
    pub mate_different_reference: u64,
    /// Different-reference pairs with mapping quality at least five.
    pub mate_different_reference_mapq5: u64,
}

/// Exact QC-pass and QC-fail partitions.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct FlagstatCounters {
    /// Records without the vendor-QC-fail flag.
    pub qc_pass: CounterPartition,
    /// Records with the vendor-QC-fail flag.
    pub qc_fail: CounterPartition,
}

/// Final deterministic counter report.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CounterReport {
    partitions: FlagstatCounters,
    alignment: AlignmentCounters,
    per_reference: Vec<PerReferenceCounters>,
    no_coordinate_mapped: u64,
    no_coordinate_unmapped: u64,
}

impl CounterReport {
    /// Exact pass/fail partitions used by compatibility projections.
    #[must_use]
    pub const fn partitions(&self) -> &FlagstatCounters {
        &self.partitions
    }

    /// Canonical aggregate alignment counters.
    #[must_use]
    pub const fn alignment_counters(&self) -> &AlignmentCounters {
        &self.alignment
    }

    /// Per-reference counters in BAM header order.
    #[must_use]
    pub fn per_reference_counters(&self) -> &[PerReferenceCounters] {
        &self.per_reference
    }

    /// No-coordinate mapped count. The validated v0.1 reader keeps this zero.
    #[must_use]
    pub const fn no_coordinate_mapped(&self) -> u64 {
        self.no_coordinate_mapped
    }

    /// No-coordinate unmapped count.
    #[must_use]
    pub const fn no_coordinate_unmapped(&self) -> u64 {
        self.no_coordinate_unmapped
    }

    /// Build a canonical summary while explicitly marking deferred coverage unavailable.
    #[must_use]
    pub fn to_summary(&self, application: BuildInfo) -> Summary {
        Summary::new(
            application,
            metric_definitions(),
            Availability::Available(self.alignment.clone()),
            Availability::Available(self.per_reference.clone()),
            Availability::unavailable("coverage is deferred until Milestone 5"),
            Vec::new(),
        )
    }

    /// Record counter and compatibility profile identity in provenance.
    pub fn apply_provenance(&self, provenance: &mut Provenance) {
        provenance.backend_versions.insert(
            String::from("samtools_compatibility"),
            SAMTOOLS_VERSION.to_owned(),
        );
        provenance.analysis_plan.insert(
            String::from("counter_profile"),
            JsonValue::String(COUNTER_PROFILE.to_owned()),
        );
        provenance.analysis_plan.insert(
            String::from("counter_semantics"),
            JsonValue::String(String::from("checked-u64; secondary-before-supplementary")),
        );
        provenance.compatibility_profiles.extend([
            SAMTOOLS_FLAGSTAT_PROFILE.to_owned(),
            SAMTOOLS_IDXSTATS_PROFILE.to_owned(),
        ]);
        provenance.compatibility_profiles.sort();
        provenance.compatibility_profiles.dedup();
    }

    /// Render a stable human-readable canonical summary.
    #[must_use]
    pub fn render_human(&self) -> String {
        let mut output = String::new();
        for (name, value) in [
            ("total", self.alignment.total),
            ("qc_pass", self.alignment.qc_pass),
            ("qc_fail", self.alignment.qc_fail),
            ("primary", self.alignment.primary),
            ("secondary", self.alignment.secondary),
            ("supplementary", self.alignment.supplementary),
            ("mapped", self.alignment.mapped),
            ("unmapped", self.alignment.unmapped),
            ("paired", self.alignment.paired),
            ("proper_pair", self.alignment.proper_pair),
            ("read1", self.alignment.read1),
            ("read2", self.alignment.read2),
            ("mate_mapped", self.alignment.mate_mapped),
            ("mate_unmapped", self.alignment.mate_unmapped),
            ("duplicate", self.alignment.duplicate),
            ("singleton", self.alignment.singleton),
        ] {
            writeln!(output, "{name}\t{value}").expect("writing to String cannot fail");
        }
        output.push_str("per_reference\n");
        for reference in &self.per_reference {
            let unmapped = match reference.unmapped {
                Availability::Available(value) => value.to_string(),
                Availability::Unavailable { ref reason } => format!("unavailable:{reason}"),
            };
            writeln!(
                output,
                "{}\t{}\t{}\t{}",
                reference.name, reference.length, reference.mapped, unmapped
            )
            .expect("writing to String cannot fail");
        }
        writeln!(
            output,
            "*\t0\t{}\t{}",
            self.no_coordinate_mapped, self.no_coordinate_unmapped
        )
        .expect("writing to String cannot fail");
        output
    }

    /// Render text matching the pinned Samtools 1.24 flagstat line contract.
    #[must_use]
    pub fn render_samtools_flagstat(&self) -> String {
        let pass = &self.partitions.qc_pass;
        let fail = &self.partitions.qc_fail;
        let mut output = String::new();
        line(
            &mut output,
            pass.total,
            fail.total,
            "in total (QC-passed reads + QC-failed reads)",
        );
        line(&mut output, pass.primary, fail.primary, "primary");
        line(&mut output, pass.secondary, fail.secondary, "secondary");
        line(
            &mut output,
            pass.supplementary,
            fail.supplementary,
            "supplementary",
        );
        line(&mut output, pass.duplicate, fail.duplicate, "duplicates");
        line(
            &mut output,
            pass.primary_duplicate,
            fail.primary_duplicate,
            "primary duplicates",
        );
        percentage_line(
            &mut output,
            pass.mapped,
            fail.mapped,
            pass.total,
            fail.total,
            "mapped",
        );
        percentage_line(
            &mut output,
            pass.primary_mapped,
            fail.primary_mapped,
            pass.primary,
            fail.primary,
            "primary mapped",
        );
        line(
            &mut output,
            pass.paired,
            fail.paired,
            "paired in sequencing",
        );
        line(&mut output, pass.read1, fail.read1, "read1");
        line(&mut output, pass.read2, fail.read2, "read2");
        percentage_line(
            &mut output,
            pass.proper_pair,
            fail.proper_pair,
            pass.paired,
            fail.paired,
            "properly paired",
        );
        line(
            &mut output,
            pass.both_mapped,
            fail.both_mapped,
            "with itself and mate mapped",
        );
        percentage_line(
            &mut output,
            pass.singleton,
            fail.singleton,
            pass.paired,
            fail.paired,
            "singletons",
        );
        line(
            &mut output,
            pass.mate_different_reference,
            fail.mate_different_reference,
            "with mate mapped to a different chr",
        );
        line(
            &mut output,
            pass.mate_different_reference_mapq5,
            fail.mate_different_reference_mapq5,
            "with mate mapped to a different chr (mapQ>=5)",
        );
        output
    }

    /// Render exact Samtools 1.24 idxstats-compatible text.
    #[must_use]
    pub fn render_samtools_idxstats(&self) -> String {
        let mut output = String::new();
        for reference in &self.per_reference {
            let unmapped = match reference.unmapped {
                Availability::Available(value) => value,
                Availability::Unavailable { .. } => 0,
            };
            writeln!(
                output,
                "{}\t{}\t{}\t{}",
                reference.name, reference.length, reference.mapped, unmapped
            )
            .expect("writing to String cannot fail");
        }
        writeln!(
            output,
            "*\t0\t{}\t{}",
            self.no_coordinate_mapped, self.no_coordinate_unmapped
        )
        .expect("writing to String cannot fail");
        output
    }
}

/// Single-pass checked collector preserving header reference order.
pub struct CounterCollector {
    partitions: FlagstatCounters,
    references: Vec<ReferenceAccumulator>,
    no_coordinate_mapped: u64,
    no_coordinate_unmapped: u64,
}

impl CounterCollector {
    /// Initialize counters from the validated header.
    #[must_use]
    pub fn new(header: &ValidatedHeader) -> Self {
        Self {
            partitions: FlagstatCounters::default(),
            references: header
                .references()
                .iter()
                .map(ReferenceAccumulator::from)
                .collect(),
            no_coordinate_mapped: 0,
            no_coordinate_unmapped: 0,
        }
    }

    /// Observe one validated record.
    ///
    /// # Errors
    /// Returns a typed fatal error on any checked-counter overflow or impossible
    /// validated-field state.
    pub fn observe(&mut self, record: &ValidatedRecord<'_>) -> Result<(), AlignGaugeError> {
        let flags = record.flags();
        let partition = if flags & FLAG_QC_FAIL == 0 {
            &mut self.partitions.qc_pass
        } else {
            &mut self.partitions.qc_fail
        };

        increment(&mut partition.total, "partition.total")?;
        if flags & FLAG_UNMAPPED == 0 {
            increment(&mut partition.mapped, "partition.mapped")?;
        } else {
            increment(&mut partition.unmapped, "partition.unmapped")?;
        }
        if flags & FLAG_DUPLICATE != 0 {
            increment(&mut partition.duplicate, "partition.duplicate")?;
        }

        match RecordClass::from_flags(flags) {
            RecordClass::Secondary => increment(&mut partition.secondary, "partition.secondary")?,
            RecordClass::Supplementary => {
                increment(&mut partition.supplementary, "partition.supplementary")?;
            }
            RecordClass::Primary => observe_primary(partition, record)?,
        }

        self.observe_reference(record)
    }

    /// Finalize into an immutable report.
    ///
    /// # Errors
    /// Returns a fatal error if combining pass/fail partitions overflows.
    pub fn finish(self) -> Result<CounterReport, AlignGaugeError> {
        let alignment = aggregate(&self.partitions)?;
        let per_reference = self
            .references
            .into_iter()
            .map(|reference| PerReferenceCounters {
                name: reference.name,
                length: reference.length,
                mapped: reference.mapped,
                unmapped: Availability::Available(reference.unmapped),
            })
            .collect();

        Ok(CounterReport {
            partitions: self.partitions,
            alignment,
            per_reference,
            no_coordinate_mapped: self.no_coordinate_mapped,
            no_coordinate_unmapped: self.no_coordinate_unmapped,
        })
    }

    fn observe_reference(&mut self, record: &ValidatedRecord<'_>) -> Result<(), AlignGaugeError> {
        let unmapped = record.flags() & FLAG_UNMAPPED != 0;
        let Some(coordinate) = record.coordinate() else {
            if unmapped {
                return increment(&mut self.no_coordinate_unmapped, "no_coordinate_unmapped");
            }
            return increment(&mut self.no_coordinate_mapped, "no_coordinate_mapped");
        };

        let index = usize::try_from(coordinate.reference_id).map_err(|source| {
            AlignGaugeError::new(
                ErrorCategory::InternalInvariant,
                "validated reference ID does not fit usize",
            )
            .with_detail("reference_id", i64::from(coordinate.reference_id))
            .with_source(source)
        })?;
        let reference = self.references.get_mut(index).ok_or_else(|| {
            AlignGaugeError::new(
                ErrorCategory::InternalInvariant,
                "validated reference ID is absent from the counter table",
            )
            .with_detail("reference_id", i64::from(coordinate.reference_id))
        })?;
        if unmapped {
            increment(&mut reference.unmapped, "reference.unmapped")
        } else {
            increment(&mut reference.mapped, "reference.mapped")
        }
    }
}

/// Analyze one local BAM through the production reader and checked counter collector.
///
/// # Errors
/// Returns any reader validation failure or checked-counter overflow.
pub fn analyze_bam(path: impl AsRef<Path>) -> Result<CounterReport, AlignGaugeError> {
    let mut reader = BamReader::open(path, FieldPlan::counters(), ReaderOptions::default())?;
    let mut collector = CounterCollector::new(reader.header());
    while let Some(record) = reader.next_record()? {
        collector.observe(&record)?;
    }
    collector.finish()
}

fn observe_primary(
    partition: &mut CounterPartition,
    record: &ValidatedRecord<'_>,
) -> Result<(), AlignGaugeError> {
    let flags = record.flags();
    let mapped = flags & FLAG_UNMAPPED == 0;
    let paired = flags & FLAG_PAIRED != 0;
    let mate_unmapped = flags & FLAG_MATE_UNMAPPED != 0;

    increment(&mut partition.primary, "partition.primary")?;
    if mapped {
        increment(&mut partition.primary_mapped, "partition.primary_mapped")?;
    }
    if flags & FLAG_DUPLICATE != 0 {
        increment(
            &mut partition.primary_duplicate,
            "partition.primary_duplicate",
        )?;
    }

    if !paired {
        return Ok(());
    }
    increment(&mut partition.paired, "partition.paired")?;
    if flags & FLAG_READ1 != 0 {
        increment(&mut partition.read1, "partition.read1")?;
    }
    if flags & FLAG_READ2 != 0 {
        increment(&mut partition.read2, "partition.read2")?;
    }
    if flags & FLAG_PROPER_PAIR != 0 && mapped {
        increment(&mut partition.proper_pair, "partition.proper_pair")?;
    }
    if mate_unmapped {
        increment(&mut partition.mate_unmapped, "partition.mate_unmapped")?;
        if mapped {
            increment(&mut partition.singleton, "partition.singleton")?;
        }
        return Ok(());
    }

    increment(&mut partition.mate_mapped, "partition.mate_mapped")?;
    if !mapped {
        return Ok(());
    }
    increment(&mut partition.both_mapped, "partition.both_mapped")?;

    let mate = required_mate_coordinate(record)?;
    let current = record.coordinate().ok_or_else(|| {
        AlignGaugeError::new(
            ErrorCategory::InternalInvariant,
            "mapped validated record lost its current coordinate",
        )
        .with_detail("record_index", record.index())
    })?;
    if mate.reference_id != current.reference_id {
        increment(
            &mut partition.mate_different_reference,
            "partition.mate_different_reference",
        )?;
        if required_mapping_quality(record)? >= 5 {
            increment(
                &mut partition.mate_different_reference_mapq5,
                "partition.mate_different_reference_mapq5",
            )?;
        }
    }
    Ok(())
}

fn required_mate_coordinate(
    record: &ValidatedRecord<'_>,
) -> Result<RecordCoordinate, AlignGaugeError> {
    match record.mate_coordinate() {
        FieldValue::Value(Some(coordinate)) => Ok(*coordinate),
        FieldValue::Value(None) | FieldValue::Missing | FieldValue::NotRequested => {
            Err(AlignGaugeError::new(
                ErrorCategory::InternalInvariant,
                "counter plan did not expose a mapped mate coordinate",
            )
            .with_detail("record_index", record.index()))
        }
    }
}

fn required_mapping_quality(record: &ValidatedRecord<'_>) -> Result<u8, AlignGaugeError> {
    match record.mapping_quality() {
        FieldValue::Value(value) => Ok(*value),
        FieldValue::Missing | FieldValue::NotRequested => Err(AlignGaugeError::new(
            ErrorCategory::InternalInvariant,
            "counter plan did not expose mapping quality",
        )
        .with_detail("record_index", record.index())),
    }
}

fn aggregate(partitions: &FlagstatCounters) -> Result<AlignmentCounters, AlignGaugeError> {
    let pass = &partitions.qc_pass;
    let fail = &partitions.qc_fail;
    Ok(AlignmentCounters {
        total: add(pass.total, fail.total, "total")?,
        qc_pass: pass.total,
        qc_fail: fail.total,
        primary: add(pass.primary, fail.primary, "primary")?,
        secondary: add(pass.secondary, fail.secondary, "secondary")?,
        supplementary: add(pass.supplementary, fail.supplementary, "supplementary")?,
        mapped: add(pass.mapped, fail.mapped, "mapped")?,
        unmapped: add(pass.unmapped, fail.unmapped, "unmapped")?,
        paired: add(pass.paired, fail.paired, "paired")?,
        proper_pair: add(pass.proper_pair, fail.proper_pair, "proper_pair")?,
        read1: add(pass.read1, fail.read1, "read1")?,
        read2: add(pass.read2, fail.read2, "read2")?,
        mate_mapped: add(pass.mate_mapped, fail.mate_mapped, "mate_mapped")?,
        mate_unmapped: add(pass.mate_unmapped, fail.mate_unmapped, "mate_unmapped")?,
        duplicate: add(pass.duplicate, fail.duplicate, "duplicate")?,
        singleton: add(pass.singleton, fail.singleton, "singleton")?,
    })
}

fn increment(counter: &mut u64, name: &'static str) -> Result<(), AlignGaugeError> {
    *counter = counter.checked_add(1).ok_or_else(|| overflow_error(name))?;
    Ok(())
}

fn add(left: u64, right: u64, name: &'static str) -> Result<u64, AlignGaugeError> {
    left.checked_add(right).ok_or_else(|| overflow_error(name))
}

fn overflow_error(name: &'static str) -> AlignGaugeError {
    AlignGaugeError::new(
        ErrorCategory::InternalInvariant,
        format!("alignment counter '{name}' overflowed"),
    )
    .with_detail("counter", name)
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ReferenceAccumulator {
    name: String,
    length: u64,
    mapped: u64,
    unmapped: u64,
}

impl From<&ReferenceSequence> for ReferenceAccumulator {
    fn from(reference: &ReferenceSequence) -> Self {
        Self {
            name: reference.name().to_owned(),
            length: reference.length(),
            mapped: 0,
            unmapped: 0,
        }
    }
}

fn line(output: &mut String, pass: u64, fail: u64, label: &str) {
    writeln!(output, "{pass} + {fail} {label}").expect("writing to String cannot fail");
}

fn percentage_line(
    output: &mut String,
    pass: u64,
    fail: u64,
    pass_total: u64,
    fail_total: u64,
    label: &str,
) {
    writeln!(
        output,
        "{pass} + {fail} {label} ({} : {})",
        percentage(pass, pass_total),
        percentage(fail, fail_total)
    )
    .expect("writing to String cannot fail");
}

fn percentage(value: u64, total: u64) -> String {
    if total == 0 {
        return String::from("N/A");
    }

    let numerator = u128::from(value) * 10_000;
    let hundredths = (numerator + (u128::from(total) / 2)) / u128::from(total);
    let whole = hundredths / 100;
    let fraction = hundredths % 100;
    format!("{whole}.{fraction:02}%")
}

fn metric_definitions() -> BTreeMap<String, MetricDefinition> {
    [
        ("total", "All validated BAM records"),
        ("qc_pass", "Records passing vendor quality checks"),
        ("qc_fail", "Records failing vendor quality checks"),
        ("primary", "Records classified as primary"),
        ("secondary", "Records classified as secondary"),
        ("supplementary", "Records classified as supplementary"),
        ("mapped", "Records without the unmapped flag"),
        ("unmapped", "Records with the unmapped flag"),
        ("paired", "Primary records carrying the paired flag"),
        (
            "proper_pair",
            "Mapped primary records carrying the proper-pair flag",
        ),
        ("read1", "Primary paired records carrying the read-one flag"),
        ("read2", "Primary paired records carrying the read-two flag"),
        ("mate_mapped", "Primary paired records whose mate is mapped"),
        (
            "mate_unmapped",
            "Primary paired records whose mate is unmapped",
        ),
        ("duplicate", "All records carrying the duplicate flag"),
        (
            "singleton",
            "Mapped primary paired records whose mate is unmapped",
        ),
    ]
    .into_iter()
    .map(|(name, description)| {
        (
            name.to_owned(),
            MetricDefinition {
                description: description.to_owned(),
                unit: String::from("records"),
            },
        )
    })
    .collect()
}
