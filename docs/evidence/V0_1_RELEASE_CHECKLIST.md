# AlignGauge v0.1 Release Checklist

## Candidate identity

- Product SHA: `f93001cf22a2315f01e6b857c295720d99e392ca`.
- Release line: `v0.1` / intended tag `v0.1.0`.
- Input format released: local coordinate-sorted BAM.
- Execution model: CPU, one validated streaming BAM reader, deterministic collectors/reduction.

The release tag is intentionally pending until the evidence commit containing this checklist passes exact-SHA validation.

## Required product gates

All four product-SHA gates succeeded on `f93001cf22a2315f01e6b857c295720d99e392ca`:

- [x] Permanent CI — run `31162822206`, job `92816809733`.
- [x] Full Runtime Validation — run `31162821531`, job `92816808007`.
- [x] Reference Validation — run `31162821541`, job `92816808194`.
- [x] HG002 Preparation Validation — run `31162819757`, job `92816800461`.

## SPEC §19.1 acceptance

- [x] Production reader/collectors/release path implemented.
- [x] Coordinate-sorted BAM validation enforced.
- [x] Corrupt, truncated, and unsorted inputs fail with typed categories.
- [x] Flag classification reconciled with pinned Samtools 1.24.
- [x] Per-reference counts reconciled with pinned Samtools 1.24.
- [x] Canonical exact coverage reconciled with ADR-0003 semantics.
- [x] Chunk-size invariance demonstrated.
- [x] Coverage memory planning and low-memory rejection demonstrated.
- [x] Actual release JSON/provenance validate against committed schemas.
- [x] Missing/unavailable metrics are never silently represented as zero.
- [x] Atomic publication and `_SUCCESS` ordering are fault-injection tested.
- [x] Canonical results are deterministic after explicitly excluding timing fields.
- [x] HG002 small-subset counters, coverage, and complete release run are reconciled.
- [x] Permanent CI passes on the exact product SHA.
- [x] Non-goals and known limitations are documented.

## CLI and output behavior

- [x] Original `qc --input <BAM>` compatibility path remains stable.
- [x] Full v0.1 release options are parsed and tested.
- [x] `--help` lists v0.1 and deferred options.
- [x] v0.2/v0.3/backend options fail explicitly rather than being ignored.
- [x] Output destination is preflighted before traversal.
- [x] Canonical `summary.json` includes alignment counters, per-reference counters, exact coverage, per-reference coverage reductions, threshold counts, and threshold percentages.
- [x] Canonical provenance records resolved configuration, field/analysis plan, one-pass traversal, coverage strategy/memory plan, backend identities, resource limits, warnings, and stage timings.
- [x] Optional Samtools compatibility files are generated only from available source metrics.
- [x] Compatibility export fails with `compatibility_unavailable` rather than manufacturing zero.
- [x] `--quiet` does not suppress correctness warnings.
- [x] `--verbose` exposes the resolved configuration.
- [x] `--threads >1` is explicitly reported as serial collector execution in v0.1.

## E2E and failure behavior

- [x] Valid synthetic BAM.
- [x] Empty BAM.
- [x] Corrupt/truncated BAM.
- [x] Unsorted BAM.
- [x] Existing destination preserved and rejected.
- [x] Permission failure path exercised where supported by the test platform.
- [x] Injected collector failure.
- [x] Injected serialization failure.
- [x] Injected publication failure.
- [x] HG002 complete v0.1 release publication.
- [x] No completed destination after analysis/publication failure.

## Performance evidence

- [x] Simple rust-htslib traversal measured.
- [x] Counters-only path measured.
- [x] Coverage-only path measured.
- [x] Combined counters+coverage path measured.
- [x] Combined path reports exactly one BAM traversal.
- [x] CPU, RAM, storage, cache state, runner, Rust, and Cargo versions recorded.
- [x] One warmup plus three measured repetitions per mode.
- [x] No unsupported speedup claim.

See `docs/evidence/V0_1_PERFORMANCE_REPORT.md` for measured values.

## Security/privacy and release boundaries

- [x] Ordinary v0.1 analysis performs local processing with no implicit reference/network retrieval.
- [x] No telemetry by default.
- [x] Routine diagnostics redact read names by default.
- [x] Atomic staging uses restrictive local permissions.
- [x] CRAM/reference retrieval remains deferred to v0.2.
- [x] BED/targeted metrics remain deferred to v0.3.
- [x] GPU/backend selection is not a released v0.1 feature.
- [x] Exact mate-overlap correction remains deferred.
- [x] Indexed multi-reader parallelism remains deferred.

## Repository governance caveats

These are visible repository-administration issues, not hidden runtime fallbacks:

- GitHub About still says `A DNA sequencer in Rust` because the available connector cannot edit repository metadata.
- Branch protection is not enabled.

Neither caveat changes the executable's validation result, but neither should be represented as complete governance work.

## Tag/release rule

- [ ] Validate the evidence commit with Permanent CI, Full Runtime, Reference Validation, and HG002.
- [ ] Only after that exact SHA is green, create tag/release `v0.1.0` pointing to the validated evidence SHA.
- [ ] Never move the release tag to an unvalidated commit.
