# AlignGauge v0.4 release validation evidence

**State:** validated release-gate implementation candidate; no `v0.4.0` tag or GitHub release exists yet.

## Candidate identity

Validated candidate SHA:

`191fd927c506d037dad57b8209d132f78a36d025`

This commit contains ADR-0011 and the permanent `ci/v0.4-release` workflow. It changes the release contract and validation surface, not the already-proved metric algorithms.

Exact candidate workflow evidence:

- V0.4 Release Validation run `31254954734`, job `93096874003` — success
- Permanent CI run `31254954580`, job `93096873958` — success
- Reference Validation run `31254954662`, job `93096874325` — success

The candidate is not itself declared the final release commit. Documentation/TODO closure and merged-master validation still occur before a tag is created.

## Release scope proved

ADR-0011 freezes these v0.4 compatibility profiles:

- `samtools-stats-1.24-multiqc-1.35`
- `picard-alignment-summary-3.4.0-all-reads-subset-v1`
- `picard-insert-size-3.4.0-all-reads-v1`

The gate also proves that `picard-wgs` and `picard-hs-metrics` are not accepted CLI formats. The candidate WGS/Hs surfaces from ADR-0009 remain deferred rather than being partially emitted.

## Released execution-mode equivalence

The v0.4 collector/reduction path remains serial and deterministic. The released concurrency mechanism is HTSlib reader/decompression concurrency through `--io-threads`.

The candidate ran both of these release analyses twice:

1. whole-input `testdata/fixtures/basic.bam`;
2. targeted `testdata/fixtures/chunk_boundary.bam` with the committed chunk-boundary targets, near distance 5, and thresholds 1 and 2.

For each input:

- serial configuration used `--io-threads 0`, which provenance records as effective reader I/O threads `1`;
- concurrent configuration used `--io-threads 2`, which provenance records as effective reader I/O threads `2`;
- `collector_threads_used` remained `1`;
- canonical `summary.json` files were byte-identical.

A separate run with `--threads 2` proved that configured collector capacity does not masquerade as an implemented parallel collector: the run succeeded with `collector_threads_used = 1` and emitted the explicit `collector_threads_serial_v0_1` warning.

Indexed reference-partition execution remains unsupported under ADR-0010.

## Samtools 1.24 proof

The release gate generated the pinned Samtools 1.24 reference output and the AlignGauge `samtools-stats-1.24-multiqc-1.35` projection from the same committed BAM.

`tools/reference/samtools/compare-stats.py` reported exact agreement for the complete released ordinary 39-row `SN` surface and the default released `IS` surface. No numerical tolerance was used.

The exact field inventory is reconciled in `docs/evidence/V0_4_COMPATIBILITY_REPORT.md`.

## MultiQC 1.35 Samtools proof

The generated Samtools reference text and generated AlignGauge compatibility text were independently parsed by the immutable pinned MultiQC 1.35 image with Docker networking disabled.

The resulting:

- `multiqc_samtools_stats.txt`; and
- `samtools_insert_size.txt`

were byte-identical reference versus AlignGauge.

Parser success alone was not accepted as evidence; parsed data equality was required.

## Picard 3.4.0 proof

The release gate generated pinned Picard 3.4.0 reference outputs and AlignGauge projections from the committed Picard fixtures.

Direct differential results were exact for:

- `picard-alignment-summary-3.4.0-all-reads-subset-v1`; and
- `picard-insert-size-3.4.0-all-reads-v1`.

The alignment-summary claim remains exactly the 13 reference-independent fields defined by ADR-0008. Reference-dependent Picard columns remain absent rather than being synthesized as zero.

The insert-size claim remains the default `ALL_READS` metrics plus the Picard-trimmed histogram. PDF chart output is outside the profile.

## MultiQC 1.35 Picard proof

The generated Picard InsertSize reference and AlignGauge projection were independently parsed through the pinned MultiQC 1.35 Picard module with filename-based sample identity. The parsed reference and AlignGauge insert-size data were byte-identical.

The same permanent validator parses the committed WGS and HsMetrics discovery fixtures, but its machine-readable report records `compatibility_claim: false` for both. Those files are not AlignGauge-generated output and are not numerical evidence.

The released 13-column AlignmentSummary subset is intentionally not claimed MultiQC-compatible because MultiQC 1.35 directly requires reference-dependent columns outside that profile.

## Fail-closed boundaries

The v0.4 gate contains no warning-only reference-tool path or tolerance fallback.

It fails when:

- a pinned tool/version contract changes;
- an unproved WGS/Hs format becomes exposed;
- serial and concurrent canonical summaries differ;
- effective execution settings are misreported;
- direct Samtools or Picard comparison is not exact;
- MultiQC cannot parse a claimed generated profile;
- parsed MultiQC data differs reference versus AlignGauge;
- WGS/Hs discovery fixtures acquire a compatibility claim;
- required evidence artifacts are missing; or
- validation leaves tracked repository state dirty.

## Remaining closure before tag

This evidence freezes the release-gate implementation candidate only.

Before `v0.4.0` is created:

1. the compatibility report must be committed in final reconciled form;
2. that evidence state must pass its triggered CI;
3. the TODO release-gate checklist must be closed without claiming an unvalidated tag;
4. PR #8 must merge with an exact validated head;
5. the resulting `master` SHA must pass every triggered permanent workflow;
6. the exact commit selected for the `v0.4.0` tag must itself have successful Permanent CI and release-gate evidence.

Only then may the tag and GitHub release be created.
