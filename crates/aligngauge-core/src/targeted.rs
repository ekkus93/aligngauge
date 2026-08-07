//! Canonical v0.3 targeted-sequencing result models.

use std::collections::BTreeMap;

use crate::{Availability, JsonValue, ToJson};

/// One maximal zero-depth run within a source target.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ZeroCoverageRunSummary {
    /// Zero-based inclusive run start.
    pub start: u64,
    /// Zero-based exclusive run end.
    pub end: u64,
}

impl ToJson for ZeroCoverageRunSummary {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (String::from("end"), self.end.to_json()),
            (String::from("start"), self.start.to_json()),
        ]))
    }
}

/// Exact canonical reduction for one original source BED interval.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PerTargetCoverageSummary {
    /// Stable accepted-record index in BED source order.
    pub source_index: u64,
    /// One-based source BED line number.
    pub line_number: u64,
    /// Exact contig name.
    pub contig: String,
    /// Zero-based inclusive target start.
    pub start: u64,
    /// Zero-based exclusive target end.
    pub end: u64,
    /// Optional BED field 4.
    pub name: Option<String>,
    /// Exact source-target length.
    pub length: u64,
    /// Sum of canonical depth over the source target.
    pub depth_sum: u64,
    /// Deterministic mean depth, unavailable for zero-length targets.
    pub mean_depth: Availability<String>,
    /// Source-target bases with depth greater than zero.
    pub covered_bases: u64,
    /// Source-target bases with depth zero.
    pub uncovered_bases: u64,
    /// Exact bases meeting each configured depth threshold.
    pub threshold_bases: BTreeMap<String, u64>,
    /// Deterministic threshold percentages, unavailable for zero-length targets.
    pub threshold_percentages: BTreeMap<String, Availability<String>>,
    /// Maximal zero-depth half-open runs in genomic order.
    pub zero_coverage_runs: Vec<ZeroCoverageRunSummary>,
    /// Longest zero-depth run length.
    pub longest_zero_coverage_run_bases: u64,
}

impl ToJson for PerTargetCoverageSummary {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (String::from("contig"), self.contig.to_json()),
            (String::from("covered_bases"), self.covered_bases.to_json()),
            (String::from("depth_sum"), self.depth_sum.to_json()),
            (String::from("end"), self.end.to_json()),
            (String::from("length"), self.length.to_json()),
            (String::from("line_number"), self.line_number.to_json()),
            (
                String::from("longest_zero_coverage_run_bases"),
                self.longest_zero_coverage_run_bases.to_json(),
            ),
            (String::from("mean_depth"), self.mean_depth.to_json()),
            (
                String::from("name"),
                self.name.as_ref().map_or(JsonValue::Null, ToJson::to_json),
            ),
            (String::from("source_index"), self.source_index.to_json()),
            (String::from("start"), self.start.to_json()),
            (
                String::from("threshold_bases"),
                self.threshold_bases.to_json(),
            ),
            (
                String::from("threshold_percentages"),
                self.threshold_percentages.to_json(),
            ),
            (
                String::from("uncovered_bases"),
                self.uncovered_bases.to_json(),
            ),
            (
                String::from("zero_coverage_runs"),
                self.zero_coverage_runs.to_json(),
            ),
        ]))
    }
}

/// Exact canonical native v0.3 targeted-sequencing reduction.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TargetedCoverageSummary {
    /// Stable native targeted metric profile.
    pub profile: String,
    /// Canonical coverage profile that supplied depth runs.
    pub coverage_profile: String,
    /// True when duplicates are excluded by the underlying coverage profile.
    pub duplicate_adjusted: bool,
    /// SHA-256 of the original target BED bytes.
    pub target_sha256: String,
    /// Exact target BED byte size.
    pub target_size_bytes: u64,
    /// Accepted source BED interval count.
    pub source_interval_count: u64,
    /// Symmetric selected/near-target distance.
    pub near_distance_bases: u64,
    /// Checked sum of validated reference lengths.
    pub genome_territory_bases: u64,
    /// Unique zero-flank target union territory.
    pub target_territory_bases: u64,
    /// Unique selected territory outside the target union.
    pub near_target_territory_bases: u64,
    /// Accepted aligned reference-base observations inside target territory.
    pub on_target_bases: u64,
    /// Accepted aligned observations inside selected but outside target territory.
    pub near_target_bases: u64,
    /// Accepted aligned observations outside selected territory.
    pub off_target_bases: u64,
    /// Unique-target depth histogram including zero depth.
    pub target_depth_histogram: BTreeMap<String, u64>,
    /// Mean depth over unique target territory.
    pub target_mean_depth: Availability<String>,
    /// Unique target positions with depth greater than zero.
    pub target_covered_bases: u64,
    /// Unique target positions with depth zero.
    pub target_uncovered_bases: u64,
    /// Unique target bases meeting configured thresholds.
    pub threshold_bases: BTreeMap<String, u64>,
    /// Cumulative threshold percentages over target territory.
    pub threshold_percentages: BTreeMap<String, Availability<String>>,
    /// Number of non-empty source targets containing at least one zero-depth base.
    pub dropout_target_count: u64,
    /// Project-native target enrichment with explicit territory denominator.
    pub target_enrichment: Availability<String>,
    /// Nearest-rank 20th-percentile depth across all unique target bases.
    pub target_depth_20th_percentile: Availability<u64>,
    /// Project-native mean-depth / D20 uniformity penalty.
    pub target_uniformity_penalty_80: Availability<String>,
    /// Per-source-target reductions in source-file order.
    pub per_target: Vec<PerTargetCoverageSummary>,
}

impl ToJson for TargetedCoverageSummary {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (
                String::from("coverage_profile"),
                self.coverage_profile.to_json(),
            ),
            (
                String::from("dropout_target_count"),
                self.dropout_target_count.to_json(),
            ),
            (
                String::from("duplicate_adjusted"),
                self.duplicate_adjusted.to_json(),
            ),
            (
                String::from("genome_territory_bases"),
                self.genome_territory_bases.to_json(),
            ),
            (
                String::from("near_distance_bases"),
                self.near_distance_bases.to_json(),
            ),
            (
                String::from("near_target_bases"),
                self.near_target_bases.to_json(),
            ),
            (
                String::from("near_target_territory_bases"),
                self.near_target_territory_bases.to_json(),
            ),
            (
                String::from("off_target_bases"),
                self.off_target_bases.to_json(),
            ),
            (
                String::from("on_target_bases"),
                self.on_target_bases.to_json(),
            ),
            (String::from("per_target"), self.per_target.to_json()),
            (String::from("profile"), self.profile.to_json()),
            (
                String::from("source_interval_count"),
                self.source_interval_count.to_json(),
            ),
            (
                String::from("target_covered_bases"),
                self.target_covered_bases.to_json(),
            ),
            (
                String::from("target_depth_20th_percentile"),
                self.target_depth_20th_percentile.to_json(),
            ),
            (
                String::from("target_depth_histogram"),
                self.target_depth_histogram.to_json(),
            ),
            (
                String::from("target_enrichment"),
                self.target_enrichment.to_json(),
            ),
            (
                String::from("target_mean_depth"),
                self.target_mean_depth.to_json(),
            ),
            (String::from("target_sha256"), self.target_sha256.to_json()),
            (
                String::from("target_size_bytes"),
                self.target_size_bytes.to_json(),
            ),
            (
                String::from("target_territory_bases"),
                self.target_territory_bases.to_json(),
            ),
            (
                String::from("target_uncovered_bases"),
                self.target_uncovered_bases.to_json(),
            ),
            (
                String::from("target_uniformity_penalty_80"),
                self.target_uniformity_penalty_80.to_json(),
            ),
            (
                String::from("threshold_bases"),
                self.threshold_bases.to_json(),
            ),
            (
                String::from("threshold_percentages"),
                self.threshold_percentages.to_json(),
            ),
        ]))
    }
}
