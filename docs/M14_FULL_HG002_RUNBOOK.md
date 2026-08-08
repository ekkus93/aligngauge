# Milestone 14 full-scale HG002 operator runbook

Milestone 14 is intentionally a maintainer-run whole-genome qualification. It is not
ordinary GitHub-hosted CI and it must not be replaced by the existing one-megabase
HG002 subset.

The pinned profile is defined by
`testdata/hg002/full-wgs-v0.5.env` and ADR-0012.

## Required capacity

The source BAM is approximately 118 GB. The deterministic ~30× prepared BAM is
expected to require tens of additional gigabytes, and the qualification campaign
needs working/output space for repeated runs and reference-tool products.

Recommended layout:

- source filesystem: at least 128 GiB free before starting a fresh source download;
- prepared/campaign filesystem: at least 80 GiB free;
- if source, prepared input, and campaign output share one filesystem, start with at
  least 200 GiB free.

These are operator planning values, not substitutes for the scripts' own exact
resource checks. The provisioner determines the current upstream object sizes from
HTTP `Content-Length`, accounts for an existing resumable partial download, and
refuses to continue without remaining bytes plus 2 GiB headroom. The preparation
script independently enforces its 64 GiB output-filesystem floor, and the campaign
runner independently enforces its 16 GiB output-filesystem floor.

## 1. Use an exact clean AlignGauge commit

Clone or update the repository and select the exact commit that will be qualified.
Do not run the campaign from a dirty working tree.

```bash
git status --short
git rev-parse HEAD
```

`tools/v0.5/run-full-hg002-qualification.sh` checks the clean-tree condition again
and records the exact commit in the campaign.

## 2. Provision the pinned complete HG002 source

The source download is explicit. Ordinary AlignGauge commands and ordinary tests do
not invoke network access or this provisioner.

Choose a large local directory outside the repository, for example:

```bash
SOURCE_ROOT=/data/aligngauge/hg002-source
bash testdata/hg002/provision-full-wgs-source.sh "$SOURCE_ROOT"
```

The provisioner:

- reads the exact source URLs and pinned MD5 values from
  `testdata/hg002/full-wgs-v0.5.env`;
- obtains exact upstream object sizes before downloading;
- writes downloads as hidden `.partial` files and uses `curl --continue-at -` so an
  interrupted transfer can be resumed explicitly by rerunning the same command;
- refuses contradictory final/partial state;
- verifies exact byte size and pinned MD5 before promoting a partial file;
- writes `source.manifest` only after both BAM and BAI verify;
- writes `_SUCCESS` last;
- re-verifies a previously completed source directory before accepting it.

A checksum mismatch is fatal and the mismatching file is not silently deleted or
replaced. This avoids destroying a 118 GB transfer without operator visibility.

The expected final source filenames are the upstream GIAB filenames recorded in the
profile environment file.

## 3. Prepare the deterministic ~30× whole-genome input

Choose a separate output directory if possible:

```bash
PREPARED_ROOT=/data/aligngauge/hg002-30x
SOURCE_BAM="$SOURCE_ROOT/HG002_GRCh38-GIABv3_Element-StdInsert_2X150_81x_20231018.bam"
SOURCE_BAI="$SOURCE_ROOT/HG002_GRCh38-GIABv3_Element-StdInsert_2X150_81x_20231018.bam.bai"

bash testdata/hg002/prepare-full-wgs.sh \
  "$SOURCE_BAM" \
  "$SOURCE_BAI" \
  "$PREPARED_ROOT"
```

Preparation verifies the complete pinned source again, downsamples the **whole
alignment** with Samtools 1.24 using seed `42` and fraction
`0.37037037037037`, indexes the result, runs `samtools quickcheck`, records exact
SHA-256/size/tool/parameter identity in `prepared.manifest`, writes `_SUCCESS` last,
and atomically publishes the prepared directory.

Do not add a region restriction, substitute another HG002 technology/reference
build, or reduce target depth to fit a smaller machine. Any of those would be a
different qualification profile and cannot satisfy Milestone 14.

## 4. Run the complete qualification campaign

Choose a campaign output directory outside the repository:

```bash
CAMPAIGN_ROOT=/data/aligngauge/v0.5-hg002-qualification

bash tools/v0.5/run-full-hg002-qualification.sh \
  "$PREPARED_ROOT" \
  "$CAMPAIGN_ROOT"
```

Defaults are the ADR-0012 minimum campaign:

- one warm-up run per execution mode;
- three measured serial runs using `--io-threads 0`;
- three measured released I/O-parallel runs using `--io-threads 2`;
- `--memory-limit 8GiB`.

The minimum run counts cannot be lowered. They may be raised explicitly:

```bash
ALIGNGAUGE_V05_WARMUP_RUNS=2 \
ALIGNGAUGE_V05_MEASURED_RUNS=5 \
ALIGNGAUGE_V05_MEMORY_LIMIT=8GiB \
bash tools/v0.5/run-full-hg002-qualification.sh \
  "$PREPARED_ROOT" \
  "$CAMPAIGN_ROOT"
```

The campaign fails closed unless all measured canonical `summary.json` files are
byte-identical across repetitions and both released I/O settings and all pinned
reference comparisons pass.

Reference checks include:

- Samtools 1.24 flagstat;
- Samtools 1.24 idxstats;
- Samtools Stats `samtools-stats-1.24-multiqc-1.35`;
- canonical coverage via streaming `samtools depth -aa` reduction;
- Picard 3.4.0 released AlignmentSummary subset;
- Picard 3.4.0 released InsertSize profile;
- pinned MultiQC 1.35 parsed-data equivalence for the generated surfaces claimed by
  v0.4.

The whole-genome depth stream is reduced directly and is not materialized as a
multi-billion-line text file.

## 5. Preserve the campaign output

A successful campaign contains:

- `qualification.json`;
- `prepared.manifest`;
- exact AlignGauge commit identity;
- environment CPU/RAM/kernel/filesystem data;
- per-run measurements;
- per-run canonical summary SHA-256 values;
- differential results and parser reports;
- final `_SUCCESS`.

Do not copy only selected metrics. Preserve the complete campaign directory until
the committed evidence report has been reconciled and reviewed.

For a handoff, record:

```bash
sha256sum "$CAMPAIGN_ROOT/qualification.json"
find "$CAMPAIGN_ROOT" -maxdepth 2 -type f -print | sort
```

## 6. Evidence closure rule

`docs/evidence/V0_5_FULL_HG002_REPORT.md` remains `BLOCKED` until the successful
campaign data are available. The report must be populated from the machine-readable
campaign without inventing, approximating, or manually normalizing failed fields.

After the report is committed, the exact evidence SHA must pass the permanent CI,
reference, v0.4 compatibility, and v0.5 hardening gates that its paths trigger.
Only then can Milestone 14 be marked complete and the final v0.5 release-candidate
artifact signing/attestation step begin.
