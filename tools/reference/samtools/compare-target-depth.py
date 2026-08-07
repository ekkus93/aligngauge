#!/usr/bin/env python3
"""Compare AlignGauge native targeted coverage primitives to pinned samtools depth."""

from __future__ import annotations

import hashlib
import json
import sys
from collections import Counter
from decimal import Decimal, ROUND_HALF_UP, localcontext
from pathlib import Path


def fail(message: str) -> None:
    raise SystemExit(message)


def ratio6(numerator: int, denominator: int) -> str:
    if denominator <= 0:
        fail("ratio denominator must be positive")
    with localcontext() as context:
        context.prec = 80
        value = Decimal(numerator) / Decimal(denominator)
        return str(value.quantize(Decimal("0.000001"), rounding=ROUND_HALF_UP))


def percentage6(numerator: int, denominator: int) -> str:
    return ratio6(numerator * 100, denominator)


def available(value):
    return {"status": "available", "value": value}


def parse_targets(path: Path):
    intervals = []
    for line_number, raw in enumerate(path.read_text().splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        first = line.split()[0]
        if first in {"track", "browser"}:
            continue
        fields = line.split()
        if not 3 <= len(fields) <= 12:
            fail(f"invalid reference BED field count at line {line_number}")
        contig, start_text, end_text = fields[:3]
        start, end = int(start_text), int(end_text)
        if start < 0 or end < start:
            fail(f"invalid reference BED coordinates at line {line_number}")
        intervals.append(
            {
                "source_index": len(intervals),
                "line_number": line_number,
                "contig": contig,
                "start": start,
                "end": end,
                "name": fields[3] if len(fields) >= 4 else None,
                "depths": [0] * (end - start),
            }
        )
    return intervals


def fill_depths(depth_path: Path, intervals):
    position_targets = {}
    for target_index, interval in enumerate(intervals):
        for offset in range(interval["end"] - interval["start"]):
            key = (interval["contig"], interval["start"] + offset)
            position_targets.setdefault(key, []).append((target_index, offset))

    union_depth = {key: 0 for key in position_targets}
    observed = set()
    for raw in depth_path.read_text().splitlines():
        if not raw.strip():
            continue
        fields = raw.split("\t")
        if len(fields) != 3:
            fail(f"unexpected samtools depth row: {raw!r}")
        contig, one_based_text, depth_text = fields
        key = (contig, int(one_based_text) - 1)
        if key not in position_targets:
            # samtools -a with -b may emit positions beyond BED in some edge cases.
            continue
        if key in observed:
            fail(f"duplicate samtools depth position: {key}")
        observed.add(key)
        depth = int(depth_text)
        if depth < 0:
            fail(f"negative samtools depth at {key}")
        union_depth[key] = depth
        for target_index, offset in position_targets[key]:
            intervals[target_index]["depths"][offset] = depth
    return union_depth


def zero_runs(interval):
    depths = interval["depths"]
    runs = []
    index = 0
    while index < len(depths):
        if depths[index] != 0:
            index += 1
            continue
        start_offset = index
        while index < len(depths) and depths[index] == 0:
            index += 1
        runs.append(
            {
                "start": interval["start"] + start_offset,
                "end": interval["start"] + index,
            }
        )
    return runs


def expected_per_target(interval, thresholds):
    depths = interval["depths"]
    length = len(depths)
    depth_sum = sum(depths)
    covered = sum(depth > 0 for depth in depths)
    threshold_bases = {str(value): sum(depth >= value for depth in depths) for value in thresholds}
    if length == 0:
        mean = {"status": "unavailable", "reason": "zero_length_target"}
        threshold_percentages = {
            str(value): {"status": "unavailable", "reason": "zero_length_target"}
            for value in thresholds
        }
    else:
        mean = available(ratio6(depth_sum, length))
        threshold_percentages = {
            str(value): available(percentage6(threshold_bases[str(value)], length))
            for value in thresholds
        }
    runs = zero_runs(interval)
    return {
        "source_index": interval["source_index"],
        "line_number": interval["line_number"],
        "contig": interval["contig"],
        "start": interval["start"],
        "end": interval["end"],
        "name": interval["name"],
        "length": length,
        "depth_sum": depth_sum,
        "mean_depth": mean,
        "covered_bases": covered,
        "uncovered_bases": length - covered,
        "threshold_bases": threshold_bases,
        "threshold_percentages": threshold_percentages,
        "zero_coverage_runs": runs,
        "longest_zero_coverage_run_bases": max(
            (run["end"] - run["start"] for run in runs), default=0
        ),
    }


def expected_aggregate(union_depth, thresholds):
    depths = list(union_depth.values())
    territory = len(depths)
    histogram = Counter(depths)
    on_target = sum(depths)
    threshold_bases = {str(value): sum(depth >= value for depth in depths) for value in thresholds}
    expected = {
        "target_territory_bases": territory,
        "on_target_bases": on_target,
        "target_depth_histogram": {str(depth): count for depth, count in sorted(histogram.items())},
        "target_covered_bases": sum(depth > 0 for depth in depths),
        "target_uncovered_bases": sum(depth == 0 for depth in depths),
        "threshold_bases": threshold_bases,
    }
    if territory == 0:
        expected["target_mean_depth"] = {"status": "unavailable", "reason": "target_territory_is_zero"}
        expected["threshold_percentages"] = {
            str(value): {"status": "unavailable", "reason": "target_territory_is_zero"}
            for value in thresholds
        }
        expected["target_depth_20th_percentile"] = {
            "status": "unavailable",
            "reason": "target_territory_is_zero",
        }
        expected["target_uniformity_penalty_80"] = {
            "status": "unavailable",
            "reason": "target_territory_is_zero",
        }
        return expected

    expected["target_mean_depth"] = available(ratio6(on_target, territory))
    expected["threshold_percentages"] = {
        str(value): available(percentage6(threshold_bases[str(value)], territory))
        for value in thresholds
    }
    rank = (territory + 4) // 5
    cumulative = 0
    d20 = None
    for depth, count in sorted(histogram.items()):
        cumulative += count
        if cumulative >= rank:
            d20 = depth
            break
    if d20 is None:
        fail("reference target histogram did not satisfy D20 rank")
    expected["target_depth_20th_percentile"] = available(d20)
    if d20 == 0:
        expected["target_uniformity_penalty_80"] = {
            "status": "unavailable",
            "reason": "target_depth_20th_percentile_is_zero",
        }
    else:
        expected["target_uniformity_penalty_80"] = available(
            ratio6(on_target, territory * d20)
        )
    return expected


def main() -> None:
    if len(sys.argv) != 5:
        fail(
            "usage: compare-target-depth.py SAMTOOLS_DEPTH TARGET_BED ALIGNGAUGE_SUMMARY OUTPUT_REPORT"
        )
    depth_path = Path(sys.argv[1])
    target_path = Path(sys.argv[2])
    summary_path = Path(sys.argv[3])
    output_path = Path(sys.argv[4])

    summary = json.loads(summary_path.read_text())
    coverage = summary["coverage"]
    if coverage.get("status") != "available":
        fail("AlignGauge coverage is unavailable")
    targeted_wrapper = coverage["value"]["targeted"]
    if targeted_wrapper.get("status") != "available":
        fail("AlignGauge targeted coverage is unavailable")
    targeted = targeted_wrapper["value"]

    target_bytes = target_path.read_bytes()
    if targeted["target_sha256"] != hashlib.sha256(target_bytes).hexdigest():
        fail("target SHA-256 differs from exact reference BED bytes")
    if targeted["target_size_bytes"] != len(target_bytes):
        fail("target byte size differs from exact reference BED bytes")

    intervals = parse_targets(target_path)
    if targeted["source_interval_count"] != len(intervals):
        fail("source interval count differs from reference BED")
    union_depth = fill_depths(depth_path, intervals)
    thresholds = sorted(int(value) for value in targeted["threshold_bases"])

    aggregate = expected_aggregate(union_depth, thresholds)
    checked_aggregate = []
    for field, expected in aggregate.items():
        actual = targeted[field]
        if actual != expected:
            fail(f"aggregate targeted mismatch for {field}: expected {expected!r}, got {actual!r}")
        checked_aggregate.append(field)

    expected_targets = [expected_per_target(interval, thresholds) for interval in intervals]
    if targeted["per_target"] != expected_targets:
        fail("per-target coverage differs from independently reduced samtools depth")
    dropout_count = sum(
        target["length"] > 0 and target["uncovered_bases"] > 0 for target in expected_targets
    )
    if targeted["dropout_target_count"] != dropout_count:
        fail("dropout target count differs from independent target reduction")

    report = {
        "schema": "aligngauge-targeted-samtools-differential-v1",
        "status": "exact",
        "reference_tool": "samtools depth",
        "compatibility_claim": None,
        "target_sha256": targeted["target_sha256"],
        "target_size_bytes": targeted["target_size_bytes"],
        "checked_aggregate_fields": checked_aggregate,
        "checked_per_target_fields": sorted(expected_targets[0]) if expected_targets else [],
        "dropout_target_count": dropout_count,
        "notes": [
            "Samtools invocation explicitly excludes UNMAP,SECONDARY,QCFAIL,DUP,SUPPLEMENTARY.",
            "Minimum base and mapping quality are both zero.",
            "Deletions remain excluded and mate-overlap removal remains disabled.",
            "This validates comparable coverage primitives and is not a Picard compatibility claim.",
        ],
    }
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(json.dumps(report, sort_keys=True))


if __name__ == "__main__":
    main()
