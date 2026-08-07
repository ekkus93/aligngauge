# AlignGauge CLI modes

AlignGauge retains the original compatibility paths while the full release path supports BAM, CRAM, and optional v0.3 targeted reductions:

- `aligngauge qc --input <BAM>` retains the original three-counter walking-skeleton output for compatibility.
- `aligngauge qc --input <BAM> --format <...>` exposes counters-only differential/compatibility projections.
- Supplying release options such as `--outdir` selects the full one-pass counters-plus-coverage analysis and atomic output publication path.
- Full CRAM analysis uses the same validated collector path and requires `--reference <FASTA>`.
- `--targets <BED>` enables native `aligngauge-targeted-v0.3` metrics in that same release traversal.
- `--near-distance <N>` selects the symmetric near-target distance; it defaults to 250 bases when targets are supplied and is rejected without `--targets`.

CRAM reference handling is fail closed. The explicit local FASTA is validated against required contig names, lengths, and `M5` identities before reference-dependent record traversal. Missing or mismatched references fail with typed errors; the implementation does not silently substitute `REF_PATH`, `REF_CACHE`, `HTS_PATH`, a remote reference service, or another local FASTA. Supplying `--reference` for BAM is rejected rather than silently ignored.

Target handling is also fail closed. BED3–BED12 input is validated against the already validated alignment sequence dictionary before record traversal. Unknown contigs, invalid source coordinates, inconsistent record widths, missing BED files, and out-of-range intervals are fatal. AlignGauge does not infer `1` ↔ `chr1`, silently drop targets, or repair malformed source intervals.

Targeted analysis reuses the exact canonical `aligngauge-v0.1` coverage sweep. Aggregate target territory uses the deterministic normalized union, while per-source-target results retain original BED identity for mean depth, thresholds, zero-coverage runs, and dropout reporting. Provenance records the exact BED path/SHA/size, target normalization, selected/near normalization, native target profile, and one alignment traversal.

The v0.3 native metrics include exact target territory, on-/near-/off-target aligned bases, target depth histogram/thresholds, per-target coverage/dropout data, native target enrichment, D20, and `target_uniformity_penalty_80`. These are not labeled Picard `CollectHsMetrics`, `FOLD_ENRICHMENT`, or `FOLD_80_BASE_PENALTY`; the filtering and bait/target model differ. Independently comparable target-depth primitives are validated exactly against pinned Samtools 1.24 in the permanent Targeted Validation workflow.

The compatibility paths must not silently inherit release-mode requirements, and release-only options must not be combined with `--format`.

Correctness warnings are always emitted to stderr, including under `--quiet`; quiet mode suppresses only routine completion output. `--verbose` emits the resolved configuration using the selected diagnostic format so accepted-but-serial settings such as `--threads >1` are visible rather than silently ignored.

Standalone `inspect` and `validate-reference` commands remain deferred until their CLI, output-schema, and error contracts are independently specified and tested.

Permanent runtime and release E2E validation exercise compatibility and release paths independently so compatibility behavior cannot silently acquire release-mode requirements. v0.2 CRAM/reference evidence is recorded in `docs/evidence/V0_2_CRAM_VALIDATION.md`; v0.3 targeted evidence is recorded in `docs/evidence/V0_3_TARGETED_VALIDATION.md`.

`v0.2.0` remains the latest published release until the exact v0.3 release-candidate documentation commit passes Permanent CI, Full Runtime Validation, Reference Validation, Targeted Validation, and HG002 Preparation Validation and is then published as `v0.3.0` without moving the validated target SHA.

This release-surface document is intentionally part of all five v0.3 release-validation trigger sets so a release-candidate documentation change cannot bypass runtime, reference, targeted, or HG002 qualification.
