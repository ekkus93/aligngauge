from pathlib import Path

MERGE_SHA = "b5ec36f05110a458fbc70a1b38debeefa2a195cd"
EVIDENCE_SHA = "ad212b839d3054aae4c1206c5c451f4c6b098b2d"


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:120]!r}")
    p.write_text(text.replace(old, new, 1))

replace_once(
    "docs/DNA_QC_ENGINE_TODO.md",
    "**Status:** Ralph Loop active — `v0.3.0` released; Milestone 11 active",
    "**Status:** Ralph Loop active — `v0.3.0` released; Milestone 11 complete; Milestone 12 next",
)
replace_once(
    "docs/DNA_QC_ENGINE_TODO.md",
    "**Status:** Implementation and differential evidence complete on pre-evidence source SHA `46b8a330cc26fd2b0f472bcc72322c01fd15311f`; final clean evidence-commit acceptance pending exact-SHA CI. Evidence: `docs/evidence/M11_PICARD_ALIGNMENT_INSERT_SIZE.md`. Milestone 11 does not publish `v0.4.0`.",
    "**Status:** Complete — clean evidence candidate `ad212b839d3054aae4c1206c5c451f4c6b098b2d` passed all six PR gates, including Permanent CI `31231342595` / `93035556380` and Picard Validation `31231342594` / `93035556665`. PR #5 merged as `b5ec36f05110a458fbc70a1b38debeefa2a195cd`; that exact merged `master` passed all seven push gates. Evidence: `docs/evidence/M11_PICARD_ALIGNMENT_INSERT_SIZE.md`. Milestone 11 does not publish `v0.4.0`; Milestone 12 is next.",
)
replace_once(
    "docs/DNA_QC_ENGINE_TODO.md",
    "- [ ] Permanent CI succeeds on the exact clean evidence commit.",
    "- [x] Permanent CI succeeds on the exact clean evidence commit.",
)

readme = Path("README.md")
text = readme.read_text()
anchor = "- v0.4+ compatibility expansion and full-scale production qualification are not part of the v0.3 release boundary.\n"
replacement = (
    "- Milestone 10 is accepted: the pinned Samtools 1.24 `SN`/`IS` subset is exact and pinned MultiQC 1.35 consumes the generated surface equivalently.\n"
    "- Milestone 11 is accepted: the pinned Picard 3.4.0 reference-independent alignment-summary subset and default `ALL_READS` insert-size profile match deterministic fixtures and HG002 exactly with no tolerance. Reference-dependent Picard alignment-summary fields remain unsupported rather than zero-filled.\n"
    "- Milestone 12 — Picard WGS/hybrid-selection and MultiQC validation — is next. `v0.4.0` has not been released.\n"
    + anchor
)
if text.count(anchor) != 1:
    raise SystemExit("README v0.4 boundary anchor drifted")
readme.write_text(text.replace(anchor, replacement, 1))

# Append merge-side proof exactly once.
evidence = Path("docs/evidence/M11_PICARD_ALIGNMENT_INSERT_SIZE.md")
text = evidence.read_text()
old_acceptance = """## Acceptance state

The implementation, direct Picard differential, edge fixtures, HG002 differential, and pre-evidence regression matrix are complete. The final Milestone 11 acceptance checkbox must be closed only after the clean commit containing this evidence and the TODO/README reconciliation passes the required exact-SHA permanent gates.
"""
new_acceptance = f"""## Clean evidence candidate acceptance

The clean evidence candidate `{EVIDENCE_SHA}` passed the complete six-gate PR matrix before merge:

| Gate | Run | Job | Result |
| --- | ---: | ---: | --- |
| Permanent CI | `31231342595` | `93035556380` | success |
| Full Runtime Validation | `31231342597` | `93035556543` | success |
| Reference Validation | `31231342616` | `93035556657` | success |
| Targeted Validation | `31231342605` | `93035556643` | success |
| Samtools Stats Validation | `31231342607` | `93035556622` | success |
| Picard Validation | `31231342594` | `93035556665` | success |

This exact SHA therefore satisfies the Milestone 11 evidence-commit acceptance gate.

## Exact merged-master validation

PR #5 merged only from the validated evidence head. The exact merge commit is `{MERGE_SHA}`.

All seven push workflows attached to that exact merge commit completed successfully:

| Gate | Run | Job | Result |
| --- | ---: | ---: | --- |
| Permanent CI | `31231472869` | `93036022316` | success |
| Full Runtime Validation | `31231472841` | `93036022164` | success |
| Reference Validation | `31231472844` | `93036022206` | success |
| Targeted Validation | `31231472845` | `93036022139` | success |
| Samtools Stats Validation | `31231472859` | `93036022163` | success |
| Picard Validation | `31231472871` | `93036022279` | success |
| HG002 Preparation Validation | `31231472840` | `93036022356` | success |

The merge-side `ci/picard` job again passed exact synthetic alignment-summary and insert-size comparisons, deterministic HG002 preparation, exact HG002 alignment-summary and insert-size differentials, unsupported-column exclusion, and evidence upload.

## Acceptance state

Milestone 11 is complete. The implementation, direct Picard differential, adversarial fixtures, exact HG002 differentials, clean evidence candidate, and exact merged `master` commit all satisfy the required fail-closed validation boundary. Milestone 12 is next. Milestone 11 does not publish or imply a `v0.4.0` release.
"""
if text.count(old_acceptance) != 1:
    raise SystemExit("M11 evidence acceptance block drifted")
evidence.write_text(text.replace(old_acceptance, new_acceptance, 1))
