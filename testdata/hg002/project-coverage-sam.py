#!/usr/bin/env python3
"""Project the prepared HG002 chr20 slice into a small deterministic coverage reference."""

from __future__ import annotations

import sys

REFERENCE = "chr20"
OFFSET = 9_950_000
PROJECTED_LENGTH = 1_100_000


def main() -> None:
    saw_hd = False
    saw_sq = False
    for line_number, raw in enumerate(sys.stdin, start=1):
        line = raw.rstrip("\n")
        if line.startswith("@"):
            if line.startswith("@HD"):
                if not saw_hd:
                    print("@HD\tVN:1.6\tSO:coordinate")
                    saw_hd = True
                continue
            if line.startswith("@SQ"):
                fields = line.split("\t")
                names = {field.split(":", 1)[0]: field.split(":", 1)[1] for field in fields[1:] if ":" in field}
                if names.get("SN") == REFERENCE and not saw_sq:
                    print(f"@SQ\tSN:{REFERENCE}\tLN:{PROJECTED_LENGTH}")
                    saw_sq = True
                continue
            if line.startswith("@PG"):
                continue
            print(line)
            continue

        if not saw_hd or not saw_sq:
            raise SystemExit("projected SAM records appeared before required @HD/@SQ headers")
        columns = line.split("\t")
        if len(columns) < 11:
            raise SystemExit(f"SAM line {line_number} has fewer than 11 columns")
        if columns[2] != REFERENCE:
            raise SystemExit(
                f"SAM line {line_number} uses unexpected reference {columns[2]!r}"
            )
        flag = int(columns[1])
        position = int(columns[3])
        if position <= OFFSET:
            raise SystemExit(
                f"SAM line {line_number} cannot be shifted into the projected reference: {position}"
            )
        projected_position = position - OFFSET
        if projected_position > PROJECTED_LENGTH:
            raise SystemExit(
                f"SAM line {line_number} projected position exceeds reference: {projected_position}"
            )
        columns[3] = str(projected_position)

        if flag & 0x1:
            flag = (flag | 0x8) & ~0x2
        columns[1] = str(flag)
        columns[6] = "*"
        columns[7] = "0"
        columns[8] = "0"
        print("\t".join(columns))

    if not saw_hd or not saw_sq:
        raise SystemExit("input SAM did not contain the required chr20 header")


if __name__ == "__main__":
    main()
