# AlignGauge

[![Permanent CI](https://github.com/ekkus93/aligngauge/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/ekkus93/aligngauge/actions/workflows/ci.yml)

A validation-first Rust engine for fast, single-pass alignment QC and coverage analysis across BAM and CRAM data for WGS, WES, and targeted sequencing.

AlignGauge analyzes existing aligned sequencing data. It is not software for controlling a physical DNA sequencer, a basecaller, an aligner, or a variant caller.

## Status

AlignGauge `v0.3.0` is the latest published release. The v0.3 tag is pinned by release policy to exact release SHA `eccd45157d34ada00a3403a2b24d606956878b62` (GitHub release ID `366930828`). That exact release SHA passed Permanent CI, Full Runtime Validation, Reference Validation, Targeted Validation, and HG002 Preparation Validation before publication. The v0.2 tag remains pinned by release policy to exact release/evidence SHA `ce3aa273da40c679c292e588584781ab1df241de`; the earlier v0.1 tag remains pinned to validated evidence SHA `9423a9d3496459fdbceb2e7bc5178b4b3100357c` (product implementation SHA `f93001cf22a2315f01e6b857c295720d99e392ca`).

- v0.1: local coordinate-sorted BAM, CPU counters, exact canonical coverage, JSON/provenance, and atomic output publication.
- v0.2: adds CRAM analysis with an explicit local FASTA, fail-closed SN/LN/M5 validation, actual local-reference provenance, BAM/CRAM canonical equivalence, and production HTSlib builds with remote reference transports excluded.
- v0.3: adds fail-closed BED3–BED12 target parsing, deterministic target normalization, `--targets <BED>`, `--near-distance <N>` (default 250), exact on/near/off-target partitioning, per-source target depth/dropout reporting, native target enrichment, and `target_uniformity_penalty_80`.
- Targeted analysis reuses the exact canonical chunked coverage sweep; counters, whole-genome coverage, and targeted reductions remain one alignment traversal.
- v0.3 makes no Picard `CollectHsMetrics`, `FOLD_ENRICHMENT`, or `FOLD_80_BASE_PENALTY` compatibility claim. Comparable target-depth primitives are instead validated exactly against pinned Samtools 1.24 under network isolation.
- CRAM reference mismatch or absence is fatal; AlignGauge does not silently fall back to inherited local or remote providers.
- Target BED contig/coordinate mismatch is fatal; AlignGauge does not infer chromosome aliases, silently drop unknown targets, or repair invalid source intervals.
- Standalone `inspect` and `validate-reference` commands remain deferred until separately specified.
- Milestone 10 is accepted: the pinned Samtools 1.24 `SN`/`IS` subset is exact and pinned MultiQC 1.35 consumes the generated surface equivalently.
- Milestone 11 is accepted: the pinned Picard 3.4.0 reference-independent alignment-summary subset and default `ALL_READS` insert-size profile match deterministic fixtures and HG002 exactly with no tolerance. Reference-dependent Picard alignment-summary fields remain unsupported rather than zero-filled.
- Milestone 12 — Picard WGS/hybrid-selection and MultiQC validation — is next. `v0.4.0` has not been released.
- v0.4+ compatibility expansion and full-scale production qualification are not part of the v0.3 release boundary.
- GPU/backend selection remains research-only until end-to-end benchmarks justify it.
- Collector execution remains deterministic and serial; `--threads >1` is accepted for configuration/provenance compatibility but emits an explicit warning rather than silently implying parallel collectors.

## Development

The repository pins Rust 1.97.1. After installing `rustup`:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
```

## Planning and evidence

- [v0.3.0 release](https://github.com/ekkus93/aligngauge/releases/tag/v0.3.0)
- [v0.2.0 release](https://github.com/ekkus93/aligngauge/releases/tag/v0.2.0)
- [v0.1.0 release](https://github.com/ekkus93/aligngauge/releases/tag/v0.1.0)
- [Product and architecture specification](docs/DNA_QC_ENGINE_SPEC.md)
- [Implementation TODO](docs/DNA_QC_ENGINE_TODO.md)
- [v0.3 targeted validation evidence](docs/evidence/V0_3_TARGETED_VALIDATION.md)
- [v0.2 CRAM validation evidence](docs/evidence/V0_2_CRAM_VALIDATION.md)
- [v0.1 validation report](docs/evidence/V0_1_VALIDATION_REPORT.md)
- [v0.1 performance report](docs/evidence/V0_1_PERFORMANCE_REPORT.md)
- [v0.1 release checklist](docs/evidence/V0_1_RELEASE_CHECKLIST.md)
- [Claude.ai specification review](docs/SPEC_REVIEW_2026-08-06.md)
- [AlignGauge naming decision](docs/ALIGNGAUGE_NAME_DECISION.md)

## License

Apache License 2.0.
