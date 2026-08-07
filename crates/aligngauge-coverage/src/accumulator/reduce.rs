//! Exact histogram, threshold, and per-reference reductions.

use std::collections::BTreeMap;

use aligngauge_core::AlignGaugeError;

use super::CoverageCollector;
use crate::report::{CoverageReport, PerReferenceCoverage, canonical_policy};
use crate::util::{
    coverage_overflow, format_percentage_six, format_ratio_six, internal_error, resource_error,
    u64_from_usize,
};

impl CoverageCollector {
    pub(super) fn finalize_unvisited_reference(
        &mut self,
        reference_index: usize,
    ) -> Result<(), AlignGaugeError> {
        let length = self
            .references
            .get(reference_index)
            .ok_or_else(|| internal_error("unvisited reference index is invalid"))?
            .length;
        self.accumulate_for_reference(reference_index, 0, length)?;
        self.finalize_reference(reference_index)?;
        self.next_reference_index = reference_index
            .checked_add(1)
            .ok_or_else(|| coverage_overflow("reference cursor"))?;
        Ok(())
    }

    pub(super) fn accumulate_run(&mut self, depth: u64, bases: u64) -> Result<(), AlignGaugeError> {
        let reference_index = self
            .current_reference_index
            .ok_or_else(|| internal_error("coverage run has no current reference"))?;
        self.accumulate_for_reference(reference_index, depth, bases)
    }

    fn accumulate_for_reference(
        &mut self,
        reference_index: usize,
        depth: u64,
        bases: u64,
    ) -> Result<(), AlignGaugeError> {
        if bases == 0 {
            return Ok(());
        }
        let existing = self.depth_histogram.get(&depth).copied();
        if existing.is_none() && self.depth_histogram.len() >= self.plan.max_histogram_bins {
            return Err(
                resource_error("coverage histogram budget was exhausted during traversal")
                    .with_detail(
                        "maximum_histogram_bins",
                        u64_from_usize(self.plan.max_histogram_bins, "histogram bins")?,
                    ),
            );
        }
        let total = existing
            .unwrap_or(0)
            .checked_add(bases)
            .ok_or_else(|| coverage_overflow("coverage histogram count"))?;
        self.depth_histogram.insert(depth, total);

        let reference = self
            .references
            .get_mut(reference_index)
            .ok_or_else(|| internal_error("coverage reduction reference disappeared"))?;
        if depth == 0 {
            reference.uncovered_reference_bases = reference
                .uncovered_reference_bases
                .checked_add(bases)
                .ok_or_else(|| coverage_overflow("uncovered reference bases"))?;
        } else {
            reference.covered_reference_bases = reference
                .covered_reference_bases
                .checked_add(bases)
                .ok_or_else(|| coverage_overflow("covered reference bases"))?;
        }
        let depth_bases = depth
            .checked_mul(bases)
            .ok_or_else(|| coverage_overflow("depth times reference bases"))?;
        reference.depth_sum = reference
            .depth_sum
            .checked_add(depth_bases)
            .ok_or_else(|| coverage_overflow("per-reference depth sum"))?;
        Ok(())
    }

    pub(super) fn finalize_reference(
        &mut self,
        reference_index: usize,
    ) -> Result<(), AlignGaugeError> {
        let reference = self
            .references
            .get_mut(reference_index)
            .ok_or_else(|| internal_error("coverage finalization reference disappeared"))?;
        if reference.finalized {
            return Err(internal_error(
                "coverage reference was finalized more than once",
            ));
        }
        let territory = reference
            .covered_reference_bases
            .checked_add(reference.uncovered_reference_bases)
            .ok_or_else(|| coverage_overflow("per-reference territory"))?;
        if territory != reference.length {
            return Err(internal_error(
                "coverage reference territory does not match header length",
            )
            .with_detail("reference_length", reference.length)
            .with_detail("evaluated_bases", territory));
        }
        if reference.depth_sum != reference.accepted_aligned_bases {
            return Err(
                internal_error("coverage depth sum does not equal accepted aligned bases")
                    .with_detail("depth_sum", reference.depth_sum)
                    .with_detail("accepted_aligned_bases", reference.accepted_aligned_bases),
            );
        }
        reference.finalized = true;
        Ok(())
    }

    pub(crate) fn finish(mut self) -> Result<CoverageReport, AlignGaugeError> {
        self.finish_current_reference()?;
        while self.next_reference_index < self.references.len() {
            self.finalize_unvisited_reference(self.next_reference_index)?;
        }

        let territory = self.references.iter().try_fold(0_u64, |total, reference| {
            total
                .checked_add(reference.length)
                .ok_or_else(|| coverage_overflow("whole-run reference territory"))
        })?;
        let histogram_territory =
            self.depth_histogram
                .values()
                .try_fold(0_u64, |total, bases| {
                    total
                        .checked_add(*bases)
                        .ok_or_else(|| coverage_overflow("histogram territory"))
                })?;
        if histogram_territory != territory {
            return Err(internal_error(
                "coverage histogram does not equal evaluated reference territory",
            )
            .with_detail("histogram_bases", histogram_territory)
            .with_detail("reference_bases", territory));
        }

        let weighted_depth =
            self.depth_histogram
                .iter()
                .try_fold(0_u128, |total, (depth, bases)| {
                    let product = u128::from(*depth)
                        .checked_mul(u128::from(*bases))
                        .ok_or_else(|| coverage_overflow("weighted histogram depth"))?;
                    total
                        .checked_add(product)
                        .ok_or_else(|| coverage_overflow("weighted histogram total"))
                })?;
        if weighted_depth != u128::from(self.total_accepted_aligned_bases) {
            return Err(internal_error(
                "weighted coverage histogram does not equal accepted aligned bases",
            )
            .with_detail("accepted_aligned_bases", self.total_accepted_aligned_bases));
        }

        let mut threshold_bases = BTreeMap::new();
        let mut threshold_percentages = BTreeMap::new();
        for threshold in &self.thresholds {
            let bases = self
                .depth_histogram
                .range(u64::from(*threshold)..)
                .try_fold(0_u64, |total, (_, count)| {
                    total
                        .checked_add(*count)
                        .ok_or_else(|| coverage_overflow("threshold base count"))
                })?;
            threshold_bases.insert(*threshold, bases);
            threshold_percentages.insert(*threshold, format_percentage_six(bases, territory)?);
        }

        let covered_reference_bases =
            self.references.iter().try_fold(0_u64, |total, reference| {
                total
                    .checked_add(reference.covered_reference_bases)
                    .ok_or_else(|| coverage_overflow("whole-run covered bases"))
            })?;
        let uncovered_reference_bases =
            self.references.iter().try_fold(0_u64, |total, reference| {
                total
                    .checked_add(reference.uncovered_reference_bases)
                    .ok_or_else(|| coverage_overflow("whole-run uncovered bases"))
            })?;
        let per_reference = self
            .references
            .into_iter()
            .map(|reference| {
                Ok(PerReferenceCoverage {
                    mean_depth: format_ratio_six(
                        reference.accepted_aligned_bases,
                        reference.length,
                    )?,
                    name: reference.name,
                    length: reference.length,
                    accepted_aligned_bases: reference.accepted_aligned_bases,
                    covered_reference_bases: reference.covered_reference_bases,
                    uncovered_reference_bases: reference.uncovered_reference_bases,
                })
            })
            .collect::<Result<Vec<_>, AlignGaugeError>>()?;

        Ok(CoverageReport {
            policy: canonical_policy(),
            total_accepted_aligned_bases: self.total_accepted_aligned_bases,
            depth_histogram: self.depth_histogram,
            threshold_bases,
            threshold_percentages,
            covered_reference_bases,
            uncovered_reference_bases,
            per_reference,
            memory_plan: self.plan,
        })
    }
}
