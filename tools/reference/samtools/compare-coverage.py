#!/usr/bin/env python3
"""Require field-exact equality between Samtools depth reductions and AlignGauge M5."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any


def differences(expected: Any, observed: Any, path: str = "$") -> list[str]:
    if type(expected) is not type(observed):
        return [f"{path}: type {type(expected).__name__} != {type(observed).__name__}"]
    if isinstance(expected, dict):
        output: list[str] = []
        expected_keys = set(expected)
        observed_keys = set(observed)
        for key in sorted(expected_keys - observed_keys):
            output.append(f"{path}.{key}: missing from AlignGauge")
        for key in sorted(observed_keys - expected_keys):
            output.append(f"{path}.{key}: unexpected AlignGauge field")
        for key in sorted(expected_keys & observed_keys):
            output.extend(differences(expected[key], observed[key], f"{path}.{key}"))
        return output
    if isinstance(expected, list):
        if len(expected) != len(observed):
            return [f"{path}: length {len(expected)} != {len(observed)}"]
        output = []
        for index, (left, right) in enumerate(zip(expected, observed, strict=True)):
            output.extend(differences(left, right, f"{path}[{index}]"))
        return output
    if expected != observed:
        return [f"{path}: expected {expected!r}, observed {observed!r}"]
    return []


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("usage: compare-coverage.py EXPECTED.json ALIGNGAUGE-PROBE.json")
    expected = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    probe = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
    if set(probe) != {"coverage", "memory_plan", "strategy"}:
        raise SystemExit(f"unexpected coverage probe top-level fields: {sorted(probe)}")
    observed = probe["coverage"]
    discrepancy = differences(expected, observed)
    if discrepancy:
        print("coverage discrepancy detected:", file=sys.stderr)
        for item in discrepancy:
            print(f"- {item}", file=sys.stderr)
        raise SystemExit(1)
    print("coverage comparison: exact match")


if __name__ == "__main__":
    main()
