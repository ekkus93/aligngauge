#!/usr/bin/env python3
"""Compare AlignGauge's supported Samtools stats SN/IS projection exactly."""

from __future__ import annotations

import json
import sys
from pathlib import Path

SCHEMA = "aligngauge-samtools-stats-differential-v1"
UNSUPPORTED_PREFIXES = {
    "CHK", "FFQ", "LFQ", "GCF", "GCL", "GCC", "GCT", "FBC", "FTC",
    "LBC", "LTC", "MPC", "RL", "FRL", "LRL", "MAPQ", "ID", "IC",
    "COV", "GCD", "RFS",
}


def fail(message: str) -> None:
    raise SystemExit(message)


def parse(path: Path, *, aligngauge: bool) -> tuple[list[tuple[str, str]], list[tuple[str, ...]]]:
    if not path.is_file():
        fail(f"missing stats text: {path}")
    sn: list[tuple[str, str]] = []
    is_rows: list[tuple[str, ...]] = []
    for number, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not raw or raw.startswith("#"):
            continue
        fields = raw.split("\t")
        prefix = fields[0]
        if aligngauge and prefix in UNSUPPORTED_PREFIXES:
            fail(f"AlignGauge emitted unsupported section {prefix} at line {number}")
        if prefix == "SN":
            if len(fields) < 3:
                fail(f"malformed SN row at {path}:{number}")
            sn.append((fields[1], fields[2]))
        elif prefix == "IS":
            if len(fields) < 6:
                fail(f"malformed IS row at {path}:{number}")
            is_rows.append(tuple(fields[1:6]))
        elif aligngauge:
            fail(f"AlignGauge emitted unsupported non-comment row {prefix!r} at line {number}")
    if not sn:
        fail(f"missing SN rows in {path}")
    return sn, is_rows


def compare(reference: Path, actual: Path, report_path: Path) -> None:
    ref_sn, ref_is = parse(reference, aligngauge=False)
    actual_sn, actual_is = parse(actual, aligngauge=True)
    discrepancies: list[dict[str, object]] = []

    if ref_sn != actual_sn:
        max_len = max(len(ref_sn), len(actual_sn))
        for index in range(max_len):
            expected = ref_sn[index] if index < len(ref_sn) else None
            observed = actual_sn[index] if index < len(actual_sn) else None
            if expected != observed:
                discrepancies.append(
                    {"section": "SN", "index": index, "expected": expected, "observed": observed}
                )

    if ref_is != actual_is:
        max_len = max(len(ref_is), len(actual_is))
        for index in range(max_len):
            expected = ref_is[index] if index < len(ref_is) else None
            observed = actual_is[index] if index < len(actual_is) else None
            if expected != observed:
                discrepancies.append(
                    {"section": "IS", "index": index, "expected": expected, "observed": observed}
                )

    report = {
        "schema": SCHEMA,
        "status": "exact" if not discrepancies else "mismatch",
        "compatibility_profile": "samtools-stats-1.24-multiqc-1.35",
        "samtools_version": "1.24",
        "multiqc_version": "1.35",
        "sn_rows": len(actual_sn),
        "is_rows": len(actual_is),
        "tolerance": None,
        "discrepancies": discrepancies,
    }
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if discrepancies:
        fail(f"Samtools stats differential mismatch; see {report_path}")


def main() -> None:
    if len(sys.argv) != 4:
        fail(f"usage: {sys.argv[0]} <samtools.txt> <aligngauge.txt> <report.json>")
    compare(Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3]))


if __name__ == "__main__":
    main()
