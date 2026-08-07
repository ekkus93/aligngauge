#!/usr/bin/env python3
"""Check coverage RSS against the pre-traversal memory plan.

GNU time reports process RSS separately from allocator/resource planning. CI therefore permits a
fixed 64 MiB measurement tolerance for runner/runtime accounting. The product plan itself must
remain below its configured hard memory limit without that tolerance.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

TOLERANCE_BYTES = 64 * 1024 * 1024


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("usage: check_coverage_rss.py GNU_TIME.txt COVERAGE_PROBE.json")
    time_text = Path(sys.argv[1]).read_text(encoding="utf-8")
    match = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", time_text)
    if match is None:
        raise SystemExit("GNU time output is missing maximum RSS")
    observed_bytes = int(match.group(1)) * 1024

    probe = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
    plan = probe["memory_plan"]
    planned_peak = int(plan["planned_peak_bytes"])
    memory_limit = int(plan["memory_limit_bytes"])
    if planned_peak > memory_limit:
        raise SystemExit(
            f"coverage plan exceeds hard limit: planned={planned_peak}, limit={memory_limit}"
        )
    allowed_rss = planned_peak + TOLERANCE_BYTES
    if observed_bytes > allowed_rss:
        raise SystemExit(
            "coverage RSS exceeds planned peak plus documented tolerance: "
            f"rss={observed_bytes}, planned={planned_peak}, tolerance={TOLERANCE_BYTES}"
        )
    print(
        "coverage RSS within plan: "
        f"rss={observed_bytes} planned={planned_peak} tolerance={TOLERANCE_BYTES}"
    )


if __name__ == "__main__":
    main()
