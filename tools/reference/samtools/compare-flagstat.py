#!/usr/bin/env python3
# Compare integer fields from two Samtools-compatible flagstat reports.

from pathlib import Path
import re
import sys

LINE = re.compile(r"^(\d+) \+ (\d+) (.+)$")
PERCENT = re.compile(r" \((?:N/A|\d+\.\d+%) : (?:N/A|\d+\.\d+%)\)$")


def parse(path: Path) -> dict[str, tuple[int, int]]:
    values: dict[str, tuple[int, int]] = {}
    for raw in path.read_text(encoding="utf-8").splitlines():
        match = LINE.match(raw)
        if match is None:
            raise SystemExit(f"{path}: unsupported flagstat line: {raw!r}")
        label = PERCENT.sub("", match.group(3))
        if label in values:
            raise SystemExit(f"{path}: duplicate flagstat label: {label}")
        values[label] = (int(match.group(1)), int(match.group(2)))
    return values


def main() -> int:
    if len(sys.argv) != 3:
        raise SystemExit(f"usage: {sys.argv[0]} EXPECTED ACTUAL")
    expected = parse(Path(sys.argv[1]))
    actual = parse(Path(sys.argv[2]))
    if expected != actual:
        labels = sorted(set(expected) | set(actual))
        for label in labels:
            if expected.get(label) != actual.get(label):
                print(
                    f"{label}: expected={expected.get(label)} actual={actual.get(label)}",
                    file=sys.stderr,
                )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
