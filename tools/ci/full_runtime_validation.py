#!/usr/bin/env python3
"""End-to-end runtime validation for the released AlignGauge CLI surface."""

from __future__ import annotations

import csv
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

FORMATS: tuple[str | None, ...] = (
    None,
    "human",
    "json",
    "samtools-flagstat",
    "samtools-idxstats",
)


def fail(message: str) -> None:
    raise SystemExit(message)


def run(binary: Path, arguments: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(binary), *arguments],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )


def qc_args(path: Path, output_format: str | None) -> list[str]:
    arguments = ["qc", "--input", str(path)]
    if output_format is not None:
        arguments.extend(["--format", output_format])
    return arguments


def require_success(
    binary: Path,
    arguments: list[str],
    *,
    label: str,
) -> subprocess.CompletedProcess[str]:
    result = run(binary, arguments)
    if result.returncode != 0:
        fail(
            f"{label}: expected success, got exit {result.returncode}\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )
    if result.stderr:
        fail(f"{label}: successful command emitted stderr:\n{result.stderr}")
    return result


def require_failure(
    binary: Path,
    arguments: list[str],
    *,
    category: str,
    label: str,
) -> subprocess.CompletedProcess[str]:
    result = run(binary, arguments)
    if result.returncode == 0:
        fail(f"{label}: expected failure but command succeeded:\n{result.stdout}")
    if result.stdout:
        fail(
            f"{label}: failed command emitted plausible stdout; expected none:\n"
            f"{result.stdout}"
        )
    marker = f"[{category}]"
    if marker not in result.stderr:
        fail(
            f"{label}: expected error category {marker}, got:\n{result.stderr}"
        )
    return result


def validate_legacy(text: str, label: str) -> None:
    rows = [line.split("\t", 1) for line in text.splitlines()]
    if any(len(row) != 2 for row in rows):
        fail(f"{label}: malformed legacy output: {text!r}")
    values = {key: value for key, value in rows}
    if set(values) != {"total", "mapped", "unmapped"}:
        fail(f"{label}: unexpected legacy keys: {sorted(values)}")
    try:
        total = int(values["total"])
        mapped = int(values["mapped"])
        unmapped = int(values["unmapped"])
    except ValueError as error:
        fail(f"{label}: legacy output contains non-integer counters: {error}")
    if min(total, mapped, unmapped) < 0:
        fail(f"{label}: legacy output contains a negative counter")
    if total != mapped + unmapped:
        fail(
            f"{label}: total invariant failed: {total} != {mapped} + {unmapped}"
        )


def validate_json(text: str, label: str) -> None:
    try:
        value = json.loads(text)
    except json.JSONDecodeError as error:
        fail(f"{label}: JSON output is invalid: {error}")
    if not isinstance(value, dict):
        fail(f"{label}: JSON output must be an object")
    if "schema_version" not in value:
        fail(f"{label}: JSON output is missing schema_version")
    if "alignment" not in value:
        fail(f"{label}: JSON output is missing alignment")


def validate_idxstats(text: str, label: str) -> None:
    lines = text.splitlines()
    if not lines:
        fail(f"{label}: idxstats output is empty")
    for line_number, line in enumerate(lines, start=1):
        columns = line.split("\t")
        if len(columns) != 4:
            fail(f"{label}: idxstats line {line_number} does not have four columns")
        if not columns[0]:
            fail(f"{label}: idxstats line {line_number} has an empty reference name")
        for column in columns[1:]:
            try:
                value = int(column)
            except ValueError as error:
                fail(f"{label}: idxstats line {line_number} has non-integer data: {error}")
            if value < 0:
                fail(f"{label}: idxstats line {line_number} has negative data")


def validate_success_output(output_format: str | None, text: str, label: str) -> None:
    if not text:
        fail(f"{label}: successful command produced empty stdout")
    if output_format is None:
        validate_legacy(text, label)
    elif output_format == "json":
        validate_json(text, label)
    elif output_format == "samtools-idxstats":
        validate_idxstats(text, label)


def validate_cli_surface(binary: Path, basic: Path) -> None:
    help_result = require_success(binary, ["--help"], label="top-level help")
    if "Usage:" not in help_result.stdout:
        fail("top-level help did not contain Usage:")

    qc_help = require_success(binary, ["qc", "--help"], label="qc help")
    if "Usage:" not in qc_help.stdout:
        fail("qc help did not contain Usage:")

    failure_cases = (
        ([], "missing subcommand"),
        (["unknown"], "unsupported subcommand"),
        (["qc"], "missing input"),
        (["qc", "--input"], "missing input value"),
        (["qc", "--input", str(basic), "--input", str(basic)], "duplicate input"),
        (["qc", "--input", str(basic), "--format"], "missing format value"),
        (["qc", "--input", str(basic), "--format", "bogus"], "unknown format"),
        (["qc", "--input", str(basic), "--bogus"], "unknown option"),
    )
    for arguments, label in failure_cases:
        require_failure(binary, arguments, category="usage", label=f"CLI {label}")

    missing = basic.parent / "definitely-does-not-exist.bam"
    for output_format in FORMATS:
        require_failure(
            binary,
            qc_args(missing, output_format),
            category="input_not_found",
            label=f"missing input format={output_format or 'legacy'}",
        )


def validate_path_handling(binary: Path, basic: Path) -> None:
    with tempfile.TemporaryDirectory(prefix="aligngauge runtime ") as temporary:
        copied = Path(temporary) / "fixture with spaces.bam"
        shutil.copyfile(basic, copied)
        first = require_success(
            binary,
            qc_args(copied, "json"),
            label="path with spaces",
        )
        validate_json(first.stdout, "path with spaces")


def load_manifest(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle, delimiter="\t"))


def validate_manifest_fixtures(binary: Path, manifest: Path) -> None:
    rows = load_manifest(manifest)
    committed = [row for row in rows if row["kind"] == "committed"]
    if not committed:
        fail("manifest contains no committed fixtures")

    valid_count = 0
    invalid_count = 0
    for row in committed:
        fixture = Path(row["path"])
        if not fixture.is_file():
            fail(f"fixture {row['id']}: file is missing: {fixture}")

        validity = row["expected_validity"]
        if validity == "valid":
            valid_count += 1
            for output_format in FORMATS:
                label = f"fixture={row['id']} format={output_format or 'legacy'}"
                first = require_success(binary, qc_args(fixture, output_format), label=label)
                second = require_success(
                    binary,
                    qc_args(fixture, output_format),
                    label=f"{label} repeat",
                )
                if first.stdout != second.stdout:
                    fail(f"{label}: output is not deterministic across repeated runs")
                validate_success_output(output_format, first.stdout, label)
        elif validity == "error":
            invalid_count += 1
            expected_error = row["expected_error"]
            if not expected_error or expected_error == "-":
                fail(f"fixture {row['id']}: error fixture lacks expected_error")
            for output_format in FORMATS:
                label = f"fixture={row['id']} format={output_format or 'legacy'}"
                first = require_failure(
                    binary,
                    qc_args(fixture, output_format),
                    category=expected_error,
                    label=label,
                )
                second = require_failure(
                    binary,
                    qc_args(fixture, output_format),
                    category=expected_error,
                    label=f"{label} repeat",
                )
                if first.stderr != second.stderr or first.returncode != second.returncode:
                    fail(f"{label}: failure behavior is not deterministic")
        else:
            fail(f"fixture {row['id']}: unsupported expected_validity={validity!r}")

    if valid_count < 1 or invalid_count < 1:
        fail(
            f"runtime corpus must contain valid and invalid cases; "
            f"got valid={valid_count}, invalid={invalid_count}"
        )
    print(
        f"runtime validation complete: {valid_count} valid fixtures, "
        f"{invalid_count} invalid fixtures, {len(FORMATS)} output modes"
    )


def main() -> None:
    if len(sys.argv) != 3:
        fail("usage: full_runtime_validation.py <aligngauge-binary> <manifest.tsv>")
    binary = Path(sys.argv[1]).resolve()
    manifest = Path(sys.argv[2]).resolve()
    if not binary.is_file():
        fail(f"AlignGauge binary does not exist: {binary}")
    if not manifest.is_file():
        fail(f"manifest does not exist: {manifest}")

    rows = load_manifest(manifest)
    basic_rows = [row for row in rows if row["id"] == "basic"]
    if len(basic_rows) != 1:
        fail("manifest must contain exactly one basic fixture")
    basic = Path(basic_rows[0]["path"]).resolve()

    validate_cli_surface(binary, basic)
    validate_path_handling(binary, basic)
    validate_manifest_fixtures(binary, manifest)


if __name__ == "__main__":
    main()
