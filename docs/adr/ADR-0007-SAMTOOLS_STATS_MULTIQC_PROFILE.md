# ADR-0007 — Samtools stats / MultiQC compatibility profile

**Status:** Accepted for Milestone 10  
**Date:** 2026-08-07  
**Decision owners:** AlignGauge Milestone 10

## Context

Milestone 10 begins the v0.4 ecosystem-compatibility work. The roadmap calls for selected `samtools stats` compatibility and MultiQC parser validation, but neither the exact Samtools surface nor the target MultiQC version was previously frozen.

Implementing every section emitted by `samtools stats` would add sequence-content, quality-cycle, indel, coverage, reference-GC, barcode, and checksum machinery that the selected MultiQC Samtools module does not consume. That would increase correctness surface and runtime cost without serving the Milestone 10 compatibility goal.

The existing AlignGauge rules require named versions, exact metric definitions, canonical state before compatibility rendering, no silent fallback, and differential evidence before a compatibility label is used.

## Decision

Milestone 10 pins:

- **Samtools 1.24** as the reference implementation;
- **MultiQC 1.35** as the parser/consumer target;
- compatibility profile name **`samtools-stats-1.24-multiqc-1.35`**.

The supported `samtools stats` surface is exactly:

1. the ordinary whole-input **`SN` Summary Numbers** section; and
2. the **`IS` Insert Sizes** section required by MultiQC's insert-size plot.

MultiQC 1.35 parses every numeric `SN` row and separately consumes `IS` columns `insert size` and `pairs total`. Therefore AlignGauge shall implement the complete ordinary, non-target `SN` section emitted by the pinned default profile rather than only the subset presently visible in MultiQC's General Stats table.

### Reference invocation profile

The differential oracle is equivalent to:

```text
samtools stats <INPUT>
```

with Samtools 1.24 defaults relevant to the supported sections:

- required flag mask: `0`;
- filter flag mask: `0`;
- no read-length filter;
- no read-group subset;
- no BED/region target mode;
- trim quality: `0`, therefore `bases trimmed = 0`;
- overlap removal disabled;
- maximum insert size: `8000`;
- main insert bulk fraction: `0.99`;
- ordinary non-sparse insert-size rendering.

The reference command is executed in the already pinned Samtools 1.24 container under network isolation.

### Canonical-first implementation

The authoritative implementation is a typed `SamtoolsStatsReport` accumulated from validated records with checked arithmetic. Compatibility text is rendered from that report. The text renderer is not an independent accumulator.

Milestone 10 does **not** silently enable this additional work for ordinary QC runs. The collector is enabled only by an explicitly selected Samtools-stats compatibility path or by tests/API calls that explicitly request it. This avoids changing the cost of existing v0.1-v0.3 release behavior merely because the compatibility implementation exists.

The public compatibility probe adds:

```text
aligngauge qc --input <BAM> --format samtools-stats
```

This probe remains BAM-only, matching the existing `samtools-flagstat` and `samtools-idxstats` probe boundary. A future unified v0.4 release-output/profile surface may expose compatibility generation for BAM and CRAM after that CLI contract is separately specified.

### Exact ordinary `SN` fields

The supported renderer emits, in Samtools 1.24 order:

- raw total sequences;
- filtered sequences;
- sequences;
- is sorted;
- 1st fragments;
- last fragments;
- reads mapped;
- reads mapped and paired;
- reads unmapped;
- reads properly paired;
- reads paired;
- reads duplicated;
- reads MQ0;
- reads QC failed;
- non-primary alignments;
- supplementary alignments;
- total length;
- total first fragment length;
- total last fragment length;
- bases mapped;
- bases mapped (cigar);
- bases trimmed;
- bases duplicated;
- mismatches;
- error rate;
- average length;
- average first fragment length;
- average last fragment length;
- maximum length;
- maximum first fragment length;
- maximum last fragment length;
- average quality;
- insert size average;
- insert size standard deviation;
- inward oriented pairs;
- outward oriented pairs;
- pairs with other orientation;
- pairs on different chromosomes;
- percentage of properly paired reads (%).

The two target-region-only `SN` rows are not part of this profile.

### Important source-level semantics

The implementation follows Samtools 1.24 source behavior where it is more precise than prose documentation:

- secondary records increment `non-primary alignments` and return before the ordinary sequence-derived counters;
- supplementary records increment `supplementary alignments` and continue, but are excluded from original-read counters;
- zero-sequence non-secondary records return before sequence-derived counters;
- duplicate read/base counters are accumulated for sequence-bearing non-secondary records, including supplementary records;
- QC-fail, mapped/unmapped, paired, MQ0, fragment counts, and total-length counters are original-record only;
- `bases mapped` is the full query sequence length for mapped original records;
- `bases mapped (cigar)` counts `M`, `I`, `=`, and `X` for mapped non-secondary records, including supplementary records;
- `mismatches` sums present `NM` values for mapped non-secondary records; an absent `NM` contributes no mismatches and is not rewritten as a record-level zero;
- error rate is mismatches divided by CIGAR-mapped bases and renders zero only when the Samtools-defined denominator is zero;
- maximum read length is Samtools' unclipped length (query sequence length plus hard clips) over sequence-bearing non-secondary records;
- average quality follows the pinned Samtools accumulation/division behavior, including its treatment of fragment-order states;
- insert-size observations require original, paired, mapped read and mapped mate;
- insert size is `abs(TLEN)` clamped to the 8000 default maximum;
- orientation is classified with the pinned Samtools read1/read2, position, strand, and mate-strand algorithm;
- before summary/`IS` output, pair observations are halved per insert-size/orientation bin exactly as Samtools 1.24 does;
- the 0.99 main-bulk rule determines the insert-size mean, standard deviation, and rendered `IS` range.

All additions and products use checked arithmetic. Any value that cannot be represented or any required planned field that is unavailable is fatal; AlignGauge does not saturate, truncate silently, or substitute an approximate algorithm.

### MultiQC-compatible header

The compatibility text begins with the pinned Samtools 1.24 header shape so MultiQC 1.35 recognizes and versions the file. A following comment identifies the content as an AlignGauge compatibility projection and names `samtools-stats-1.24-multiqc-1.35`. Canonical provenance/evidence must never imply that Samtools itself produced AlignGauge's file.

## Explicitly unsupported Samtools-stats sections

Milestone 10 does not claim compatibility for:

- `CHK` checksums;
- `FFQ` / `LFQ` quality-by-cycle tables;
- `GCF` / `GCL` read-GC histograms;
- `GCC` / `GCT`, `FBC` / `FTC`, `LBC` / `LTC` base-composition sections;
- barcode base/quality sections;
- `MPC` mismatch-per-cycle data;
- `RL` / `FRL` / `LRL` read-length histograms;
- `MAPQ` mapping-quality histogram;
- `ID` indel-length distribution;
- `IC` indels-per-cycle;
- `COV` coverage distribution;
- `GCD` GC-depth data;
- `RFS` reference-sequence statistics;
- Samtools `-t` / `-g` target-region summary extensions;
- split-by-tag output;
- custom `-f`, `-F`, `-d`, `-q`, `-i`, `-m`, `-p`, or region/filter profiles.

These sections/options require separate semantics and differential evidence before any compatibility claim.

## Validation requirements

Milestone 10 acceptance requires:

1. synthetic fixtures that exercise primary/secondary/supplementary, QC fail, duplicate, mapped/unmapped, MQ0, NM present/missing, CIGAR insertion/deletion/clipping, insert-size/orientation, and zero-denominator cases;
2. exact field-by-field comparison of the supported `SN` section against pinned Samtools 1.24;
3. exact `IS` comparison for the supported default insert-size profile;
4. an HG002 subset differential;
5. actual MultiQC 1.35 parsing of both Samtools reference text and AlignGauge compatibility text, with equal parsed Samtools-stats data for the supported fields and equal insert-size totals;
6. explicit evidence that unsupported sections are absent from the AlignGauge renderer rather than partially emitted;
7. permanent CI on the exact Milestone 10 evidence commit.

No blanket numerical tolerance is permitted. Formatting-sensitive floating values are compared to the pinned Samtools textual representation when an exact compatibility claim is made.

## Consequences

This decision keeps Milestone 10 narrowly useful to MultiQC while preserving room for later v0.4 milestones. Picard profiles, grouping dimensions, mate-overlap correction, and broader Samtools sections remain separate work. Ordinary existing QC remains unchanged unless the Samtools-stats compatibility path is explicitly selected.
