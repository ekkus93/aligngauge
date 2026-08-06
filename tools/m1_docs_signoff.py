from pathlib import Path

TODO_PATH = Path("docs/DNA_QC_ENGINE_TODO.md")
EVIDENCE_PATH = Path("docs/evidence/M1_CORE_CONTRACTS.md")


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    return text.replace(old, new)


todo = TODO_PATH.read_text(encoding="utf-8")
todo = replace_once(
    todo,
    "**Current repository:** `ekkus93/rust-dna-sequencer`  \n**Recommended repository name:** `ekkus93/aligngauge`  ",
    "**Repository:** `ekkus93/aligngauge`\n",
    "repository header",
)
todo = replace_once(
    todo,
    "**Status:** Revised staged implementation plan  ",
    "**Status:** Ralph Loop active — Milestone 1 complete; Milestone 2 next",
    "status header",
)
start_marker = "## Milestone 1 — Core model, errors, and atomic output\n"
end_marker = "\n---\n\n## Milestone 2 — Test corpus and differential harness"
start = todo.index(start_marker)
end = todo.index(end_marker, start)
block = todo[start:end]
block = block.replace("- [ ]", "- [x]")
block = replace_once(
    block,
    "- [x] `--memory-limit` parser with checked units.",
    "- [x] Memory-limit parser with checked units; CLI exposure remains Milestone 6.",
    "memory-limit clarification",
)
block = replace_once(
    block,
    "- [x] `--preserve-failed-staging`.",
    "- [x] Preserve-failed-staging policy and resolved configuration field; CLI exposure remains Milestone 6.",
    "preserved-staging clarification",
)
block = replace_once(
    block,
    "Create `docs/evidence/M1_CORE_CONTRACTS.md`.",
    "- [x] Created `docs/evidence/M1_CORE_CONTRACTS.md`.",
    "evidence task",
)
block = replace_once(
    block,
    start_marker,
    start_marker
    + "\n**Status:** Complete — evidence SHA `ffafa45c1d6dea99c50f61e05498690d594bae27`; "
    + "Permanent CI run `31095937384`, job `92597853728`, success.\n",
    "milestone status",
)
if "- [ ]" in block:
    raise SystemExit("Milestone 1 still contains an unchecked task")
todo = todo[:start] + block + todo[end:]
TODO_PATH.write_text(todo, encoding="utf-8")

evidence = EVIDENCE_PATH.read_text(encoding="utf-8")
evidence = replace_once(
    evidence,
    "## Milestone conclusion\n\nThe Milestone 1 implementation contracts pass on the exact implementation signoff\nSHA. Milestone closure additionally requires Permanent CI to pass on the evidence\ncommit that introduces this document; that final evidence-run identity is recorded\nin the subsequent TODO signoff update.\n",
    "## Evidence-commit signoff\n\nThe evidence document was introduced by exact SHA\n`ffafa45c1d6dea99c50f61e05498690d594bae27`. Permanent CI passed on that SHA:\n\n- **Run:** `31095937384`\n- **Job:** `92597853728`\n- **Conclusion:** success\n\n## Milestone conclusion\n\nMilestone 1 is complete. The implementation and evidence commits both passed the\npermanent read-only workflow without skipped quality gates or known hidden\nfallbacks. Milestone 2 is the next implementation boundary.\n",
    "evidence conclusion",
)
EVIDENCE_PATH.write_text(evidence, encoding="utf-8")
Path(__file__).unlink()
