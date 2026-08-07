//! Command-line orchestration over the production BAM reader and v0.1 collectors.

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
use aligngauge_hts::{BamReader, FieldPlan, ReaderOptions};
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
    let coverage_options = CoverageOptions::new(
        config.memory_limit_bytes,
        config.coverage_thresholds.clone(),
    )?;
    let memory_plan = CoverageMemoryPlan::plan(
        coverage_options.memory_limit_bytes,
        1,
        coverage_options.chunk_size_override,
    )?;
    let input_identity = input_identity(&config.input)?;
    let field_plan = FieldPlan::counters().union(&FieldPlan::coverage());
    let effective_io_threads = effective_io_threads(config.io_threads);

    let traversal_started = Instant::now();
    let mut reader = BamReader::open(
        &config.input,
        field_plan.clone(),
        ReaderOptions {
            io_threads: effective_io_threads,
        },
    )?;
    let header_identity = reader.header().identity().sha256().to_owned();
    let mut counter_collector = CounterCollector::new(reader.header());
    let mut coverage_collector =
        CoverageCollector::new(reader.header(), coverage_options.thresholds, memory_plan)?;

    while let Some(record) = reader.next_record()? {
        counter_collector.observe(&record)?;
        coverage_collector.observe(&record)?;
    }
    let traversal_ns = elapsed_ns(traversal_started, "BAM traversal")?;

    let reduction_started = Instant::now();
    let counters = counter_collector.finish()?;
    let coverage = coverage_collector.finish()?;
    let reduction_ns = elapsed_ns(reduction_started, "collector finalization")?;

    let application = build_info();
    let warnings = release_warnings(config);
    let mut summary = counters.to_summary(application.clone());
    summary.coverage = Availability::Available(coverage.to_core_summary());
    add_coverage_metric_definitions(&mut summary.metric_definitions);
    summary.warnings = warnings.clone();

    let mut normalization_actions = Vec::new();
    if config.io_threads == 0 {
        normalization_actions.push(String::from(
            "io_threads=0 normalized to the serial HTSlib reader setting io_threads=1",
        ));
    }
    let mut provenance = Provenance::new(
        application,
        config.clone(),
        input_identity,
        Availability::Available(header_identity),
        BTreeMap::from([(String::from("rust-htslib"), String::from("1.0.1"))]),
        BTreeMap::from([
            (String::from("field_plan"), field_plan.to_json()),
            (String::from("bam_traversals"), JsonValue::Unsigned(1)),
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
        ]),
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
            (String::from("bam_traversal"), traversal_ns),
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
            "v0.1 does not compute a whole-file checksum during the single streaming pass",
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
