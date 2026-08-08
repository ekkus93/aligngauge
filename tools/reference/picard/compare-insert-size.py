#!/usr/bin/env python3
"""Compare Picard 3.4.0 default ALL_READS insert metrics and histogram exactly."""

from __future__ import annotations

import json
import sys
from pathlib import Path

SCHEMA = "aligngauge-picard-insert-size-differential-v1"
PROFILE = "picard-insert-size-3.4.0-all-reads-v1"
METRIC_COLUMNS = [
    "MEDIAN_INSERT_SIZE",
    "MODE_INSERT_SIZE",
    "MEDIAN_ABSOLUTE_DEVIATION",
    "MIN_INSERT_SIZE",
    "MAX_INSERT_SIZE",
    "MEAN_INSERT_SIZE",
    "STANDARD_DEVIATION",
    "READ_PAIRS",
    "PAIR_ORIENTATION",
    "WIDTH_OF_10_PERCENT",
    "WIDTH_OF_20_PERCENT",
    "WIDTH_OF_30_PERCENT",
    "WIDTH_OF_40_PERCENT",
    "WIDTH_OF_50_PERCENT",
    "WIDTH_OF_60_PERCENT",
    "WIDTH_OF_70_PERCENT",
    "WIDTH_OF_80_PERCENT",
    "WIDTH_OF_90_PERCENT",
    "WIDTH_OF_95_PERCENT",
    "WIDTH_OF_99_PERCENT",
    "SAMPLE",
    "LIBRARY",
    "READ_GROUP",
]


def fail(message: str) -> None:
    raise SystemExit(message)


def parse(path: Path) -> tuple[list[str], list[dict[str, str]], list[str], list[list[str]]]:
    if not path.is_file():
        fail(f"missing insert-size metrics text: {path}")
    lines = path.read_text(encoding="utf-8").splitlines()
    metrics_class = "## METRICS CLASS\tpicard.analysis.InsertSizeMetrics"
    if metrics_class not in lines:
        # Picard legitimately emits an empty file when there are no usable pairs.
        if not any(line.strip() for line in lines):
            return [], [], [], []
        fail(f"missing InsertSizeMetrics class in non-empty file {path}")
    class_index = lines.index(metrics_class)
    cursor = class_index + 1
    while cursor < len(lines) and (not lines[cursor] or lines[cursor].startswith("#")):
        cursor += 1
    if cursor >= len(lines):
        fail(f"missing insert-size metric header in {path}")
    header = lines[cursor].split("\t")
    cursor += 1
    rows: list[dict[str, str]] = []
    while cursor < len(lines):
        line = lines[cursor]
        if line.startswith("## HISTOGRAM"):
            break
        cursor += 1
        if not line or line.startswith("#"):
            continue
        values = line.split("\t")
        if len(values) != len(header):
            fail(f"metric row/header width mismatch in {path}: {line!r}")
        rows.append(dict(zip(header, values, strict=True)))

    histogram_header: list[str] = []
    histogram_rows: list[list[str]] = []
    while cursor < len(lines) and not lines[cursor].startswith("## HISTOGRAM"):
        cursor += 1
    if cursor < len(lines):
        cursor += 1
        while cursor < len(lines) and (not lines[cursor] or lines[cursor].startswith("#")):
            cursor += 1
        if cursor < len(lines):
            histogram_header = lines[cursor].split("\t")
            cursor += 1
            while cursor < len(lines):
                line = lines[cursor]
                cursor += 1
                if not line or line.startswith("#"):
                    continue
                values = line.split("\t")
                if len(values) != len(histogram_header):
                    fail(f"histogram row/header width mismatch in {path}: {line!r}")
                histogram_rows.append(values)
    return header, rows, histogram_header, histogram_rows


def compare(reference: Path, actual: Path, report_path: Path) -> None:
    ref_header, ref_rows, ref_hist_header, ref_hist_rows = parse(reference)
    act_header, act_rows, act_hist_header, act_hist_rows = parse(actual)
    discrepancies: list[dict[str, object]] = []

    if not ref_header and not act_header:
        pass
    else:
        if act_header != METRIC_COLUMNS:
            discrepancies.append(
                {"section": "metrics_header", "expected": METRIC_COLUMNS, "observed": act_header}
            )
        missing = [field for field in METRIC_COLUMNS if field not in ref_header]
        if missing:
            fail(f"Picard reference output is missing expected InsertSizeMetrics fields: {missing!r}")

        ref_by_orientation = {row["PAIR_ORIENTATION"]: row for row in ref_rows}
        act_by_orientation = {row["PAIR_ORIENTATION"]: row for row in act_rows}
        orientations = list(dict.fromkeys(
            [row["PAIR_ORIENTATION"] for row in ref_rows]
            + [row["PAIR_ORIENTATION"] for row in act_rows]
        ))
        for orientation in orientations:
            expected = ref_by_orientation.get(orientation)
            observed = act_by_orientation.get(orientation)
            if expected is None or observed is None:
                discrepancies.append(
                    {
                        "section": "metrics",
                        "orientation": orientation,
                        "expected_present": expected is not None,
                        "observed_present": observed is not None,
                    }
                )
                continue
            for field in METRIC_COLUMNS:
                if expected[field] != observed[field]:
                    discrepancies.append(
                        {
                            "section": "metrics",
                            "orientation": orientation,
                            "field": field,
                            "expected": expected[field],
                            "observed": observed[field],
                        }
                    )

        if ref_hist_header != act_hist_header:
            discrepancies.append(
                {
                    "section": "histogram_header",
                    "expected": ref_hist_header,
                    "observed": act_hist_header,
                }
            )
        if ref_hist_rows != act_hist_rows:
            limit = max(len(ref_hist_rows), len(act_hist_rows))
            for index in range(limit):
                expected = ref_hist_rows[index] if index < len(ref_hist_rows) else None
                observed = act_hist_rows[index] if index < len(act_hist_rows) else None
                if expected != observed:
                    discrepancies.append(
                        {
                            "section": "histogram",
                            "index": index,
                            "expected": expected,
                            "observed": observed,
                        }
                    )

    report = {
        "schema": SCHEMA,
        "status": "exact" if not discrepancies else "mismatch",
        "compatibility_profile": PROFILE,
        "picard_version": "3.4.0",
        "metric_rows": len(act_rows),
        "histogram_rows": len(act_hist_rows),
        "tolerance": None,
        "discrepancies": discrepancies,
    }
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if discrepancies:
        fail(f"Picard insert-size differential mismatch; see {report_path}")


def main() -> None:
    if len(sys.argv) != 4:
        fail(f"usage: {sys.argv[0]} <picard.txt> <aligngauge.txt> <report.json>")
    compare(Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3]))


if __name__ == "__main__":
    main()
