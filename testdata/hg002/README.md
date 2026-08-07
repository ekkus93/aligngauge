# HG002 GRCh38-GIABv3 Small Validation Subset

The v0.1 real-data fixture is prepared explicitly from the GIAB HG002 Element
AVITI alignment:

- source: `HG002_GRCh38-GIABv3_Element-StdInsert_2X150_81x_20231018.bam`
- source BAM MD5: `f5360b7adbc798c90a78f290de928eca`
- source BAI MD5: `1d7fd88891eee203c02fb852cac95301`
- region: `chr20:10000000-11000000`
- subsampling seed: `42`
- subsampling fraction: `0.37037037037037`
- nominal source depth: 81×
- nominal target depth: approximately 30×

Run:

```bash
testdata/hg002/prepare.sh
```

The script downloads only the 9 MB BAI directly, verifies its upstream MD5, and
uses remote indexed BAM access through the explicitly invoked Samtools
container. It does not download the 118 GB BAM. Because the whole BAM is not
downloaded, its upstream whole-file MD5 is recorded but cannot be recomputed by
this preparation path.

The prepared subset and index are written under `testdata/local/`, which is
ignored by Git. `prepared.manifest` records local SHA-256 values and all
parameters. Ordinary tests never invoke this script and never use the network.

## Milestone 2 validation evidence

Two-pass preparation succeeded on source SHA `45211236419e5bebc7c0d09d5cb35d65174cc11a` in workflow run
`31100844104`, job `92613759310`. This maintainer-authored documentation update is
the exact-SHA validation trigger for the final Milestone 2 evidence state; the
resulting commit must independently pass Permanent CI, Reference Validation, and
HG002 Preparation before Milestone 2 is considered complete.

## Whole-repository validation

This file is also used as a harmless trigger for explicit HG002 validation after
repository-wide history reconciliation. The trigger changes documentation only;
all code, fixtures, reference scripts, and workflow semantics remain unchanged.

The 2026-08-06 whole-repository runtime audit uses this documentation-only change
to force Permanent CI, Full Runtime Validation, Reference Validation, and HG002
Preparation to execute together on one exact final `master` SHA.
