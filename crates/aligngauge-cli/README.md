# AlignGauge CLI modes

The v0.1 command-line surface deliberately keeps three explicit paths:

- `aligngauge qc --input <BAM>` retains the original three-counter walking-skeleton output for compatibility.
- `aligngauge qc --input <BAM> --format <...>` exposes counters-only differential/compatibility projections.
- Supplying v0.1 release options such as `--outdir` selects the full one-pass counters-plus-coverage release analysis and atomic output publication path.

The compatibility paths must not silently inherit release-mode requirements, and release-only options must not be combined with `--format`.

Correctness warnings are always emitted to stderr, including under `--quiet`; quiet mode suppresses only routine completion output. `--verbose` emits the resolved configuration using the selected diagnostic format so accepted-but-serial settings such as `--threads >1` are visible rather than silently ignored.

Permanent runtime and release E2E validation exercise these paths independently so compatibility behavior cannot silently acquire release-mode requirements.
