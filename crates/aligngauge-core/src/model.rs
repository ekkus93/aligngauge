//! Canonical v0.1 summary and provenance models.

use std::collections::BTreeMap;

use crate::config::ResolvedConfig;
use crate::json::{JsonValue, ToJson};

/// Canonical summary schema version.
pub const SUMMARY_SCHEMA_VERSION: &str = "1.0.0";
/// Canonical provenance schema version.
pub const PROVENANCE_SCHEMA_VERSION: &str = "1.0.0";

/// Application identity recorded in canonical outputs.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BuildInfo {
    /// Application version.
    pub version: String,
    /// Source revision, or an explicit unavailable reason.
    pub git_commit: Availability<String>,
}

impl ToJson for BuildInfo {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (String::from("git_commit"), self.git_commit.to_json()),
            (String::from("version"), self.version.to_json()),
        ]))
    }
}

/// A metric definition and unit.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MetricDefinition {
    /// Human-readable definition.
    pub description: String,
    /// Stable unit spelling.
    pub unit: String,
}

impl ToJson for MetricDefinition {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (String::from("description"), self.description.to_json()),
            (String::from("unit"), self.unit.to_json()),
        ]))
    }
}

/// Explicitly available or unavailable data.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Availability<T> {
    /// A defined value.
    Available(T),
    /// An unavailable value with a non-empty reason.
    Unavailable {
        /// Reason the value is unavailable.
        reason: String,
    },
}

impl<T> Availability<T> {
    /// Construct an unavailable value.
    ///
    /// # Panics
    ///
    /// Panics when `reason` is empty, preventing ambiguous unavailable values.
    #[must_use]
    pub fn unavailable(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        assert!(
            !reason.trim().is_empty(),
            "unavailable reason must not be empty"
        );
        Self::Unavailable { reason }
    }
}

impl<T: ToJson> ToJson for Availability<T> {
    fn to_json(&self) -> JsonValue {
        match self {
            Self::Available(value) => JsonValue::Object(BTreeMap::from([
                (String::from("status"), "available".to_json()),
                (String::from("value"), value.to_json()),
            ])),
            Self::Unavailable { reason } => JsonValue::Object(BTreeMap::from([
                (String::from("reason"), reason.to_json()),
                (String::from("status"), "unavailable".to_json()),
            ])),
        }
    }
}

/// Canonical v0.1 alignment counters.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct AlignmentCounters {
    /// Total records.
    pub total: u64,
    /// Records passing vendor quality checks.
    pub qc_pass: u64,
    /// Records failing vendor quality checks.
    pub qc_fail: u64,
    /// Primary records.
    pub primary: u64,
    /// Secondary records.
    pub secondary: u64,
    /// Supplementary records.
    pub supplementary: u64,
    /// Mapped records.
    pub mapped: u64,
    /// Unmapped records.
    pub unmapped: u64,
    /// Paired records.
    pub paired: u64,
    /// Proper-pair records.
    pub proper_pair: u64,
    /// Read-one records.
    pub read1: u64,
    /// Read-two records.
    pub read2: u64,
    /// Records with mapped mates.
    pub mate_mapped: u64,
    /// Records with unmapped mates.
    pub mate_unmapped: u64,
    /// Duplicate records.
    pub duplicate: u64,
    /// Singleton records.
    pub singleton: u64,
}

impl ToJson for AlignmentCounters {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (String::from("duplicate"), self.duplicate.to_json()),
            (String::from("mapped"), self.mapped.to_json()),
            (String::from("mate_mapped"), self.mate_mapped.to_json()),
            (
                String::from("mate_unmapped"),
                self.mate_unmapped.to_json(),
            ),
            (String::from("paired"), self.paired.to_json()),
            (String::from("primary"), self.primary.to_json()),
            (String::from("proper_pair"), self.proper_pair.to_json()),
            (String::from("qc_fail"), self.qc_fail.to_json()),
            (String::from("qc_pass"), self.qc_pass.to_json()),
            (String::from("read1"), self.read1.to_json()),
            (String::from("read2"), self.read2.to_json()),
            (String::from("secondary"), self.secondary.to_json()),
            (String::from("singleton"), self.singleton.to_json()),
            (
                String::from("supplementary"),
                self.supplementary.to_json(),
            ),
            (String::from("total"), self.total.to_json()),
            (String::from("unmapped"), self.unmapped.to_json()),
        ]))
    }
}

/// Per-reference record counters.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PerReferenceCounters {
    /// Reference name.
    pub name: String,
    /// Declared reference length.
    pub length: u64,
    /// Mapped records assigned to the reference.
    pub mapped: u64,
    /// Unmapped records assigned by the selected compatibility definition.
    pub unmapped: Availability<u64>,
}

impl ToJson for PerReferenceCounters {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (String::from("length"), self.length.to_json()),
            (String::from("mapped"), self.mapped.to_json()),
            (String::from("name"), self.name.to_json()),
            (String::from("unmapped"), self.unmapped.to_json()),
        ]))
    }
}

/// Canonical coverage policy.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CoveragePolicy {
    /// Stable profile name.
    pub name: String,
    /// Minimum mapping quality.
    pub minimum_mapq: u32,
    /// Whether duplicate records are included.
    pub include_duplicates: bool,
    /// Whether vendor-QC-fail records are included.
    pub include_qc_fail: bool,
    /// Whether secondary records are included.
    pub include_secondary: bool,
    /// Whether supplementary records are included.
    pub include_supplementary: bool,
    /// Whether exact mate-overlap correction is enabled.
    pub mate_overlap_correction: bool,
}

impl ToJson for CoveragePolicy {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (
                String::from("include_duplicates"),
                self.include_duplicates.to_json(),
            ),
            (
                String::from("include_qc_fail"),
                self.include_qc_fail.to_json(),
            ),
            (
                String::from("include_secondary"),
                self.include_secondary.to_json(),
            ),
            (
                String::from("include_supplementary"),
                self.include_supplementary.to_json(),
            ),
            (
                String::from("mate_overlap_correction"),
                self.mate_overlap_correction.to_json(),
            ),
            (
                String::from("minimum_mapq"),
                self.minimum_mapq.to_json(),
            ),
            (String::from("name"), self.name.to_json()),
        ]))
    }
}

/// Exact canonical coverage results.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CoverageSummary {
    /// Coverage policy used.
    pub policy: CoveragePolicy,
    /// Accepted aligned reference bases.
    pub total_accepted_aligned_bases: u64,
    /// Whole-run depth histogram, keyed by depth.
    pub depth_histogram: BTreeMap<String, u64>,
    /// Reference bases meeting each configured threshold.
    pub threshold_bases: BTreeMap<String, u64>,
    /// Covered reference bases.
    pub covered_reference_bases: u64,
    /// Uncovered reference bases.
    pub uncovered_reference_bases: u64,
}

impl ToJson for CoverageSummary {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (
                String::from("covered_reference_bases"),
                self.covered_reference_bases.to_json(),
            ),
            (
                String::from("depth_histogram"),
                self.depth_histogram.to_json(),
            ),
            (String::from("policy"), self.policy.to_json()),
            (
                String::from("threshold_bases"),
                self.threshold_bases.to_json(),
            ),
            (
                String::from("total_accepted_aligned_bases"),
                self.total_accepted_aligned_bases.to_json(),
            ),
            (
                String::from("uncovered_reference_bases"),
                self.uncovered_reference_bases.to_json(),
            ),
        ]))
    }
}

/// Correctness-preserving warning.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Warning {
    /// Stable warning code.
    pub code: String,
    /// Human-readable warning.
    pub message: String,
}

impl ToJson for Warning {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (String::from("code"), self.code.to_json()),
            (String::from("message"), self.message.to_json()),
        ]))
    }
}

/// Canonical `summary.json` model.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Summary {
    /// Independent output schema version.
    pub schema_version: String,
    /// Application identity.
    pub application: BuildInfo,
    /// Stable metric definitions keyed by metric name.
    pub metric_definitions: BTreeMap<String, MetricDefinition>,
    /// Alignment counters.
    pub alignment_counters: Availability<AlignmentCounters>,
    /// Per-reference counters in deterministic reference order.
    pub per_reference_counters: Availability<Vec<PerReferenceCounters>>,
    /// Coverage results.
    pub coverage: Availability<CoverageSummary>,
    /// Correctness-preserving warnings.
    pub warnings: Vec<Warning>,
}

impl Summary {
    /// Construct a summary with deterministic list ordering.
    #[must_use]
    pub fn new(
        application: BuildInfo,
        metric_definitions: BTreeMap<String, MetricDefinition>,
        alignment_counters: Availability<AlignmentCounters>,
        per_reference_counters: Availability<Vec<PerReferenceCounters>>,
        coverage: Availability<CoverageSummary>,
        mut warnings: Vec<Warning>,
    ) -> Self {
        warnings.sort_by(|left, right| {
            left.code
                .cmp(&right.code)
                .then_with(|| left.message.cmp(&right.message))
        });
        let per_reference_counters = match per_reference_counters {
            Availability::Available(mut counters) => {
                counters.sort_by(|left, right| left.name.cmp(&right.name));
                Availability::Available(counters)
            }
            unavailable => unavailable,
        };
        Self {
            schema_version: SUMMARY_SCHEMA_VERSION.to_owned(),
            application,
            metric_definitions,
            alignment_counters,
            per_reference_counters,
            coverage,
            warnings,
        }
    }
}

impl ToJson for Summary {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (
                String::from("alignment_counters"),
                self.alignment_counters.to_json(),
            ),
            (String::from("application"), self.application.to_json()),
            (String::from("coverage"), self.coverage.to_json()),
            (
                String::from("metric_definitions"),
                self.metric_definitions.to_json(),
            ),
            (
                String::from("per_reference_counters"),
                self.per_reference_counters.to_json(),
            ),
            (
                String::from("schema_version"),
                self.schema_version.to_json(),
            ),
            (String::from("warnings"), self.warnings.to_json()),
        ]))
    }
}

/// Input identity recorded in provenance.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InputIdentity {
    /// User-supplied local path.
    pub path: String,
    /// Input size in bytes.
    pub size_bytes: Availability<u64>,
    /// Checksum policy and result.
    pub checksum: Availability<String>,
}

impl ToJson for InputIdentity {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (String::from("checksum"), self.checksum.to_json()),
            (String::from("path"), self.path.to_json()),
            (String::from("size_bytes"), self.size_bytes.to_json()),
        ]))
    }
}

/// Reproducibility-relevant operating-system and CPU identity.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SystemInfo {
    /// Operating-system family.
    pub os: String,
    /// CPU architecture.
    pub architecture: String,
    /// Available logical CPU count.
    pub logical_cpus: Availability<u64>,
}

impl ToJson for SystemInfo {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (
                String::from("architecture"),
                self.architecture.to_json(),
            ),
            (
                String::from("logical_cpus"),
                self.logical_cpus.to_json(),
            ),
            (String::from("os"), self.os.to_json()),
        ]))
    }
}

/// Canonical `provenance.json` model.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Provenance {
    /// Independent output schema version.
    pub schema_version: String,
    /// Application identity.
    pub application: BuildInfo,
    /// Fully resolved configuration.
    pub resolved_config: ResolvedConfig,
    /// Input identity.
    pub input: InputIdentity,
    /// Header identity.
    pub header_identity: Availability<String>,
    /// Backend versions keyed by component.
    pub backend_versions: BTreeMap<String, String>,
    /// Immutable analysis-plan description.
    pub analysis_plan: BTreeMap<String, JsonValue>,
    /// Resource limits keyed by stable name.
    pub resource_limits: BTreeMap<String, u64>,
    /// Stage timings in integer nanoseconds.
    pub stage_timings_ns: BTreeMap<String, u64>,
    /// Input-normalization actions.
    pub normalization_actions: Vec<String>,
    /// Enabled compatibility profiles.
    pub compatibility_profiles: Vec<String>,
    /// Correctness-preserving warnings.
    pub warnings: Vec<Warning>,
    /// Structured errors retained for incomplete diagnostic bundles.
    pub errors: Vec<JsonValue>,
    /// Reproducibility-relevant system identity.
    pub system: SystemInfo,
}

impl Provenance {
    /// Construct provenance with deterministic list ordering.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        application: BuildInfo,
        resolved_config: ResolvedConfig,
        input: InputIdentity,
        header_identity: Availability<String>,
        backend_versions: BTreeMap<String, String>,
        analysis_plan: BTreeMap<String, JsonValue>,
        resource_limits: BTreeMap<String, u64>,
        stage_timings_ns: BTreeMap<String, u64>,
        mut normalization_actions: Vec<String>,
        mut compatibility_profiles: Vec<String>,
        mut warnings: Vec<Warning>,
        errors: Vec<JsonValue>,
        system: SystemInfo,
    ) -> Self {
        normalization_actions.sort();
        compatibility_profiles.sort();
        warnings.sort_by(|left, right| {
            left.code
                .cmp(&right.code)
                .then_with(|| left.message.cmp(&right.message))
        });
        Self {
            schema_version: PROVENANCE_SCHEMA_VERSION.to_owned(),
            application,
            resolved_config,
            input,
            header_identity,
            backend_versions,
            analysis_plan,
            resource_limits,
            stage_timings_ns,
            normalization_actions,
            compatibility_profiles,
            warnings,
            errors,
            system,
        }
    }
}

impl ToJson for Provenance {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (
                String::from("analysis_plan"),
                JsonValue::Object(self.analysis_plan.clone()),
            ),
            (String::from("application"), self.application.to_json()),
            (
                String::from("backend_versions"),
                self.backend_versions.to_json(),
            ),
            (
                String::from("compatibility_profiles"),
                self.compatibility_profiles.to_json(),
            ),
            (String::from("errors"), JsonValue::Array(self.errors.clone())),
            (
                String::from("header_identity"),
                self.header_identity.to_json(),
            ),
            (String::from("input"), self.input.to_json()),
            (
                String::from("normalization_actions"),
                self.normalization_actions.to_json(),
            ),
            (
                String::from("resolved_config"),
                self.resolved_config.to_json(),
            ),
            (
                String::from("resource_limits"),
                self.resource_limits.to_json(),
            ),
            (
                String::from("schema_version"),
                self.schema_version.to_json(),
            ),
            (
                String::from("stage_timings_ns"),
                self.stage_timings_ns.to_json(),
            ),
            (String::from("system"), self.system.to_json()),
            (String::from("warnings"), self.warnings.to_json()),
        ]))
    }
}
