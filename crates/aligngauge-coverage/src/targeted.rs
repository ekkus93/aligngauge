//! Exact v0.3 targeted reductions over canonical coverage depth runs.

use std::collections::BTreeMap;

use aligngauge_core::{
    AlignGaugeError, Availability, JsonValue, PerTargetCoverageSummary, Provenance,
    TargetedCoverageSummary, ZeroCoverageRunSummary,
};
use aligngauge_formats::{
    BedSourceInterval, MergedTargetInterval, TargetFileIdentity, TargetNormalizationProvenance,
    TargetSet,
};
use aligngauge_hts::ValidatedHeader;

use crate::util::{
    coverage_overflow, format_percentage_six, format_ratio_six, format_ratio_u128_six,
    internal_error, resource_error, u64_from_usize,
};

/// Stable native targeted profile name.
pub const TARGETED_PROFILE: &str = "aligngauge-targeted-v0.3";
/// Default symmetric near-target expansion.
pub const DEFAULT_NEAR_DISTANCE_BASES: u64 = 250;

const TARGET_ZERO_RUN_BUDGET_BYTES: u64 = 64_u64 << 20;
const TARGET_HISTOGRAM_BUDGET_BYTES: u64 = 16_u64 << 20;
const ZERO_RUN_ESTIMATED_BYTES: u64 = 64;
const HISTOGRAM_ENTRY_ESTIMATED_BYTES: u64 = 64;
const SOURCE_STATE_BASE_BYTES: u64 = 256;
const SOURCE_THRESHOLD_STATE_BYTES: u64 = 16;
const UNION_INTERVAL_STATE_BYTES: u64 = 64;
const TARGET_FILE_MEMORY_MULTIPLIER: u64 = 2;

/// Memory reservation derived from one normalized target definition.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct TargetedMemoryReservation {
    pub(crate) additional_bytes: u64,
    pub(crate) max_zero_runs: usize,
    pub(crate) max_histogram_bins: usize,
}

/// Canonical targeted report plus normalization details needed by provenance.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TargetedCoverageReport {
    summary: TargetedCoverageSummary,
    target_normalization: TargetNormalizationProvenance,
    selected_normalization: TargetNormalizationProvenance,
}

impl TargetedCoverageReport {
    /// Canonical v0.3 targeted summary.
    #[must_use]
    pub const fn summary(&self) -> &TargetedCoverageSummary {
        &self.summary
    }

    /// Clone the canonical targeted summary into `aligngauge-core` output state.
    #[must_use]
    pub fn to_core_summary(&self) -> TargetedCoverageSummary {
        self.summary.clone()
    }

    /// Add target identity, profile, normalization, and resource semantics to provenance.
    pub fn apply_provenance(&self, provenance: &mut Provenance) {
        provenance.analysis_plan.insert(
            String::from("targeted_profile"),
            JsonValue::String(TARGETED_PROFILE.to_owned()),
        );
        provenance.analysis_plan.insert(
            String::from("target_sha256"),
            JsonValue::String(self.summary.target_sha256.clone()),
        );
        provenance.analysis_plan.insert(
            String::from("target_size_bytes"),
            JsonValue::Unsigned(self.summary.target_size_bytes),
        );
        provenance.analysis_plan.insert(
            String::from("near_distance_bases"),
            JsonValue::Unsigned(self.summary.near_distance_bases),
        );
        provenance.analysis_plan.insert(
            String::from("target_metric_compatibility"),
            JsonValue::String(String::from("native-no-picard-compatibility-claim")),
        );

        let identity = TargetFileIdentity {
            path: None,
            size_bytes: self.summary.target_size_bytes,
            sha256: self.summary.target_sha256.clone(),
            source_interval_count: self.summary.source_interval_count,
        };
        provenance
            .normalization_actions
            .extend(self.target_normalization.actions(&identity));
        provenance.normalization_actions.extend(
            self.selected_normalization
                .actions(&identity)
                .into_iter()
                .map(|action| format!("selected:{action}")),
        );
        provenance.normalization_actions.sort();
        provenance.normalization_actions.dedup();
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct CompactInterval {
    start: u64,
    end: u64,
}

impl From<&MergedTargetInterval> for CompactInterval {
    fn from(value: &MergedTargetInterval) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

#[derive(Debug)]
struct SourceTargetState {
    source_index: u64,
    line_number: u64,
    contig: String,
    start: u64,
    end: u64,
    name: Option<String>,
    next_position: u64,
    depth_sum: u64,
    covered_bases: u64,
    uncovered_bases: u64,
    threshold_bases: Vec<u64>,
    zero_runs: Vec<ZeroCoverageRunSummary>,
    open_zero_start: Option<u64>,
    longest_zero_coverage_run_bases: u64,
}

impl SourceTargetState {
    fn new(interval: BedSourceInterval, threshold_count: usize) -> Self {
        Self {
            source_index: interval.source_index,
            line_number: interval.line_number,
            contig: interval.contig,
            start: interval.start,
            end: interval.end,
            name: interval.name,
            next_position: interval.start,
            depth_sum: 0,
            covered_bases: 0,
            uncovered_bases: 0,
            threshold_bases: vec![0; threshold_count],
            zero_runs: Vec::new(),
            open_zero_start: None,
            longest_zero_coverage_run_bases: 0,
        }
    }

    fn observe_segment(
        &mut self,
        start: u64,
        end: u64,
        depth: u64,
        thresholds: &[u32],
        total_zero_runs: &mut usize,
        max_zero_runs: usize,
    ) -> Result<(), AlignGaugeError> {
        if start >= end {
            return Ok(());
        }
        if start != self.next_position {
            return Err(
                internal_error("per-target coverage segments are not contiguous")
                    .with_detail("source_index", self.source_index)
                    .with_detail("expected_position", self.next_position)
                    .with_detail("observed_position", start),
            );
        }
        let bases = end - start;
        let weighted = depth
            .checked_mul(bases)
            .ok_or_else(|| coverage_overflow("per-target depth sum"))?;
        self.depth_sum = self
            .depth_sum
            .checked_add(weighted)
            .ok_or_else(|| coverage_overflow("per-target depth sum"))?;

        if depth == 0 {
            self.uncovered_bases = self
                .uncovered_bases
                .checked_add(bases)
                .ok_or_else(|| coverage_overflow("per-target uncovered bases"))?;
            if self.open_zero_start.is_none() {
                self.open_zero_start = Some(start);
            }
        } else {
            self.covered_bases = self
                .covered_bases
                .checked_add(bases)
                .ok_or_else(|| coverage_overflow("per-target covered bases"))?;
            self.close_zero_run(start, total_zero_runs, max_zero_runs)?;
        }

        for (index, threshold) in thresholds.iter().enumerate() {
            if depth >= u64::from(*threshold) {
                let value = self
                    .threshold_bases
                    .get_mut(index)
                    .ok_or_else(|| internal_error("per-target threshold state is missing"))?;
                *value = value
                    .checked_add(bases)
                    .ok_or_else(|| coverage_overflow("per-target threshold bases"))?;
            }
        }
        self.next_position = end;
        Ok(())
    }

    fn close_zero_run(
        &mut self,
        end: u64,
        total_zero_runs: &mut usize,
        max_zero_runs: usize,
    ) -> Result<(), AlignGaugeError> {
        let Some(start) = self.open_zero_start.take() else {
            return Ok(());
        };
        if start >= end {
            return Err(internal_error("zero-coverage run is not positive length")
                .with_detail("source_index", self.source_index));
        }
        if *total_zero_runs >= max_zero_runs {
            return Err(resource_error(
                "target zero-coverage run budget was exhausted during traversal",
            )
            .with_detail(
                "maximum_zero_coverage_runs",
                u64_from_usize(max_zero_runs, "zero-coverage run capacity")?,
            ));
        }
        *total_zero_runs = (*total_zero_runs)
            .checked_add(1)
            .ok_or_else(|| coverage_overflow("zero-coverage run count"))?;
        let length = end - start;
        self.longest_zero_coverage_run_bases = self.longest_zero_coverage_run_bases.max(length);
        self.zero_runs.push(ZeroCoverageRunSummary { start, end });
        Ok(())
    }

    fn finish(
        mut self,
        thresholds: &[u32],
        total_zero_runs: &mut usize,
        max_zero_runs: usize,
    ) -> Result<PerTargetCoverageSummary, AlignGaugeError> {
        if self.next_position != self.end {
            return Err(internal_error(
                "per-target coverage did not evaluate the complete source interval",
            )
            .with_detail("source_index", self.source_index)
            .with_detail("expected_end", self.end)
            .with_detail("observed_end", self.next_position));
        }
        self.close_zero_run(self.end, total_zero_runs, max_zero_runs)?;

        let length = self.end - self.start;
        let territory = self
            .covered_bases
            .checked_add(self.uncovered_bases)
            .ok_or_else(|| coverage_overflow("per-target territory"))?;
        if territory != length {
            return Err(internal_error(
                "per-target covered and uncovered bases do not equal target length",
            )
            .with_detail("source_index", self.source_index)
            .with_detail("target_length", length)
            .with_detail("evaluated_bases", territory));
        }

        let mean_depth = if length == 0 {
            Availability::unavailable("zero_length_target")
        } else {
            Availability::Available(format_ratio_six(self.depth_sum, length)?)
        };
        let mut threshold_bases = BTreeMap::new();
        let mut threshold_percentages = BTreeMap::new();
        for (index, threshold) in thresholds.iter().enumerate() {
            let bases = *self.threshold_bases.get(index).ok_or_else(|| {
                internal_error("per-target threshold finalization state is missing")
            })?;
            threshold_bases.insert(threshold.to_string(), bases);
            threshold_percentages.insert(
                threshold.to_string(),
                if length == 0 {
                    Availability::unavailable("zero_length_target")
                } else {
                    Availability::Available(format_percentage_six(bases, length)?)
                },
            );
        }

        Ok(PerTargetCoverageSummary {
            source_index: self.source_index,
            line_number: self.line_number,
            contig: self.contig,
            start: self.start,
            end: self.end,
            name: self.name,
            length,
            depth_sum: self.depth_sum,
            mean_depth,
            covered_bases: self.covered_bases,
            uncovered_bases: self.uncovered_bases,
            threshold_bases,
            threshold_percentages,
            zero_coverage_runs: self.zero_runs,
            longest_zero_coverage_run_bases: self.longest_zero_coverage_run_bases,
        })
    }
}

#[derive(Debug, Default)]
struct ReferenceTargetState {
    target_union: Vec<CompactInterval>,
    selected_union: Vec<CompactInterval>,
    target_cursor: usize,
    selected_cursor: usize,
    sources: Vec<SourceTargetState>,
    source_next: usize,
    active_sources: Vec<usize>,
}

impl ReferenceTargetState {
    fn finalize_layout(&mut self) {
        self.target_union
            .sort_by_key(|interval| (interval.start, interval.end));
        self.selected_union
            .sort_by_key(|interval| (interval.start, interval.end));
        self.sources
            .sort_by_key(|source| (source.start, source.end, source.source_index));
    }

    fn observe_source_run(
        &mut self,
        run_start: u64,
        run_end: u64,
        depth: u64,
        thresholds: &[u32],
        total_zero_runs: &mut usize,
        max_zero_runs: usize,
    ) -> Result<(), AlignGaugeError> {
        self.active_sources
            .retain(|index| self.sources[*index].end > run_start);

        while let Some(source) = self.sources.get(self.source_next) {
            if source.start >= run_end {
                break;
            }
            let index = self.source_next;
            self.source_next = self
                .source_next
                .checked_add(1)
                .ok_or_else(|| coverage_overflow("source-target cursor"))?;
            if source.start < source.end && source.end > run_start {
                self.active_sources.push(index);
            }
        }

        let (sources, active_sources) = (&mut self.sources, &self.active_sources);
        for index in active_sources.iter().copied() {
            let source = sources
                .get_mut(index)
                .ok_or_else(|| internal_error("active source-target index is invalid"))?;
            let start = run_start.max(source.start);
            let end = run_end.min(source.end);
            source.observe_segment(
                start,
                end,
                depth,
                thresholds,
                total_zero_runs,
                max_zero_runs,
            )?;
        }
        Ok(())
    }
}

/// Stateful exact targeted reducer fed by canonical coverage runs.
pub(crate) struct TargetedReducer {
    identity: TargetFileIdentity,
    target_normalization: TargetNormalizationProvenance,
    selected_normalization: TargetNormalizationProvenance,
    near_distance_bases: u64,
    thresholds: Vec<u32>,
    genome_territory_bases: u64,
    target_territory_bases: u64,
    selected_territory_bases: u64,
    references: BTreeMap<usize, ReferenceTargetState>,
    target_depth_histogram: BTreeMap<u64, u64>,
    on_target_bases: u64,
    selected_aligned_bases: u64,
    total_zero_runs: usize,
    max_zero_runs: usize,
    max_histogram_bins: usize,
}

struct TargetPartition {
    near_aligned: u64,
    off_aligned: u64,
    near_territory: u64,
}

struct TargetThresholds {
    bases: BTreeMap<String, u64>,
    percentages: BTreeMap<String, Availability<String>>,
}

struct AggregateTargetMetrics {
    target_covered_bases: u64,
    target_uncovered_bases: u64,
    target_mean_depth: Availability<String>,
    threshold_bases: BTreeMap<String, u64>,
    threshold_percentages: BTreeMap<String, Availability<String>>,
    target_depth_20th_percentile: Availability<u64>,
    target_uniformity_penalty_80: Availability<String>,
}

impl TargetedReducer {
    pub(crate) fn new(
        header: &ValidatedHeader,
        target_set: TargetSet,
        selected_set: TargetSet,
        thresholds: Vec<u32>,
        near_distance_bases: u64,
        reservation: TargetedMemoryReservation,
    ) -> Result<Self, AlignGaugeError> {
        validate_target_sets(&target_set, &selected_set, near_distance_bases)?;
        let genome_territory_bases =
            header
                .references()
                .iter()
                .try_fold(0_u64, |total, reference| {
                    total
                        .checked_add(reference.length())
                        .ok_or_else(|| coverage_overflow("genome territory"))
                })?;
        let target_territory_bases = target_set.normalization.aggregate_territory_bases;
        let selected_territory_bases = selected_set.normalization.aggregate_territory_bases;

        let identity = target_set.identity;
        let target_normalization = target_set.normalization;
        let selected_normalization = selected_set.normalization;
        let mut references: BTreeMap<usize, ReferenceTargetState> = BTreeMap::new();

        for interval in &target_set.merged_intervals {
            references
                .entry(interval.contig_index)
                .or_default()
                .target_union
                .push(interval.into());
        }
        for interval in &selected_set.merged_intervals {
            references
                .entry(interval.contig_index)
                .or_default()
                .selected_union
                .push(interval.into());
        }
        for interval in target_set.source_intervals {
            let contig_index = interval.contig_index;
            references
                .entry(contig_index)
                .or_default()
                .sources
                .push(SourceTargetState::new(interval, thresholds.len()));
        }
        for state in references.values_mut() {
            state.finalize_layout();
            ensure_target_union_is_selected(state)?;
        }

        let mut target_depth_histogram = BTreeMap::new();
        target_depth_histogram.insert(0, 0);
        Ok(Self {
            identity,
            target_normalization,
            selected_normalization,
            near_distance_bases,
            thresholds,
            genome_territory_bases,
            target_territory_bases,
            selected_territory_bases,
            references,
            target_depth_histogram,
            on_target_bases: 0,
            selected_aligned_bases: 0,
            total_zero_runs: 0,
            max_zero_runs: reservation.max_zero_runs,
            max_histogram_bins: reservation.max_histogram_bins,
        })
    }

    pub(crate) fn observe_run(
        &mut self,
        reference_index: usize,
        start: u64,
        end: u64,
        depth: u64,
    ) -> Result<(), AlignGaugeError> {
        if start >= end {
            return Ok(());
        }
        let Some(state) = self.references.get_mut(&reference_index) else {
            return Ok(());
        };

        let target_overlap =
            overlap_union(&state.target_union, &mut state.target_cursor, start, end)?;
        let selected_overlap = overlap_union(
            &state.selected_union,
            &mut state.selected_cursor,
            start,
            end,
        )?;
        if selected_overlap < target_overlap {
            return Err(internal_error(
                "selected target overlap is smaller than target overlap",
            ));
        }

        state.observe_source_run(
            start,
            end,
            depth,
            &self.thresholds,
            &mut self.total_zero_runs,
            self.max_zero_runs,
        )?;

        if target_overlap > 0 {
            let existing = self.target_depth_histogram.get(&depth).copied();
            if existing.is_none() && self.target_depth_histogram.len() >= self.max_histogram_bins {
                return Err(resource_error(
                    "target depth histogram budget was exhausted during traversal",
                )
                .with_detail(
                    "maximum_target_histogram_bins",
                    u64_from_usize(self.max_histogram_bins, "target histogram bins")?,
                ));
            }
            let bases = existing
                .unwrap_or(0)
                .checked_add(target_overlap)
                .ok_or_else(|| coverage_overflow("target depth histogram bases"))?;
            self.target_depth_histogram.insert(depth, bases);

            let weighted = depth
                .checked_mul(target_overlap)
                .ok_or_else(|| coverage_overflow("on-target aligned bases"))?;
            self.on_target_bases = self
                .on_target_bases
                .checked_add(weighted)
                .ok_or_else(|| coverage_overflow("on-target aligned bases"))?;
        }
        if selected_overlap > 0 {
            let weighted = depth
                .checked_mul(selected_overlap)
                .ok_or_else(|| coverage_overflow("selected aligned bases"))?;
            self.selected_aligned_bases = self
                .selected_aligned_bases
                .checked_add(weighted)
                .ok_or_else(|| coverage_overflow("selected aligned bases"))?;
        }
        Ok(())
    }

    pub(crate) fn finish(
        mut self,
        total_accepted_aligned_bases: u64,
    ) -> Result<TargetedCoverageReport, AlignGaugeError> {
        let partition = self.validate_partition(total_accepted_aligned_bases)?;
        let aggregate = self.aggregate_target_metrics()?;
        let target_enrichment = target_enrichment(
            self.on_target_bases,
            total_accepted_aligned_bases,
            self.target_territory_bases,
            self.genome_territory_bases,
        )?;
        let (per_target, dropout_target_count) = self.finish_per_targets()?;

        let summary = TargetedCoverageSummary {
            profile: TARGETED_PROFILE.to_owned(),
            coverage_profile: crate::COVERAGE_PROFILE.to_owned(),
            duplicate_adjusted: true,
            target_sha256: self.identity.sha256,
            target_size_bytes: self.identity.size_bytes,
            source_interval_count: self.identity.source_interval_count,
            near_distance_bases: self.near_distance_bases,
            genome_territory_bases: self.genome_territory_bases,
            target_territory_bases: self.target_territory_bases,
            near_target_territory_bases: partition.near_territory,
            on_target_bases: self.on_target_bases,
            near_target_bases: partition.near_aligned,
            off_target_bases: partition.off_aligned,
            target_depth_histogram: self
                .target_depth_histogram
                .into_iter()
                .map(|(depth, bases)| (depth.to_string(), bases))
                .collect(),
            target_mean_depth: aggregate.target_mean_depth,
            target_covered_bases: aggregate.target_covered_bases,
            target_uncovered_bases: aggregate.target_uncovered_bases,
            threshold_bases: aggregate.threshold_bases,
            threshold_percentages: aggregate.threshold_percentages,
            dropout_target_count,
            target_enrichment,
            target_depth_20th_percentile: aggregate.target_depth_20th_percentile,
            target_uniformity_penalty_80: aggregate.target_uniformity_penalty_80,
            per_target,
        };
        Ok(TargetedCoverageReport {
            summary,
            target_normalization: self.target_normalization,
            selected_normalization: self.selected_normalization,
        })
    }

    fn validate_partition(
        &self,
        total_accepted_aligned_bases: u64,
    ) -> Result<TargetPartition, AlignGaugeError> {
        let histogram_territory =
            self.target_depth_histogram
                .values()
                .try_fold(0_u64, |total, bases| {
                    total
                        .checked_add(*bases)
                        .ok_or_else(|| coverage_overflow("target histogram territory"))
                })?;
        if histogram_territory != self.target_territory_bases {
            return Err(internal_error(
                "target depth histogram does not equal normalized target territory",
            )
            .with_detail("histogram_bases", histogram_territory)
            .with_detail("target_territory_bases", self.target_territory_bases));
        }
        let weighted_target_depth =
            self.target_depth_histogram
                .iter()
                .try_fold(0_u128, |total, (depth, bases)| {
                    let value = u128::from(*depth)
                        .checked_mul(u128::from(*bases))
                        .ok_or_else(|| coverage_overflow("weighted target histogram depth"))?;
                    total
                        .checked_add(value)
                        .ok_or_else(|| coverage_overflow("weighted target histogram depth"))
                })?;
        if weighted_target_depth != u128::from(self.on_target_bases) {
            return Err(internal_error(
                "weighted target histogram does not equal on-target aligned bases",
            )
            .with_detail("on_target_bases", self.on_target_bases));
        }
        if self.selected_aligned_bases < self.on_target_bases {
            return Err(internal_error(
                "selected aligned bases are smaller than on-target aligned bases",
            ));
        }
        if self.selected_aligned_bases > total_accepted_aligned_bases {
            return Err(internal_error(
                "selected aligned bases exceed total accepted aligned bases",
            )
            .with_detail("selected_aligned_bases", self.selected_aligned_bases)
            .with_detail("total_accepted_aligned_bases", total_accepted_aligned_bases));
        }
        let near_target_bases = self
            .selected_aligned_bases
            .checked_sub(self.on_target_bases)
            .ok_or_else(|| internal_error("near-target subtraction underflowed"))?;
        let off_target_bases = total_accepted_aligned_bases
            .checked_sub(self.selected_aligned_bases)
            .ok_or_else(|| internal_error("off-target subtraction underflowed"))?;
        let partition = self
            .on_target_bases
            .checked_add(near_target_bases)
            .and_then(|value| value.checked_add(off_target_bases))
            .ok_or_else(|| coverage_overflow("target aligned-base partition"))?;
        if partition != total_accepted_aligned_bases {
            return Err(internal_error(
                "target aligned-base partition does not equal total accepted aligned bases",
            ));
        }
        let near_target_territory_bases = self
            .selected_territory_bases
            .checked_sub(self.target_territory_bases)
            .ok_or_else(|| internal_error("selected territory is smaller than target territory"))?;
        Ok(TargetPartition {
            near_aligned: near_target_bases,
            off_aligned: off_target_bases,
            near_territory: near_target_territory_bases,
        })
    }

    fn aggregate_target_metrics(&self) -> Result<AggregateTargetMetrics, AlignGaugeError> {
        let target_covered_bases =
            self.target_depth_histogram
                .iter()
                .try_fold(0_u64, |total, (depth, bases)| {
                    if *depth == 0 {
                        Ok(total)
                    } else {
                        total
                            .checked_add(*bases)
                            .ok_or_else(|| coverage_overflow("covered target bases"))
                    }
                })?;
        let target_uncovered_bases = self.target_depth_histogram.get(&0).copied().unwrap_or(0);
        let target_mean_depth = if self.target_territory_bases == 0 {
            Availability::unavailable("target_territory_is_zero")
        } else {
            Availability::Available(format_ratio_six(
                self.on_target_bases,
                self.target_territory_bases,
            )?)
        };
        let thresholds = self.target_thresholds()?;
        let target_depth_20th_percentile = target_depth_20th_percentile(
            &self.target_depth_histogram,
            self.target_territory_bases,
        )?;
        let target_uniformity_penalty_80 = self.target_uniformity(&target_depth_20th_percentile)?;
        Ok(AggregateTargetMetrics {
            target_covered_bases,
            target_uncovered_bases,
            target_mean_depth,
            threshold_bases: thresholds.bases,
            threshold_percentages: thresholds.percentages,
            target_depth_20th_percentile,
            target_uniformity_penalty_80,
        })
    }

    fn target_thresholds(&self) -> Result<TargetThresholds, AlignGaugeError> {
        let mut threshold_bases = BTreeMap::new();
        let mut threshold_percentages = BTreeMap::new();
        for threshold in &self.thresholds {
            let bases = self
                .target_depth_histogram
                .range(u64::from(*threshold)..)
                .try_fold(0_u64, |total, (_, count)| {
                    total
                        .checked_add(*count)
                        .ok_or_else(|| coverage_overflow("target threshold bases"))
                })?;
            threshold_bases.insert(threshold.to_string(), bases);
            threshold_percentages.insert(
                threshold.to_string(),
                if self.target_territory_bases == 0 {
                    Availability::unavailable("target_territory_is_zero")
                } else {
                    Availability::Available(format_percentage_six(
                        bases,
                        self.target_territory_bases,
                    )?)
                },
            );
        }
        Ok(TargetThresholds {
            bases: threshold_bases,
            percentages: threshold_percentages,
        })
    }

    fn target_uniformity(
        &self,
        percentile: &Availability<u64>,
    ) -> Result<Availability<String>, AlignGaugeError> {
        match percentile {
            Availability::Available(0) => Ok(Availability::unavailable(
                "target_depth_20th_percentile_is_zero",
            )),
            Availability::Available(depth) => {
                let denominator = u128::from(self.target_territory_bases)
                    .checked_mul(u128::from(*depth))
                    .ok_or_else(|| coverage_overflow("target uniformity denominator"))?;
                Ok(Availability::Available(format_ratio_u128_six(
                    u128::from(self.on_target_bases),
                    denominator,
                )?))
            }
            Availability::Unavailable { reason } => Ok(Availability::Unavailable {
                reason: reason.clone(),
            }),
        }
    }

    fn finish_per_targets(
        &mut self,
    ) -> Result<(Vec<PerTargetCoverageSummary>, u64), AlignGaugeError> {
        let references = std::mem::take(&mut self.references);
        let mut per_target = Vec::new();
        for state in references.into_values() {
            for source in state.sources {
                per_target.push(source.finish(
                    &self.thresholds,
                    &mut self.total_zero_runs,
                    self.max_zero_runs,
                )?);
            }
        }
        per_target.sort_by_key(|target| target.source_index);
        let dropout_target_count = per_target.iter().try_fold(0_u64, |count, target| {
            if target.length > 0 && target.uncovered_bases > 0 {
                count
                    .checked_add(1)
                    .ok_or_else(|| coverage_overflow("dropout target count"))
            } else {
                Ok(count)
            }
        })?;
        Ok((per_target, dropout_target_count))
    }
}

pub(crate) fn memory_reservation(
    target_set: &TargetSet,
    selected_set: &TargetSet,
    threshold_count: usize,
) -> Result<TargetedMemoryReservation, AlignGaugeError> {
    let source_count = target_set.identity.source_interval_count;
    let threshold_count = u64_from_usize(threshold_count, "target threshold count")?;
    let per_source = SOURCE_STATE_BASE_BYTES
        .checked_add(
            SOURCE_THRESHOLD_STATE_BYTES
                .checked_mul(threshold_count)
                .ok_or_else(|| coverage_overflow("target threshold memory reservation"))?,
        )
        .ok_or_else(|| coverage_overflow("target source-state memory reservation"))?;
    let source_bytes = source_count
        .checked_mul(per_source)
        .ok_or_else(|| coverage_overflow("target source-state memory reservation"))?;
    let merged_count = u64_from_usize(
        target_set
            .merged_intervals
            .len()
            .checked_add(selected_set.merged_intervals.len())
            .ok_or_else(|| coverage_overflow("target merged-interval count"))?,
        "target merged intervals",
    )?;
    let union_bytes = merged_count
        .checked_mul(UNION_INTERVAL_STATE_BYTES)
        .ok_or_else(|| coverage_overflow("target union memory reservation"))?;
    let file_bytes = target_set
        .identity
        .size_bytes
        .checked_mul(TARGET_FILE_MEMORY_MULTIPLIER)
        .ok_or_else(|| coverage_overflow("target file memory reservation"))?;
    let additional_bytes = [
        TARGET_ZERO_RUN_BUDGET_BYTES,
        TARGET_HISTOGRAM_BUDGET_BYTES,
        source_bytes,
        union_bytes,
        file_bytes,
    ]
    .into_iter()
    .try_fold(0_u64, |total, value| {
        total
            .checked_add(value)
            .ok_or_else(|| coverage_overflow("targeted memory reservation"))
    })?;
    let max_zero_runs = usize::try_from(TARGET_ZERO_RUN_BUDGET_BYTES / ZERO_RUN_ESTIMATED_BYTES)
        .map_err(|source| {
            internal_error("zero-coverage run capacity does not fit usize").with_source(source)
        })?;
    let max_histogram_bins = usize::try_from(
        TARGET_HISTOGRAM_BUDGET_BYTES / HISTOGRAM_ENTRY_ESTIMATED_BYTES,
    )
    .map_err(|source| {
        internal_error("target histogram capacity does not fit usize").with_source(source)
    })?;
    Ok(TargetedMemoryReservation {
        additional_bytes,
        max_zero_runs,
        max_histogram_bins,
    })
}

fn validate_target_sets(
    target_set: &TargetSet,
    selected_set: &TargetSet,
    near_distance_bases: u64,
) -> Result<(), AlignGaugeError> {
    if target_set.identity != selected_set.identity {
        return Err(internal_error(
            "target and selected normalization do not share the same input identity",
        ));
    }
    if target_set.normalization.flank_bases != 0 {
        return Err(internal_error(
            "target normalization must use zero flank for aggregate territory",
        ));
    }
    if selected_set.normalization.flank_bases != near_distance_bases {
        return Err(internal_error(
            "selected normalization flank does not match near-target distance",
        ));
    }
    if selected_set.normalization.aggregate_territory_bases
        < target_set.normalization.aggregate_territory_bases
    {
        return Err(internal_error(
            "selected target territory is smaller than target territory",
        ));
    }
    Ok(())
}

fn ensure_target_union_is_selected(state: &ReferenceTargetState) -> Result<(), AlignGaugeError> {
    let mut selected_index = 0_usize;
    for target in &state.target_union {
        while let Some(selected) = state.selected_union.get(selected_index) {
            if selected.end <= target.start {
                selected_index = selected_index
                    .checked_add(1)
                    .ok_or_else(|| coverage_overflow("selected interval cursor"))?;
            } else {
                break;
            }
        }
        let selected = state.selected_union.get(selected_index).ok_or_else(|| {
            internal_error("target interval is absent from selected target territory")
        })?;
        if selected.start > target.start || selected.end < target.end {
            return Err(internal_error(
                "target interval is not fully contained by selected target territory",
            ));
        }
    }
    Ok(())
}

fn overlap_union(
    intervals: &[CompactInterval],
    cursor: &mut usize,
    start: u64,
    end: u64,
) -> Result<u64, AlignGaugeError> {
    while let Some(interval) = intervals.get(*cursor) {
        if interval.end <= start {
            *cursor = cursor
                .checked_add(1)
                .ok_or_else(|| coverage_overflow("target interval cursor"))?;
        } else {
            break;
        }
    }
    let mut index = *cursor;
    let mut overlap = 0_u64;
    while let Some(interval) = intervals.get(index) {
        if interval.start >= end {
            break;
        }
        let overlap_start = start.max(interval.start);
        let overlap_end = end.min(interval.end);
        if overlap_start < overlap_end {
            overlap = overlap
                .checked_add(overlap_end - overlap_start)
                .ok_or_else(|| coverage_overflow("target run overlap"))?;
        }
        index = index
            .checked_add(1)
            .ok_or_else(|| coverage_overflow("target overlap cursor"))?;
    }
    Ok(overlap)
}

fn target_depth_20th_percentile(
    histogram: &BTreeMap<u64, u64>,
    territory: u64,
) -> Result<Availability<u64>, AlignGaugeError> {
    if territory == 0 {
        return Ok(Availability::unavailable("target_territory_is_zero"));
    }
    let rank = territory / 5 + u64::from(!territory.is_multiple_of(5));
    let mut cumulative = 0_u64;
    for (depth, bases) in histogram {
        cumulative = cumulative
            .checked_add(*bases)
            .ok_or_else(|| coverage_overflow("target percentile cumulative bases"))?;
        if cumulative >= rank {
            return Ok(Availability::Available(*depth));
        }
    }
    Err(internal_error(
        "target depth histogram cannot satisfy the 20th-percentile rank",
    ))
}

fn target_enrichment(
    on_target_bases: u64,
    total_accepted_aligned_bases: u64,
    target_territory_bases: u64,
    genome_territory_bases: u64,
) -> Result<Availability<String>, AlignGaugeError> {
    if total_accepted_aligned_bases == 0 {
        return Ok(Availability::unavailable("no_accepted_aligned_bases"));
    }
    if target_territory_bases == 0 {
        return Ok(Availability::unavailable("target_territory_is_zero"));
    }
    if genome_territory_bases == 0 {
        return Ok(Availability::unavailable("genome_territory_is_zero"));
    }
    let numerator = u128::from(on_target_bases)
        .checked_mul(u128::from(genome_territory_bases))
        .ok_or_else(|| coverage_overflow("target enrichment numerator"))?;
    let denominator = u128::from(total_accepted_aligned_bases)
        .checked_mul(u128::from(target_territory_bases))
        .ok_or_else(|| coverage_overflow("target enrichment denominator"))?;
    Ok(Availability::Available(format_ratio_u128_six(
        numerator,
        denominator,
    )?))
}

#[cfg(test)]
mod tests {
    use super::{target_depth_20th_percentile, target_enrichment};
    use aligngauge_core::Availability;
    use std::collections::BTreeMap;

    #[test]
    fn percentile_includes_zero_depth_and_uses_nearest_rank() {
        let histogram = BTreeMap::from([(0, 2), (1, 3), (7, 5)]);
        assert_eq!(
            target_depth_20th_percentile(&histogram, 10).expect("percentile"),
            Availability::Available(0)
        );
        let histogram = BTreeMap::from([(0, 1), (1, 4), (7, 5)]);
        assert_eq!(
            target_depth_20th_percentile(&histogram, 10).expect("percentile"),
            Availability::Available(1)
        );
    }

    #[test]
    fn enrichment_uses_explicit_denominator_and_unavailability() {
        assert_eq!(
            target_enrichment(50, 100, 10, 1_000).expect("enrichment"),
            Availability::Available(String::from("50.000000"))
        );
        assert_eq!(
            target_enrichment(0, 0, 10, 1_000).expect("unavailable"),
            Availability::unavailable("no_accepted_aligned_bases")
        );
    }
}
