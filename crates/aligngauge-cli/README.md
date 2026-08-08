# AlignGauge CLI modes

AlignGauge retains the original compatibility paths while the full release path supports BAM, CRAM, and optional v0.3 targeted reductions:

- `aligngauge qc --input <BAM>` retains the original three-counter walking-skeleton output for compatibility.
- `aligngauge qc --input <BAM> --format <...>` exposes explicit differential/compatibility projections, including `samtools-flagstat`, `samtools-idxstats`, the v0.4 `samtools-stats` profile, and the v0.4 `picard-alignment-summary` / `picard-insert-size` profiles.
- Supplying release options such as `--outdir` selects the full one-pass counters-plus-coverage analysis and atomic output publication path.
- Full CRAM analysis uses the same validated collector path and requires `--reference <FASTA>`.
- `--targets <BED>` enables native `aligngauge-targeted-v0.3` metrics in that same release traversal.
- `--near-distance <N>` selects the symmetric near-target distance; it defaults to 250 bases when targets are supplied and is rejected without `--targets`.

CRAM reference handling is fail closed. The explicit local FASTA is validated against required contig names, lengths, and `M5` identities before reference-dependent record traversal. Missing or mismatched references fail with typed errors; the implementation does not silently substitute `REF_PATH`, `REF_CACHE`, `HTS_PATH`, a remote reference service, or another local FASTA. Supplying `--reference` for BAM is rejected rather than silently ignored.

Target handling is also fail closed. BED3–BED12 input is validated against the already validated alignment sequence dictionary before record traversal. Unknown contigs, invalid source coordinates, inconsistent record widths, missing BED files, and out-of-range intervals are fatal. AlignGauge does not infer `1` ↔ `chr1`, silently drop targets, or repair malformed source intervals.

Targeted analysis reuses the exact canonical `aligngauge-v0.1` coverage sweep. Aggregate target territory uses the deterministic normalized union, while per-source-target results retain original BED identity for mean depth, thresholds, zero-coverage runs, and dropout reporting. Provenance records the exact BED path/SHA/size, target normalization, selected/near normalization, native target profile, and one alignment traversal.

The v0.3 native metrics include exact target territory, on-/near-/off-target aligned bases, target depth histogram/thresholds, per-target coverage/dropout data, native target enrichment, D20, and `target_uniformity_penalty_80`. These are not labeled Picard `CollectHsMetrics`, `FOLD_ENRICHMENT`, or `FOLD_80_BASE_PENALTY`; the filtering and bait/target model differ. Independently comparable target-depth primitives are validated exactly against pinned Samtools 1.24 in the permanent Targeted Validation workflow.

## v0.4 compatibility release surface

ADR-0011 freezes the v0.4 compatibility surface rather than allowing milestone work to become an implicit compatibility promise.

### Samtools Stats

`--format samtools-stats` is the explicit BAM-only `samtools-stats-1.24-multiqc-1.35` compatibility profile. It derives the complete ordinary 39-row `SN` Summary Numbers section and the default `IS` insert-size section from a checked typed canonical report.

The profile is validated exactly against pinned Samtools 1.24. Pinned MultiQC 1.35 independently parses generated Samtools reference and AlignGauge text; the parsed `multiqc_samtools_stats.txt` and `samtools_insert_size.txt` data must be byte-identical. Unsupported Samtools Stats sections remain omitted rather than partially approximated.

### Picard AlignmentSummaryMetrics

`--format picard-alignment-summary` is `picard-alignment-summary-3.4.0-all-reads-subset-v1`. It claims exactly the 13 reference-independent Picard 3.4.0 fields documented by ADR-0008 and validated by direct differential tests.

Reference-dependent alignment-summary fields remain unsupported and absent rather than being synthesized as zero. Because pinned MultiQC 1.35 directly requires reference-dependent columns outside this 13-field subset, the released alignment-summary profile is **not** claimed MultiQC-compatible.

### Picard InsertSizeMetrics

`--format picard-insert-size` is `picard-insert-size-3.4.0-all-reads-v1`. It claims Picard 3.4.0 default `ALL_READS` metrics rows and the trimmed insert-size histogram. Direct differential output must match pinned Picard exactly with no tolerance.

Pinned MultiQC 1.35 independently parses generated Picard reference and AlignGauge InsertSize output using explicit filename-based sample identity; parsed reference and AlignGauge data must be byte-identical. PDF chart output and SAMPLE/LIBRARY/READ_GROUP accumulation levels remain outside the profile.

### WGS and HsMetrics are deferred

`picard-wgs` and `picard-hs-metrics` are not released formats in v0.4.

Milestone 12 selected candidate WGS/Hs surfaces, and Milestone 13 proved the exact Picard/HTSJDK overlap primitives required by those future collectors. That does not prove complete WGS/Hs filtering, denominators, coverage/target reductions, renderer semantics, Picard `FOLD_80_BASE_PENALTY`, complete metric differentials, or generated-output MultiQC equivalence.

Accordingly the CLI rejects those format names instead of emitting plausible-looking partial files. The WGS/Hs files under `tools/reference/multiqc/fixtures/` remain discovery-only parser fixtures whose machine-readable compatibility claims are false.

Native `target_uniformity_penalty_80` remains distinct from Picard `FOLD_80_BASE_PENALTY`; no value is copied or relabeled.

## v0.4 execution-mode contract

The authoritative collector/reduction path remains deterministic and serial. `--threads >1` is accepted as a configured resource value for compatibility/provenance, but the runtime reports `collector_threads_used = 1` and emits the explicit `collector_threads_serial_v0_1` correctness warning. There is no silent parallel collector implementation.

Released bounded concurrency is HTSlib reader/decompression concurrency through `--io-threads`. `--io-threads 0` normalizes to one effective reader thread; positive values may use additional HTSlib I/O workers while preserving one logical ordered record stream.

The permanent `ci/v0.4-release` gate requires byte-identical canonical `summary.json` results between serial decoding and `--io-threads 2` for both ordinary whole-input and targeted release paths. Provenance is expected to differ where it truthfully reports configured/effective I/O threads and timings.

Indexed reference-partition execution remains unsupported under ADR-0010.

## Compatibility/release separation

The compatibility paths must not silently inherit release-mode requirements, and release-only options must not be combined with `--format`.

Correctness warnings are always emitted to stderr, including under `--quiet`; quiet mode suppresses only routine completion output. `--verbose` emits the resolved configuration using the selected diagnostic format so accepted-but-serial settings such as `--threads >1` are visible rather than silently ignored.

Standalone `inspect` and `validate-reference` commands remain deferred until their CLI, output-schema, and error contracts are independently specified and tested.

Permanent runtime and release E2E validation exercise compatibility and release paths independently so compatibility behavior cannot silently acquire release-mode requirements. Evidence is recorded in `docs/evidence/V0_2_CRAM_VALIDATION.md`, `docs/evidence/V0_3_TARGETED_VALIDATION.md`, `docs/evidence/M10_SAMTOOLS_STATS_MULTIQC.md`, `docs/evidence/M11_PICARD_ALIGNMENT_INSERT_SIZE.md`, `docs/evidence/M13_EXACT_OVERLAP.md`, `docs/evidence/V0_4_COMPATIBILITY_REPORT.md`, and `docs/evidence/V0_4_RELEASE_VALIDATION.md`.

`v0.3.0` remains the latest published product release while the v0.4 release candidate is being closed. No `v0.4.0` tag or GitHub release is created until the exact release commit passes Permanent CI and the permanent v0.4 release-validation gate. The eventual tag is pinned to that validated commit rather than created speculatively.

This release-surface document is intentionally part of the standing validation trigger sets so compatibility-surface documentation changes cannot bypass runtime, reference, targeted, Samtools Stats, Picard, MultiQC, exact-overlap, or v0.4 qualification.
