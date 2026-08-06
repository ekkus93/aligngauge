# AlignGauge Implementation TODO

**Repository:** `ekkus93/aligngauge`

**Companion specification:** `docs/DNA_QC_ENGINE_SPEC.md`  
**Status:** Ralph Loop active — Milestone 2 complete; Milestone 3 next
**Last updated:** 2026-08-06  
**Supersedes:** Initial `DNA_QC_ENGINE_TODO.md` dated 2026-08-05

## 1. How to use this TODO

The specification is authoritative for semantics, fixture requirements, error
categories, input/output contracts, and release criteria. This TODO owns task order,
implementation decomposition, and evidence.

A milestone may be marked complete only when:

- its implementation tasks are complete;
- its required tests pass;
- its evidence document is committed;
- permanent CI passes on the exact claimed commit;
- known discrepancies are documented rather than hidden.

Individual checkbox completion does not require a separate CI claim. CI evidence is
recorded at milestone boundaries.

## 2. Global engineering rules

These rules apply to all milestones:

- [ ] Never encode missing or unavailable data as zero.
- [ ] Never swallow a collector, parser, I/O, validation, or output error.
- [ ] Never add a fallback without an explicit policy, test, diagnostic, and
      provenance field.
- [ ] Never silently switch from exact to approximate semantics.
- [ ] Never fetch references, test data, or other content implicitly at runtime.
- [ ] Never publish partial outputs as complete.
- [ ] Use checked arithmetic for coordinates, lengths, counters, and allocation
      sizes.
- [ ] Keep the CPU path authoritative.
- [ ] Derive compatibility output from the canonical model where possible.
- [ ] Pin reference-tool versions used for differential claims.
- [ ] Treat fixture and benchmark identity as part of the evidence.
- [ ] Update the specification before implementing a semantic change.
- [ ] Do not introduce GPU-facing CLI/config/schema surface before an acceleration
      spike passes the admission gate in SPEC §4.7.

## 3. Release map

| Release | Product boundary |
|---|---|
| v0.1 | BAM, CPU, counters, exact canonical coverage, JSON/provenance |
| v0.2 | CRAM and fail-closed local reference validation |
| v0.3 | WES and targeted-panel metrics |
| v0.4 | Expanded Samtools/Picard/MultiQC compatibility |
| v0.5 | Full-scale HG002 and production-beta qualification |
| Research | Optional hardware acceleration; no release obligation |

---

# Part I — v0.1 BAM CPU foundation

## Milestone 0 — Repository identity and minimum controls

### 0.1 Project identity

- [ ] Rename the GitHub repository to `aligngauge`, or record an ADR explaining why
      the old name is retained.
- [ ] Set the repository description to the wording in SPEC §1.2.
- [ ] Update README title and product boundary.
- [ ] Reserve the binary name `aligngauge`.
- [ ] Use `aligngauge-*` crate prefixes; do not use the old `rds-*` or
      `bamgauge-*` prefixes.

### 0.2 Minimal repository files

- [ ] Add `rust-toolchain.toml` with a pinned stable channel.
- [ ] Add workspace `Cargo.toml`.
- [ ] Add `README.md`.
- [ ] Add `CONTRIBUTING.md`.
- [ ] Add `SECURITY.md`.
- [ ] Add `CODE_OF_CONDUCT.md` if public contributions are invited.
- [ ] Add `docs/adr/`.
- [ ] Add `docs/evidence/`.

### 0.3 Initial CI

Keep initial CI small enough to maintain.

- [ ] Build on Linux.
- [ ] Run unit and integration tests.
- [ ] Run `cargo fmt --check`.
- [ ] Run Clippy with warnings denied.
- [ ] Run `cargo doc` with warnings denied.
- [ ] Verify repository-generated files are clean after tests.
- [ ] Pin third-party GitHub Actions to immutable SHAs.
- [ ] Use least-privilege workflow permissions.
- [ ] Disable persisted checkout credentials on read-only jobs.

### 0.4 Milestone evidence

Create `docs/evidence/M0_REPOSITORY_FOUNDATION.md` containing:

- exact commit;
- toolchain;
- CI run and job IDs;
- repository name decision;
- known deferred controls.

### Milestone 0 acceptance gate

- [ ] Permanent CI succeeds on the exact evidence commit.
- [ ] README accurately states that AlignGauge analyzes aligned data and is not a
      physical sequencer or basecaller.

---

## Milestone 0.5 — Walking skeleton

This milestone intentionally precedes elaborate abstractions.

### 0.5.1 Minimal vertical slice

- [ ] Add one CLI crate and only the minimum supporting module structure.
- [ ] Implement:

  ```bash
  aligngauge qc --input sample.bam
  ```

- [ ] Open BAM through pinned `rust-htslib`.
- [ ] Traverse all records.
- [ ] Count total, mapped, and unmapped records.
- [ ] Print the three counts to stdout.
- [ ] Return nonzero for a missing file.
- [ ] Return nonzero for a truncated file.
- [ ] Do not add the final JSON schema yet.
- [ ] Do not add the final planner abstraction yet.
- [ ] Do not add a staging directory yet.

### 0.5.2 Probe backend ergonomics

Record findings about:

- [ ] record borrowing and reuse;
- [ ] CIGAR access cost;
- [ ] tag access behavior;
- [ ] error propagation on truncation;
- [ ] multithreaded decoding controls;
- [ ] long-CIGAR/`CG` behavior;
- [ ] whether a normalized record view is needed.

Write `docs/adr/ADR-0001-HTSLIB_RECORD_BOUNDARY.md`.

### 0.5.3 Walking-skeleton tests

- [ ] Tiny valid mapped/unmapped BAM.
- [ ] Empty valid BAM.
- [ ] Truncated BGZF block.
- [ ] Malformed BAM record if the backend allows a stable fixture.
- [ ] Assert nonzero exit and no plausible-looking counts on corruption.

### Milestone 0.5 acceptance gate

- [ ] The CLI-to-HTSlib-to-result path works end to end.
- [ ] Backend ergonomics are documented before workspace boundaries are frozen.
- [ ] CI succeeds on the exact evidence commit.
- [ ] Create `docs/evidence/M0_5_WALKING_SKELETON.md`.

---

## Milestone 1 — Core model, errors, and atomic output

**Status:** Complete — evidence SHA `ffafa45c1d6dea99c50f61e05498690d594bae27`; Permanent CI run `31095937384`, job `92597853728`, success.

### 1.1 Error taxonomy

Implement stable categories from SPEC §14.

- [x] Typed error enum.
- [x] Stable exit-code mapping.
- [x] Human rendering.
- [x] JSON rendering.
- [x] Source-chain preservation.
- [x] Redaction of read names by default.
- [x] Tests for every category used in v0.1.

Do not duplicate the category list here; tests shall iterate the specification-owned
mapping encoded in source.

### 1.2 Configuration

- [x] Typed v0.1 configuration.
- [x] Deterministic precedence from SPEC §6.3.
- [x] Unknown-key rejection.
- [x] Memory-limit parser with checked units; CLI exposure remains Milestone 6.
- [x] Coverage-threshold parser with duplicate/order normalization.
- [x] Resolved-config serialization for provenance.
- [x] Configuration schema/version field.

### 1.3 Canonical output models

- [x] `summary.json` Rust types.
- [x] `provenance.json` Rust types.
- [x] Explicit unavailable-value representation.
- [x] Stable ordering.
- [x] JSON schemas.
- [x] Golden serialization fixtures.
- [x] Tests proving `None`/unavailable cannot serialize as metric zero.

### 1.4 Atomic publication

Implement the exact ordering in SPEC §10.2.

- [x] Same-filesystem staging directory.
- [x] Required-file flush.
- [x] `_SUCCESS` written inside staging last.
- [x] Staging metadata synchronization where supported.
- [x] Atomic rename.
- [x] Destination-exists policy captured in ADR.
- [x] Cleanup on error.
- [x] Preserve-failed-staging policy and resolved configuration field; CLI exposure remains Milestone 6.
- [x] Fault-injection tests at every publication step.
- [x] Observer test proving the destination never exposes a partially built run.

### 1.5 Milestone evidence

- [x] Created `docs/evidence/M1_CORE_CONTRACTS.md`.

### Milestone 1 acceptance gate

- [x] Error and output contracts pass tests.
- [x] Atomic publication survives injected failures.
- [x] Permanent CI succeeds on the exact evidence commit.

---

## Milestone 2 — Test corpus and differential harness

**Status:** Complete — implementation SHA `45211236419e5bebc7c0d09d5cb35d65174cc11a`; Permanent CI run `31100841806`, job `92613749893`; Reference Validation run `31100842135`, job `92613751393`; HG002 Preparation run `31100844104`, job `92613759310`; all successful.

### 2.1 Test-data manifest

- [x] Define a versioned manifest format.
- [x] Record source, checksums, generation commands, reference build, and licensing.
- [x] Enforce local checksum verification.
- [x] Refuse implicit downloads during ordinary tests.
- [x] Provide explicit preparation commands.
- [x] Keep large data out of Git.

### 2.2 Synthetic fixture generator

Implement all cases owned by SPEC §15.1.

- [x] Generate fixtures deterministically.
- [x] Generate indexes where needed.
- [x] Include expected validity/error category.
- [x] Include expected canonical metrics where applicable.
- [x] Include long-CIGAR/`CG` fixture.
- [x] Include coordinate-regression fixture.
- [x] Include chunk-boundary fixtures.
- [x] Include multi-track memory fixture.

### 2.3 Reference-tool environment

- [x] Pin Samtools version and container digest.
- [x] Add scripts for `flagstat` and `idxstats`.
- [x] Select and document the v0.1 coverage baseline in `ADR-0003-COVERAGE_BASELINE.md`.
- [x] Run differential tools in a network-disabled sandbox.
- [x] Capture stdout, stderr, exit status, wall time, and tool version.
- [x] Fail when a baseline command fails or emits an incomplete artifact.

### 2.4 HG002 small subset

- [x] Add explicit preparation script for the selected chr20 region.
- [x] Pin source URL/accession and checksum.
- [x] Pin downsampling seed/fraction.
- [x] Validate local SHA-256.
- [x] Record the matching reference build.
- [x] Generate a manifest entry and evidence README.
- [x] Do not commit the large source alignment.

### 2.5 Differential harness

- [x] Structured expected-result format.
- [x] Field-level integer comparison.
- [x] Explicit float rounding comparison where needed.
- [x] Machine-readable discrepancy report.
- [x] No blanket tolerances.
- [x] Every expected difference requires a named compatibility note.

### Milestone 2 acceptance gate

- [x] All synthetic fixtures regenerate identically.
- [x] Baseline tools run network-isolated.
- [x] HG002 subset preparation is reproducible.
- [x] Create `docs/evidence/M2_TEST_CORPUS.md`.
- [x] Permanent CI succeeds on the exact evidence commit.

---

## Milestone 3 — Production BAM reader and validation

### 3.1 I/O boundary

- [ ] Refine the walking-skeleton boundary based on ADR-0001.
- [ ] Wrap rust-htslib errors without losing causal detail.
- [ ] Reuse record buffers.
- [ ] Expose only required validated fields.
- [ ] Bound allocations derived from headers and records.
- [ ] Record pinned HTSlib/rust-htslib versions.

### 3.2 Header validation

- [ ] Parse references and lengths.
- [ ] Parse sort-order metadata.
- [ ] Parse read-group records without trusting their values.
- [ ] Detect duplicate/contradictory reference declarations.
- [ ] Detect invalid lengths and arithmetic overflow.
- [ ] Produce header identity for provenance.

### 3.3 Coordinate validation

- [ ] Validate nondecreasing mapped coordinates.
- [ ] Define and test unmapped-tail behavior.
- [ ] Reject reference-ID regressions.
- [ ] Reject position regressions.
- [ ] Produce redacted actionable diagnostics.
- [ ] Test header-claims-coordinate but records-regress.
- [ ] Test absent/unknown sort-order with actually sorted records.

### 3.4 Record validation

- [ ] Checked CIGAR/reference-span arithmetic.
- [ ] Reference-bound validation.
- [ ] Invalid flag-combination policy where required.
- [ ] Oversized-CIGAR policy from SPEC §7.4.
- [ ] Malformed optional-tag behavior.
- [ ] Unknown read-group behavior.
- [ ] No silent use of missing `NM`/`MD` as zero.

### 3.5 Required-field planning

Keep v0.1 planning minimal.

- [ ] Determine fields required by counters.
- [ ] Determine fields required by coverage.
- [ ] Avoid sequence/quality decoding when not needed and supported.
- [ ] Record the resolved field plan in provenance.
- [ ] Do not introduce backend or GPU dimensions.

### Milestone 3 acceptance gate

- [ ] All valid v0.1 fixtures stream correctly.
- [ ] Every corrupt/unsorted/unsupported fixture fails with its expected category.
- [ ] No completed output is published after reader failure.
- [ ] Create `docs/evidence/M3_BAM_VALIDATION.md`.
- [ ] Permanent CI succeeds on the exact evidence commit.

---

## Milestone 4 — Flag and per-reference counters

### 4.1 Classification model

- [ ] Implement mutually defined primary/secondary/supplementary classification.
- [ ] Match pinned Samtools priority for dual-flag records.
- [ ] Implement QC-pass/QC-fail partitions.
- [ ] Implement mapped/unmapped, paired, proper-pair, read1/read2, mate, duplicate,
      and singleton counters required by SPEC §11.
- [ ] Use checked `u64`.
- [ ] Prohibit saturating arithmetic.

### 4.2 Per-reference model

- [ ] Mapped counts by reference.
- [ ] Unmapped/no-coordinate counts under the pinned profile.
- [ ] Stable reference ordering from the header.
- [ ] Empty reference behavior.
- [ ] Unknown/invalid reference-ID failure.

### 4.3 Canonical integration

- [ ] Map counters into `summary.json`.
- [ ] Record profile/tool version in provenance.
- [ ] Explicitly mark counters not collected or not applicable.
- [ ] Add human summary rendering.
- [ ] Derive initial Samtools-like compatibility text from canonical values.

### 4.4 Differential validation

- [ ] Compare every applicable synthetic fixture to pinned `samtools flagstat`.
- [ ] Compare per-reference fixture results to pinned `samtools idxstats`.
- [ ] Reconcile the dual-secondary/supplementary case.
- [ ] Compare the HG002 subset.
- [ ] Document every non-match.

### Milestone 4 acceptance gate

- [ ] No unexplained integer discrepancy remains.
- [ ] Repeated runs produce identical canonical counters.
- [ ] Create `docs/evidence/M4_COUNTERS.md`.
- [ ] Permanent CI succeeds on the exact evidence commit.

---

## Milestone 5 — Exact chunked coverage

### 5.1 Coverage event generation

Implement SPEC §12.1–§12.2.

- [ ] Apply the `aligngauge-v0.1` record policy.
- [ ] Emit blocks for `M`, `=`, and `X`.
- [ ] Exclude `I`, `D`, `N`, `S`, `H`, and `P`.
- [ ] Use checked coordinate arithmetic.
- [ ] Reject blocks outside reference bounds.
- [ ] Test each CIGAR operation and combination.
- [ ] Test very long `D`/`N`.
- [ ] Test chunk-crossing blocks.

### 5.2 Chunked accumulator

- [ ] Implement one parameterized chunked sweep.
- [ ] Carry exact depth across chunk boundaries.
- [ ] Store pending future-end events safely.
- [ ] Flush completed chunks deterministically.
- [ ] Handle empty contigs and contig transitions.
- [ ] Select chunk size from the memory plan.
- [ ] Record chunk size and strategy in provenance.
- [ ] Do not create separate whole-contig and target algorithms.

### 5.3 Memory planner

- [ ] Account for all active coverage tracks.
- [ ] Account for delta entries.
- [ ] Account for pending cross-chunk events.
- [ ] Account for reader and output buffers.
- [ ] Include a safety margin.
- [ ] Reject an impossible plan before traversal.
- [ ] Test one-track and multi-track estimates.
- [ ] Test low-memory failure.
- [ ] Verify observed peak RSS against planned bounds with documented tolerance.

### 5.4 Histogram and threshold reduction

- [ ] Integer depth histogram.
- [ ] Accepted aligned-base count.
- [ ] Covered/uncovered reference bases.
- [ ] Per-reference mean depth.
- [ ] Configurable cumulative thresholds.
- [ ] Deterministic CPU-side floating-point finalization.
- [ ] Document median/percentile rounding if implemented.

### 5.5 Differential and property testing

- [ ] Compare against ADR-0002 baseline.
- [ ] Property: changing chunk size does not change canonical results.
- [ ] Property: sum of histogram counts equals evaluated reference territory.
- [ ] Property: sum of depth × count equals accepted covered-base total under the
      selected semantics.
- [ ] Property: adding an excluded record does not change coverage.
- [ ] Fuzz CIGAR-to-block conversion.
- [ ] Validate HG002 subset.

### Milestone 5 acceptance gate

- [ ] All chunk sizes tested yield identical canonical output.
- [ ] No unresolved coverage discrepancy remains.
- [ ] Memory-limit enforcement is demonstrated.
- [ ] Create `docs/evidence/M5_COVERAGE.md`.
- [ ] Permanent CI succeeds on the exact evidence commit.

---

## Milestone 6 — v0.1 release integration

### 6.1 CLI completion

- [ ] Implement final v0.1 options from SPEC §6.2.
- [ ] Helpful `--help`.
- [ ] Stable nonzero exit codes.
- [ ] Refuse v0.2/v0.3 options such as CRAM reference or targets with clear
      unsupported-feature diagnostics.
- [ ] Validate output destination before expensive traversal.

### 6.2 Output completion

- [ ] Finalize v0.1 JSON schemas.
- [ ] Finalize provenance.
- [ ] Finalize human summary.
- [ ] Finalize optional Samtools-like compatibility files.
- [ ] Ensure compatibility exporters fail when required source metrics are absent.
- [ ] Exclude volatile timing fields from deterministic-result comparison.

### 6.3 End-to-end tests

- [ ] Valid synthetic BAM.
- [ ] Empty BAM.
- [ ] Corrupt BAM.
- [ ] Unsorted BAM.
- [ ] Output destination exists.
- [ ] Permission failure.
- [ ] Injected collector failure.
- [ ] Injected serialization failure.
- [ ] Injected publication failure.
- [ ] HG002 subset complete run.

### 6.4 v0.1 performance baseline

- [ ] Measure simple rust-htslib traversal.
- [ ] Measure counters only.
- [ ] Measure coverage only.
- [ ] Measure counters plus coverage.
- [ ] Confirm counters plus coverage uses one input traversal.
- [ ] Record CPU, RAM, storage, cache state, and versions.
- [ ] Run sufficient repetitions to identify variance.
- [ ] Do not claim a speedup unsupported by the data.

### 6.5 v0.1 release evidence

Create:

- `docs/evidence/V0_1_VALIDATION_REPORT.md`
- `docs/evidence/V0_1_PERFORMANCE_REPORT.md`
- `docs/evidence/V0_1_RELEASE_CHECKLIST.md`

### v0.1 release gate

Verify every criterion in SPEC §19.1.

- [ ] All criteria mapped to tests/evidence.
- [ ] No known silent fallback.
- [ ] No unexplained differential discrepancy.
- [ ] CI succeeds on the exact release commit.
- [ ] Tag and release only after exact-SHA validation.

---

# Part II — v0.2 CRAM and reference integrity

## Milestone 7 — CRAM local-reference design

### 7.1 Pin and inspect backend behavior

- [ ] Pin rust-htslib and HTSlib.
- [ ] Verify version-specific `REF_PATH` and `REF_CACHE` behavior.
- [ ] Identify every implicit reference-provider path.
- [ ] Confirm how to disable HTTP/HTTPS reference retrieval.
- [ ] Record findings in `ADR-0004-CRAM_REFERENCE_RESOLUTION.md`.

### 7.2 Enforce local-only resolution

- [ ] Override inherited reference environment before opening CRAM.
- [ ] Require explicit local FASTA where needed.
- [ ] Validate contig names, lengths, and MD5.
- [ ] Fail on missing local sequence.
- [ ] Fail on supplied-reference mismatch.
- [ ] Never fall back to an alternate reference.
- [ ] Record actual local FASTA identity in provenance.

### 7.3 Network-isolation tests

- [ ] Run CRAM tests with no network access.
- [ ] Use a CRAM that would otherwise invite MD5 lookup.
- [ ] Assert no DNS/HTTP attempt where observable.
- [ ] Assert missing reference fails.
- [ ] Assert mismatched reference fails.
- [ ] Assert correct local reference succeeds.

### 7.4 BAM/CRAM equivalence

- [ ] Produce equivalent BAM and CRAM fixtures.
- [ ] Compare canonical counters.
- [ ] Compare canonical coverage.
- [ ] Compare provenance differences only where format-specific.
- [ ] Test CRAM truncation and corruption.

### v0.2 release gate

Verify SPEC §19.2.

- [ ] Create `docs/evidence/V0_2_CRAM_VALIDATION.md`.
- [ ] Permanent CI succeeds on the exact release commit.

---

# Part III — v0.3 targeted sequencing

## Milestone 8 — BED and target normalization

### 8.1 Parser

Implement SPEC §9.

- [ ] Skip blank, comment, `track`, and `browser` lines.
- [ ] Accept CRLF and trailing whitespace.
- [ ] Strictly parse interval lines.
- [ ] Reject invalid coordinates.
- [ ] Apply explicit unknown-contig policy.
- [ ] Preserve names and source line identity.
- [ ] Fuzz parser.
- [ ] Test real vendor-style target files.

### 8.2 Normalization

- [ ] Deterministic sorting.
- [ ] Overlap merging for aggregate territory.
- [ ] Mapping from merged regions to source intervals.
- [ ] Configurable flanks.
- [ ] Provenance of normalization.
- [ ] Target checksum and identity.

## Milestone 9 — Targeted metrics

- [ ] Reuse the canonical chunked coverage engine.
- [ ] Target territory.
- [ ] On-target bases.
- [ ] Near-target bases.
- [ ] Off-target bases.
- [ ] Per-target mean depth.
- [ ] Threshold percentages.
- [ ] Zero-coverage target runs.
- [ ] Per-target dropout report.
- [ ] Duplicate-adjusted profile where defined.
- [ ] Fold enrichment with explicit denominator.
- [ ] Fold-80 or ADR-approved named equivalent.
- [ ] HG002 exome/target differential report.
- [ ] Create `docs/evidence/V0_3_TARGETED_VALIDATION.md`.

### v0.3 release gate

- [ ] Every target metric has a specification definition and fixture.
- [ ] No compatibility label is used without differential evidence.
- [ ] Permanent CI succeeds on the exact release commit.

---

# Part IV — v0.4 compatibility expansion

## Milestone 10 — Samtools stats subsets

- [ ] Select exact sections consumed by target MultiQC versions.
- [ ] Pin Samtools.
- [ ] Define every metric and filter.
- [ ] Implement canonical accumulators.
- [ ] Derive compatibility text.
- [ ] Differential fixtures.
- [ ] HG002 subset validation.
- [ ] Document unsupported sections.

## Milestone 11 — Picard alignment and insert-size profiles

- [ ] Pin Picard version.
- [ ] Define alignment-summary subset.
- [ ] Implement insert-size histogram.
- [ ] Reproduce or explicitly rename MAD trimming behavior.
- [ ] Test tie-breaking and rounding.
- [ ] Separate compatibility from “similar metric.”
- [ ] Differential fixtures for edge distributions.
- [ ] Document expected differences.

## Milestone 12 — WGS/hybrid-selection and MultiQC

- [ ] Select Picard WGS metrics in scope.
- [ ] Select hybrid-selection metrics in scope.
- [ ] Define fold-80 behavior.
- [ ] Add MultiQC fixture discovery tests.
- [ ] Run pinned MultiQC parser.
- [ ] Ensure parser failures fail CI.
- [ ] Create `docs/evidence/V0_4_COMPATIBILITY_REPORT.md`.

## Milestone 13 — Exact overlap correction and parallelism

### 13.1 Overlap correction ADR

- [ ] Name the reference tool/profile.
- [ ] Define which records participate.
- [ ] Define primary/supplementary semantics.
- [ ] Define pairing key.
- [ ] Define bounded-state behavior.
- [ ] Define behavior when the bound is exceeded.
- [ ] Decide whether exact correction forces streaming mode.
- [ ] Prohibit indexed parallel execution until exactness is proven.

### 13.2 Indexed parallel research and implementation

Only implement if measured value justifies complexity.

- [ ] Model readers, descriptors, buffers, and decompression pools.
- [ ] Include them in `--memory-limit`.
- [ ] Partition references deterministically.
- [ ] Preserve serial equivalence.
- [ ] Handle contig boundaries.
- [ ] Disable incompatible exact-overlap mode.
- [ ] Benchmark local NVMe and slower storage.
- [ ] Keep streaming mode available and authoritative.

### v0.4 release gate

- [ ] Compatibility report reconciles every claimed field.
- [ ] MultiQC parses generated outputs.
- [ ] Serial and released parallel modes agree.
- [ ] Permanent CI succeeds on the exact release commit.

---

# Part V — v0.5 production-beta qualification

## Milestone 14 — Full-scale HG002 validation

- [ ] Prepare approximately 30× HG002 WGS with exact manifest.
- [ ] Provision sufficient local storage.
- [ ] Run pinned reference tools.
- [ ] Run AlignGauge streaming CPU.
- [ ] Run any released parallel CPU mode.
- [ ] Reconcile every claimed metric.
- [ ] Repeat enough runs for variance.
- [ ] Record wall time, CPU time, peak RSS, bytes read, and output sizes.
- [ ] Publish `docs/evidence/V0_5_FULL_HG002_REPORT.md`.

## Milestone 15 — Hardening

- [ ] Parser fuzzing campaign.
- [ ] CIGAR fuzzing campaign.
- [ ] Output fault injection.
- [ ] Sanitizer-compatible native dependency tests where feasible.
- [ ] Dependency vulnerability audit.
- [ ] License inventory.
- [ ] SBOM generation.
- [ ] Reproducible-build assessment.
- [ ] Signed checksums and release artifacts.
- [ ] Upgrade/migration documentation for schemas.

### v0.5 release gate

- [ ] Full-scale report is complete.
- [ ] Resource requirements are documented.
- [ ] Known limitations are explicit.
- [ ] No release-blocking fuzz or security finding remains.
- [ ] Permanent CI succeeds on the exact release commit.

---

# Part VI — Hardware-acceleration research

## Research Milestone G1 — Profiling decision

- [ ] Profile released CPU pipeline.
- [ ] Identify actual top bottleneck by wall-clock contribution.
- [ ] Record storage and cache conditions.
- [ ] Decide whether any GPU spike is justified.
- [ ] Create ADR; closing the research path as “not justified” is a valid result.

## Research Milestone G2 — Candidate spike

If profiling justifies a spike:

- [ ] Prototype the bottleneck with no public CLI/config commitment.
- [ ] Prefer investigating compressed-input work when decompression dominates.
- [ ] Include transfer, normalization, synchronization, and startup overhead.
- [ ] Compare complete canonical output.
- [ ] Benchmark end to end.
- [ ] Measure power/resource cost where practical.
- [ ] Assess maintenance and platform burden.

## Hardware-acceleration admission gate

A candidate enters a release only if:

- [ ] results are canonically equivalent;
- [ ] wall-clock improvement is reproducible on a named workload;
- [ ] the improvement remains after PCIe and preparation overhead;
- [ ] CPU fallback remains complete and explicit;
- [ ] user-facing selection semantics are designed;
- [ ] provenance additions are designed;
- [ ] maintenance cost is accepted in an ADR.

Failure to pass the gate means no GPU crate, flag, or schema surface is added.
