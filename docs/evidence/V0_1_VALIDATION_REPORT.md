# AlignGauge v0.1 Validation Report

## Status

**Release candidate validated.** This report covers the v0.1 BAM/CPU release candidate at product SHA `f93001cf22a2315f01e6b857c295720d99e392ca`. The release tag must not be created until the evidence commit containing this report passes the same exact-SHA validation gates.

## Exact product-SHA signoff

The following workflows all completed successfully on `f93001cf22a2315f01e6b857c295720d99e392ca`:

- Permanent CI: run `31162822206`, job `92816809733`.
- Full Runtime Validation: run `31162821531`, job `92816808007`.
- Reference Validation: run `31162821541`, job `92816808194`.
- HG002 Preparation Validation: run `31162819757`, job `92816800461`.

The HG002 run also produced artifact `8987812158`, digest `sha256:8058c031c80dc6bc736ce744b3349c2a669c30c43212ab7449114ef8c03d2a06`.

## Release-path hardening evidence

A read-only candidate gate, run `31160870468` / job `92810662686`, passed strict Clippy, the complete workspace tests, the v0.1 release fault/E2E suite, live JSON-Schema validation, and rustdoc before the validated files were published.

Final CLI-option validation passed in run `31162669950` / job `92816332875`. It verifies the complete v0.1 option surface, help text, configuration-file execution, JSON diagnostics, explicit rejection of future options, compatibility/release mode separation, correctness warnings under `--quiet`, and resolved-configuration output under `--verbose`.

## SPEC §19.1 mapping

| Criterion | Evidence |
| --- | --- |
| 1. Walking skeleton replaced by production code | Full v0.1 release path composes validated reader, counters, coverage, provenance, and atomic publication. The original three-counter CLI is retained only as an explicit compatibility path. |
| 2. Coordinate-sorted BAM validated | Milestone 3 reader validation plus permanent fixture suite. |
| 3. Corrupt/truncated/unsorted fail correctly | `release_v0_1` E2E tests and Full Runtime invalid-fixture matrix. Failures publish no plausible stdout/output directory. |
| 4. Flag classification matches pinned Samtools | Exact synthetic and HG002 `flagstat` differential gates. |
| 5. Per-reference counts match pinned profile | Exact synthetic and HG002 `idxstats` differential gates. |
| 6. Canonical coverage matches `aligngauge-v0.1` | Exact Samtools depth baseline under ADR-0003 plus synthetic/HG002 coverage differentials. |
| 7. Chunk boundaries do not alter results | Milestone 5 chunk-size invariance/property tests. |
| 8. Multi-track memory planning enforced | Milestone 5 planner tests, low-memory rejection, sparse large-reference runtime gate, and RSS evidence. |
| 9. Canonical JSON/provenance validate | Committed schemas plus live schema validation of an actual release publication. Coverage now includes per-reference reductions and cumulative threshold percentages. |
| 10. Missing metrics never become zero | `Availability` contract tests and fail-closed Samtools idxstats exporter: unavailable required source data returns `compatibility_unavailable` rather than zero. |
| 11. Atomic output publication | Atomic publisher fault injection at every checkpoint plus release publication failure test; `_SUCCESS` is synchronized inside staging last before atomic rename. |
| 12. Repeated canonical output deterministic | Release E2E compares repeated summaries/provenance after removing explicitly volatile `stage_timings_ns`. |
| 13. HG002 reconciled | Exact counters and coverage differentials plus complete one-pass v0.1 HG002 release in run `31162819757`. |
| 14. Permanent CI exact release candidate | Permanent CI run `31162822206` succeeded on the product SHA; the evidence SHA must also pass before tagging. |
| 15. Non-goals/limitations documented | README, CLI mode documentation, this report, and release checklist state the v0.1 boundaries. |

## v0.1 E2E failure matrix

The committed `release_v0_1` integration suite exercises:

- valid synthetic BAM;
- empty BAM;
- corrupt/truncated BAM;
- coordinate-unsorted BAM;
- existing output destination with sentinel preservation;
- output permission failure where the platform permits the test;
- injected counter/coverage collector failure;
- injected serialization checkpoint failure;
- injected atomic-publication failure;
- deterministic repeat comparison;
- optional Samtools compatibility-file publication.

The HG002 workflow separately executes the complete release CLI on the prepared real-data subset and requires `summary.json`, `provenance.json`, and `_SUCCESS`.

## Fail-closed and observability decisions

- No missing counter or coverage value is converted to zero.
- The Samtools idxstats exporter fails with `compatibility_unavailable` if a required per-reference unmapped value is unavailable.
- Reader, collector, serialization, and publication failures abort the operation; no completed destination is exposed.
- `--threads >1` does not silently imply collector parallelism. v0.1 remains a deterministic single collector thread and emits canonical plus stderr warning `collector_threads_serial_v0_1`.
- `--quiet` suppresses routine completion output but never correctness warnings.
- `--verbose` emits the resolved configuration in the selected diagnostic format.
- v0.2/v0.3/backend options are rejected explicitly rather than ignored.
- Compatibility `--format` mode cannot silently switch into the full release path.

## v0.1 scope and known limitations

v0.1 supports local coordinate-sorted BAM input, CPU execution, one validated streaming BAM reader, alignment/per-reference counters, exact canonical coverage, canonical JSON/provenance, and atomic output publication.

Not released in v0.1:

- CRAM or reference-resolution behavior (v0.2);
- BED/targeted sequencing metrics (v0.3);
- GPU/backend selection;
- exact mate-overlap correction;
- indexed multi-reader parallel traversal;
- parallel collector execution despite accepting a configured `threads` value for forward-compatible configuration/provenance.

AlignGauge performs no implicit network retrieval during ordinary v0.1 analysis and has no telemetry by default.

Repository-governance caveats outside the executable remain visible: the GitHub About description is stale, and branch protection is not enabled. Neither is treated as evidence of runtime correctness.

## Differential disposition

No unexplained applicable integer discrepancy remains for pinned Samtools 1.24 `flagstat`, `idxstats`, or ADR-0003 depth semantics. No blanket tolerance is used for canonical integer comparisons.
