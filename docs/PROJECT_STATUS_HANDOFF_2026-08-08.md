# AlignGauge project status handoff — 2026-08-08

This document is the restart point for AlignGauge development as of 2026-08-08. It is intentionally detailed so a future maintainer or a new ChatGPT/Claude Code session can pick up without reconstructing the release history from conversation logs.

## 1. Executive summary

AlignGauge is a validation-first Rust engine for alignment QC and coverage analysis over existing BAM/CRAM data. The latest published release is `v0.4.0`. Work on `v0.5.0` has reached the production-beta qualification phase.

The code, compatibility work, release-hardening machinery, and full-HG002 qualification runner are implemented and validated. The remaining hard blocker is physical execution of Milestone 14 on the real deterministic ~30× HG002 whole-genome workload.

That campaign is currently paused because the available maintainer machine does not have enough free disk space. This is an intentional hold, not a waived requirement. Do **not** mark Milestone 14 complete and do **not** tag `v0.5.0` until the real campaign has succeeded and its machine-readable evidence has been reconciled.

Baseline `master` immediately before this handoff document was added:

```text
d6001a0f424d8e6c4b9390af301c3d3fa6bc3bce
```

When resuming later, first inspect the then-current `master`. If it has moved, review the intervening changes before using the commands in this handoff. If exact continuity is required, the baseline above identifies the validated repository state from which this handoff was written.

## 2. Published release state

### v0.4.0

Latest published release:

- tag: `v0.4.0`
- exact release SHA: `5be4aa4e5df3e8feb17fdde46c408683ac08bb53`
- GitHub release ID: `367190259`
- PR #8 merge SHA: `c1ded07bad71b330aa712d65ec38850de009a218`

The exact v0.4 release SHA passed:

- Permanent CI run `31255804251`, job `93098870269`
- Reference Validation run `31255804281`, job `93098876451`
- V0.4 Release Validation run `31255804250`, job `93098876838`

### v0.4 compatibility boundary

v0.4 makes exact compatibility claims only for already-proved profiles:

1. `samtools-stats-1.24-multiqc-1.35`
2. `picard-alignment-summary-3.4.0-all-reads-subset-v1`
3. `picard-insert-size-3.4.0-all-reads-v1`

Earlier Samtools `flagstat` and `idxstats` projections also remain released.

Important non-claims remain explicit:

- Picard WgsMetrics is not a released compatibility profile.
- Picard HsMetrics is not a released compatibility profile.
- Native `target_uniformity_penalty_80` is not Picard `FOLD_80_BASE_PENALTY`.
- Indexed reference-partition execution is not released.
- Collector execution remains serial.
- `--threads >1` does not mean collector parallelism; provenance records one collector thread and emits the explicit `collector_threads_serial_v0_1` warning.
- Released bounded concurrency is HTSlib reader/decompression concurrency through `--io-threads`.
- The v0.4 gate proves byte-identical canonical output between serial decoding and `--io-threads 2` for released whole-input and targeted paths.

Do not widen any of those claims merely because v0.5 performs a larger qualification workload.

Key v0.4 documents:

- `docs/adr/ADR-0011-V0_4_RELEASE_SCOPE.md`
- `docs/evidence/V0_4_COMPATIBILITY_REPORT.md`
- `docs/evidence/V0_4_RELEASE_VALIDATION.md`
- `docs/evidence/M13_EXACT_OVERLAP.md`

## 3. Current development target: v0.5 production-beta qualification

v0.5 is governed primarily by:

- `docs/adr/ADR-0012-V0_5_PRODUCTION_BETA_QUALIFICATION.md`
- `docs/DNA_QC_ENGINE_TODO.md`
- `docs/M14_FULL_HG002_RUNBOOK.md`
- `docs/evidence/V0_5_FULL_HG002_REPORT.md`

Milestone 15 hardening is largely implemented and validated. Milestone 14 full-scale HG002 execution is the remaining substantive qualification blocker.

## 4. Milestone 14 status — PAUSED / BLOCKED on disk capacity

The permanent report intentionally says:

```text
State: BLOCKED — full ~30× whole-genome qualification has not yet been executed.
```

That state is correct and must remain correct until the real campaign finishes.

### Exact HG002 qualification profile

Milestone 14 freezes this source and preparation profile:

- source BAM: `HG002_GRCh38-GIABv3_Element-StdInsert_2X150_81x_20231018.bam`
- source BAM MD5: `f5360b7adbc798c90a78f290de928eca`
- source BAI MD5: `1d7fd88891eee203c02fb852cac95301`
- reference build: `GRCh38-GIABv3`
- source nominal depth: `81×`
- target nominal depth: approximately `30×`
- deterministic subsample seed: `42`
- deterministic subsample fraction: `0.37037037037037`
- region: the **whole alignment**, with no interval restriction

The source URLs and identity are committed in:

```text
testdata/hg002/full-wgs-v0.5.env
```

The existing one-megabase chr20 HG002 fixture is useful for ordinary CI, but it is explicitly **not acceptable** as Milestone 14 evidence.

Do not substitute:

- the small chr20 fixture;
- a different HG002 sequencing technology;
- another reference build;
- a lower depth;
- a restricted region;
- remote partial BAM access in place of the required local full source.

Any of those would be a different qualification profile.

## 5. Why work is paused

The full source BAM is approximately 118 GB. The deterministic ~30× prepared BAM requires additional tens of gigabytes, and the qualification campaign needs space for repeated outputs and differential/reference artifacts.

The active implementation environment had only about 38 GB free when work was paused. That is below even the preparation script's fail-closed 64 GiB minimum and far below the practical capacity needed for the complete source plus campaign.

Recommended planning values from the committed runbook:

- source filesystem: at least **128 GiB free** before a fresh source download;
- prepared/campaign filesystem: at least **80 GiB free**;
- if everything shares one filesystem: begin with at least **200 GiB free**.

These are planning values only. The scripts independently perform stricter runtime checks based on actual conditions.

No requirement was reduced to accommodate the smaller machine.

## 6. Full-HG002 machinery already implemented

### 6.1 Source provisioner

Committed path:

```text
testdata/hg002/provision-full-wgs-source.sh
```

It is maintainer-only and is never invoked implicitly by normal AlignGauge runtime or ordinary tests.

The provisioner:

- reads the pinned source URLs, MD5 values, build, and profile from `full-wgs-v0.5.env`;
- obtains exact upstream object sizes before transfer;
- checks local free space against remaining download bytes plus headroom;
- downloads into hidden resumable `.partial` files;
- uses resumable transfer semantics rather than restarting a huge transfer unnecessarily;
- verifies final byte size and pinned MD5 before promotion;
- rejects contradictory final/partial state;
- rejects unexpected files in the source directory;
- requires the source destination to resolve outside the repository;
- writes `source.manifest` only after both BAM and BAI are valid;
- writes `_SUCCESS` last;
- revalidates completed state rather than trusting `_SUCCESS` alone.

A checksum mismatch is fatal. The provisioner does not silently delete or replace a huge bad transfer; operator intervention is required.

### 6.2 Deterministic ~30× preparation

Committed path:

```text
testdata/hg002/prepare-full-wgs.sh
```

It requires the full BAM and BAI locally, verifies the pinned source identities, downsamples the whole alignment using the frozen seed/fraction and pinned Samtools, indexes the result, performs `samtools quickcheck`, records exact SHA-256 values, sizes, profile, source identity, tool identity, and parameters in `prepared.manifest`, then publishes `_SUCCESS` last.

Preparation fails on missing input, wrong checksum, insufficient space, pre-existing destination, failed Samtools operation, missing index, or failed quickcheck.

### 6.3 Qualification runner

Committed path:

```text
tools/v0.5/run-full-hg002-qualification.sh
```

The runner requires a clean exact repository commit and an output directory outside the repository. Its default minimum campaign is:

- one warm-up run per execution mode;
- three measured serial runs with `--io-threads 0`;
- three measured released I/O-parallel runs with `--io-threads 2`;
- `--memory-limit 8GiB`.

The minimum measured-run count cannot be lowered.

It records:

- exact AlignGauge commit;
- prepared BAM/BAI SHA-256 and byte sizes;
- CPU topology;
- installed RAM;
- OS/kernel;
- input/output filesystem and mount information;
- cache policy;
- memory limit;
- configured/effective reader thread settings;
- wall time;
- user CPU time;
- system CPU time;
- peak RSS;
- logical/physical byte counters where Linux exposes them;
- output sizes;
- canonical summary SHA-256 values;
- variance statistics.

It requires byte-identical canonical `summary.json` across all measured repetitions and both released I/O modes.

It also runs the full released differential surface against pinned tools:

- Samtools 1.24 `flagstat`;
- Samtools 1.24 `idxstats`;
- Samtools Stats exact profile;
- canonical coverage against streaming `samtools depth -aa` reduction;
- Picard 3.4.0 released AlignmentSummary subset;
- Picard 3.4.0 released InsertSize profile;
- MultiQC 1.35 parsed-data equivalence for the generated surfaces already claimed by v0.4.

Whole-genome depth is streamed into the reducer rather than materialized as a multi-billion-line text file.

A successful campaign creates `qualification.json` and `_SUCCESS` only after all required assertions pass.

## 7. Exact restart procedure when sufficient storage is available

### 7.1 Re-establish repository state

On the machine that will run the campaign:

```bash
git switch master
git pull --ff-only
git status --short
git rev-parse HEAD
```

The working tree must be clean.

If `master` has moved beyond the handoff baseline, review the changes and make sure ADR-0012, the full-WGS profile, and the qualification runner have not changed incompatibly. Do not blindly reuse old evidence after semantic changes.

### 7.2 Check capacity and Docker

```bash
df -hT
docker info --format 'Docker server: {{.ServerVersion}}'
```

Choose storage paths with the capacity described above.

### 7.3 Provision the exact source

Example:

```bash
SOURCE_ROOT=/data/aligngauge/hg002-source

bash testdata/hg002/provision-full-wgs-source.sh \
  "$SOURCE_ROOT"
```

The expected final source filenames are:

```text
HG002_GRCh38-GIABv3_Element-StdInsert_2X150_81x_20231018.bam
HG002_GRCh38-GIABv3_Element-StdInsert_2X150_81x_20231018.bam.bai
```

If the transfer is interrupted, rerun the same provisioner command. Its `.partial` state is designed for explicit resumption.

Do not manually fabricate `source.manifest` or `_SUCCESS`.

### 7.4 Prepare the deterministic ~30× BAM

```bash
PREPARED_ROOT=/data/aligngauge/hg002-30x
SOURCE_BAM="$SOURCE_ROOT/HG002_GRCh38-GIABv3_Element-StdInsert_2X150_81x_20231018.bam"
SOURCE_BAI="$SOURCE_ROOT/HG002_GRCh38-GIABv3_Element-StdInsert_2X150_81x_20231018.bam.bai"

bash testdata/hg002/prepare-full-wgs.sh \
  "$SOURCE_BAM" \
  "$SOURCE_BAI" \
  "$PREPARED_ROOT"
```

Preserve the resulting `prepared.manifest` and `_SUCCESS`.

### 7.5 Run the full qualification campaign

```bash
CAMPAIGN_ROOT=/data/aligngauge/v0.5-hg002-qualification

bash tools/v0.5/run-full-hg002-qualification.sh \
  "$PREPARED_ROOT" \
  "$CAMPAIGN_ROOT"
```

Do not lower the minimum warm-up or measured-run counts.

If more repetitions are desired:

```bash
ALIGNGAUGE_V05_WARMUP_RUNS=2 \
ALIGNGAUGE_V05_MEASURED_RUNS=5 \
ALIGNGAUGE_V05_MEMORY_LIMIT=8GiB \
bash tools/v0.5/run-full-hg002-qualification.sh \
  "$PREPARED_ROOT" \
  "$CAMPAIGN_ROOT"
```

### 7.6 Preserve the complete campaign

Do not copy only selected metrics. Keep the complete output directory until evidence reconciliation is finished.

Record at minimum:

```bash
sha256sum "$CAMPAIGN_ROOT/qualification.json"
find "$CAMPAIGN_ROOT" -maxdepth 2 -type f -print | sort
```

The campaign directory contains the authoritative machine-readable facts for the permanent report.

## 8. Milestone 14 evidence closure after the campaign

Permanent evidence destination:

```text
docs/evidence/V0_5_FULL_HG002_REPORT.md
```

Do not manually invent or normalize values. Populate the report directly from the successful campaign artifacts.

The report must reconcile at least:

- exact AlignGauge commit;
- prepared BAM and BAI SHA-256;
- prepared BAM and BAI byte sizes;
- exact `prepared.manifest` identity;
- CPU model/topology;
- installed RAM;
- OS/kernel;
- storage/filesystem identity;
- cache policy;
- configured memory limit;
- configured/effective reader thread settings;
- warm-up count;
- at least three measured serial runs;
- at least three measured released I/O-parallel runs;
- wall/user/system CPU time per run;
- peak RSS per run;
- logical/physical bytes where available;
- output sizes;
- variance summary;
- canonical summary SHA-256 for every measured run;
- byte-identical serial/parallel equivalence;
- Samtools flagstat exact differential;
- Samtools idxstats byte-identical differential;
- Samtools Stats exact differential;
- canonical coverage exact differential;
- Picard AlignmentSummary exact released-subset differential;
- Picard InsertSize exact released-profile differential;
- MultiQC exact parsed-data comparisons for the claimed generated profiles;
- `qualification.json` SHA-256;
- final campaign `_SUCCESS`.

Only after those facts are reconciled may the M14 TODO boxes be checked.

## 9. Milestone 15 hardening status

Hardening implementation evidence SHA:

```text
0e14af01c2f218aaca371c414133403e8e88c96d
```

Important validation from that evidence state:

- V0.5 Hardening Validation run `31265637214` — success
- Permanent CI run `31265637151` — success

The hardening program includes:

- BED parser fuzzing;
- raw CIGAR/coverage-boundary fuzzing;
- deterministic output-publication fault injection;
- AddressSanitizer/LeakSanitizer coverage of the Rust/native HTS boundary;
- dependency advisory checks;
- license/source policy checks;
- deterministic license inventory;
- deterministic CycloneDX SBOM;
- two-build reproducibility assessment;
- schema upgrade/migration documentation.

The current TODO records all implemented hardening items complete except:

```text
[ ] Signed checksums and release artifacts.
```

That final item is deliberately deferred to the exact v0.5 release candidate/publication step so signatures/attestations bind the actual release artifacts.

### Native HTS ownership shim

LeakSanitizer found an upstream `rust-htslib` malformed-header ownership leak. The mitigation is isolated in the private `aligngauge-hts-ffi` crate. Normal application crates continue to forbid unsafe code. Do not broaden the unsafe-code exception.

See ADR-0012 for the exact rationale and ownership contract.

### Dependency advisory policy

`cargo-deny` remains fail-closed for vulnerability, unsoundness, unknown source, and disallowed license findings.

The transitive `custom_derive 0.1.7` / `RUSTSEC-2025-0058` item is classified as unmaintained maintenance debt rather than a patched vulnerability. No advisory ID is blanket ignored. Direct workspace unmaintained dependencies remain release-blocking.

## 10. Most recent merged provisioning validation

PR #10 added the explicit full-source provisioner and M14 operator runbook.

Validated PR evidence head:

```text
64d453bc7cd1f46928e72232fc26f8a7f43f806d
```

Merge SHA:

```text
d8b2657671ada20c61a462824e4b3d9ddfc3f06b
```

Every workflow triggered by that merge SHA succeeded:

- Permanent CI `31268413220` / job `93130179422`
- Full Runtime Validation `31268413233` / job `93130179542`
- Reference Validation `31268413288` / job `93130179485`
- Targeted Validation `31268413243` / job `93130179388`
- Samtools Stats Validation `31268413242` / job `93130179297`
- Picard Validation `31268413253` / job `93130179343`
- V0.4 Release Validation `31268413251` / job `93130179351`
- V0.5 Hardening Validation `31268413258` / aggregate job `93130496496`
- M2 HG002 Preparation Validation `31268413240` / job `93130179267`

The documentation closeout baseline `d6001a0f424d8e6c4b9390af301c3d3fa6bc3bce` then passed:

- Permanent CI `31268603198`
- Reference Validation `31268603206`
- V0.5 Hardening Validation `31268603209`

The last hardening run again passed the provisioner preflight, ASan/LSan, both 20,000-run fuzz campaigns, dependency/license/source audit, deterministic SBOM/license inventory generation, reproducibility assessment, and aggregate hardening gate.

### Prior CI harness failure worth remembering

An early provisioner-preflight attempt failed because the workflow redirected a usage-test result into `target/` before creating that directory. That failure was left red and the harness was fixed by explicitly creating `target/`. No provisioner assertion or hardening gate was suppressed.

If a future CI failure occurs, preserve the same discipline: fix the actual cause; do not add warning-only fallbacks, `|| true`, tolerance widening, or substitute evidence merely to make a gate green.

## 11. Current v0.5 TODO / release-gate state

Milestone 14 remains open because the physical campaign has not run:

- [ ] Prepare approximately 30× HG002 WGS with exact manifest.
- [ ] Provision sufficient local storage.
- [ ] Run pinned reference tools.
- [ ] Run AlignGauge streaming CPU.
- [ ] Run any released parallel CPU mode.
- [ ] Reconcile every claimed metric.
- [ ] Repeat enough runs for variance.
- [ ] Record wall time, CPU time, peak RSS, bytes read, and output sizes.
- [ ] Publish `docs/evidence/V0_5_FULL_HG002_REPORT.md` as COMPLETE from the real full campaign.

Milestone 15 is complete except release-bound signing/attestation:

- [x] Parser fuzzing campaign.
- [x] CIGAR fuzzing campaign.
- [x] Output fault injection.
- [x] Sanitizer-compatible native dependency tests where feasible.
- [x] Dependency vulnerability audit.
- [x] License inventory.
- [x] SBOM generation.
- [x] Reproducible-build assessment.
- [ ] Signed checksums and release artifacts.
- [x] Upgrade/migration documentation for schemas.

Current v0.5 release gate:

- [ ] Full-scale report is complete.
- [x] Resource requirements are documented.
- [x] Known limitations are explicit.
- [x] No release-blocking fuzz or security finding remains.
- [ ] Permanent CI succeeds on the exact release commit.

The two unchecked release-gate items are intentionally real blockers, not bookkeeping omissions.

## 12. What happens after Milestone 14 succeeds

Once the real campaign finishes successfully:

1. Preserve the complete campaign directory.
2. Reconcile `qualification.json` and all differential artifacts into `docs/evidence/V0_5_FULL_HG002_REPORT.md`.
3. Change that report from `BLOCKED` to `COMPLETE` only if every required claim is supported.
4. Update `docs/DNA_QC_ENGINE_TODO.md` to close the actual M14 items only after evidence exists.
5. Commit the evidence state.
6. Require every permanent workflow triggered by that exact evidence commit to succeed.
7. Select a final v0.5 release candidate only from an exact validated commit.
8. Build the deterministic release artifacts for that exact candidate.
9. Complete signed checksums / cryptographic provenance or attestation for those exact artifacts.
10. Run the exact v0.5 release gate on the exact candidate.
11. Only then create the `v0.5.0` tag/release.
12. Remove any temporary write-enabled publisher/helper workflow immediately after publication, if such a helper is used.
13. Perform post-release README/TODO/evidence closure and validate the resulting `master` again.

Do not tag first and attempt to backfill evidence later.

## 13. Key file map

### Product/specification

- `docs/DNA_QC_ENGINE_SPEC.md` — product and architecture specification
- `docs/DNA_QC_ENGINE_TODO.md` — implementation/release checklist
- `docs/adr/ADR-0012-V0_5_PRODUCTION_BETA_QUALIFICATION.md` — v0.5 qualification contract

### M14 full-HG002 work

- `testdata/hg002/full-wgs-v0.5.env` — exact source/profile identity
- `testdata/hg002/provision-full-wgs-source.sh` — explicit resumable source acquisition
- `testdata/hg002/prepare-full-wgs.sh` — deterministic ~30× whole-genome preparation
- `docs/M14_FULL_HG002_RUNBOOK.md` — operator procedure
- `tools/v0.5/run-full-hg002-qualification.sh` — complete campaign runner
- `tools/v0.5/measure-command.py` — per-command resource measurement
- `docs/evidence/V0_5_FULL_HG002_REPORT.md` — permanent M14 evidence destination; currently BLOCKED

### M15 / release hardening

- `.github/workflows/v0.5-hardening.yml` — standing hardening gate
- `tools/v0.5/generate-sbom.py` — deterministic SBOM generation
- `tools/v0.5/build-release-artifacts.sh` — deterministic release-artifact build/assessment machinery

### Prior release evidence

- `docs/evidence/V0_4_COMPATIBILITY_REPORT.md`
- `docs/evidence/V0_4_RELEASE_VALIDATION.md`
- `docs/evidence/M13_EXACT_OVERLAP.md`
- `docs/evidence/V0_3_TARGETED_VALIDATION.md`
- `docs/evidence/V0_2_CRAM_VALIDATION.md`
- `docs/evidence/V0_1_VALIDATION_REPORT.md`

## 14. Non-negotiable correctness rules to preserve

The project has deliberately chosen fail-closed behavior. Future work should preserve these rules:

- Never represent a missing or unsupported metric as numeric zero.
- Never silently switch reference sources.
- Never silently drop invalid target intervals or infer chromosome aliases.
- Never relax an exact differential into an unexplained tolerance merely to pass CI.
- Never treat parser exit success alone as semantic compatibility evidence.
- Never substitute the small HG002 fixture for the required full M14 campaign.
- Never reduce the M14 depth, region, repetition count, or reference-tool surface because of hardware limits.
- Never imply collector parallelism when only HTSlib reader/decompression concurrency is active.
- Never relabel native `target_uniformity_penalty_80` as Picard `FOLD_80_BASE_PENALTY`.
- Never expose deferred Picard WgsMetrics/HsMetrics profiles without full exact evidence.
- Never silently continue after checksum, storage, corruption, reference, output-publication, fuzz, sanitizer, audit, or release-signature failure.
- Temporary write-enabled release helpers must not remain standing after they have served their one-time purpose.

If a required resource is unavailable, keep the milestone blocked and document the reason, as we are doing now.

## 15. Recommended first message / action when resuming

A future session can start with:

> Resume AlignGauge v0.5 from `docs/PROJECT_STATUS_HANDOFF_2026-08-08.md`. We now have sufficient local disk space for the full HG002 Milestone 14 campaign. First verify current `master`, compare it to the handoff baseline, then walk through `docs/M14_FULL_HG002_RUNBOOK.md` without weakening any evidence gate.

The first actual machine commands should be the repository-state and disk-capacity checks from section 7 above. Do not begin a 118 GB transfer until the chosen paths have sufficient capacity.

## 16. Pause disposition

The project is intentionally paused here because the required disk capacity is not currently available.

Nothing is wrong with the implementation and there is no discovered correctness issue requiring a workaround. The honest next step simply requires more storage.

Until that resource is available:

- leave Milestone 14 unchecked;
- leave `docs/evidence/V0_5_FULL_HG002_REPORT.md` `BLOCKED`;
- leave the signed-release-artifact item unchecked;
- do not publish `v0.5.0`.

That is the complete current restart state.
