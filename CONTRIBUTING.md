# Contributing to AlignGauge

AlignGauge is validation-first. Changes that alter metric semantics, input handling, compatibility claims, or fallback behavior must update the specification before implementation.

## Development requirements

1. Use the Rust toolchain pinned in `rust-toolchain.toml`.
2. Add tests for new behavior and failure paths.
3. Keep the CPU implementation authoritative.
4. Do not add silent fallback, implicit network access, or zero-as-missing-data behavior.
5. Run the permanent local gates documented in `README.md`.
6. Include evidence for milestone completion under `docs/evidence/`.

## Pull requests

Keep changes narrowly scoped. Describe the affected specification section, tests, compatibility implications, and any known limitations. Generated files must be committed and the working tree must remain clean after tests.
