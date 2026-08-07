#!/usr/bin/env python3
from pathlib import Path

path = Path("docs/DNA_QC_ENGINE_TODO.md")
text = path.read_text()
old = '''## Milestone 10 — Samtools stats subsets

**Status:** Active — profile frozen by ADR-0007: Samtools 1.24 + MultiQC 1.35; exact ordinary `SN` + `IS` only.

- [x] Select exact sections consumed by target MultiQC versions.
- [x] Pin Samtools 1.24 and MultiQC 1.35.
- [x] Define every supported metric/filter and the unsupported-section boundary in SPEC §12.8 and ADR-0007.
- [ ] Implement canonical accumulators.
- [ ] Derive compatibility text.
- [ ] Differential fixtures.
- [ ] HG002 subset validation.
- [ ] Document unsupported sections.
'''
new = '''## Milestone 10 — Samtools stats subsets

**Status:** Implementation and evidence complete — pre-evidence validated head `13ec94f52cd99ed95cb0ee6a1e29103e7c9a2065`; Permanent CI run `31213026956`, job `92980046645`; Full Runtime Validation run `31213027456`, job `92980063525`; Reference Validation run `31213027019`, job `92980047514`; Targeted Validation run `31213027345`, job `92980064084`; Samtools Stats Validation run `31213026957`, job `92980062520`; all successful. Evidence: `docs/evidence/M10_SAMTOOLS_STATS_MULTIQC.md`. Final Milestone 10 acceptance remains gated on the exact evidence candidate and merged-master validation; this milestone does not publish `v0.4.0`.

- [x] Select exact sections consumed by target MultiQC versions.
- [x] Pin Samtools 1.24 and MultiQC 1.35.
- [x] Define every supported metric/filter and the unsupported-section boundary in SPEC §12.8 and ADR-0007.
- [x] Implement canonical accumulators.
- [x] Derive compatibility text.
- [x] Differential fixtures.
- [x] HG002 subset validation.
- [x] Document unsupported sections.
- [x] Create `docs/evidence/M10_SAMTOOLS_STATS_MULTIQC.md`.

### Milestone 10 acceptance gate

- [x] The claimed ordinary 39-row `SN` and complete default `IS` surfaces match pinned Samtools 1.24 exactly with no tolerance.
- [x] Pinned MultiQC 1.35 produces byte-identical Samtools-stats and insert-size data from the Samtools and AlignGauge texts.
- [x] Unsupported `samtools stats` sections are explicitly documented and absent from the compatibility renderer.
- [ ] Permanent CI succeeds on the exact evidence commit and the exact merged `master` commit is validated.
'''
count = text.count(old)
if count != 1:
    raise SystemExit(f"expected one Milestone 10 block, found {count}")
path.write_text(text.replace(old, new, 1))
