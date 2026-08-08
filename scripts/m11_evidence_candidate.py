from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:120]!r}")
    p.write_text(text.replace(old, new, 1))

# Reconcile the exact M11 TODO block without touching M12 or the v0.4 release gate.
replace_once(
    "docs/DNA_QC_ENGINE_TODO.md",
    '''## Milestone 11 — Picard alignment and insert-size profiles

- [ ] Pin Picard version.
- [ ] Define alignment-summary subset.
- [ ] Implement insert-size histogram.
- [ ] Reproduce or explicitly rename MAD trimming behavior.
- [ ] Test tie-breaking and rounding.
- [ ] Separate compatibility from “similar metric.”
- [ ] Differential fixtures for edge distributions.
- [ ] Document expected differences.
''',
    '''## Milestone 11 — Picard alignment and insert-size profiles

**Status:** Implementation and differential evidence complete on pre-evidence source SHA `46b8a330cc26fd2b0f472bcc72322c01fd15311f`; final clean evidence-commit acceptance pending exact-SHA CI. Evidence: `docs/evidence/M11_PICARD_ALIGNMENT_INSERT_SIZE.md`. Milestone 11 does not publish `v0.4.0`.

- [x] Pin Picard version.
- [x] Define alignment-summary subset.
- [x] Implement insert-size histogram.
- [x] Reproduce or explicitly rename MAD trimming behavior.
- [x] Test tie-breaking and rounding.
- [x] Separate compatibility from “similar metric.”
- [x] Differential fixtures for edge distributions.
- [x] Document expected differences.
- [x] Create `docs/evidence/M11_PICARD_ALIGNMENT_INSERT_SIZE.md`.

### Milestone 11 acceptance gate

- [x] The claimed 13-column reference-independent alignment-summary subset matches pinned Picard 3.4.0 exactly on deterministic fixtures and HG002 with no tolerance.
- [x] The default `ALL_READS` insert-size metrics and trimmed histogram match pinned Picard 3.4.0 exactly on deterministic fixtures and HG002 with no tolerance.
- [x] Reference-dependent alignment-summary fields, PDF compatibility, and non-`ALL_READS` accumulation levels are explicitly documented as unsupported/deferred rather than zero-filled or approximated.
- [ ] Permanent CI succeeds on the exact clean evidence commit.
''',
)

# Document the public compatibility probes without changing the latest released version.
replace_once(
    "crates/aligngauge-cli/README.md",
    "Milestone 10 is now accepted: the `samtools-stats` compatibility probe is validated exactly against pinned Samtools 1.24 and pinned MultiQC 1.35, with all unsupported `samtools stats` sections intentionally omitted. Milestone 11 is next.\n",
    "Milestone 10 remains accepted. Milestone 11 now adds two Picard 3.4.0 compatibility probes as an evidence candidate; `v0.3.0` remains the latest released product and no `v0.4.0` release is implied.\n",
)
replace_once(
    "crates/aligngauge-cli/README.md",
    "aligngauge qc --input <BAM> --format samtools-stats\n```\n",
    "aligngauge qc --input <BAM> --format samtools-stats\naligngauge qc --input <BAM> --format picard-alignment-summary\naligngauge qc --input <BAM> --format picard-insert-size\n```\n",
)
replace_once(
    "crates/aligngauge-cli/README.md",
    "The compatibility path remains intentionally separate from the release report path. It does not publish `summary.json`, `provenance.json`, or `run-metadata.json`.\n",
    "The compatibility path remains intentionally separate from the release report path. It does not publish `summary.json`, `provenance.json`, or `run-metadata.json`. The Picard alignment-summary profile claims only its explicitly documented 13 reference-independent columns; the insert-size profile claims the default `ALL_READS` metrics table and trimmed histogram, not the PDF chart or SAMPLE/LIBRARY/READ_GROUP breakdowns.\n",
)
