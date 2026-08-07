# AlignGauge CLI modes

AlignGauge retains the original compatibility paths while the full release path now supports both BAM and CRAM:

- `aligngauge qc --input <BAM>` retains the original three-counter walking-skeleton output for compatibility.
- `aligngauge qc --input <BAM> --format <...>` exposes counters-only differential/compatibility projections.
- Supplying release options such as `--outdir` selects the full one-pass counters-plus-coverage analysis and atomic output publication path.
- Full CRAM analysis uses the same validated collector path and requires `--reference <FASTA>`.

CRAM reference handling is fail closed. The explicit local FASTA is validated against required contig names, lengths, and `M5` identities before reference-dependent record traversal. Missing or mismatched references fail with typed errors; the implementation does not silently substitute `REF_PATH`, `REF_CACHE`, `HTS_PATH`, a remote reference service, or another local FASTA. Supplying `--reference` for BAM is rejected rather than silently ignored.

The compatibility paths must not silently inherit release-mode requirements, and release-only options must not be combined with `--format`.

Correctness warnings are always emitted to stderr, including under `--quiet`; quiet mode suppresses only routine completion output. `--verbose` emits the resolved configuration using the selected diagnostic format so accepted-but-serial settings such as `--threads >1` are visible rather than silently ignored.

Standalone `inspect` and `validate-reference` commands are deferred beyond v0.2 until their CLI, output-schema, and error contracts are independently specified and tested. The v0.2 reference-integrity surface is `qc --reference <FASTA>` plus the shared validation API.

Permanent runtime and release E2E validation exercise the compatibility and release paths independently so compatibility behavior cannot silently acquire release-mode requirements. v0.2 CRAM/reference evidence is recorded in `docs/evidence/V0_2_CRAM_VALIDATION.md`. `v0.2.0` is released and pinned by release policy to exact validated release/evidence SHA `ce3aa273da40c679c292e588584781ab1df241de`.
