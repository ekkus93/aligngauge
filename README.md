# AlignGauge

[![Permanent CI](https://github.com/ekkus93/aligngauge/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/ekkus93/aligngauge/actions/workflows/ci.yml)

A validation-first Rust engine for fast, single-pass alignment QC and coverage analysis across BAM and CRAM data for WGS, WES, and targeted sequencing.

AlignGauge analyzes existing aligned sequencing data. It is not software for controlling a physical DNA sequencer, a basecaller, an aligner, or a variant caller.

## Status

The v0.1 BAM/CPU release candidate is implemented and has passed its product-SHA validation gates. Release evidence is being validated before the `v0.1.0` tag is created.

- v0.1: local coordinate-sorted BAM, CPU counters, exact canonical coverage, JSON/provenance, and atomic output publication.
- v0.2: CRAM with strictly local reference resolution; not released in v0.1.
- v0.3+: targeted metrics, compatibility expansion, and full-scale qualification; not released in v0.1.
- GPU/backend selection is not a released v0.1 feature and remains research-only until end-to-end benchmarks justify it.
- v0.1 collector execution is deterministic and serial; `--threads >1` is accepted for configuration/provenance compatibility but emits an explicit warning rather than silently implying parallel collectors.

## Development

The repository pins Rust 1.97.1. After installing `rustup`:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
```

## Planning and evidence

- [Product and architecture specification](docs/DNA_QC_ENGINE_SPEC.md)
- [Implementation TODO](docs/DNA_QC_ENGINE_TODO.md)
- [v0.1 validation report](docs/evidence/V0_1_VALIDATION_REPORT.md)
- [v0.1 performance report](docs/evidence/V0_1_PERFORMANCE_REPORT.md)
- [v0.1 release checklist](docs/evidence/V0_1_RELEASE_CHECKLIST.md)
- [Claude.ai specification review](docs/SPEC_REVIEW_2026-08-06.md)
- [AlignGauge naming decision](docs/ALIGNGAUGE_NAME_DECISION.md)

## License

Apache License 2.0.