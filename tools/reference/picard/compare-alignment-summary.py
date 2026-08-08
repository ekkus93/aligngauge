#!/usr/bin/env python3
"""Compare the exact Milestone 11 Picard alignment-summary subset."""

from __future__ import annotations

import json
import sys
from pathlib import Path

SCHEMA = "aligngauge-picard-alignment-summary-differential-v1"
PROFILE = "picard-alignment-summary-3.4.0-all-reads-subset-v1"
CLAIMED = [
    "CATEGORY",
    "TOTAL_READS",
    "PF_READS",
    "PCT_PF_READS",
    "PF_NOISE_READS",
    "PCT_ADAPTER",
    "MEAN_READ_LENGTH",
    "SD_READ_LENGTH",
    "MEDIAN_READ_LENGTH",
    "MAD_READ_LENGTH",
    "MIN_READ_LENGTH",
    "MAX_READ_LENGTH",
    "BAD_CYCLES",
]


def fail(message: str) -> None:
    raise SystemExit(message)


def parse_metrics(path: Path) -> tuple[list[str], list[dict[str, str]]]:
    if not path.is_file():
        fail(f"missing Picard metrics text: {path}")
    lines = path.read_text(encoding="utf-8").splitlines()
    class_index = None
    for index, line in enumerate(lines):
        if line == "## METRICS CLASS\tpicard.analysis.AlignmentSummaryMetrics":
            class_index = index
            break
    if class_index is None:
        fail(f"missing AlignmentSummaryMetrics class in {path}")
    cursor = class_index + 1
    while cursor < len(lines) and (not lines[cursor] or lines[cursor].startswith("#")):
        cursor += 1
    if cursor >= len(lines):
        fail(f"missing alignment-summary header in {path}")
    header = lines[cursor].split("\t")
    cursor += 1
    rows: list[dict[str, str]] = []
    while cursor < len(lines):
        line = lines[cursor]
        cursor += 1
        if not line or line.startswith("## HISTOGRAM"):
            break
        if line.startswith("#"):
            continue
        values = line.split("\t")
        if len(values) != len(header):
            fail(f"row/header width mismatch in {path}: {line!r}")
        rows.append(dict(zip(header, values, strict=True)))
    return header, rows


def compare(reference: Path, actual: Path, report_path: Path) -> None:
    reference_header, reference_rows = parse_metrics(reference)
    actual_header, actual_rows = parse_metrics(actual)
    if actual_header != CLAIMED:
        fail(
            "AlignGauge alignment-summary output must contain exactly the claimed subset; "
            f"observed columns={actual_header!r}"
        )
    missing = [field for field in CLAIMED if field not in reference_header]
    if missing:
        fail(f"Picard reference output is missing claimed fields: {missing!r}")

    reference_by_category = {row["CATEGORY"]: row for row in reference_rows}
    actual_by_category = {row["CATEGORY"]: row for row in actual_rows}
    discrepancies: list[dict[str, object]] = []

    categories = list(dict.fromkeys(
        [row["CATEGORY"] for row in reference_rows]
        + [row["CATEGORY"] for row in actual_rows]
    ))
    for category in categories:
        expected = reference_by_category.get(category)
        observed = actual_by_category.get(category)
        if expected is None or observed is None:
            discrepancies.append(
                {
                    "category": category,
                    "field": "CATEGORY",
                    "expected": expected is not None,
                    "observed": observed is not None,
                }
            )
            continue
        for field in CLAIMED:
            expected_value = expected[field]
            observed_value = observed[field]
            if expected_value != observed_value:
                discrepancies.append(
                    {
                        "category": category,
                        "field": field,
                        "expected": expected_value,
                        "observed": observed_value,
                    }
                )

    report = {
        "schema": SCHEMA,
        "status": "exact" if not discrepancies else "mismatch",
        "compatibility_profile": PROFILE,
        "picard_version": "3.4.0",
        "claimed_columns": CLAIMED,
        "categories": categories,
        "tolerance": None,
        "discrepancies": discrepancies,
    }
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if discrepancies:
        fail(f"Picard alignment-summary differential mismatch; see {report_path}")


def main() -> None:
    if len(sys.argv) != 4:
        fail(f"usage: {sys.argv[0]} <picard.txt> <aligngauge.txt> <report.json>")
    compare(Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3]))


if __name__ == "__main__":
    main()
