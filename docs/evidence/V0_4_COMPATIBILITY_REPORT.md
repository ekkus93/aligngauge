# AlignGauge v0.4 compatibility report

**Report state:** reconciled v0.4 release-candidate compatibility boundary. This document does not itself create the `v0.4.0` tag or GitHub release.

**Reference implementations**

- Samtools 1.24
- Picard 3.4.0
- HTSJDK 4.2.0 as bundled by pinned Picard 3.4.0
- MultiQC 1.35 from the immutable image in `tools/reference/multiqc/image.lock`

**Release-scope authority:** ADR-0011.

## v0.4 compatibility matrix

| Surface | v0.4 disposition | Direct numerical evidence | MultiQC 1.35 disposition |
|---|---|---|---|
| `samtools-stats-1.24-multiqc-1.35` | released exact profile | exact Samtools 1.24 differential | generated output parsed; parsed data byte-identical |
| `picard-alignment-summary-3.4.0-all-reads-subset-v1` | released exact 13-field subset | exact Picard 3.4.0 differential | **not claimed compatible**; parser requires reference-dependent columns outside the released subset |
| `picard-insert-size-3.4.0-all-reads-v1` | released exact profile | exact Picard 3.4.0 differential | generated output parsed; parsed data byte-identical |
| Picard WgsMetrics candidate | **deferred; not emitted** | M13 exact overlap primitive only; complete WGS differential absent | discovery-only fixture; `compatibility_claim: false` |
| Picard HsMetrics candidate | **deferred; not emitted** | M13 exact overlap primitive only; complete Hs differential absent | discovery-only fixture; `compatibility_claim: false` |
| native `aligngauge-targeted-v0.3` | supported native profile | v0.3 native validation | no Picard HsMetrics claim |
| indexed reference-partition execution | unsupported | not admitted by ADR-0010 | not applicable |

Existing `samtools-flagstat` and `samtools-idxstats` compatibility projections from earlier releases remain available and are not widened by v0.4.

## Final field reconciliation: Samtools Stats

The released Samtools profile is exactly the ordinary, non-target Samtools 1.24 `SN` section plus the default `IS` section frozen by ADR-0007. The v0.4 release validator generates the pinned Samtools reference and AlignGauge projection from the same committed input and requires exact comparison with no blanket tolerance.

Every claimed ordinary `SN` row is reconciled below.

| # | Samtools 1.24 field | v0.4 status |
|---:|---|---|
| 1 | `raw total sequences` | exact |
| 2 | `filtered sequences` | exact |
| 3 | `sequences` | exact |
| 4 | `is sorted` | exact |
| 5 | `1st fragments` | exact |
| 6 | `last fragments` | exact |
| 7 | `reads mapped` | exact |
| 8 | `reads mapped and paired` | exact |
| 9 | `reads unmapped` | exact |
| 10 | `reads properly paired` | exact |
| 11 | `reads paired` | exact |
| 12 | `reads duplicated` | exact |
| 13 | `reads MQ0` | exact |
| 14 | `reads QC failed` | exact |
| 15 | `non-primary alignments` | exact |
| 16 | `supplementary alignments` | exact |
| 17 | `total length` | exact |
| 18 | `total first fragment length` | exact |
| 19 | `total last fragment length` | exact |
| 20 | `bases mapped` | exact |
| 21 | `bases mapped (cigar)` | exact |
| 22 | `bases trimmed` | exact |
| 23 | `bases duplicated` | exact |
| 24 | `mismatches` | exact |
| 25 | `error rate` | exact pinned textual semantics |
| 26 | `average length` | exact |
| 27 | `average first fragment length` | exact |
| 28 | `average last fragment length` | exact |
| 29 | `maximum length` | exact |
| 30 | `maximum first fragment length` | exact |
| 31 | `maximum last fragment length` | exact |
| 32 | `average quality` | exact |
| 33 | `insert size average` | exact |
| 34 | `insert size standard deviation` | exact |
| 35 | `inward oriented pairs` | exact |
| 36 | `outward oriented pairs` | exact |
| 37 | `pairs with other orientation` | exact |
| 38 | `pairs on different chromosomes` | exact |
| 39 | `percentage of properly paired reads (%)` | exact pinned textual semantics |

### Samtools `IS` section

The released `IS` surface uses Samtools 1.24 default insert-size behavior from ADR-0007, including the 8000 maximum, orientation classification, pair-observation halving, 0.99 main-bulk rule, mean/standard-deviation calculation, and ordinary non-sparse rendering.

The complete supported `IS` output is directly compared against pinned Samtools 1.24. MultiQC's consumed `insert size` and `pairs total` data are also parsed from both reference and AlignGauge text and required to be byte-identical.

### Explicitly unsupported Samtools Stats surfaces

v0.4 makes no compatibility claim for the sections excluded by ADR-0007, including `CHK`, quality-cycle tables, GC/read-length/mapping-quality/indel/coverage/reference-GC/barcode sections, target-region-only summary extensions, split-by-tag output, or non-default filter/region profiles. Absence is intentional; partial rows are not emitted under the released profile.

## Final field reconciliation: Picard AlignmentSummaryMetrics

The released alignment-summary profile is exactly the Picard 3.4.0 reference-independent subset frozen by ADR-0008.

| # | Picard field | v0.4 status |
|---:|---|---|
| 1 | `CATEGORY` | exact |
| 2 | `TOTAL_READS` | exact |
| 3 | `PF_READS` | exact |
| 4 | `PCT_PF_READS` | exact |
| 5 | `PF_NOISE_READS` | exact |
| 6 | `PCT_ADAPTER` | exact |
| 7 | `MEAN_READ_LENGTH` | exact |
| 8 | `SD_READ_LENGTH` | exact |
| 9 | `MEDIAN_READ_LENGTH` | exact |
| 10 | `MAD_READ_LENGTH` | exact |
| 11 | `MIN_READ_LENGTH` | exact |
| 12 | `MAX_READ_LENGTH` | exact |
| 13 | `BAD_CYCLES` | exact |

Category-row behavior, secondary/supplementary exclusion, Picard default adapter matching, no-call cycle semantics, and empty/unpaired behavior are part of the direct Picard differential contract.

Reference-dependent alignment/mismatch/indel/error, strand-balance, chimera, pair-alignment, clipping, `PF_READS_ALIGNED`, `PF_ALIGNED_BASES`, and other fields outside the 13-field profile remain unsupported and absent. They are never serialized as fake zeroes.

Because pinned MultiQC 1.35 directly requires reference-dependent columns outside this profile, v0.4 does **not** claim MultiQC compatibility for the AlignmentSummary subset. Direct Picard exactness remains the release claim.

## Final field reconciliation: Picard InsertSizeMetrics

The released profile is Picard 3.4.0 default `ALL_READS` behavior with `DEVIATIONS=10.0`, default histogram width, Java-float `MINIMUM_PCT=0.05f` semantics, duplicates excluded, and no PDF chart claim.

Every emitted orientation row reconciles these fields:

| Picard field | v0.4 status |
|---|---|
| `READ_PAIRS` | exact |
| `PAIR_ORIENTATION` | exact |
| `MEDIAN_INSERT_SIZE` | exact |
| `MODE_INSERT_SIZE` | exact |
| `MEDIAN_ABSOLUTE_DEVIATION` | exact |
| `MIN_INSERT_SIZE` | exact |
| `MAX_INSERT_SIZE` | exact |
| `MEAN_INSERT_SIZE` | exact after Picard MAD trimming |
| `STANDARD_DEVIATION` | exact after Picard MAD trimming |
| `WIDTH_OF_10_PERCENT` | exact |
| `WIDTH_OF_20_PERCENT` | exact |
| `WIDTH_OF_30_PERCENT` | exact |
| `WIDTH_OF_40_PERCENT` | exact |
| `WIDTH_OF_50_PERCENT` | exact |
| `WIDTH_OF_60_PERCENT` | exact |
| `WIDTH_OF_70_PERCENT` | exact |
| `WIDTH_OF_80_PERCENT` | exact |
| `WIDTH_OF_90_PERCENT` | exact |
| `WIDTH_OF_95_PERCENT` | exact |
| `WIDTH_OF_99_PERCENT` | exact |

The complete Picard-trimmed insert-size histogram table is also part of the exact direct comparison. FR/RF/TANDEM orientation, second-of-pair observation selection, secondary/supplementary/duplicate exclusion, nonzero TLEN requirement, checked absolute TLEN, the promoted Java binary32 `MINIMUM_PCT` boundary, median/MAD trim width, HTSJDK histogram tie behavior, and decimal rendering are covered by ADR-0008 and its differential evidence.

Pinned MultiQC 1.35 independently parses the generated Picard reference and AlignGauge outputs. The parsed Picard InsertSize data are required to be byte-identical after explicit filename-based sample identity normalization; parser exit alone is insufficient.

PDF chart generation is outside the compatibility profile.

## WGS and HsMetrics disposition

ADR-0009 selected future candidate surfaces; M13 then supplied and differentially proved exact overlap primitives. Neither event makes a complete WGS or HsMetrics collector exist.

### Picard WgsMetrics

The candidate remains **not emitted in v0.4**. The CLI does not expose `picard-wgs`.

M13 proves `picard-wgs-3.4.0-default-overlap-v1`, including the pinned per-locus ordering, base-quality/no-call-before-overlap behavior, raw query-name identity, secondary/supplementary boundary, hard memory budget, and pinned locus accumulation cap. Complete WGS filtering, depth reduction, capping, exclusion denominators, histogram, renderer, direct metric differential, and generated-output MultiQC proof remain absent.

Therefore no WGS candidate field is a v0.4 compatibility claim.

### Picard HsMetrics

The candidate remains **not emitted in v0.4**. The CLI does not expose `picard-hs-metrics`.

M13 proves `picard-hs-3.4.0-default-overlap-v1` against pinned HTSJDK 4.2.0. Complete Picard PF filtering, bait/target placement, denominator semantics, enrichment, usable-base fractions, depth reductions, renderer, and direct metric differential remain absent.

The native `target_uniformity_penalty_80` is still distinct from Picard `FOLD_80_BASE_PENALTY`. The native value is not relabeled or copied into a Picard field.

The WGS and Hs files under `tools/reference/multiqc/fixtures/` are discovery-only parser fixtures. The permanent machine-readable MultiQC report records `compatibility_claim: false` for both.

## Released execution modes

v0.4 has one authoritative deterministic collector/reduction path. Collector execution remains serial.

`--threads > 1` is a configured resource value, not an implemented parallel collector. A v0.4 release-gate run with `--threads 2` must continue to record `collector_threads_used = 1` and emit `collector_threads_serial_v0_1`.

Released bounded concurrency is HTSlib reader/decompression concurrency through `--io-threads` while preserving one logical ordered record stream.

The v0.4 candidate proves both:

- whole-input serial (`--io-threads 0`, effective reader threads 1) vs `--io-threads 2` canonical `summary.json`: **byte-identical**;
- targeted serial vs `--io-threads 2` canonical `summary.json`: **byte-identical**.

Provenance is intentionally not byte-identical because it truthfully records configured/effective I/O-thread settings and timing values.

Indexed reference-partition execution remains unsupported by ADR-0010 and is not a released mode.

## Pinned MultiQC generated-output proof

v0.4 claims MultiQC 1.35 compatibility only where generated AlignGauge output has been independently parsed and compared to parsed reference output:

1. `samtools-stats-1.24-multiqc-1.35` — parsed `multiqc_samtools_stats.txt` and `samtools_insert_size.txt` are byte-identical reference vs AlignGauge;
2. `picard-insert-size-3.4.0-all-reads-v1` — parsed Picard insert-size data are byte-identical reference vs AlignGauge.

The Picard AlignmentSummary 13-field subset is intentionally outside the MultiQC claim. WGS/Hs are discovery-only and carry false compatibility claims.

All pinned MultiQC executions use the immutable 1.35 image and network isolation. No parser failure is converted to a warning, no expected parsed file may be absent, and no success marker is written before all assertions pass.

## v0.4 release-gate implementation candidate

Candidate SHA:

`191fd927c506d037dad57b8209d132f78a36d025`

Successful workflows on that exact SHA:

- V0.4 Release Validation run `31254954734`, job `93096874003` — success
- Permanent CI run `31254954580`, job `93096873958` — success
- Reference Validation run `31254954662`, job `93096874325` — success

The v0.4 gate proved, on one commit:

- released CLI profile boundaries;
- WGS/Hs non-promotion;
- whole-input serial/I/O-thread canonical equivalence;
- targeted serial/I/O-thread canonical equivalence;
- explicit serial collector behavior under `--threads 2`;
- exact generated Samtools Stats differential;
- pinned MultiQC Samtools parsed-data equivalence;
- exact generated Picard AlignmentSummary differential;
- exact generated Picard InsertSize differential;
- pinned MultiQC Picard InsertSize parsed-data equivalence;
- discovery-only WGS/Hs false compatibility claims;
- fail-closed artifact creation and clean repository state.

Detailed release-gate evidence is in `docs/evidence/V0_4_RELEASE_VALIDATION.md`.

## Prior milestone evidence carried into v0.4

The release claim also rests on the existing permanent evidence chain rather than replacing it:

- Milestone 10: `docs/evidence/M10_SAMTOOLS_STATS_MULTIQC.md`
- Milestone 11: `docs/evidence/M11_PICARD_ALIGNMENT_INSERT_SIZE.md`
- Milestone 12: pinned Picard/MultiQC discovery and generated InsertSize validation recorded in this report's repository history
- Milestone 13: `docs/evidence/M13_EXACT_OVERLAP.md`

The v0.4 release validator is additive. It does not weaken or substitute for Permanent CI, Reference Validation, Samtools Stats Validation, Picard Validation, MultiQC Validation, Exact Overlap Validation, Full Runtime Validation, Targeted Validation, or Samtools Stats Validation where those workflows are triggered.

## Release status

The **compatibility-report reconciliation requirement is satisfied in this candidate state**, subject to CI validating this committed report.

The report does not claim that a `v0.4.0` tag already exists. The remaining repository operations are evidence/TODO closure, exact merge validation on `master`, selection of an exact green release commit, and only then creation of the `v0.4.0` tag and GitHub release.
