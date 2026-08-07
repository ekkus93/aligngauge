//! Command-line orchestration over the production BAM/CRAM reader and release collectors.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

use aligngauge_core::{
    AlignGaugeError, Availability, BuildInfo, ErrorCategory, InputIdentity, JsonValue,
    MetricDefinition, OutputBundle, Provenance, ResolvedConfig, Summary, SystemInfo, ToJson,
    Warning,
};
use aligngauge_coverage::{CoverageCollector, CoverageMemoryPlan, CoverageOptions, CoverageReport};
use aligngauge_hts::{
    AlignmentFormat, BamReader, FieldPlan, HTS_SYS_VERSION, HTSLIB_COMPATIBILITY_VERSION,
    HTSLIB_NETWORK_TRANSPORT_ENABLED, RUST_HTSLIB_VERSION, ReaderOptions, detect_alignment_format,
};
use aligngauge_metrics::{CounterCollector, analyze_bam as analyze_metrics_bam};

pub use aligngauge_metrics::CounterReport;

/// Legacy three-counter projection retained for the walking-skeleton compatibility probe.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct BamCounts {
    /// Number of decoded and validated records.
    pub total: u64,
    /// Number of records without the unmapped flag.
    pub mapped: u64,
    /// Number of records carrying the unmapped flag.
    pub unmapped: u64,
}

/// Deterministic release-pipeline checkpoints exposed for fault-injection tests.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ReleaseCheckpoint {
    /// Immediately before the counter collector observes a validated record.
    BeforeCounterCollector,
    /// After counters observe a record but before coverage observes it.
    BeforeCoverageCollector,
    /// Immediately before canonical output serialization.
    BeforeSerialization,
}

/// Hook used to inject deterministic release-pipeline failures.
pub trait ReleaseHook {
    /// Observe one checkpoint.
    ///
    /// # Errors
    /// Returning an error aborts the release operation immediately.
    fn checkpoint(&mut self, checkpoint: ReleaseCheckpoint) -> Result<(), AlignGaugeError>;
}

#[derive(Default)]
struct NoopReleaseHook;

impl ReleaseHook for NoopReleaseHook {
    fn checkpoint(&mut self, _checkpoint: ReleaseCheckpoint) -> Result<(), AlignGaugeError> {
        Ok(())
    }
}

/// Final in-memory v0.1 analysis result produced by one BAM traversal.
#[derive(Debug)]
pub struct ReleaseReport {
    counters: CounterReport,
    coverage: CoverageReport,
    summary: Summary,
    provenance: Provenance,
    input_traversals: u64,
}

impl ReleaseReport {
    /// Exact alignment counters.
    #[must_use]
    pub const fn counters(&self) -> &CounterReport {
        &self.counters
    }

    /// Exact canonical coverage.
    #[must_use]
    pub const fn coverage(&self) -> &CoverageReport {
        &self.coverage
    }

    /// Canonical summary model.
    #[must_use]
    pub const fn summary(&self) -> &Summary {
        &self.summary
    }

    /// Canonical provenance model.
    #[must_use]
    pub const fn provenance(&self) -> &Provenance {
        &self.provenance
    }

    /// Number of complete BAM traversals used to produce counters plus coverage.
    #[must_use]
    pub const fn input_traversals(&self) -> u64 {
        self.input_traversals
    }

    /// Build the required atomic output bundle.
    #[must_use]
    pub fn output_bundle(&self) -> OutputBundle {
        OutputBundle::new(
            self.summary.to_json_pretty().into_bytes(),
            self.provenance.to_json_pretty().into_bytes(),
        )
    }

    /// Build the canonical bundle after an injectable serialization checkpoint.
    ///
    /// # Errors
    /// Returns the injected typed failure before any output bundle is exposed.
    pub fn output_bundle_with_hook(
        &self,
        hook: &mut impl ReleaseHook,
    ) -> Result<OutputBundle, AlignGaugeError> {
        hook.checkpoint(ReleaseCheckpoint::BeforeSerialization)?;
        Ok(self.output_bundle())
    }

    /// Build the canonical bundle plus pinned Samtools compatibility files.
    ///
    /// # Errors
    /// Fails closed if a required compatibility source metric is unavailable.
    pub fn output_bundle_with_samtools_compatibility(
        &self,
    ) -> Result<OutputBundle, AlignGaugeError> {
        let mut bundle = self.output_bundle();
        bundle.insert(
            "samtools.flagstat.txt",
            self.counters.render_samtools_flagstat().into_bytes(),
        )?;
        bundle.insert(
            "samtools.idxstats.txt",
            self.counters.render_samtools_idxstats()?.into_bytes(),
        )?;
        Ok(bundle)
    }

    /// Stable human-readable v0.1 completion summary.
    #[must_use]
    pub fn render_human(&self) -> String {
        use std::fmt::Write as _;

        let mut output = self.counters.render_human();
        output.push_str("coverage\n");
        writeln!(
            output,
            "accepted_aligned_bases\t{}",
            self.coverage.total_accepted_aligned_bases()
        )
        .expect("writing to String cannot fail");
        let covered = self
            .coverage
            .per_reference()
            .iter()
            .map(|reference| reference.covered_reference_bases)
            .sum::<u64>();
        let uncovered = self
            .coverage
            .per_reference()
            .iter()
            .map(|reference| reference.uncovered_reference_bases)
            .sum::<u64>();
        writeln!(output, "covered_reference_bases\t{covered}")
            .expect("writing to String cannot fail");
        writeln!(output, "uncovered_reference_bases\t{uncovered}")
            .expect("writing to String cannot fail");
        for (threshold, percentage) in self.coverage.threshold_percentages() {
            writeln!(
                output,
                "coverage_at_least_{threshold}x_percent\t{percentage}"
            )
            .expect("writing to String cannot fail");
        }
        output.push_str("per_reference_coverage\n");
        for reference in self.coverage.per_reference() {
            writeln!(
                output,
                "{}\t{}\t{}\t{}\t{}\t{}",
                reference.name,
                reference.length,
                reference.accepted_aligned_bases,
                reference.covered_reference_bases,
                reference.uncovered_reference_bases,
                reference.mean_depth,
            )
            .expect("writing to String cannot fail");
        }
        output
    }
}

/// Analyze a BAM with all Milestone 4 counters.
///
/// This compatibility entry point intentionally remains counters-only. The v0.1 release path is
/// [`analyze_release`], which feeds counters and coverage from one reader traversal.
///
/// # Errors
/// Returns a typed reader failure or checked-counter overflow.
pub fn analyze_bam(path: impl AsRef<Path>) -> Result<CounterReport, AlignGaugeError> {
    analyze_metrics_bam(path)
}

/// Analyze one v0.1 run through one validated BAM traversal.
///
/// The exact coverage memory plan and input identity are resolved before record traversal. The
/// union field plan is opened once, and every validated record is passed to the counter collector
/// and then the coverage collector before the reader advances.
///
/// # Errors
/// Returns a typed configuration, resource, input-validation, collector, or provenance failure.
pub fn analyze_release(config: &ResolvedConfig) -> Result<ReleaseReport, AlignGaugeError> {
    analyze_release_with_reference(config, None)
}

/// Analyze a BAM or CRAM release run, requiring an explicit local FASTA for CRAM.
///
/// # Errors
/// Returns the first configuration, reference-integrity, reader, collector, or provenance failure.
pub fn analyze_release_with_reference(
    config: &ResolvedConfig,
    reference: Option<&Path>,
) -> Result<ReleaseReport, AlignGaugeError> {
    analyze_release_with_reference_and_hook(config, reference, &mut NoopReleaseHook)
}

fn release_coverage_setup(
    config: &ResolvedConfig,
) -> Result<(CoverageOptions, CoverageMemoryPlan), AlignGaugeError> {
    let coverage_options = CoverageOptions::new(
        config.memory_limit_bytes,
        config.coverage_thresholds.clone(),
    )?;
    let memory_plan = CoverageMemoryPlan::plan(
        coverage_options.memory_limit_bytes,
        1,
        coverage_options.chunk_size_override,
    )?;
    Ok((coverage_options, memory_plan))
}

fn open_release_reader(
    config: &ResolvedConfig,
    reference: Option<&Path>,
    field_plan: &FieldPlan,
    effective_io_threads: usize,
) -> Result<(AlignmentFormat, BamReader), AlignGaugeError> {
    let input_format = detect_alignment_format(&config.input)?;
    let reader = match input_format {
        AlignmentFormat::Bam => {
            if let Some(reference) = reference {
                return Err(AlignGaugeError::new(
                    ErrorCategory::Configuration,
                    "--reference is valid only for CRAM input",
                )
                .with_detail("reference", reference.to_string_lossy().into_owned()));
            }
            BamReader::open(
                &config.input,
                field_plan.clone(),
                ReaderOptions {
                    io_threads: effective_io_threads,
                },
            )?
        }
        AlignmentFormat::Cram => {
            let reference = reference.ok_or_else(|| {
                AlignGaugeError::new(
                    ErrorCategory::ReferenceRequired,
                    "CRAM input requires an explicit local FASTA supplied with --reference",
                )
                .with_detail("input", config.input.to_string_lossy().into_owned())
            })?;
            BamReader::open_cram(
                &config.input,
                reference,
                field_plan.clone(),
                ReaderOptions {
                    io_threads: effective_io_threads,
                },
            )?
        }
    };
    Ok((input_format, reader))
}

fn release_analysis_plan(
    field_plan: &FieldPlan,
    input_format: AlignmentFormat,
    reference_identity: Option<JsonValue>,
    config: &ResolvedConfig,
    effective_io_threads: usize,
) -> Result<BTreeMap<String, JsonValue>, AlignGaugeError> {
    Ok(BTreeMap::from([
        (String::from("field_plan"), field_plan.to_json()),
        (
            String::from("input_format"),
            input_format.as_str().to_json(),
        ),
        (String::from("alignment_traversals"), JsonValue::Unsigned(1)),
        (
            String::from("bam_traversals"),
            JsonValue::Unsigned(u64::from(matches!(input_format, AlignmentFormat::Bam))),
        ),
        (
            String::from("cram_traversals"),
            JsonValue::Unsigned(u64::from(matches!(input_format, AlignmentFormat::Cram))),
        ),
        (
            String::from("htslib_network_transport_enabled"),
            HTSLIB_NETWORK_TRANSPORT_ENABLED.to_json(),
        ),
        (
            String::from("local_reference"),
            reference_identity.unwrap_or(JsonValue::Null),
        ),
        (
            String::from("configured_collector_threads"),
            JsonValue::Unsigned(usize_to_u64(config.threads, "configured threads")?),
        ),
        (
            String::from("collector_threads_used"),
            JsonValue::Unsigned(1),
        ),
        (
            String::from("configured_io_threads"),
            JsonValue::Unsigned(usize_to_u64(config.io_threads, "configured I/O threads")?),
        ),
        (
            String::from("effective_reader_io_threads"),
            JsonValue::Unsigned(usize_to_u64(effective_io_threads, "effective I/O threads")?),
        ),
    ]))
}

/// Analyze one v0.1 run with deterministic fault-injection checkpoints.
///
/// # Errors
/// Returns the first injected or production release failure.
pub fn analyze_release_with_hook(
    config: &ResolvedConfig,
    hook: &mut impl ReleaseHook,
) -> Result<ReleaseReport, AlignGaugeError> {
    analyze_release_with_reference_and_hook(config, None, hook)
}

/// Analyze a BAM or CRAM release run with deterministic fault-injection checkpoints.
///
/// # Errors
/// Returns the first injected or production release failure.
pub fn analyze_release_with_reference_and_hook(
    config: &ResolvedConfig,
    reference: Option<&Path>,
    hook: &mut impl ReleaseHook,
) -> Result<ReleaseReport, AlignGaugeError> {
    let (coverage_options, memory_plan) = release_coverage_setup(config)?;
    let input_identity = input_identity(&config.input)?;
    let field_plan = FieldPlan::counters().union(&FieldPlan::coverage());
    let effective_io_threads = effective_io_threads(config.io_threads);

    let traversal_started = Instant::now();
    let (input_format, mut reader) =
        open_release_reader(config, reference, &field_plan, effective_io_threads)?;
    let reference_identity = reader.reference_identity().map(ToJson::to_json);
    let header_identity = reader.header().identity().sha256().to_owned();
    let mut counter_collector = CounterCollector::new(reader.header());
    let mut coverage_collector =
        CoverageCollector::new(reader.header(), coverage_options.thresholds, memory_plan)?;

    while let Some(record) = reader.next_record()? {
        hook.checkpoint(ReleaseCheckpoint::BeforeCounterCollector)?;
        counter_collector.observe(&record)?;
        hook.checkpoint(ReleaseCheckpoint::BeforeCoverageCollector)?;
        coverage_collector.observe(&record)?;
    }
    let traversal_ns = elapsed_ns(traversal_started, "alignment traversal")?;

    let reduction_started = Instant::now();
    let counters = counter_collector.finish()?;
    let coverage = coverage_collector.finish()?;
    let reduction_ns = elapsed_ns(reduction_started, "collector finalization")?;

    let application = build_info();
    let warnings = release_warnings(config);
    let mut summary = counters.to_summary(application.clone());
    summary.coverage = Availability::Available(coverage.to_core_summary());
    add_coverage_metric_definitions(&mut summary.metric_definitions);
    summary.warnings.clone_from(&warnings);

    let normalization_actions = release_normalization_actions(config);
    let mut provenance = Provenance::new(
        application,
        config.clone(),
        input_identity,
        Availability::Available(header_identity),
        BTreeMap::from([
            (String::from("hts-sys"), String::from(HTS_SYS_VERSION)),
            (
                String::from("htslib"),
                String::from(HTSLIB_COMPATIBILITY_VERSION),
            ),
            (
                String::from("rust-htslib"),
                String::from(RUST_HTSLIB_VERSION),
            ),
        ]),
        release_analysis_plan(
            &field_plan,
            input_format,
            reference_identity,
            config,
            effective_io_threads,
        )?,
        BTreeMap::from([
            (
                String::from("memory_limit_bytes"),
                config.memory_limit_bytes,
            ),
            (
                String::from("configured_threads"),
                usize_to_u64(config.threads, "configured threads")?,
            ),
            (
                String::from("configured_io_threads"),
                usize_to_u64(config.io_threads, "configured I/O threads")?,
            ),
        ]),
        BTreeMap::from([
            (String::from("alignment_traversal"), traversal_ns),
            (String::from("collector_finalization"), reduction_ns),
        ]),
        normalization_actions,
        Vec::new(),
        warnings,
        Vec::new(),
        system_info(),
    );
    counters.apply_provenance(&mut provenance);
    coverage.apply_provenance(&mut provenance)?;

    Ok(ReleaseReport {
        counters,
        coverage,
        summary,
        provenance,
        input_traversals: 1,
    })
}

fn release_normalization_actions(config: &ResolvedConfig) -> Vec<String> {
    if config.io_threads == 0 {
        vec![String::from(
            "io_threads=0 normalized to the serial HTSlib reader setting io_threads=1",
        )]
    } else {
        Vec::new()
    }
}

/// Validate a BAM and return the original three-counter projection.
///
/// # Errors
/// Returns a typed reader failure or checked-counter overflow.
pub fn count_bam(path: impl AsRef<Path>) -> Result<BamCounts, AlignGaugeError> {
    let report = analyze_bam(path)?;
    let counters = report.alignment_counters();
    Ok(BamCounts {
        total: counters.total,
        mapped: counters.mapped,
        unmapped: counters.unmapped,
    })
}

fn build_info() -> BuildInfo {
    BuildInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        git_commit: Availability::unavailable("build revision was not embedded"),
    }
}

fn input_identity(path: &Path) -> Result<InputIdentity, AlignGaugeError> {
    let metadata = fs::metadata(path).map_err(|source| {
        let category = if source.kind() == std::io::ErrorKind::NotFound {
            ErrorCategory::InputNotFound
        } else {
            ErrorCategory::InputFormat
        };
        AlignGaugeError::new(
            category,
            format!("failed to read input metadata for '{}'", path.display()),
        )
        .with_detail("input", path.to_string_lossy().into_owned())
        .with_source(source)
    })?;
    Ok(InputIdentity {
        path: path.to_string_lossy().into_owned(),
        size_bytes: Availability::Available(metadata.len()),
        checksum: Availability::unavailable(
            "the streaming release path does not compute a whole-file checksum during analysis",
        ),
    })
}

fn effective_io_threads(configured: usize) -> usize {
    configured.max(1)
}

fn elapsed_ns(started: Instant, stage: &'static str) -> Result<u64, AlignGaugeError> {
    u64::try_from(started.elapsed().as_nanos()).map_err(|source| {
        AlignGaugeError::new(
            ErrorCategory::InternalInvariant,
            format!("{stage} timing does not fit u64 nanoseconds"),
        )
        .with_source(source)
    })
}

fn usize_to_u64(value: usize, name: &'static str) -> Result<u64, AlignGaugeError> {
    u64::try_from(value).map_err(|source| {
        AlignGaugeError::new(
            ErrorCategory::InternalInvariant,
            format!("{name} does not fit canonical u64 provenance"),
        )
        .with_source(source)
    })
}

fn release_warnings(config: &ResolvedConfig) -> Vec<Warning> {
    let mut warnings = Vec::new();
    if config.threads > 1 {
        warnings.push(Warning {
            code: String::from("collector_threads_serial_v0_1"),
            message: format!(
                "v0.1 uses one deterministic collector thread; --threads={} is recorded as a resource limit but does not create parallel collectors",
                config.threads
            ),
        });
    }
    warnings
}

fn add_coverage_metric_definitions(definitions: &mut BTreeMap<String, MetricDefinition>) {
    for (name, description, unit) in [
        (
            "total_accepted_aligned_bases",
            "Accepted reference-aligned M/= /X bases under the aligngauge-v0.1 coverage policy",
            "bases",
        ),
        (
            "covered_reference_bases",
            "Reference positions with depth greater than zero",
            "bases",
        ),
        (
            "uncovered_reference_bases",
            "Reference positions with zero depth",
            "bases",
        ),
        (
            "depth_histogram",
            "Exact reference-base count keyed by integer depth",
            "bases",
        ),
        (
            "coverage_threshold_bases",
            "Reference bases meeting each configured cumulative depth threshold",
            "bases",
        ),
    ] {
        definitions.insert(
            name.to_owned(),
            MetricDefinition {
                description: description.to_owned(),
                unit: unit.to_owned(),
            },
        );
    }
}

fn system_info() -> SystemInfo {
    let logical_cpus = std::thread::available_parallelism()
        .ok()
        .and_then(|count| u64::try_from(count.get()).ok())
        .map_or_else(
            || Availability::unavailable("logical CPU count is unavailable"),
            Availability::Available,
        );
    SystemInfo {
        os: std::env::consts::OS.to_owned(),
        architecture: std::env::consts::ARCH.to_owned(),
        logical_cpus,
    }
}
