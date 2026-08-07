#!/usr/bin/env python3
"""Apply the first Milestone 9 canonical targeted-metrics integration slice."""

from __future__ import annotations

import json
import re
from pathlib import Path


def read(path: str) -> str:
    return Path(path).read_text()


def write(path: str, text: str) -> None:
    Path(path).write_text(text)


def replace_once(path: str, old: str, new: str) -> None:
    text = read(path)
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:120]!r}")
    write(path, text.replace(old, new, 1))


def regex_once(path: str, pattern: str, replacement: str) -> None:
    text = read(path)
    updated, count = re.subn(pattern, lambda _: replacement, text, count=1, flags=re.S)
    if count != 1:
        raise SystemExit(f"{path}: expected one regex match, found {count}: {pattern[:120]!r}")
    write(path, updated)


# Core module/export and schema version.
replace_once(
    "crates/aligngauge-core/src/lib.rs",
    "pub mod model;\n",
    "pub mod model;\npub mod targeted;\n",
)
replace_once(
    "crates/aligngauge-core/src/lib.rs",
    "    Provenance, RecordInclusion, Summary, SystemInfo, Warning,\n};\n",
    "    Provenance, RecordInclusion, Summary, SystemInfo, Warning,\n};\n"
    "pub use targeted::{\n"
    "    PerTargetCoverageSummary, TargetedCoverageSummary, ZeroCoverageRunSummary,\n"
    "};\n",
)
replace_once(
    "crates/aligngauge-core/src/model.rs",
    "use crate::json::{JsonValue, ToJson};\n",
    "use crate::json::{JsonValue, ToJson};\nuse crate::targeted::TargetedCoverageSummary;\n",
)
replace_once(
    "crates/aligngauge-core/src/model.rs",
    'pub const SUMMARY_SCHEMA_VERSION: &str = "1.0.0";',
    'pub const SUMMARY_SCHEMA_VERSION: &str = "1.1.0";',
)
replace_once(
    "crates/aligngauge-core/src/model.rs",
    "    /// Per-reference reductions in BAM header order.\n"
    "    pub per_reference: Vec<PerReferenceCoverageSummary>,\n"
    "}\n",
    "    /// Per-reference reductions in BAM header order.\n"
    "    pub per_reference: Vec<PerReferenceCoverageSummary>,\n"
    "    /// Native targeted-sequencing reductions, or an explicit unavailable reason.\n"
    "    pub targeted: Availability<TargetedCoverageSummary>,\n"
    "}\n",
)
replace_once(
    "crates/aligngauge-core/src/model.rs",
    "            (\n"
    "                String::from(\"threshold_bases\"),\n"
    "                self.threshold_bases.to_json(),\n"
    "            ),\n",
    "            (String::from(\"targeted\"), self.targeted.to_json()),\n"
    "            (\n"
    "                String::from(\"threshold_bases\"),\n"
    "                self.threshold_bases.to_json(),\n"
    "            ),\n",
)

# Coverage crate dependency and public surface.
replace_once(
    "crates/aligngauge-coverage/Cargo.toml",
    'aligngauge-core = { path = "../aligngauge-core" }\n',
    'aligngauge-core = { path = "../aligngauge-core" }\n'
    'aligngauge-formats = { path = "../aligngauge-formats" }\n',
)
replace_once(
    "crates/aligngauge-coverage/src/lib.rs",
    "use aligngauge_core::AlignGaugeError;\n",
    "use aligngauge_core::AlignGaugeError;\n"
    "use aligngauge_formats::{\n"
    "    SequenceContig, SequenceDictionary, TargetNormalizationConfig, normalize_targets,\n"
    "    parse_bed_path,\n"
    "};\n",
)
replace_once(
    "crates/aligngauge-coverage/src/lib.rs",
    "mod report;\nmod util;\n",
    "mod report;\nmod targeted;\nmod util;\n",
)
replace_once(
    "crates/aligngauge-coverage/src/lib.rs",
    "pub use report::{CoverageReport, PerReferenceCoverage};\n",
    "pub use report::{CoverageReport, PerReferenceCoverage};\n"
    "pub use targeted::{\n"
    "    DEFAULT_NEAR_DISTANCE_BASES, TARGETED_PROFILE, TargetedCoverageReport,\n"
    "};\n",
)
replace_once(
    "crates/aligngauge-coverage/src/lib.rs",
    "    collector.finish()\n}\n\n#[cfg(test)]\n",
    "    collector.finish()\n}\n\n"
    "/// Analyze one local BAM with canonical whole-genome and native targeted reductions.\n"
    "///\n"
    "/// This convenience entry point is primarily useful for exact coverage differential tests.\n"
    "/// Production release orchestration feeds the same targeted collector from its shared reader.\n"
    "///\n"
    "/// # Errors\n"
    "/// Returns typed target, resource, reader-validation, or checked-arithmetic failures.\n"
    "pub fn analyze_bam_with_targets(\n"
    "    path: impl AsRef<Path>,\n"
    "    targets: impl AsRef<Path>,\n"
    "    near_distance_bases: u64,\n"
    "    options: CoverageOptions,\n"
    ") -> Result<CoverageReport, AlignGaugeError> {\n"
    "    let plan = CoverageMemoryPlan::plan(\n"
    "        options.memory_limit_bytes,\n"
    "        1,\n"
    "        options.chunk_size_override,\n"
    "    )?;\n"
    "    let mut reader = BamReader::open(path, FieldPlan::coverage(), ReaderOptions::default())?;\n"
    "    let dictionary = SequenceDictionary::new(\n"
    "        reader\n"
    "            .header()\n"
    "            .references()\n"
    "            .iter()\n"
    "            .map(|reference| SequenceContig {\n"
    "                name: reference.name().to_owned(),\n"
    "                length: reference.length(),\n"
    "            })\n"
    "            .collect(),\n"
    "    )?;\n"
    "    let parsed = parse_bed_path(targets.as_ref(), &dictionary)?;\n"
    "    let target_set = normalize_targets(\n"
    "        parsed.clone(),\n"
    "        TargetNormalizationConfig { flank_bases: 0 },\n"
    "    )?;\n"
    "    let selected_set = normalize_targets(\n"
    "        parsed,\n"
    "        TargetNormalizationConfig {\n"
    "            flank_bases: near_distance_bases,\n"
    "        },\n"
    "    )?;\n"
    "    let mut collector = CoverageCollector::new_targeted(\n"
    "        reader.header(),\n"
    "        options.thresholds,\n"
    "        plan,\n"
    "        target_set,\n"
    "        selected_set,\n"
    "        near_distance_bases,\n"
    "    )?;\n"
    "    while let Some(record) = reader.next_record()? {\n"
    "        collector.observe(&record)?;\n"
    "    }\n"
    "    collector.finish()\n"
    "}\n\n#[cfg(test)]\n",
)

# Memory-plan accounting for targeted reduction state.
replace_once(
    "crates/aligngauge-coverage/src/plan.rs",
    "    }\n}\n\nfn validate_plan_inputs(\n",
    "    }\n\n"
    "    /// Reserve additional exact reduction state before traversal.\n"
    "    ///\n"
    "    /// # Errors\n"
    "    /// Returns `resource_limit` if the additional state would exceed the hard memory limit.\n"
    "    pub fn with_additional_reduction_bytes(\n"
    "        mut self,\n"
    "        additional_bytes: u64,\n"
    "    ) -> Result<Self, AlignGaugeError> {\n"
    "        self.reduction_state_bytes = self\n"
    "            .reduction_state_bytes\n"
    "            .checked_add(additional_bytes)\n"
    "            .ok_or_else(|| resource_error(\"coverage reduction-state budget overflowed\"))?;\n"
    "        self.planned_peak_bytes = self\n"
    "            .planned_peak_bytes\n"
    "            .checked_add(additional_bytes)\n"
    "            .ok_or_else(|| resource_error(\"coverage planned peak overflowed\"))?;\n"
    "        if self.planned_peak_bytes > self.memory_limit_bytes {\n"
    "            return Err(resource_error(\n"
    "                \"targeted reduction state exceeds the coverage memory limit\",\n"
    "            )\n"
    "            .with_detail(\"memory_limit_bytes\", self.memory_limit_bytes)\n"
    "            .with_detail(\"planned_peak_bytes\", self.planned_peak_bytes)\n"
    "            .with_detail(\"targeted_reduction_bytes\", additional_bytes));\n"
    "        }\n"
    "        Ok(self)\n"
    "    }\n"
    "}\n\nfn validate_plan_inputs(\n",
)

# Exact u128 ratio rendering without overflowing numerator * 1e6.
replace_once(
    "crates/aligngauge-coverage/src/util.rs",
    "pub(crate) fn delta_bytes(\n",
    "pub(crate) fn format_ratio_u128_six(\n"
    "    numerator: u128,\n"
    "    denominator: u128,\n"
    ") -> Result<String, AlignGaugeError> {\n"
    "    if denominator == 0 {\n"
    "        return Err(internal_error(\"u128 decimal ratio denominator is zero\"));\n"
    "    }\n"
    "    let mut whole = numerator / denominator;\n"
    "    let mut remainder = numerator % denominator;\n"
    "    let mut fraction = 0_u32;\n"
    "    for _ in 0..6 {\n"
    "        let mut digit = 0_u32;\n"
    "        let mut next_remainder = 0_u128;\n"
    "        for _ in 0..10 {\n"
    "            if next_remainder >= denominator - remainder {\n"
    "                next_remainder -= denominator - remainder;\n"
    "                digit = digit\n"
    "                    .checked_add(1)\n"
    "                    .ok_or_else(|| coverage_overflow(\"u128 decimal digit\"))?;\n"
    "            } else {\n"
    "                next_remainder += remainder;\n"
    "            }\n"
    "        }\n"
    "        if digit > 9 {\n"
    "            return Err(internal_error(\"u128 decimal digit exceeded nine\"));\n"
    "        }\n"
    "        fraction = fraction\n"
    "            .checked_mul(10)\n"
    "            .and_then(|value| value.checked_add(digit))\n"
    "            .ok_or_else(|| coverage_overflow(\"u128 decimal fraction\"))?;\n"
    "        remainder = next_remainder;\n"
    "    }\n"
    "    let round_up = remainder != 0 && remainder >= denominator - remainder;\n"
    "    if round_up {\n"
    "        fraction = fraction\n"
    "            .checked_add(1)\n"
    "            .ok_or_else(|| coverage_overflow(\"u128 decimal rounding\"))?;\n"
    "        if fraction == 1_000_000 {\n"
    "            fraction = 0;\n"
    "            whole = whole\n"
    "                .checked_add(1)\n"
    "                .ok_or_else(|| coverage_overflow(\"u128 decimal whole part\"))?;\n"
    "        }\n"
    "    }\n"
    "    Ok(format!(\"{whole}.{fraction:06}\"))\n"
    "}\n\n"
    "pub(crate) fn delta_bytes(\n",
)

# Coverage collector gains an optional targeted reducer but keeps the exact same depth algorithm.
replace_once(
    "crates/aligngauge-coverage/src/accumulator/mod.rs",
    "use aligngauge_core::AlignGaugeError;\n",
    "use aligngauge_core::AlignGaugeError;\nuse aligngauge_formats::TargetSet;\n",
)
replace_once(
    "crates/aligngauge-coverage/src/accumulator/mod.rs",
    "use crate::plan::CoverageMemoryPlan;\n",
    "use crate::plan::CoverageMemoryPlan;\n"
    "use crate::targeted::{TargetedReducer, memory_reservation};\n",
)
replace_once(
    "crates/aligngauge-coverage/src/accumulator/mod.rs",
    "    total_accepted_aligned_bases: u64,\n}\n",
    "    total_accepted_aligned_bases: u64,\n"
    "    targeted: Option<TargetedReducer>,\n"
    "}\n",
)
regex_once(
    "crates/aligngauge-coverage/src/accumulator/mod.rs",
    r"    pub fn new\(\n        header: &ValidatedHeader,\n        thresholds: Vec<u32>,\n        plan: CoverageMemoryPlan,\n    \) -> Result<Self, AlignGaugeError> \{.*?\n    \}\n\n    /// Observe one record",
    "    pub fn new(\n"
    "        header: &ValidatedHeader,\n"
    "        thresholds: Vec<u32>,\n"
    "        plan: CoverageMemoryPlan,\n"
    "    ) -> Result<Self, AlignGaugeError> {\n"
    "        Self::build(header, thresholds, plan, None)\n"
    "    }\n\n"
    "    /// Initialize canonical coverage with native v0.3 targeted reductions.\n"
    "    ///\n"
    "    /// # Errors\n"
    "    /// Returns a typed resource or target invariant failure before traversal.\n"
    "    pub fn new_targeted(\n"
    "        header: &ValidatedHeader,\n"
    "        thresholds: Vec<u32>,\n"
    "        plan: CoverageMemoryPlan,\n"
    "        target_set: TargetSet,\n"
    "        selected_set: TargetSet,\n"
    "        near_distance_bases: u64,\n"
    "    ) -> Result<Self, AlignGaugeError> {\n"
    "        let reservation = memory_reservation(&target_set, &selected_set, thresholds.len())?;\n"
    "        let plan = plan.with_additional_reduction_bytes(reservation.additional_bytes)?;\n"
    "        let targeted = TargetedReducer::new(\n"
    "            header,\n"
    "            target_set,\n"
    "            selected_set,\n"
    "            thresholds.clone(),\n"
    "            near_distance_bases,\n"
    "            reservation,\n"
    "        )?;\n"
    "        Self::build(header, thresholds, plan, Some(targeted))\n"
    "    }\n\n"
    "    fn build(\n"
    "        header: &ValidatedHeader,\n"
    "        thresholds: Vec<u32>,\n"
    "        plan: CoverageMemoryPlan,\n"
    "        targeted: Option<TargetedReducer>,\n"
    "    ) -> Result<Self, AlignGaugeError> {\n"
    "        let delta_len = plan\n"
    "            .chunk_size_bases\n"
    "            .checked_add(1)\n"
    "            .ok_or_else(|| resource_error(\"coverage delta length overflowed\"))?;\n"
    "        let mut depth_histogram = BTreeMap::new();\n"
    "        depth_histogram.insert(0, 0);\n"
    "        Ok(Self {\n"
    "            thresholds,\n"
    "            plan,\n"
    "            references: header\n"
    "                .references()\n"
    "                .iter()\n"
    "                .map(ReferenceReduction::from_header)\n"
    "                .collect(),\n"
    "            next_reference_index: 0,\n"
    "            current_reference_index: None,\n"
    "            chunk_start: 0,\n"
    "            chunk_end: 0,\n"
    "            current_depth: 0,\n"
    "            delta: vec![0_i128; delta_len],\n"
    "            active_delta_positions: 0,\n"
    "            pending_events: BTreeMap::new(),\n"
    "            depth_histogram,\n"
    "            total_accepted_aligned_bases: 0,\n"
    "            targeted,\n"
    "        })\n"
    "    }\n\n"
    "    /// Observe one record",
)

# Thread absolute run positions through reduction callbacks.
replace_once(
    "crates/aligngauge-coverage/src/accumulator/reduce.rs",
    "use crate::report::{CoverageReport, PerReferenceCoverage, canonical_policy};\n",
    "use crate::report::{CoverageReport, PerReferenceCoverage, canonical_policy};\n",
)
regex_once(
    "crates/aligngauge-coverage/src/accumulator/reduce.rs",
    r"    pub\(super\) fn accumulate_run\(&mut self, depth: u64, bases: u64\) -> Result<\(\), AlignGaugeError> \{.*?\n    \}\n\n    pub\(super\) fn finalize_reference",
    "    pub(super) fn accumulate_run(\n"
    "        &mut self,\n"
    "        start: u64,\n"
    "        depth: u64,\n"
    "        bases: u64,\n"
    "    ) -> Result<(), AlignGaugeError> {\n"
    "        let reference_index = self\n"
    "            .current_reference_index\n"
    "            .ok_or_else(|| internal_error(\"coverage run has no current reference\"))?;\n"
    "        self.accumulate_for_reference(reference_index, start, depth, bases)\n"
    "    }\n\n"
    "    fn accumulate_for_reference(\n"
    "        &mut self,\n"
    "        reference_index: usize,\n"
    "        start: u64,\n"
    "        depth: u64,\n"
    "        bases: u64,\n"
    "    ) -> Result<(), AlignGaugeError> {\n"
    "        if bases == 0 {\n"
    "            return Ok(());\n"
    "        }\n"
    "        let end = start\n"
    "            .checked_add(bases)\n"
    "            .ok_or_else(|| coverage_overflow(\"coverage run end\"))?;\n"
    "        if let Some(targeted) = &mut self.targeted {\n"
    "            targeted.observe_run(reference_index, start, end, depth)?;\n"
    "        }\n"
    "        let existing = self.depth_histogram.get(&depth).copied();\n"
    "        if existing.is_none() && self.depth_histogram.len() >= self.plan.max_histogram_bins {\n"
    "            return Err(\n"
    "                resource_error(\"coverage histogram budget was exhausted during traversal\")\n"
    "                    .with_detail(\n"
    "                        \"maximum_histogram_bins\",\n"
    "                        u64_from_usize(self.plan.max_histogram_bins, \"histogram bins\")?,\n"
    "                    ),\n"
    "            );\n"
    "        }\n"
    "        let total = existing\n"
    "            .unwrap_or(0)\n"
    "            .checked_add(bases)\n"
    "            .ok_or_else(|| coverage_overflow(\"coverage histogram count\"))?;\n"
    "        self.depth_histogram.insert(depth, total);\n\n"
    "        let reference = self\n"
    "            .references\n"
    "            .get_mut(reference_index)\n"
    "            .ok_or_else(|| internal_error(\"coverage reduction reference disappeared\"))?;\n"
    "        if depth == 0 {\n"
    "            reference.uncovered_reference_bases = reference\n"
    "                .uncovered_reference_bases\n"
    "                .checked_add(bases)\n"
    "                .ok_or_else(|| coverage_overflow(\"uncovered reference bases\"))?;\n"
    "        } else {\n"
    "            reference.covered_reference_bases = reference\n"
    "                .covered_reference_bases\n"
    "                .checked_add(bases)\n"
    "                .ok_or_else(|| coverage_overflow(\"covered reference bases\"))?;\n"
    "        }\n"
    "        let depth_bases = depth\n"
    "            .checked_mul(bases)\n"
    "            .ok_or_else(|| coverage_overflow(\"depth times reference bases\"))?;\n"
    "        reference.depth_sum = reference\n"
    "            .depth_sum\n"
    "            .checked_add(depth_bases)\n"
    "            .ok_or_else(|| coverage_overflow(\"per-reference depth sum\"))?;\n"
    "        Ok(())\n"
    "    }\n\n"
    "    pub(super) fn finalize_reference",
)
replace_once(
    "crates/aligngauge-coverage/src/accumulator/reduce.rs",
    "        self.accumulate_for_reference(reference_index, 0, length)?;\n",
    "        self.accumulate_for_reference(reference_index, 0, 0, length)?;\n",
)
replace_once(
    "crates/aligngauge-coverage/src/accumulator/reduce.rs",
    "        Ok(CoverageReport {\n"
    "            policy: canonical_policy(),\n",
    "        let targeted = self\n"
    "            .targeted\n"
    "            .take()\n"
    "            .map(|targeted| targeted.finish(self.total_accepted_aligned_bases))\n"
    "            .transpose()?;\n\n"
    "        Ok(CoverageReport {\n"
    "            policy: canonical_policy(),\n",
)
replace_once(
    "crates/aligngauge-coverage/src/accumulator/reduce.rs",
    "            memory_plan: self.plan,\n"
    "        })\n",
    "            memory_plan: self.plan,\n"
    "            targeted,\n"
    "        })\n",
)

# Update every exact sweep site with the absolute run start.
events = "crates/aligngauge-coverage/src/accumulator/events.rs"
replace_once(
    events,
    "        self.accumulate_run(self.current_depth, skip_end - self.chunk_start)?;\n",
    "        self.accumulate_run(\n"
    "            self.chunk_start,\n"
    "            self.current_depth,\n"
    "            skip_end - self.chunk_start,\n"
    "        )?;\n",
)
replace_once(
    events,
    "            self.accumulate_run(self.current_depth, chunk_len_u64)?;\n",
    "            self.accumulate_run(self.chunk_start, self.current_depth, chunk_len_u64)?;\n",
)
replace_once(
    events,
    "                self.accumulate_run(self.current_depth, 1)?;\n",
    "                let position = self\n"
    "                    .chunk_start\n"
    "                    .checked_add(u64::try_from(offset).map_err(|source| {\n"
    "                        internal_error(\"coverage chunk offset does not fit u64\")\n"
    "                            .with_source(source)\n"
    "                    })?)\n"
    "                    .ok_or_else(|| coverage_overflow(\"coverage run position\"))?;\n"
    "                self.accumulate_run(position, self.current_depth, 1)?;\n",
)
replace_once(
    events,
    "                self.accumulate_run(self.current_depth, reference_length - self.chunk_start)?;\n",
    "                self.accumulate_run(\n"
    "                    self.chunk_start,\n"
    "                    self.current_depth,\n"
    "                    reference_length - self.chunk_start,\n"
    "                )?;\n",
)

# Canonical CoverageReport includes explicit targeted availability.
replace_once(
    "crates/aligngauge-coverage/src/report.rs",
    "use aligngauge_core::{\n"
    "    AlignGaugeError, CoveragePolicy, CoverageSummary, JsonValue, MateOverlapPolicy,\n",
    "use aligngauge_core::{\n"
    "    AlignGaugeError, Availability, CoveragePolicy, CoverageSummary, JsonValue, MateOverlapPolicy,\n",
)
replace_once(
    "crates/aligngauge-coverage/src/report.rs",
    "use crate::plan::CoverageMemoryPlan;\n",
    "use crate::plan::CoverageMemoryPlan;\nuse crate::targeted::TargetedCoverageReport;\n",
)
replace_once(
    "crates/aligngauge-coverage/src/report.rs",
    "    pub(crate) memory_plan: CoverageMemoryPlan,\n}\n",
    "    pub(crate) memory_plan: CoverageMemoryPlan,\n"
    "    pub(crate) targeted: Option<TargetedCoverageReport>,\n"
    "}\n",
)
replace_once(
    "crates/aligngauge-coverage/src/report.rs",
    "    pub const fn memory_plan(&self) -> &CoverageMemoryPlan {\n"
    "        &self.memory_plan\n"
    "    }\n\n",
    "    pub const fn memory_plan(&self) -> &CoverageMemoryPlan {\n"
    "        &self.memory_plan\n"
    "    }\n\n"
    "    /// Native targeted reductions when a target BED was supplied.\n"
    "    #[must_use]\n"
    "    pub const fn targeted(&self) -> Option<&TargetedCoverageReport> {\n"
    "        self.targeted.as_ref()\n"
    "    }\n\n",
)
replace_once(
    "crates/aligngauge-coverage/src/report.rs",
    "            per_reference: self\n"
    "                .per_reference\n"
    "                .iter()\n"
    "                .map(|reference| PerReferenceCoverageSummary {\n"
    "                    name: reference.name.clone(),\n"
    "                    length: reference.length,\n"
    "                    accepted_aligned_bases: reference.accepted_aligned_bases,\n"
    "                    covered_reference_bases: reference.covered_reference_bases,\n"
    "                    uncovered_reference_bases: reference.uncovered_reference_bases,\n"
    "                    mean_depth: reference.mean_depth.clone(),\n"
    "                })\n"
    "                .collect(),\n"
    "        }\n",
    "            per_reference: self\n"
    "                .per_reference\n"
    "                .iter()\n"
    "                .map(|reference| PerReferenceCoverageSummary {\n"
    "                    name: reference.name.clone(),\n"
    "                    length: reference.length,\n"
    "                    accepted_aligned_bases: reference.accepted_aligned_bases,\n"
    "                    covered_reference_bases: reference.covered_reference_bases,\n"
    "                    uncovered_reference_bases: reference.uncovered_reference_bases,\n"
    "                    mean_depth: reference.mean_depth.clone(),\n"
    "                })\n"
    "                .collect(),\n"
    "            targeted: self.targeted.as_ref().map_or_else(\n"
    "                || Availability::unavailable(\"target_bed_not_supplied\"),\n"
    "                |report| Availability::Available(report.to_core_summary()),\n"
    "            ),\n"
    "        }\n",
)
replace_once(
    "crates/aligngauge-coverage/src/report.rs",
    "        provenance.resource_limits.insert(\n"
    "            String::from(\"coverage_planned_peak_bytes\"),\n"
    "            self.memory_plan.planned_peak_bytes,\n"
    "        );\n"
    "        Ok(())\n",
    "        provenance.resource_limits.insert(\n"
    "            String::from(\"coverage_planned_peak_bytes\"),\n"
    "            self.memory_plan.planned_peak_bytes,\n"
    "        );\n"
    "        if let Some(targeted) = &self.targeted {\n"
    "            targeted.apply_provenance(provenance);\n"
    "        }\n"
    "        Ok(())\n",
)
replace_once(
    "crates/aligngauge-coverage/src/report.rs",
    "            (\n"
    "                String::from(\"threshold_bases\"),\n",
    "            (\n"
    "                String::from(\"targeted\"),\n"
    "                self.targeted.as_ref().map_or_else(\n"
    "                    || Availability::<aligngauge_core::TargetedCoverageSummary>::unavailable(\n"
    "                        \"target_bed_not_supplied\",\n"
    "                    )\n"
    "                    .to_json(),\n"
    "                    |report| Availability::Available(report.to_core_summary()).to_json(),\n"
    "                ),\n"
    "            ),\n"
    "            (\n"
    "                String::from(\"threshold_bases\"),\n",
)

# Fix two WIP reducer ownership/deref details before compilation.
replace_once(
    "crates/aligngauge-coverage/src/targeted.rs",
    "    TargetedCoverageSummary, ToJson, ZeroCoverageRunSummary,\n",
    "    TargetedCoverageSummary, ZeroCoverageRunSummary,\n",
)
replace_once(
    "crates/aligngauge-coverage/src/targeted.rs",
    "        *total_zero_runs = total_zero_runs\n"
    "            .checked_add(1)\n",
    "        *total_zero_runs = (*total_zero_runs)\n"
    "            .checked_add(1)\n",
)
replace_once(
    "crates/aligngauge-coverage/src/targeted.rs",
    "        let target_uniformity_penalty_80 = match target_depth_20th_percentile {\n"
    "            Availability::Available(0) => {\n"
    "                Availability::unavailable(\"target_depth_20th_percentile_is_zero\")\n"
    "            }\n"
    "            Availability::Available(depth) => {\n"
    "                let denominator = u128::from(self.target_territory_bases)\n"
    "                    .checked_mul(u128::from(depth))\n"
    "                    .ok_or_else(|| coverage_overflow(\"target uniformity denominator\"))?;\n"
    "                Availability::Available(format_ratio_u128_six(\n"
    "                    u128::from(self.on_target_bases),\n"
    "                    denominator,\n"
    "                )?)\n"
    "            }\n"
    "            Availability::Unavailable { reason } => Availability::Unavailable { reason },\n"
    "        };\n",
    "        let target_uniformity_penalty_80 = match &target_depth_20th_percentile {\n"
    "            Availability::Available(0) => {\n"
    "                Availability::unavailable(\"target_depth_20th_percentile_is_zero\")\n"
    "            }\n"
    "            Availability::Available(depth) => {\n"
    "                let denominator = u128::from(self.target_territory_bases)\n"
    "                    .checked_mul(u128::from(*depth))\n"
    "                    .ok_or_else(|| coverage_overflow(\"target uniformity denominator\"))?;\n"
    "                Availability::Available(format_ratio_u128_six(\n"
    "                    u128::from(self.on_target_bases),\n"
    "                    denominator,\n"
    "                )?)\n"
    "            }\n"
    "            Availability::Unavailable { reason } => Availability::Unavailable {\n"
    "                reason: reason.clone(),\n"
    "            },\n"
    "        };\n",
)

# Canonical golden sample now makes targeted unavailability explicit.
replace_once(
    "crates/aligngauge-core/tests/contracts.rs",
    "        per_reference: vec![PerReferenceCoverageSummary {\n"
    "            name: String::from(\"chr1\"),\n"
    "            length: 1_000,\n"
    "            accepted_aligned_bases: 100,\n"
    "            covered_reference_bases: 100,\n"
    "            uncovered_reference_bases: 900,\n"
    "            mean_depth: String::from(\"0.100000\"),\n"
    "        }],\n"
    "    };\n",
    "        per_reference: vec![PerReferenceCoverageSummary {\n"
    "            name: String::from(\"chr1\"),\n"
    "            length: 1_000,\n"
    "            accepted_aligned_bases: 100,\n"
    "            covered_reference_bases: 100,\n"
    "            uncovered_reference_bases: 900,\n"
    "            mean_depth: String::from(\"0.100000\"),\n"
    "        }],\n"
    "        targeted: Availability::unavailable(\"target_bed_not_supplied\"),\n"
    "    };\n",
)

golden_path = Path("crates/aligngauge-core/tests/golden/summary.json")
golden = json.loads(golden_path.read_text())
golden["schema_version"] = "1.1.0"
golden["coverage"]["value"]["targeted"] = {
    "reason": "target_bed_not_supplied",
    "status": "unavailable",
}
golden_path.write_text(json.dumps(golden, indent=2, sort_keys=True) + "\n")

# Strict summary schema 1.1.0 with typed targeted structures.
schema_path = Path("schemas/summary.schema.json")
schema = json.loads(schema_path.read_text())
schema["$id"] = "https://aligngauge.dev/schemas/summary-1.1.0.json"
schema["properties"]["schema_version"] = {"const": "1.1.0"}
coverage = schema["$defs"]["coverage"]
if "targeted" not in coverage["required"]:
    coverage["required"].append("targeted")
coverage["properties"]["targeted"] = {
    "oneOf": [
        {"$ref": "#/$defs/unavailable"},
        {
            "type": "object",
            "additionalProperties": False,
            "required": ["status", "value"],
            "properties": {
                "status": {"const": "available"},
                "value": {"$ref": "#/$defs/targeted_coverage"},
            },
        },
    ]
}
availability_string = {
    "oneOf": [
        {"$ref": "#/$defs/unavailable"},
        {
            "type": "object",
            "additionalProperties": False,
            "required": ["status", "value"],
            "properties": {
                "status": {"const": "available"},
                "value": {"type": "string", "pattern": "^[0-9]+\\.[0-9]{6}$"},
            },
        },
    ]
}
availability_u64 = {
    "oneOf": [
        {"$ref": "#/$defs/unavailable"},
        {
            "type": "object",
            "additionalProperties": False,
            "required": ["status", "value"],
            "properties": {
                "status": {"const": "available"},
                "value": {"type": "integer", "minimum": 0},
            },
        },
    ]
}
schema["$defs"]["zero_coverage_run"] = {
    "type": "object",
    "additionalProperties": False,
    "required": ["start", "end"],
    "properties": {
        "start": {"type": "integer", "minimum": 0},
        "end": {"type": "integer", "minimum": 0},
    },
}
schema["$defs"]["per_target_coverage"] = {
    "type": "object",
    "additionalProperties": False,
    "required": [
        "source_index", "line_number", "contig", "start", "end", "name", "length",
        "depth_sum", "mean_depth", "covered_bases", "uncovered_bases", "threshold_bases",
        "threshold_percentages", "zero_coverage_runs", "longest_zero_coverage_run_bases",
    ],
    "properties": {
        "source_index": {"type": "integer", "minimum": 0},
        "line_number": {"type": "integer", "minimum": 1},
        "contig": {"type": "string"},
        "start": {"type": "integer", "minimum": 0},
        "end": {"type": "integer", "minimum": 0},
        "name": {"type": ["string", "null"]},
        "length": {"type": "integer", "minimum": 0},
        "depth_sum": {"type": "integer", "minimum": 0},
        "mean_depth": availability_string,
        "covered_bases": {"type": "integer", "minimum": 0},
        "uncovered_bases": {"type": "integer", "minimum": 0},
        "threshold_bases": {"type": "object", "additionalProperties": {"type": "integer", "minimum": 0}},
        "threshold_percentages": {"type": "object", "additionalProperties": availability_string},
        "zero_coverage_runs": {"type": "array", "items": {"$ref": "#/$defs/zero_coverage_run"}},
        "longest_zero_coverage_run_bases": {"type": "integer", "minimum": 0},
    },
}
schema["$defs"]["targeted_coverage"] = {
    "type": "object",
    "additionalProperties": False,
    "required": [
        "profile", "coverage_profile", "duplicate_adjusted", "target_sha256",
        "target_size_bytes", "source_interval_count", "near_distance_bases",
        "genome_territory_bases", "target_territory_bases", "near_target_territory_bases",
        "on_target_bases", "near_target_bases", "off_target_bases", "target_depth_histogram",
        "target_mean_depth", "target_covered_bases", "target_uncovered_bases", "threshold_bases",
        "threshold_percentages", "dropout_target_count", "target_enrichment",
        "target_depth_20th_percentile", "target_uniformity_penalty_80", "per_target",
    ],
    "properties": {
        "profile": {"const": "aligngauge-targeted-v0.3"},
        "coverage_profile": {"type": "string"},
        "duplicate_adjusted": {"type": "boolean"},
        "target_sha256": {"type": "string", "pattern": "^[0-9a-f]{64}$"},
        "target_size_bytes": {"type": "integer", "minimum": 0},
        "source_interval_count": {"type": "integer", "minimum": 0},
        "near_distance_bases": {"type": "integer", "minimum": 0},
        "genome_territory_bases": {"type": "integer", "minimum": 0},
        "target_territory_bases": {"type": "integer", "minimum": 0},
        "near_target_territory_bases": {"type": "integer", "minimum": 0},
        "on_target_bases": {"type": "integer", "minimum": 0},
        "near_target_bases": {"type": "integer", "minimum": 0},
        "off_target_bases": {"type": "integer", "minimum": 0},
        "target_depth_histogram": {"type": "object", "additionalProperties": {"type": "integer", "minimum": 0}},
        "target_mean_depth": availability_string,
        "target_covered_bases": {"type": "integer", "minimum": 0},
        "target_uncovered_bases": {"type": "integer", "minimum": 0},
        "threshold_bases": {"type": "object", "additionalProperties": {"type": "integer", "minimum": 0}},
        "threshold_percentages": {"type": "object", "additionalProperties": availability_string},
        "dropout_target_count": {"type": "integer", "minimum": 0},
        "target_enrichment": availability_string,
        "target_depth_20th_percentile": availability_u64,
        "target_uniformity_penalty_80": availability_string,
        "per_target": {"type": "array", "items": {"$ref": "#/$defs/per_target_coverage"}},
    },
}
schema_path.write_text(json.dumps(schema, indent=2, sort_keys=True) + "\n")

# Exercise overflow-safe u128 ratio formatting explicitly.
replace_once(
    "crates/aligngauge-coverage/src/tests.rs",
    "use crate::util::{format_percentage_six, format_ratio_six};\n",
    "use crate::util::{format_percentage_six, format_ratio_six, format_ratio_u128_six};\n",
)
replace_once(
    "crates/aligngauge-coverage/src/tests.rs",
    "fn ratio_rounding_is_deterministic() {\n",
    "fn u128_ratio_rendering_handles_large_exact_products() {\n"
    "    let numerator = u128::from(u64::MAX) * u128::from(u64::MAX - 2);\n"
    "    let denominator = u128::from(u64::MAX) * u128::from(u64::MAX - 1);\n"
    "    assert_eq!(\n"
    "        format_ratio_u128_six(numerator, denominator).expect(\"u128 ratio\"),\n"
    "        \"1.000000\"\n"
    "    );\n"
    "    assert_eq!(\n"
    "        format_ratio_u128_six(1, 3).expect(\"u128 ratio\"),\n"
    "        \"0.333333\"\n"
    "    );\n"
    "    assert_eq!(\n"
    "        format_ratio_u128_six(2, 3).expect(\"u128 ratio\"),\n"
    "        \"0.666667\"\n"
    "    );\n"
    "}\n\n"
    "#[test]\n"
    "fn ratio_rounding_is_deterministic() {\n",
)

print("Milestone 9 core targeted integration edits applied")
