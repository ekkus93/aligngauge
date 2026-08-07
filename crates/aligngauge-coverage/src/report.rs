//! Deterministic coverage report models and canonical/provenance projections.

use std::collections::BTreeMap;

use aligngauge_core::{
    AlignGaugeError, CoveragePolicy, CoverageSummary, JsonValue, MateOverlapPolicy,
    PerReferenceCoverageSummary, Provenance, RecordInclusion, ToJson,
};

use crate::plan::CoverageMemoryPlan;
use crate::util::u64_from_usize;
use crate::{COVERAGE_PROFILE, COVERAGE_STRATEGY};

/// Per-reference exact reductions.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PerReferenceCoverage {
    /// Reference name in BAM header order.
    pub name: String,
    /// Declared reference length.
    pub length: u64,
    /// Sum of accepted `M`/`=`/`X` block lengths.
    pub accepted_aligned_bases: u64,
    /// Reference positions with depth greater than zero.
    pub covered_reference_bases: u64,
    /// Reference positions with zero depth.
    pub uncovered_reference_bases: u64,
    /// Mean depth formatted with deterministic six-decimal half-up rounding.
    pub mean_depth: String,
}

impl ToJson for PerReferenceCoverage {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (
                String::from("accepted_aligned_bases"),
                self.accepted_aligned_bases.into(),
            ),
            (
                String::from("covered_reference_bases"),
                self.covered_reference_bases.into(),
            ),
            (String::from("length"), self.length.into()),
            (
                String::from("mean_depth"),
                JsonValue::String(self.mean_depth.clone()),
            ),
            (String::from("name"), JsonValue::String(self.name.clone())),
            (
                String::from("uncovered_reference_bases"),
                self.uncovered_reference_bases.into(),
            ),
        ]))
    }
}

/// Exact deterministic Milestone 5 coverage artifact.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CoverageReport {
    pub(crate) policy: CoveragePolicy,
    pub(crate) total_accepted_aligned_bases: u64,
    pub(crate) depth_histogram: BTreeMap<u64, u64>,
    pub(crate) threshold_bases: BTreeMap<u32, u64>,
    pub(crate) threshold_percentages: BTreeMap<u32, String>,
    pub(crate) covered_reference_bases: u64,
    pub(crate) uncovered_reference_bases: u64,
    pub(crate) per_reference: Vec<PerReferenceCoverage>,
    pub(crate) memory_plan: CoverageMemoryPlan,
}

impl CoverageReport {
    /// Canonical coverage policy.
    #[must_use]
    pub const fn policy(&self) -> &CoveragePolicy {
        &self.policy
    }

    /// Sum of accepted aligned `M`/`=`/`X` bases.
    #[must_use]
    pub const fn total_accepted_aligned_bases(&self) -> u64 {
        self.total_accepted_aligned_bases
    }

    /// Exact whole-territory depth histogram.
    #[must_use]
    pub const fn depth_histogram(&self) -> &BTreeMap<u64, u64> {
        &self.depth_histogram
    }

    /// Exact cumulative base counts at configured thresholds.
    #[must_use]
    pub const fn threshold_bases(&self) -> &BTreeMap<u32, u64> {
        &self.threshold_bases
    }

    /// Deterministically formatted cumulative percentages.
    #[must_use]
    pub const fn threshold_percentages(&self) -> &BTreeMap<u32, String> {
        &self.threshold_percentages
    }

    /// Per-reference exact reductions in BAM header order.
    #[must_use]
    pub fn per_reference(&self) -> &[PerReferenceCoverage] {
        &self.per_reference
    }

    /// Memory plan selected before traversal.
    #[must_use]
    pub const fn memory_plan(&self) -> &CoverageMemoryPlan {
        &self.memory_plan
    }

    /// Canonical aggregate model already reserved in `aligngauge-core`.
    #[must_use]
    pub fn to_core_summary(&self) -> CoverageSummary {
        CoverageSummary {
            policy: self.policy.clone(),
            total_accepted_aligned_bases: self.total_accepted_aligned_bases,
            depth_histogram: self
                .depth_histogram
                .iter()
                .map(|(depth, bases)| (depth.to_string(), *bases))
                .collect(),
            threshold_bases: self
                .threshold_bases
                .iter()
                .map(|(threshold, bases)| (threshold.to_string(), *bases))
                .collect(),
            threshold_percentages: self
                .threshold_percentages
                .iter()
                .map(|(threshold, percentage)| (threshold.to_string(), percentage.clone()))
                .collect(),
            covered_reference_bases: self.covered_reference_bases,
            uncovered_reference_bases: self.uncovered_reference_bases,
            per_reference: self
                .per_reference
                .iter()
                .map(|reference| PerReferenceCoverageSummary {
                    name: reference.name.clone(),
                    length: reference.length,
                    accepted_aligned_bases: reference.accepted_aligned_bases,
                    covered_reference_bases: reference.covered_reference_bases,
                    uncovered_reference_bases: reference.uncovered_reference_bases,
                    mean_depth: reference.mean_depth.clone(),
                })
                .collect(),
        }
    }

    /// Add coverage strategy and resource planning to canonical provenance.
    ///
    /// # Errors
    /// Returns `internal_invariant` if a platform-sized planned value cannot be represented in
    /// canonical provenance.
    pub fn apply_provenance(&self, provenance: &mut Provenance) -> Result<(), AlignGaugeError> {
        provenance.analysis_plan.insert(
            String::from("coverage_profile"),
            JsonValue::String(COVERAGE_PROFILE.to_owned()),
        );
        provenance.analysis_plan.insert(
            String::from("coverage_strategy"),
            JsonValue::String(COVERAGE_STRATEGY.to_owned()),
        );
        provenance.analysis_plan.insert(
            String::from("coverage_chunk_size_bases"),
            JsonValue::Unsigned(u64_from_usize(
                self.memory_plan.chunk_size_bases,
                "chunk size",
            )?),
        );
        provenance.analysis_plan.insert(
            String::from("coverage_memory_plan"),
            self.memory_plan.to_json()?,
        );
        provenance.resource_limits.insert(
            String::from("coverage_memory_limit_bytes"),
            self.memory_plan.memory_limit_bytes,
        );
        provenance.resource_limits.insert(
            String::from("coverage_planned_peak_bytes"),
            self.memory_plan.planned_peak_bytes,
        );
        Ok(())
    }
}

impl ToJson for CoverageReport {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (
                String::from("covered_reference_bases"),
                self.covered_reference_bases.into(),
            ),
            (
                String::from("depth_histogram"),
                JsonValue::Object(
                    self.depth_histogram
                        .iter()
                        .map(|(depth, bases)| (depth.to_string(), (*bases).into()))
                        .collect(),
                ),
            ),
            (String::from("per_reference"), self.per_reference.to_json()),
            (String::from("policy"), self.policy.to_json()),
            (
                String::from("schema"),
                JsonValue::String(String::from("aligngauge-coverage-v1")),
            ),
            (
                String::from("threshold_bases"),
                JsonValue::Object(
                    self.threshold_bases
                        .iter()
                        .map(|(threshold, bases)| (threshold.to_string(), (*bases).into()))
                        .collect(),
                ),
            ),
            (
                String::from("threshold_percentages"),
                JsonValue::Object(
                    self.threshold_percentages
                        .iter()
                        .map(|(threshold, percentage)| {
                            (threshold.to_string(), JsonValue::String(percentage.clone()))
                        })
                        .collect(),
                ),
            ),
            (
                String::from("total_accepted_aligned_bases"),
                self.total_accepted_aligned_bases.into(),
            ),
            (
                String::from("uncovered_reference_bases"),
                self.uncovered_reference_bases.into(),
            ),
        ]))
    }
}

pub(crate) fn canonical_policy() -> CoveragePolicy {
    CoveragePolicy {
        name: COVERAGE_PROFILE.to_owned(),
        minimum_mapq: 0,
        duplicates: RecordInclusion::Exclude,
        qc_fail: RecordInclusion::Exclude,
        secondary: RecordInclusion::Exclude,
        supplementary: RecordInclusion::Exclude,
        mate_overlap: MateOverlapPolicy::CountBoth,
    }
}
