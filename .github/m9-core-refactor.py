#!/usr/bin/env python3
"""Refactor generated Milestone 9 finalizers to satisfy strict pedantic limits."""

import re
from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:100]!r}")
    file.write_text(text.replace(old, new, 1))


def regex_once(path: str, pattern: str, replacement: str) -> None:
    file = Path(path)
    text = file.read_text()
    updated, count = re.subn(pattern, lambda _: replacement, text, count=1, flags=re.S)
    if count != 1:
        raise SystemExit(f"{path}: expected one regex match, found {count}")
    file.write_text(updated)


reduce = "crates/aligngauge-coverage/src/accumulator/reduce.rs"
replace_once(
    reduce,
    "        let targeted = self\n"
    "            .targeted\n"
    "            .take()\n"
    "            .map(|targeted| targeted.finish(self.total_accepted_aligned_bases))\n"
    "            .transpose()?;\n",
    "        let targeted = self.finish_targeted()?;\n",
)
replace_once(
    reduce,
    "    pub(super) fn finalize_reference(\n",
    "    fn finish_targeted(\n"
    "        &mut self,\n"
    "    ) -> Result<Option<crate::targeted::TargetedCoverageReport>, AlignGaugeError> {\n"
    "        let total_accepted_aligned_bases = self.total_accepted_aligned_bases;\n"
    "        self.targeted\n"
    "            .take()\n"
    "            .map(|targeted| targeted.finish(total_accepted_aligned_bases))\n"
    "            .transpose()\n"
    "    }\n\n"
    "    pub(super) fn finalize_reference(\n",
)

targeted = "crates/aligngauge-coverage/src/targeted.rs"
replace_once(
    targeted,
    "    max_histogram_bins: usize,\n}\n\nimpl TargetedReducer {\n",
    "    max_histogram_bins: usize,\n}\n\n"
    "struct TargetPartition {\n"
    "    near_target_bases: u64,\n"
    "    off_target_bases: u64,\n"
    "    near_target_territory_bases: u64,\n"
    "}\n\n"
    "struct AggregateTargetMetrics {\n"
    "    target_covered_bases: u64,\n"
    "    target_uncovered_bases: u64,\n"
    "    target_mean_depth: Availability<String>,\n"
    "    threshold_bases: BTreeMap<String, u64>,\n"
    "    threshold_percentages: BTreeMap<String, Availability<String>>,\n"
    "    target_depth_20th_percentile: Availability<u64>,\n"
    "    target_uniformity_penalty_80: Availability<String>,\n"
    "}\n\n"
    "impl TargetedReducer {\n",
)

finish_replacement = r'''    pub(crate) fn finish(
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
            near_target_territory_bases: partition.near_target_territory_bases,
            on_target_bases: self.on_target_bases,
            near_target_bases: partition.near_target_bases,
            off_target_bases: partition.off_target_bases,
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
        let histogram_territory = self
            .target_depth_histogram
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
        let weighted_target_depth = self.target_depth_histogram.iter().try_fold(
            0_u128,
            |total, (depth, bases)| {
                let value = u128::from(*depth)
                    .checked_mul(u128::from(*bases))
                    .ok_or_else(|| coverage_overflow("weighted target histogram depth"))?;
                total
                    .checked_add(value)
                    .ok_or_else(|| coverage_overflow("weighted target histogram depth"))
            },
        )?;
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
            .with_detail(
                "total_accepted_aligned_bases",
                total_accepted_aligned_bases,
            ));
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
            .ok_or_else(|| {
                internal_error("selected territory is smaller than target territory")
            })?;
        Ok(TargetPartition {
            near_target_bases,
            off_target_bases,
            near_target_territory_bases,
        })
    }

    fn aggregate_target_metrics(&self) -> Result<AggregateTargetMetrics, AlignGaugeError> {
        let target_covered_bases = self.target_depth_histogram.iter().try_fold(
            0_u64,
            |total, (depth, bases)| {
                if *depth == 0 {
                    Ok(total)
                } else {
                    total
                        .checked_add(*bases)
                        .ok_or_else(|| coverage_overflow("covered target bases"))
                }
            },
        )?;
        let target_uncovered_bases = self.target_depth_histogram.get(&0).copied().unwrap_or(0);
        let target_mean_depth = if self.target_territory_bases == 0 {
            Availability::unavailable("target_territory_is_zero")
        } else {
            Availability::Available(format_ratio_six(
                self.on_target_bases,
                self.target_territory_bases,
            )?)
        };
        let (threshold_bases, threshold_percentages) = self.target_thresholds()?;
        let target_depth_20th_percentile = target_depth_20th_percentile(
            &self.target_depth_histogram,
            self.target_territory_bases,
        )?;
        let target_uniformity_penalty_80 =
            self.target_uniformity(&target_depth_20th_percentile)?;
        Ok(AggregateTargetMetrics {
            target_covered_bases,
            target_uncovered_bases,
            target_mean_depth,
            threshold_bases,
            threshold_percentages,
            target_depth_20th_percentile,
            target_uniformity_penalty_80,
        })
    }

    fn target_thresholds(
        &self,
    ) -> Result<
        (
            BTreeMap<String, u64>,
            BTreeMap<String, Availability<String>>,
        ),
        AlignGaugeError,
    > {
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
        Ok((threshold_bases, threshold_percentages))
    }

    fn target_uniformity(
        &self,
        percentile: &Availability<u64>,
    ) -> Result<Availability<String>, AlignGaugeError> {
        match percentile {
            Availability::Available(0) => {
                Ok(Availability::unavailable("target_depth_20th_percentile_is_zero"))
            }
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
'''

regex_once(
    targeted,
    r"    pub\(crate\) fn finish\(\n        mut self,\n        total_accepted_aligned_bases: u64,\n    \) -> Result<TargetedCoverageReport, AlignGaugeError> \{.*?\n    \}\n}\n\npub\(crate\) fn memory_reservation",
    finish_replacement + "}\n\npub(crate) fn memory_reservation",
)

print("Milestone 9 finalizers refactored")
