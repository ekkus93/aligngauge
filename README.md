# AlignGauge

[![Permanent CI](https://github.com/ekkus93/aligngauge/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/ekkus93/aligngauge/actions/workflows/ci.yml)

A validation-first Rust engine for fast, single-pass alignment QC and coverage analysis across BAM and CRAM data for WGS, WES, and targeted sequencing.

AlignGauge analyzes existing aligned sequencing data. It is not software for controlling a physical DNA sequencer, a basecaller, an aligner, or a variant caller.

## Status

AlignGauge `v0.4.0` is the latest published release, pinned to exact release SHA `5be4aa4e5df3e8feb17fdde46c408683ac08bb53` (GitHub release ID `367190259`). The v0.3 tag is pinned by release policy to exact release SHA `eccd45157d34ada00a3403a2b24d606956878b62` (GitHub release ID `366930828`). That exact release SHA passed Permanent CI, Full Runtime Validation, Reference Validation, Targeted Validation, and HG002 Preparation Validation before publication. The v0.2 tag remains pinned by release policy to exact release/evidence SHA `ce3aa273da40c679c292e588584781ab1df241de`; the earlier v0.1 tag remains pinned to validated evidence SHA `9423a9d3496459fdbceb2e7bc5178b4b3100357c` (product implementation SHA `f93001cf22a2315f01e6b857c295720d99e392ca`).

v0.5 production-beta qualification is currently paused at Milestone 14 because the real ~30× whole-genome HG002 campaign requires more local disk capacity than is presently available. No requirement has been waived and `v0.5.0` has not been published. See the [2026-08-08 project status handoff](docs/PROJECT_STATUS_HANDOFF_2026-08-08.md) for the exact restart state, validation history, storage requirements, and resume procedure.

- v0.1: local coordinate-sorted BAM, CPU counters, exact canonical coverage, JSON/provenance, and atomic output publication.
- v0.2: adds CRAM analysis with an explicit local FASTA, fail-closed SN/LN/M5 validation, actual local-reference provenance, BAM/CRAM canonical equivalence, and production HTSlib builds with remote reference transports excluded.
- v0.3: adds fail-closed BED3–BED12 target parsing, deterministic target normalization, `--targets <BED>`, `--near-distance <N>` (default 250), exact on/near/off-target partitioning, per-source target depth/dropout reporting, native target enrichment, and `target_uniformity_penalty_80`.
- v0.4: adds the exact `samtools-stats-1.24-multiqc-1.35` compatibility profile, the exact Picard 3.4.0 13-field reference-independent AlignmentSummary subset, the exact Picard 3.4.0 default `ALL_READS` InsertSize profile, pinned MultiQC 1.35 generated-output validation for the supported parser surfaces, and an explicit release gate proving serial versus released HTSlib I/O-thread canonical equivalence.
- Targeted analysis reuses the exact canonical chunked coverage sweep; counters, whole-genome coverage, and targeted reductions remain one alignment traversal.
- Native targeted metrics do not claim Picard `CollectHsMetrics`, `FOLD_ENRICHMENT`, or `FOLD_80_BASE_PENALTY` compatibility. Comparable target-depth primitives are independently validated against pinned Samtools 1.24.
- Picard WgsMetrics and HsMetrics remain explicitly deferred from v0.4. Milestone 13 proves their exact overlap primitives against pinned Picard 3.4.0 / HTSJDK 4.2.0, but complete WGS/Hs record filtering, reductions, renderers, full metric differentials, and generated-output MultiQC proof have not been promoted to release profiles.
- Native `target_uniformity_penalty_80` remains distinct from Picard `FOLD_80_BASE_PENALTY`; there is no alias, copied value, or zero-filled fallback.
- CRAM reference mismatch or absence is fatal; AlignGauge does not silently fall back to inherited local or remote providers.
- Target BED contig/coordinate mismatch is fatal; AlignGauge does not infer chromosome aliases, silently drop unknown targets, or repair invalid source intervals.
- Standalone `inspect` and `validate-reference` commands remain deferred until separately specified.
- Collector execution remains deterministic and serial. `--threads >1` is accepted for configuration/provenance compatibility but emits `collector_threads_serial_v0_1` rather than implying parallel collectors.
- Released bounded concurrency is HTSlib reader/decompression concurrency through `--io-threads`; the v0.4 release gate requires byte-identical canonical summaries between serial decoding and `--io-threads 2` for whole-input and targeted paths.
- Indexed reference-partition execution is not admitted for v0.4.
- GPU/backend selection remains research-only until end-to-end benchmarks justify it.

`v0.4.0` was published only after exact release SHA `5be4aa4e5df3e8feb17fdde46c408683ac08bb53` passed Permanent CI, Reference Validation, and the permanent v0.4 release-validation gate. The tag is evidence of that validated commit and points directly to it.

## Development

The repository pins Rust 1.97.1. After installing `rustup`:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
```

## Planning and evidence

- [Current v0.5 project status handoff (2026-08-08)](docs/PROJECT_STATUS_HANDOFF_2026-08-08.md)
- [v0.4.0 release](https://github.com/ekkus93/aligngauge/releases/tag/v0.4.0)
- [v0.3.0 release](https://github.com/ekkus93/aligngauge/releases/tag/v0.3.0)
- [v0.2.0 release](https://github.com/ekkus93/aligngauge/releases/tag/v0.2.0)
- [v0.1.0 release](https://github.com/ekkus93/aligngauge/releases/tag/v0.1.0)
- [Product and architecture specification](docs/DNA_QC_ENGINE_SPEC.md)
- [Implementation TODO](docs/DNA_QC_ENGINE_TODO.md)
- [v0.4 compatibility report](docs/evidence/V0_4_COMPATIBILITY_REPORT.md)
- [v0.4 release validation evidence](docs/evidence/V0_4_RELEASE_VALIDATION.md)
- [v0.4 release-scope ADR](docs/adr/ADR-0011-V0_4_RELEASE_SCOPE.md)
- [Milestone 13 exact-overlap evidence](docs/evidence/M13_EXACT_OVERLAP.md)
- [v0.3 targeted validation evidence](docs/evidence/V0_3_TARGETED_VALIDATION.md)
- [v0.2 CRAM validation evidence](docs/evidence/V0_2_CRAM_VALIDATION.md)
- [v0.1 validation report](docs/evidence/V0_1_VALIDATION_REPORT.md)
- [v0.1 performance report](docs/evidence/V0_1_PERFORMANCE_REPORT.md)
- [v0.1 release checklist](docs/evidence/V0_1_RELEASE_CHECKLIST.md)
- [Claude.ai specification review](docs/SPEC_REVIEW_2026-08-06.md)
- [AlignGauge naming decision](docs/ALIGNGAUGE_NAME_DECISION.md)

## License

Apache License 2.0.
