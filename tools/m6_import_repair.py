#!/usr/bin/env python3
from pathlib import Path

replacements = {
    Path("crates/aligngauge-cli/src/lib.rs"): (
        "MetricDefinition, OutputBundle, Provenance, ResolvedConfig, Summary, SystemInfo, Warning,\n",
        "MetricDefinition, OutputBundle, Provenance, ResolvedConfig, Summary, SystemInfo, ToJson, Warning,\n",
    ),
    Path("crates/aligngauge-cli/src/main.rs"): (
        "ProcessEnvironment, resolve_config,\n",
        "ProcessEnvironment, ToJson, resolve_config,\n",
    ),
}

for path, (old, new) in replacements.items():
    text = path.read_text(encoding="utf-8")
    if text.count(old) != 1:
        raise SystemExit(f"unexpected import shape in {path}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")
