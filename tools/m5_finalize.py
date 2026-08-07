#!/usr/bin/env python3
"""Close Milestone 5 TODO state after the evidence candidate is committed."""

from pathlib import Path

TODO = Path("docs/DNA_QC_ENGINE_TODO.md")

text = TODO.read_text(encoding="utf-8")
old_status = "**Status:** Ralph Loop active — Milestone 4 complete; Milestone 5 next"
new_status = "**Status:** Ralph Loop active — Milestone 5 complete; Milestone 6 next"
if text.count(old_status) != 1:
    raise SystemExit("unexpected top-level Ralph status while closing Milestone 5")
text = text.replace(old_status, new_status, 1)

start_marker = "## Milestone 5 — Exact chunked coverage\n"
end_marker = "\n---\n\n## Milestone 6 — v0.1 release integration"
start = text.find(start_marker)
end = text.find(end_marker, start)
if start < 0 or end < 0:
    raise SystemExit("Milestone 5 section markers were not found exactly")
section = text[start:end]
if "**Status:**" in section:
    raise SystemExit("Milestone 5 already contains a status line")
section = section.replace(
    start_marker,
    start_marker
    + "\n**Status:** Complete — implementation source SHA "
      "`27b056e5766354a63ab6a81e69cf02e8f991170b`; evidence in "
      "`docs/evidence/M5_COVERAGE.md`.\n",
    1,
)
section = section.replace("- [ ]", "- [x]")
section = section.replace(
    "Compare against ADR-0002 baseline.",
    "Compare against ADR-0003 baseline.",
    1,
)
if "- [ ]" in section:
    raise SystemExit("Milestone 5 still contains unchecked tasks")
if "ADR-0002 baseline" in section:
    raise SystemExit("stale Milestone 5 ADR reference remains")
text = text[:start] + section + text[end:]
TODO.write_text(text, encoding="utf-8")
