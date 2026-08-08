# AlignGauge v0.5 full-scale HG002 qualification report

**State:** BLOCKED — full ~30× whole-genome qualification has not yet been executed. This file is the permanent evidence destination and must not be changed to a passing state without attaching the exact machine-readable campaign identity described below.

## Required profile

ADR-0012 freezes the v0.5 full-scale input as the deterministic ~30× whole-genome subsample of:

- source: `HG002_GRCh38-GIABv3_Element-StdInsert_2X150_81x_20231018.bam`
- source BAM MD5: `f5360b7adbc798c90a78f290de928eca`
- source BAI MD5: `1d7fd88891eee203c02fb852cac95301`
- reference build: `GRCh38-GIABv3`
- source nominal depth: 81×
- target nominal depth: approximately 30×
- subsample seed: `42`
- subsample fraction: `0.37037037037037`
- region: whole alignment, with no interval restriction

The standing one-megabase chr20 HG002 fixture is not acceptable M14 evidence.

## Explicit source provisioning

The repository now provides an explicit maintainer-only source provisioner. It is never
called by ordinary tests or AlignGauge runtime:

```bash
SOURCE_ROOT=/data/aligngauge/hg002-source
bash testdata/hg002/provision-full-wgs-source.sh "$SOURCE_ROOT"
```

The provisioner reads the pinned source URLs and MD5 values from
`testdata/hg002/full-wgs-v0.5.env`, obtains the current exact upstream object sizes,
checks remaining local capacity, downloads through resumable hidden `.partial` files,
verifies byte size and pinned MD5 before promoting each file, writes
`source.manifest`, and writes `_SUCCESS` last. An interrupted transfer may be resumed
by rerunning the same explicit command. A checksum mismatch, contradictory
partial/final state, insufficient free space, missing content length, or network error
is fatal.

A checksum failure does not silently delete or replace the transferred file. The
operator must resolve that state explicitly.

See `docs/M14_FULL_HG002_RUNBOOK.md` for the complete capacity, provisioning,
preparation, qualification, and evidence-handoff procedure.

## Preparation

Preparation command after the complete pinned source is provisioned:

```bash
testdata/hg002/prepare-full-wgs.sh \
  /path/to/HG002_GRCh38-GIABv3_Element-StdInsert_2X150_81x_20231018.bam \
  /path/to/HG002_GRCh38-GIABv3_Element-StdInsert_2X150_81x_20231018.bam.bai \
  testdata/local/hg002-grch38-giabv3-element-aviti-30x-v1
```

The preparation script requires the complete source BAM/BAI locally, verifies the pinned upstream MD5 values, checks the output filesystem resource floor, performs deterministic whole-alignment downsampling with pinned Samtools, indexes and quickchecks the result, writes exact SHA-256/size/tool/parameter identity, writes `_SUCCESS` last, and atomically publishes the prepared directory.

It does not implicitly download the 118 GB source, use remote partial access, replace the source technology/build, reduce the requested region, or lower the target depth.

## Qualification command

```bash
tools/v0.5/run-full-hg002-qualification.sh \
  testdata/local/hg002-grch38-giabv3-element-aviti-30x-v1 \
  /path/to/v0.5-hg002-qualification
```

The campaign must run from a clean exact AlignGauge commit and creates its result only after every required assertion succeeds.

## Qualification machinery validation

The full-scale preparation/campaign machinery and the independent v0.5 hardening program are now implemented in PR #9.

Validated hardening evidence SHA:

`0e14af01c2f218aaca371c414133403e8e88c96d`

Every workflow triggered by that exact evidence state succeeded, including Permanent CI and V0.5 Hardening Validation run `31265637214`. The hardening gate includes full ASan/LeakSanitizer HTS coverage, both 20,000-run fuzz campaigns, dependency/license/source audit, deterministic SBOM/license inventories, and a byte-identical two-build release-artifact assessment.

The implementation checklist was reconciled in bot bookkeeping commit `195bcb296c77b6aafdb92f56a39341ea1dc7a26f`. Milestone 15 now records all implemented hardening items complete except signed/attested release artifacts, which remain an exact-release-candidate/publication item. Milestone 14 remains entirely open.

This section records readiness of the machinery only. It is not full-HG002 evidence and does not alter the `BLOCKED` state above.

## Evidence required to change this report to COMPLETE

Copy from the successful campaign without reinterpretation:

- exact AlignGauge commit;
- exact prepared BAM SHA-256, BAI SHA-256, byte sizes, and `prepared.manifest`;
- CPU model/topology;
- installed RAM;
- OS/kernel;
- input and output filesystem/storage identity;
- cache policy;
- configured memory limit;
- configured/effective reader thread settings;
- warm-up count;
- at least three measured serial runs;
- at least three measured released I/O-parallel runs;
- per-run wall time;
- per-run user CPU time;
- per-run system CPU time;
- per-run peak RSS;
- Linux logical/physical byte counters where observable;
- output directory size per run;
- variance summary;
- canonical summary SHA-256 for every measured run;
- byte-identical serial/parallel canonical equivalence;
- exact Samtools flagstat differential;
- byte-identical Samtools idxstats differential;
- exact Samtools Stats differential;
- exact canonical coverage differential;
- exact released Picard AlignmentSummary subset differential;
- exact released Picard InsertSize differential;
- exact pinned MultiQC parsed-data comparison for the released generated profiles;
- `qualification.json` SHA-256;
- campaign `_SUCCESS`.

## Current blocker

The repository and GitHub-hosted CI do not contain the complete 118 GB source BAM. The
new provisioner makes acquisition explicit and resumable, but it deliberately does
not make the full campaign an ordinary CI download. Full-scale qualification still
requires a maintainer execution environment with sufficient persistent local storage.

The active execution environment used for this implementation work has only about 38 GB free, below the preparation script's fail-closed 64 GiB minimum even before provisioning the 118 GB source. It cannot execute the real M14 campaign.

This is a release blocker, not an allowed warning or waiver. `v0.5.0` must not be tagged until this report is populated from a successful full campaign and the remaining v0.5 release gate is green on the exact release commit.
