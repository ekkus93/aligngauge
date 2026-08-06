from pathlib import Path

path = Path("docs/DNA_QC_ENGINE_TODO.md")
text = path.read_text(encoding="utf-8")


def replace_once(old: str, new: str, label: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    text = text.replace(old, new)


replace_once(
    "**Status:** Ralph Loop active — Milestone 1 complete; Milestone 2 next",
    "**Status:** Ralph Loop active — Milestone 2 implementation in progress",
    "TODO status",
)
replace_once(
    "- [ ] Select and document the v0.1 coverage baseline in ADR-0002.",
    "- [ ] Select and document the v0.1 coverage baseline in ADR-0003.",
    "coverage ADR number",
)
replace_once(
    "Record findings in `ADR-0003-CRAM_REFERENCE_RESOLUTION.md`.",
    "Record findings in `ADR-0004-CRAM_REFERENCE_RESOLUTION.md`.",
    "CRAM ADR number",
)
replace_once(
    "## Milestone 2 — Test corpus and differential harness\n",
    "## Milestone 2 — Test corpus and differential harness\n\n**Status:** Implementation in progress.\n",
    "Milestone 2 status",
)
path.write_text(text, encoding="utf-8")
Path(__file__).unlink()
