# AlignGauge CLI modes

AlignGauge retains the original compatibility paths while the full release path supports BAM, CRAM, and optional v0.3 targeted reductions:

- `aligngauge qc --input <BAM>` retains the original three-counter walking-skeleton output for compatibility.
- `aligngauge qc --input <BAM> --format <...>` exposes explicit differential/compatibility projections, including `samtools-flagstat`, `samtools-idxstats`, and the Milestone 10 `samtools-stats` subset.
- Supplying release options such as `--outdir` selects the full one-pass counters-plus-coverage analysis and atomic output publication path.
- Full CRAM analysis uses the same validated collector path and requires `--reference <FASTA>`.
- `--targets <BED>` enables native `aligngauge-targeted-v0.3` metrics in that same release traversal.
- `--near-distance <N>` selects the symmetric near-target distance; it defaults to 250 bases when targets are supplied and is rejected without `--targets`.

CRAM reference handling is fail closed. The explicit local FASTA is validated against required contig names, lengths, and `M5` identities before reference-dependent record traversal. Missing or mismatched references fail with typed errors; the implementation does not silently substitute `REF_PATH`, `REF_CACHE`, `HTS_PATH`, a remote reference service, or another local FASTA. Supplying `--reference` for BAM is rejected rather than silently ignored.

Target handling is also fail closed. BED3–BED12 input is validated against the already validated alignment sequence dictionary before record traversal. Unknown contigs, invalid source coordinates, inconsistent record widths, missing BED files, and out-of-range intervals are fatal. AlignGauge does not infer `1` ↔ `chr1`, silently drop targets, or repair malformed source intervals.

Targeted analysis reuses the exact canonical `aligngauge-v0.1` coverage sweep. Aggregate target territory uses the deterministic normalized union, while per-source-target results retain original BED identity for mean depth, thresholds, zero-coverage runs, and dropout reporting. Provenance records the exact BED path/SHA/size, target normalization, selected/near normalization, native target profile, and one alignment traversal.

The v0.3 native metrics include exact target territory, on-/near-/off-target aligned bases, target depth histogram/thresholds, per-target coverage/dropout data, native target enrichment, D20, and `target_uniformity_penalty_80`. These are not labeled Picard `CollectHsMetrics`, `FOLD_ENRICHMENT`, or `FOLD_80_BASE_PENALTY`; the filtering and bait/target model differ. Independently comparable target-depth primitives are validated exactly against pinned Samtools 1.24 in the permanent Targeted Validation workflow.

Milestone 10 adds `--format samtools-stats` as the explicit BAM-only `samtools-stats-1.24-multiqc-1.35` compatibility probe. It derives the complete ordinary 39-row `SN` Summary Numbers section and the `IS` insert-size section from a checked typed canonical report. The profile is validated exactly against pinned Samtools 1.24 and by running pinned MultiQC 1.35 on both the Samtools reference text and AlignGauge text. Unsupported Samtools-stats sections are omitted rather than partially approximated. This compatibility collector is not silently enabled for ordinary v0.1-v0.3 QC runs.

Milestone 10 acceptance is closed after exact validation of both the clean evidence candidate and the merged `master` commit. The Ralph loop proceeds to Milestone 11 — Picard alignment and insert-size profiles. This M10 closeout still does not publish or imply `v0.4.0`.

The compatibility paths must not silently inherit release-mode requirements, and release-only options must not be combined with `--format`.

Correctness warnings are always emitted to stderr, including under `--quiet`; quiet mode suppresses only routine completion output. `--verbose` emits the resolved configuration using the selected diagnostic format so accepted-but-serial settings such as `--threads >1` are visible rather than silently ignored.

Standalone `inspect` and `validate-reference` commands remain deferred until their CLI, output-schema, and error contracts are independently specified and tested.

Permanent runtime and release E2E validation exercise compatibility and release paths independently so compatibility behavior cannot silently acquire release-mode requirements. v0.2 CRAM/reference evidence is recorded in `docs/evidence/V0_2_CRAM_VALIDATION.md`; v0.3 targeted evidence is recorded in `docs/evidence/V0_3_TARGETED_VALIDATION.md`; Milestone 10 Samtools-stats/MultiQC evidence is recorded in `docs/evidence/M10_SAMTOOLS_STATS_MULTIQC.md`.

`v0.3.0` is published from exact release SHA `eccd45157d34ada00a3403a2b24d606956878b62`. Before publication that exact SHA passed Permanent CI, Full Runtime Validation, Reference Validation, Targeted Validation, and HG002 Preparation Validation; the release tag remains policy-pinned to that validated commit while `master` may advance with post-release documentation. Milestone 10 is v0.4 development work and does not by itself publish `v0.4.0`.

This release-surface document is intentionally part of the standing validation trigger sets so a compatibility-surface documentation change cannot bypass runtime, reference, targeted, or Samtools-stats qualification.
