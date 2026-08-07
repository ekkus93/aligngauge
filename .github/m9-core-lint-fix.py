#!/usr/bin/env python3
"""Resolve remaining strict-Clippy shape issues in the generated M9 reducer."""

from pathlib import Path

PATH = Path("crates/aligngauge-coverage/src/targeted.rs")
text = PATH.read_text()


def replace_once(old: str, new: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one match, found {count}: {old[:120]!r}")
    text = text.replace(old, new, 1)


replace_once(
    "struct TargetPartition {\n"
    "    near_target_bases: u64,\n"
    "    off_target_bases: u64,\n"
    "    near_target_territory_bases: u64,\n"
    "}\n\n"
    "struct AggregateTargetMetrics {",
    "struct TargetPartition {\n"
    "    near_aligned: u64,\n"
    "    off_aligned: u64,\n"
    "    near_territory: u64,\n"
    "}\n\n"
    "struct TargetThresholds {\n"
    "    bases: BTreeMap<String, u64>,\n"
    "    percentages: BTreeMap<String, Availability<String>>,\n"
    "}\n\n"
    "struct AggregateTargetMetrics {",
)
replace_once(
    "            near_target_territory_bases: partition.near_target_territory_bases,\n"
    "            on_target_bases: self.on_target_bases,\n"
    "            near_target_bases: partition.near_target_bases,\n"
    "            off_target_bases: partition.off_target_bases,",
    "            near_target_territory_bases: partition.near_territory,\n"
    "            on_target_bases: self.on_target_bases,\n"
    "            near_target_bases: partition.near_aligned,\n"
    "            off_target_bases: partition.off_aligned,",
)
replace_once(
    "        Ok(TargetPartition {\n"
    "            near_target_bases,\n"
    "            off_target_bases,\n"
    "            near_target_territory_bases,\n"
    "        })",
    "        Ok(TargetPartition {\n"
    "            near_aligned: near_target_bases,\n"
    "            off_aligned: off_target_bases,\n"
    "            near_territory: near_target_territory_bases,\n"
    "        })",
)
replace_once(
    "        let (threshold_bases, threshold_percentages) = self.target_thresholds()?;",
    "        let thresholds = self.target_thresholds()?;",
)
replace_once(
    "            threshold_bases,\n"
    "            threshold_percentages,",
    "            threshold_bases: thresholds.bases,\n"
    "            threshold_percentages: thresholds.percentages,",
)
replace_once(
    "    fn target_thresholds(\n"
    "        &self,\n"
    "    ) -> Result<\n"
    "        (\n"
    "            BTreeMap<String, u64>,\n"
    "            BTreeMap<String, Availability<String>>,\n"
    "        ),\n"
    "        AlignGaugeError,\n"
    "    > {",
    "    fn target_thresholds(&self) -> Result<TargetThresholds, AlignGaugeError> {",
)
replace_once(
    "        Ok((threshold_bases, threshold_percentages))\n"
    "    }",
    "        Ok(TargetThresholds {\n"
    "            bases: threshold_bases,\n"
    "            percentages: threshold_percentages,\n"
    "        })\n"
    "    }",
)

PATH.write_text(text)
print("Milestone 9 remaining lint shapes refactored")
