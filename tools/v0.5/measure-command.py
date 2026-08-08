#!/usr/bin/env python3
"""Run one command and capture Linux process/resource evidence as JSON."""

from __future__ import annotations

import argparse
import json
import os
import resource
import subprocess
import sys
import time
from pathlib import Path


def read_proc_io(pid: int) -> dict[str, int]:
    values: dict[str, int] = {}
    try:
        text = Path(f"/proc/{pid}/io").read_text(encoding="utf-8")
    except (FileNotFoundError, PermissionError, ProcessLookupError):
        return values
    for line in text.splitlines():
        key, separator, value = line.partition(":")
        if not separator:
            continue
        try:
            values[key.strip()] = int(value.strip())
        except ValueError:
            continue
    return values


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = list(args.command)
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        parser.error("a command is required after --")

    args.output.parent.mkdir(parents=True, exist_ok=True)
    started_wall = time.time_ns()
    started_monotonic = time.monotonic_ns()
    process = subprocess.Popen(command)
    last_io: dict[str, int] = {}
    while process.poll() is None:
        snapshot = read_proc_io(process.pid)
        if snapshot:
            last_io = snapshot
        time.sleep(0.05)
    snapshot = read_proc_io(process.pid)
    if snapshot:
        last_io = snapshot
    return_code = process.wait()
    ended_monotonic = time.monotonic_ns()
    ended_wall = time.time_ns()
    usage = resource.getrusage(resource.RUSAGE_CHILDREN)

    report = {
        "schema": "aligngauge-command-measurement-v1",
        "command": command,
        "exit_status": return_code,
        "started_unix_ns": started_wall,
        "ended_unix_ns": ended_wall,
        "wall_seconds": (ended_monotonic - started_monotonic) / 1_000_000_000,
        "user_cpu_seconds": usage.ru_utime,
        "system_cpu_seconds": usage.ru_stime,
        "peak_rss_kib": usage.ru_maxrss,
        "linux_proc_io": {
            "logical_read_bytes": last_io.get("rchar"),
            "logical_write_bytes": last_io.get("wchar"),
            "physical_read_bytes": last_io.get("read_bytes"),
            "physical_write_bytes": last_io.get("write_bytes"),
            "read_syscalls": last_io.get("syscr"),
            "write_syscalls": last_io.get("syscw"),
        },
    }
    args.output.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return return_code


if __name__ == "__main__":
    sys.exit(main())
