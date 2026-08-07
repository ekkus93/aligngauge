#!/usr/bin/env python3
"""Reduce pinned ``samtools depth -aa`` output to exact M5 coverage metrics."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def ratio_six(numerator: int, denominator: int) -> str:
    if denominator == 0:
        return "0.000000"
    scale = 1_000_000
    rounded = (numerator * scale + denominator // 2) // denominator
    return f"{rounded // scale}.{rounded % scale:06d}"


def percentage_six(numerator: int, denominator: int) -> str:
    if denominator == 0:
        return "0.000000"
    scale = 100_000_000
    rounded = (numerator * scale + denominator // 2) // denominator
    return f"{rounded // 1_000_000}.{rounded % 1_000_000:06d}"


def parse_thresholds(text: str) -> list[int]:
    values = sorted({int(item.strip()) for item in text.split(",")})
    if not values or values[0] <= 0:
        raise SystemExit("coverage thresholds must be positive")
    return values


def summarize(path: Path, thresholds: list[int]) -> dict[str, object]:
    histogram: dict[int, int] = {}
    references: list[dict[str, object]] = []
    seen_references: set[str] = set()
    current_name: str | None = None
    current_length = 0
    current_depth_sum = 0
    current_covered = 0
    total_depth = 0

    def finish_reference() -> None:
        nonlocal current_name, current_length, current_depth_sum, current_covered
        if current_name is None:
            return
        references.append(
            {
                "accepted_aligned_bases": current_depth_sum,
                "covered_reference_bases": current_covered,
                "length": current_length,
                "mean_depth": ratio_six(current_depth_sum, current_length),
                "name": current_name,
                "uncovered_reference_bases": current_length - current_covered,
            }
        )
        seen_references.add(current_name)

    with path.open("r", encoding="utf-8") as handle:
        for line_number, raw in enumerate(handle, start=1):
            line = raw.rstrip("\n")
            columns = line.split("\t")
            if len(columns) != 3:
                raise SystemExit(f"depth line {line_number} does not have three columns")
            name, position_text, depth_text = columns
            try:
                position = int(position_text)
                depth = int(depth_text)
            except ValueError as error:
                raise SystemExit(f"depth line {line_number} is not numeric: {error}") from error
            if position <= 0 or depth < 0:
                raise SystemExit(f"depth line {line_number} contains a negative/zero field")

            if name != current_name:
                finish_reference()
                if name in seen_references:
                    raise SystemExit(f"reference {name!r} is not contiguous in depth output")
                current_name = name
                current_length = 0
                current_depth_sum = 0
                current_covered = 0
                if position != 1:
                    raise SystemExit(f"reference {name!r} does not begin at position 1")
            expected_position = current_length + 1
            if position != expected_position:
                raise SystemExit(
                    f"reference {name!r} position is not contiguous: "
                    f"expected {expected_position}, got {position}"
                )
            current_length = position
            current_depth_sum += depth
            total_depth += depth
            if depth > 0:
                current_covered += 1
            histogram[depth] = histogram.get(depth, 0) + 1

    finish_reference()
    if not references:
        raise SystemExit("samtools depth capture contains no reference territory")

    territory = sum(int(reference["length"]) for reference in references)
    covered = sum(int(reference["covered_reference_bases"]) for reference in references)
    threshold_bases = {
        str(threshold): sum(count for depth, count in histogram.items() if depth >= threshold)
        for threshold in thresholds
    }
    return {
        "covered_reference_bases": covered,
        "depth_histogram": {str(depth): histogram[depth] for depth in sorted(histogram)},
        "per_reference": references,
        "policy": {
            "include_duplicates": False,
            "include_qc_fail": False,
            "include_secondary": False,
            "include_supplementary": False,
            "mate_overlap_correction": False,
            "minimum_mapq": 0,
            "name": "aligngauge-v0.1",
        },
        "schema": "aligngauge-coverage-v1",
        "threshold_bases": threshold_bases,
        "threshold_percentages": {
            threshold: percentage_six(bases, territory)
            for threshold, bases in threshold_bases.items()
        },
        "total_accepted_aligned_bases": total_depth,
        "uncovered_reference_bases": territory - covered,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("depth_stdout", type=Path)
    parser.add_argument("output_json", type=Path)
    parser.add_argument("--thresholds", default="1,10,20,30")
    args = parser.parse_args()
    summary = summarize(args.depth_stdout, parse_thresholds(args.thresholds))
    args.output_json.parent.mkdir(parents=True, exist_ok=True)
    args.output_json.write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
