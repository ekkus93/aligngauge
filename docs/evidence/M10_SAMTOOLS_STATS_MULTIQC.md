# Milestone 10 — Samtools Stats / MultiQC Validation Evidence

**Milestone:** 10 — Samtools stats subsets  
**Date:** 2026-08-07  
**Pre-evidence validated head:** `13ec94f52cd99ed95cb0ee6a1e29103e7c9a2065`  
**Compatibility profile:** `samtools-stats-1.24-multiqc-1.35`  
**Reference implementation:** Samtools 1.24  
**Consumer/parser target:** MultiQC 1.35  
**Disposition:** Implementation and differential validation are complete. Final Milestone 10 acceptance remains gated on Permanent CI for the exact evidence candidate and subsequent merged-master validation. Milestone 10 does not publish or imply a `v0.4.0` release.

## Scope

Milestone 10 implements the first v0.4 ecosystem-compatibility profile without attempting to reproduce all of `samtools stats`.

The public compatibility probe is:

```text
aligngauge qc --input <BAM> --format samtools-stats
```

The claimed surface is exactly:

1. the ordinary whole-input `SN` Summary Numbers section emitted by Samtools 1.24 under its default whole-input profile; and
2. the `IS` Insert Sizes section consumed by MultiQC 1.35.

MultiQC 1.35 stores every numeric `SN` row and separately consumes `IS` rows for the insert-size plot. AlignGauge therefore emits all 39 ordinary non-target `SN` rows, not merely the subset currently displayed in MultiQC General Stats.

The compatibility probe is BAM-only in Milestone 10, matching the existing `samtools-flagstat` and `samtools-idxstats` compatibility-probe boundary. A future unified v0.4 BAM/CRAM compatibility-output surface is separately specified work.

## Semantic freeze

SPEC §12.8 and ADR-0007 freeze the profile before implementation.

### Reference profile

The supported differential profile is equivalent to:

```text
samtools stats <INPUT>
```

with the pinned Samtools 1.24 defaults relevant to `SN` and `IS`:

- required flag mask `0`;
- filter flag mask `0`;
- no read-length filter;
- no read-group subset;
- no BED/region target mode;
- trim quality `0`;
- overlap removal disabled;
- maximum insert size `8000`;
- main insert bulk fraction `0.99`;
- ordinary non-sparse insert-size rendering.

No custom `-f`, `-F`, `-d`, `-q`, `-i`, `-m`, `-p`, target, region, or split-by-tag profile is covered by the Milestone 10 compatibility claim.

## Pinned tools

### Samtools

Repository lock:

`tools/reference/samtools/image.lock`

- version: `1.24`
- tag: `quay.io/biocontainers/samtools:1.24--h9dcdb79_1`
- image: `quay.io/biocontainers/samtools@sha256:a130447589651ed09252aa95a5e4f4132942cdb54d835d81a04a9a930d656561`

Reference executions use Docker `--network none`.

An earlier stale image digest no longer resolved from Quay during Milestone 10 bring-up. The implementation did not silently fall back to `latest`; the repository's current Samtools 1.24 lock was used as the authoritative executable reference.

### MultiQC

Repository lock:

`tools/reference/multiqc/image.lock`

- version: `1.35`
- tag: `quay.io/biocontainers/multiqc:1.35--pyhdfd78af_1`
- image: `quay.io/biocontainers/multiqc@sha256:b65e3fe879df27b92334dda0fd987a6e21bdee09a2848551d4f287099a93b7ac`

The pinned container reports exactly:

```text
multiqc, version 1.35
```

MultiQC parser executions also use Docker `--network none`.

## Canonical implementation

The authoritative implementation is the checked typed model in:

`crates/aligngauge-metrics/src/samtools_stats.rs`

It provides:

- `SamtoolsStatsCollector`;
- `SamtoolsStatsReport`;
- `InsertSizeRow`;
- `analyze_samtools_stats_bam`;
- a renderer derived from the completed report.

The compatibility text is not accumulated independently.

The dedicated field plan requests exactly the additional validated fields required by the profile:

- flags;
- current coordinates;
- mate coordinates;
- mapping quality;
- CIGAR;
- edit distance / `NM`;
- base qualities;
- template length / `TLEN`.

Normal v0.1-v0.3 QC paths do not silently request those additional fields merely because the compatibility collector exists. Packed sequence materialization remains unsupported by the reader plan.

All integer accumulation is checked. AlignGauge does not use saturating arithmetic or switch to an approximate stats path on overflow.

## Source-level Samtools semantics captured

The implementation follows the pinned Samtools 1.24 source behavior rather than looser prose approximations. Important examples include:

- secondary records increment `non-primary alignments` and return before ordinary sequence-derived counters;
- supplementary records increment `supplementary alignments`, remain excluded from original-read counters, but may contribute duplicate, CIGAR-mapped, and `NM` state;
- sequence-bearing duplicate non-secondary records contribute duplicate read/base counters;
- missing record-level `NM` contributes no mismatch observation and is not rewritten into an artificial record-level zero;
- `bases mapped` uses full query sequence length for mapped original records;
- `bases mapped (cigar)` counts `M`, `I`, `=`, and `X` for mapped non-secondary records;
- maximum read length is unclipped query length, including hard clips;
- insert observations require an original paired read with current read and mate mapped;
- `abs(TLEN)` is clamped to 8000;
- Samtools orientation classification is reproduced exactly;
- pair observations are halved per insert-size/orientation bin before summary and `IS` rendering;
- the cumulative `> 0.99` main-bulk rule defines insert-size mean, standard deviation, and rendered range.

During development, an initially transcribed synthetic expectation disagreed with the executable Samtools 1.24 oracle. The test expectation was corrected to the captured 1.24 output rather than changing the implementation to match the transcription.

## Exact synthetic oracle

Pinned executable-oracle run:

- run `31208689930`
- job `92965975258`
- result: success

### `basic.bam`

Exact Samtools 1.24 observations include:

| Metric | Value |
| --- | ---: |
| raw total sequences | 3 |
| filtered sequences | 0 |
| sequences | 3 |
| first fragments | 3 |
| last fragments | 0 |
| reads mapped | 2 |
| reads unmapped | 1 |
| total length | 24 |
| bases mapped | 20 |
| bases mapped (cigar) | 20 |
| mismatches | 0 |
| average length | 8 |
| maximum length | 10 |
| average quality | 30.0 |
| insert-size rows | 0 |

### `flags_and_pairs.bam`

Exact Samtools 1.24 observations include:

| Metric | Value |
| --- | ---: |
| raw total sequences | 6 |
| first fragments | 5 |
| last fragments | 1 |
| reads mapped | 6 |
| reads mapped and paired | 3 |
| reads properly paired | 2 |
| reads paired | 4 |
| reads duplicated | 1 |
| reads QC failed | 1 |
| non-primary alignments | 2 |
| supplementary alignments | 1 |
| total length | 40 |
| bases mapped | 40 |
| bases mapped (cigar) | 45 |
| bases duplicated | 5 |
| mismatches | 0 |
| average length | 7 |
| average quality | 30.0 |
| insert-size average | 70.0 |
| pairs with other orientation | 1 |
| properly paired reads | 33.3% |

The non-sparse `IS` section contains sizes 0 through 70. Size 70 is exactly:

```text
IS	70	1	0	0	1
```

Core implementation validation:

- run `31211870470`
- job `92976333905`
- result: success

That run passed workspace compilation, strict Clippy with warnings denied, the complete workspace test suite, exact synthetic oracle assertions, the public CLI compatibility probe, and restricted-diff cleanup.

## Independent Samtools differential

Permanent reference runner:

`tools/reference/samtools/run-stats.sh`

It captures:

- pinned image;
- exact version;
- invocation;
- stdout;
- stderr;
- exit status;
- wall time;
- `_SUCCESS`.

The command is run in a network-disabled container and fails closed if the reference command or expected `SN` output is incomplete.

Permanent comparator:

`tools/reference/samtools/compare-stats.py`

It independently parses and requires exact equality for:

- `SN` row count;
- `SN` row order;
- every `SN` field name;
- every rendered `SN` value string;
- complete `IS` row count;
- every `IS` insert-size/orientation tuple.

The differential report schema is:

```text
aligngauge-samtools-stats-differential-v1
```

A successful report records:

```text
status = exact
tolerance = null
compatibility_profile = samtools-stats-1.24-multiqc-1.35
```

No blanket or field-specific tolerance is applied to the claimed surface.

## Actual MultiQC 1.35 parser validation

Permanent parser validator:

`tools/reference/multiqc/validate-samtools-stats.sh`

The validator runs pinned MultiQC 1.35 twice under network isolation:

1. once on the real Samtools 1.24 reference text;
2. once on the AlignGauge compatibility text.

It then requires byte-identical MultiQC-generated data surfaces:

- `multiqc_samtools_stats.txt`;
- `samtools_insert_size.txt`.

The validation report schema is:

```text
aligngauge-multiqc-samtools-stats-validation-v1
```

A successful report records both comparisons as `byte-identical` and includes SHA-256 identities of the parsed outputs.

The exploratory pinned-parser run was:

- run `31212123583`
- job `92977144927`
- result: success

It proved that MultiQC 1.35 discovered both the real Samtools and AlignGauge files, parsed all 39 `SN` rows identically, produced identical derived Samtools stats data, and generated the same insert-size data from the 71 `IS` rows in the paired synthetic fixture.

## HG002 differential

The permanent `Samtools Stats Validation` workflow reuses the repository's deterministic HG002 preparation process and then performs the same exact differential against pinned Samtools 1.24.

On pre-evidence head `13ec94f52cd99ed95cb0ee6a1e29103e7c9a2065` it passed:

- deterministic HG002 subset preparation;
- exact 39-row `SN` comparison;
- exact complete `IS` comparison;
- pinned MultiQC 1.35 parser equivalence on the HG002 outputs;
- explicit absence of unsupported Samtools-stats sections;
- evidence artifact upload.

No tolerance is applied.

## Unsupported sections

Milestone 10 intentionally does not emit or claim compatibility for:

- `CHK` checksums;
- `FFQ` / `LFQ` quality-by-cycle sections;
- `GCF` / `GCL` GC histograms;
- `GCC` / `GCT`, `FBC` / `FTC`, `LBC` / `LTC` base-composition sections;
- barcode sections;
- `MPC` mismatch-per-cycle;
- `RL` / `FRL` / `LRL` read-length histograms;
- `MAPQ` histogram;
- `ID` indel-length distribution;
- `IC` indels-per-cycle;
- `COV` coverage distribution;
- `GCD` GC-depth;
- `RFS` reference statistics;
- target-region summary extensions;
- split-by-tag output;
- custom Samtools filtering/target profiles.

Both the exact comparator and permanent workflow fail if the AlignGauge renderer emits one of the explicitly unsupported section prefixes.

## Pre-evidence exact-head validation

All standing PR gates passed on exact head `13ec94f52cd99ed95cb0ee6a1e29103e7c9a2065`:

| Gate | Run | Job | Result |
| --- | --- | --- | --- |
| Permanent CI | `31213026956` | `92980046645` | success |
| Full Runtime Validation | `31213027456` | `92980063525` | success |
| Reference Validation | `31213027019` | `92980047514` | success |
| Targeted Validation | `31213027345` | `92980064084` | success |
| Samtools Stats Validation | `31213026957` | `92980062520` | success |

The M10 validation artifact is:

- artifact ID: `9007460490`
- digest: `sha256:e92565e7a04ffa8607ed55684978ae4ac8cd51e2a3ed928321056d8b3b655bec`
- size: 52,571 bytes

The artifact contains the synthetic and HG002 Samtools captures, AlignGauge compatibility outputs, exact differential reports, MultiQC parser reports, and HG002 preparation identity.

## Remaining acceptance operation

Milestone 10 implementation and differential evidence are complete. The remaining acceptance sequence is procedural and fail closed:

1. commit this evidence and the corresponding TODO/documentation state;
2. require Permanent CI and the standing Runtime/Reference/Targeted/Samtools-Stats gates on that exact evidence candidate;
3. merge only the exact green PR head;
4. validate the exact merged `master` SHA;
5. only then mark Milestone 10 complete and advance the Ralph loop to Milestone 11.

This milestone does not publish `v0.4.0`. The v0.4 release boundary remains open until its later compatibility milestones are complete.
