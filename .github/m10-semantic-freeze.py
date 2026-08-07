#!/usr/bin/env python3
from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:100]!r}")
    p.write_text(text.replace(old, new, 1))

spec = Path("docs/DNA_QC_ENGINE_SPEC.md")
text = spec.read_text()
anchor = "\n## 13. Architecture\n"
if text.count(anchor) != 1:
    raise SystemExit(f"spec architecture anchor count={text.count(anchor)}")
section = r'''
### 12.8 v0.4 Samtools stats / MultiQC subset

Milestone 10 defines the first v0.4 compatibility profile as
`samtools-stats-1.24-multiqc-1.35`. Samtools 1.24 is the reference implementation
and MultiQC 1.35 is the consumer/parser target.

The claimed surface is intentionally narrower than the complete `samtools stats`
output. AlignGauge shall implement the ordinary whole-input `SN` Summary Numbers
section and the `IS` Insert Sizes section consumed by the pinned MultiQC parser.
The complete ordinary non-target `SN` field set is supported because MultiQC stores
all numeric `SN` rows in its parsed data even when only a subset is currently
shown in General Stats or plots.

The reference profile is equivalent to default `samtools stats <INPUT>` for the
supported sections: no required/filter flag masks, no read-group or read-length
subset, no target/region mode, trim quality zero, overlap removal disabled,
maximum insert size 8000, main insert bulk 0.99, and ordinary non-sparse insert-size
rendering. Custom `samtools stats` filtering or target options are not part of the
Milestone 10 compatibility claim.

The authoritative state is a typed checked `SamtoolsStatsReport`. Compatibility
text is derived from that report and is never accumulated independently. The
collector is enabled only by an explicit Samtools-stats compatibility request;
ordinary existing QC shall not silently acquire the additional sequence-quality,
NM, or insert-size work merely because this compatibility implementation exists.
The Milestone 10 public probe is:

```text
aligngauge qc --input <BAM> --format samtools-stats
```

This compatibility probe remains BAM-only, matching the existing flagstat and
idxstats probe boundary. A unified BAM/CRAM v0.4 compatibility-output surface is a
separate CLI contract and is not silently inferred here.

The ordinary `SN` fields, their exact Samtools 1.24 source semantics, insert-size
classification/halving/main-bulk behavior, renderer header policy, and unsupported
sections are normative in ADR-0007. In particular, missing record-level `NM` is
not rewritten as a metric zero; it contributes no mismatch observation exactly as
Samtools 1.24 does. All AlignGauge accumulator arithmetic remains checked and any
unrepresentable required state is fatal.

Milestone 10 does not claim compatibility for `CHK`, quality-by-cycle, GC/base
composition, barcode, mismatch-per-cycle, read-length histogram, MAPQ histogram,
indel, coverage, GC-depth, reference-statistics, target-region, split-tag, or
custom-filter sections. Unsupported sections shall be omitted rather than emitted
partially or approximately.

Differential acceptance requires exact supported `SN` and `IS` comparison against
pinned Samtools 1.24 on synthetic fixtures and the pinned HG002 subset. It also
requires actual MultiQC 1.35 parser validation showing that Samtools reference text
and AlignGauge compatibility text produce equal parsed data for the supported
surface. No blanket tolerance is permitted.
'''
spec.write_text(text.replace(anchor, "\n" + section + anchor, 1))

replace_once(
    "docs/DNA_QC_ENGINE_TODO.md",
    "**Status:** Ralph Loop active — `v0.3.0` released; Milestone 9 complete; Milestone 10 next",
    "**Status:** Ralph Loop active — `v0.3.0` released; Milestone 9 complete; Milestone 10 active",
)
replace_once(
    "docs/DNA_QC_ENGINE_TODO.md",
    "## Milestone 10 — Samtools stats subsets\n\n- [ ] Select exact sections consumed by target MultiQC versions.",
    "## Milestone 10 — Samtools stats subsets\n\n**Status:** Active — profile frozen by ADR-0007: Samtools 1.24 + MultiQC 1.35; exact ordinary `SN` + `IS` only.\n\n- [x] Select exact sections consumed by target MultiQC versions.",
)
replace_once(
    "docs/DNA_QC_ENGINE_TODO.md",
    "- [ ] Pin Samtools.\n- [ ] Define every metric and filter.",
    "- [x] Pin Samtools 1.24 and MultiQC 1.35.\n- [x] Define every supported metric/filter and the unsupported-section boundary in SPEC §12.8 and ADR-0007.",
)
