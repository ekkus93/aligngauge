#!/usr/bin/env python3
"""Collect a reproducible v0.1 performance baseline without making speedup claims."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import statistics
import subprocess
import tempfile
import time
from pathlib import Path

MODES = ("reader", "counters", "coverage", "combined")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--probe", type=Path, required=True)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--memory-limit-bytes", type=int, required=True)
    parser.add_argument("--repetitions", type=int, default=3)
    args = parser.parse_args()
    if args.memory_limit_bytes <= 0:
        parser.error("--memory-limit-bytes must be greater than zero")
    if args.repetitions < 3:
        parser.error("--repetitions must be at least 3")
    return args


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def command_output(command: list[str]) -> str:
    completed = subprocess.run(command, check=True, capture_output=True, text=True)
    return completed.stdout.strip()


def parse_probe_output(text: str) -> dict[str, int | str]:
    values: dict[str, int | str] = {}
    for token in text.strip().split():
        if "=" not in token:
            raise RuntimeError(f"unexpected benchmark probe token: {token!r}")
        key, value = token.split("=", 1)
        if key in values:
            raise RuntimeError(f"duplicate benchmark probe key: {key}")
        values[key] = value if key == "mode" else int(value)
    if "mode" not in values:
        raise RuntimeError("benchmark probe did not report mode")
    return values


def parse_time_file(path: Path) -> dict[str, float | int]:
    values: dict[str, float | int] = {}
    for token in path.read_text(encoding="utf-8").strip().split():
        key, raw = token.split("=", 1)
        if key == "max_rss_kib":
            values[key] = int(raw)
        else:
            values[key] = float(raw)
    required = {"elapsed_seconds", "user_seconds", "system_seconds", "max_rss_kib"}
    if set(values) != required:
        raise RuntimeError(f"incomplete GNU time output: {values}")
    return values


def run_probe(
    probe: Path,
    mode: str,
    input_path: Path,
    memory_limit_bytes: int,
) -> tuple[dict[str, int | str], dict[str, float | int]]:
    with tempfile.TemporaryDirectory(prefix="aligngauge-v01-time-") as directory:
        timing_path = Path(directory) / "timing.txt"
        command = [
            "/usr/bin/time",
            "-f",
            "elapsed_seconds=%e user_seconds=%U system_seconds=%S max_rss_kib=%M",
            "-o",
            str(timing_path),
            str(probe),
            mode,
            str(input_path),
            str(memory_limit_bytes),
        ]
        started = time.perf_counter_ns()
        completed = subprocess.run(command, capture_output=True, text=True)
        wall_ns = time.perf_counter_ns() - started
        if completed.returncode != 0:
            raise RuntimeError(
                f"benchmark mode {mode} failed with {completed.returncode}: {completed.stderr.strip()}"
            )
        semantics = parse_probe_output(completed.stdout)
        if semantics.get("mode") != mode:
            raise RuntimeError(f"benchmark mode mismatch: expected {mode}, got {semantics}")
        timing = parse_time_file(timing_path)
        timing["controller_wall_seconds"] = wall_ns / 1_000_000_000
        return semantics, timing


def summarize(measurements: list[dict[str, float | int]]) -> dict[str, dict[str, float | int]]:
    summary: dict[str, dict[str, float | int]] = {}
    for key in (
        "elapsed_seconds",
        "controller_wall_seconds",
        "user_seconds",
        "system_seconds",
        "max_rss_kib",
    ):
        values = [measurement[key] for measurement in measurements]
        summary[key] = {
            "min": min(values),
            "median": statistics.median(values),
            "max": max(values),
        }
    return summary


def first_cpu_model() -> str:
    cpuinfo = Path("/proc/cpuinfo")
    if not cpuinfo.exists():
        return "unavailable"
    for line in cpuinfo.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.lower().startswith("model name") and ":" in line:
            return line.split(":", 1)[1].strip()
    return "unavailable"


def memory_total_kib() -> int | None:
    meminfo = Path("/proc/meminfo")
    if not meminfo.exists():
        return None
    for line in meminfo.read_text(encoding="utf-8").splitlines():
        if line.startswith("MemTotal:"):
            return int(line.split()[1])
    return None


def validate_semantics(outputs: dict[str, dict[str, int | str]]) -> None:
    reader_records = outputs["reader"].get("records")
    counter_records = outputs["counters"].get("records")
    combined_records = outputs["combined"].get("records")
    if not (reader_records == counter_records == combined_records):
        raise RuntimeError(
            "record totals differ across reader/counters/combined modes: "
            f"{reader_records}, {counter_records}, {combined_records}"
        )

    coverage_bases = outputs["coverage"].get("accepted_bases")
    combined_bases = outputs["combined"].get("accepted_bases")
    if coverage_bases != combined_bases:
        raise RuntimeError(
            "accepted coverage bases differ between coverage and combined modes: "
            f"{coverage_bases} != {combined_bases}"
        )
    if outputs["combined"].get("input_traversals") != 1:
        raise RuntimeError("combined mode did not report exactly one BAM traversal")


def main() -> int:
    args = parse_args()
    probe = args.probe.resolve()
    input_path = args.input.resolve()
    if not probe.is_file():
        raise SystemExit(f"probe does not exist: {probe}")
    if not input_path.is_file():
        raise SystemExit(f"input does not exist: {input_path}")

    mode_results: dict[str, dict[str, object]] = {}
    semantic_outputs: dict[str, dict[str, int | str]] = {}

    for mode in MODES:
        warmup_semantics, _ = run_probe(probe, mode, input_path, args.memory_limit_bytes)
        measurements: list[dict[str, float | int]] = []
        measured_semantics: dict[str, int | str] | None = None
        for _ in range(args.repetitions):
            semantics, timing = run_probe(probe, mode, input_path, args.memory_limit_bytes)
            if measured_semantics is None:
                measured_semantics = semantics
            elif semantics != measured_semantics:
                raise RuntimeError(
                    f"semantic output changed between measured {mode} runs: "
                    f"{measured_semantics} != {semantics}"
                )
            measurements.append(timing)
        if measured_semantics is None or warmup_semantics != measured_semantics:
            raise RuntimeError(f"warmup and measured semantics differ for {mode}")
        semantic_outputs[mode] = measured_semantics
        mode_results[mode] = {
            "semantic_output": measured_semantics,
            "measurements": measurements,
            "summary": summarize(measurements),
        }

    validate_semantics(semantic_outputs)

    result = {
        "schema": "aligngauge-v0.1-performance-baseline-v1",
        "commit_sha": command_output(["git", "rev-parse", "HEAD"]),
        "input": {
            "path": str(input_path),
            "size_bytes": input_path.stat().st_size,
            "sha256": sha256_file(input_path),
        },
        "benchmark": {
            "probe": str(probe),
            "memory_limit_bytes": args.memory_limit_bytes,
            "warmups_per_mode": 1,
            "measured_repetitions_per_mode": args.repetitions,
            "cache_state": (
                "warm-cache after one warmup per mode; cold cache not measured because "
                "the hosted runner does not grant cache-drop privilege"
            ),
            "timing_scope": "process invocation including process startup",
            "modes": mode_results,
        },
        "environment": {
            "platform": platform.platform(),
            "kernel": platform.release(),
            "machine": platform.machine(),
            "cpu_model": first_cpu_model(),
            "logical_cpus": os.cpu_count(),
            "memory_total_kib": memory_total_kib(),
            "storage": command_output(["df", "-T", "-k", str(input_path.parent)]),
            "rustc": command_output(["rustc", "--version", "--verbose"]),
            "cargo": command_output(["cargo", "--version"]),
            "runner_image_os": os.environ.get("ImageOS"),
            "runner_image_version": os.environ.get("ImageVersion"),
        },
        "interpretation": {
            "speedup_claim": None,
            "note": (
                "This is a baseline measurement for variance and regression context. "
                "No performance superiority or speedup is claimed."
            ),
        },
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
