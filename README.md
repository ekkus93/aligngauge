# AlignGauge

[![Permanent CI](https://github.com/ekkus93/aligngauge/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/ekkus93/aligngauge/actions/workflows/ci.yml)

A validation-first Rust engine for fast, single-pass alignment QC and coverage analysis across BAM and CRAM data for WGS, WES, and targeted sequencing.

AlignGauge analyzes existing aligned sequencing data. It is not software for controlling a physical DNA sequencer, a basecaller, an aligner, or a variant caller.

## Status

AlignGauge `v0.1.0` is released and remains the latest published tag while the `v0.2.0` release candidate completes exact-SHA validation. The v0.1 tag is pinned by release policy to validated evidence SHA `9423a9d3496459fdbceb2e7bc5178b4b3100357c` (product implementation SHA `f93001cf22a2315f01e6b857c295720d99e392ca`).

The v0.2 CRAM/reference-integrity implementation and its specification reconciliation were merged to `master` at `72dc4ca1fb64c5f49be984ed9c2fac99e0cb64b0`. The `v0.2.0` tag is intentionally pending until the release-candidate commit containing this status and the committed v0.2 evidence passes the permanent exact-SHA release gates.

- v0.1: local coordinate-sorted BAM, CPU counters, exact canonical coverage, JSON/provenance, and atomic output publication.
- v0.2 release candidate: adds CRAM analysis with an explicit local FASTA, fail-closed SN/LN/M5 validation, actual local-reference provenance, BAM/CRAM canonical equivalence, and production HTSlib builds with remote reference transports excluded.
- CRAM reference mismatch or absence is fatal; AlignGauge does not silently fall back to inherited local or remote providers.
- Standalone `inspect` and `validate-reference` commands are deferred beyond v0.2 until separately specified; the v0.2 integrity surface is the released `qc --reference <FASTA>` path and shared validation API.
- v0.3+: targeted metrics, compatibility expansion, and full-scale qualification are not part of v0.2.
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

- [v0.1.0 release](https://github.com/ekkus93/aligngauge/releases/tag/v0.1.0)
- [Product and architecture specification](docs/DNA_QC_ENGINE_SPEC.md)
- [Implementation TODO](docs/DNA_QC_ENGINE_TODO.md)
- [v0.2 CRAM validation evidence](docs/evidence/V0_2_CRAM_VALIDATION.md)
- [v0.1 validation report](docs/evidence/V0_1_VALIDATION_REPORT.md)
- [v0.1 performance report](docs/evidence/V0_1_PERFORMANCE_REPORT.md)
- [v0.1 release checklist](docs/evidence/V0_1_RELEASE_CHECKLIST.md)
- [Claude.ai specification review](docs/SPEC_REVIEW_2026-08-06.md)
- [AlignGauge naming decision](docs/ALIGNGAUGE_NAME_DECISION.md)

## License

Apache License 2.0.
