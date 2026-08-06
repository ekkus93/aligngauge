# AlignGauge

A validation-first Rust engine for fast, single-pass alignment QC and coverage analysis across BAM and CRAM data for WGS, WES, and targeted sequencing.

AlignGauge analyzes existing aligned sequencing data. It is not software for controlling a physical DNA sequencer, a basecaller, an aligner, or a variant caller.

## Status

Implementation is beginning from the staged roadmap in `docs/DNA_QC_ENGINE_TODO.md`.

- v0.1: coordinate-sorted BAM, CPU counters, exact canonical coverage, JSON, and provenance.
- v0.2: CRAM with strictly local reference resolution.
- v0.3+: targeted metrics, compatibility expansion, and full-scale qualification.
- GPU work is research-only until an end-to-end benchmark proves value.

## Development

The repository pins Rust 1.97.1. After installing `rustup`:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
```

## Planning documents

- [Product and architecture specification](docs/DNA_QC_ENGINE_SPEC.md)
- [Implementation TODO](docs/DNA_QC_ENGINE_TODO.md)
- [Claude.ai specification review](docs/SPEC_REVIEW_2026-08-06.md)
- [AlignGauge naming decision](docs/ALIGNGAUGE_NAME_DECISION.md)

## License

Apache License 2.0.
