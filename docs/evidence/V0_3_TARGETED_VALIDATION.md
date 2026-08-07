# v0.3 Targeted Sequencing Validation Evidence

**Milestone:** 9 — Targeted metrics  
**Date:** 2026-08-07  
**Pre-evidence validated head:** `04ba604c120256b55c51a8a5e7871e4f9850181d`  
**Native metric profile:** `aligngauge-targeted-v0.3`  
**Underlying coverage profile:** `aligngauge-v0.1`  
**Summary schema:** `1.1.0`  
**Disposition:** Complete — `v0.3.0` was published from exact validated release SHA `eccd45157d34ada00a3403a2b24d606956878b62` after all five release-relevant gates succeeded.

## Scope

Milestone 9 completes the v0.3 targeted-sequencing product boundary begun by Milestone 8. It adds native target-aware reductions to the existing exact chunked coverage engine rather than creating a second pileup or a target-specific read traversal.

The released targeted surface is:

```text
aligngauge qc --input <BAM|CRAM> --outdir <DIR> --targets <BED> [--near-distance <N>]
```

`--near-distance` defaults to 250 bases when a target BED is supplied and is invalid without `--targets`.

The target BED is parsed and normalized against the already validated alignment sequence dictionary under ADR-0005. The targeted metric formulas and compatibility boundary are defined by SPEC §12.7 and ADR-0006.

## Semantic contract

### One authoritative coverage sweep

Targeted analysis reuses canonical `aligngauge-v0.1` coverage semantics:

- mapped records only;
- primary records only;
- QC-pass records only;
- duplicates excluded;
- minimum MAPQ 0;
- no base-quality filter;
- no mate-overlap correction;
- only `M`, `=`, and `X` consume covered reference bases.

The same depth run emitted by the canonical chunked accumulator is reduced simultaneously into whole-genome and targeted state. Production provenance records `alignment_traversals = 1` for targeted BAM and CRAM runs.

No target-specific BAM rescan, pileup fallback, or approximate path exists.

### Territory and aligned-base partition

The zero-flank normalized BED union defines target territory. A second normalization using `near_distance_bases` defines selected territory. Near-target territory is selected territory minus target territory.

Every accepted aligned reference-base observation is assigned exactly once:

```text
on_target_bases + near_target_bases + off_target_bases
    == total_accepted_aligned_bases
```

The reducer checks this invariant before returning a report.

Aggregate target depth uses unique normalized target territory. Overlapping source BED intervals therefore do not double-count aggregate territory or aggregate depth.

### Per-source target model

Per-target output retains original source BED identity:

- source index;
- source line number;
- contig/start/end;
- optional BED name;
- target length;
- exact depth sum and mean;
- covered/uncovered bases;
- configured threshold counts and percentages;
- maximal zero-depth half-open runs;
- longest zero-depth run.

Overlapping source targets may each receive shared genomic bases in their own source-level reports; this is intentional and independent from aggregate union accounting.

A valid zero-length source target is retained, but denominator-based metrics are explicitly unavailable.

### Native enrichment

`target_enrichment` is defined as:

```text
(on_target_bases / total_accepted_aligned_bases)
/ (target_territory_bases / genome_territory_bases)
```

The implementation uses checked integer products before deterministic six-decimal rendering. Required zero denominators produce explicit unavailability rather than zero, infinity, or fallback values.

### Native uniformity penalty

Milestone 9 does not claim Picard `FOLD_80_BASE_PENALTY` compatibility. The ADR-approved native metric is:

```text
target_uniformity_penalty_80 = mean_target_depth / D20
```

`D20` is the deterministic nearest-rank 20th-percentile depth across all bases in unique target territory, including zero-depth bases. If `D20 == 0`, the metric is explicitly unavailable.

## Canonical output and provenance

Summary schema `1.1.0` adds typed `coverage.targeted` availability. Non-targeted runs explicitly emit:

```text
targeted = unavailable("target_bed_not_supplied")
```

Targeted runs record:

- native profile;
- underlying coverage profile;
- exact target SHA-256 and byte size;
- source interval count;
- near distance;
- genome, target, and near-target territory;
- on/near/off aligned bases;
- target histogram and thresholds;
- per-source target reports;
- dropout count;
- enrichment;
- D20;
- native uniformity penalty.

Provenance records the original BED path plus exact byte identity and both zero-flank and selected-set normalization actions. It also records:

```text
targeted_profile = aligngauge-targeted-v0.3
target_metric_compatibility = native-no-picard-compatibility-claim
```

## Resource accounting

The targeted reducer participates in the existing hard memory plan before traversal. The plan reserves bounded state for:

- source-target state and configured threshold counters;
- normalized target/selected union intervals;
- target BED bytes;
- target depth histogram;
- zero-coverage runs.

If the additional state would exceed `--memory-limit`, initialization fails with `resource_limit`; AlignGauge does not silently switch to an unbounded or approximate target algorithm.

The validated HG002 v0.3 run used a 1 GiB limit and recorded a planned coverage peak of `617614388` bytes.

## Synthetic exact oracle

Committed fixture:

`crates/aligngauge-coverage/tests/fixtures/chunk_boundary_targets.bed`

The target oracle deliberately covers overlapping source targets, a chunk boundary, a target with a dropout run, and a valid zero-length target.

For the existing `chunk_boundary.bam` fixture with near distance 5:

| Metric | Exact expected value |
| --- | ---: |
| total accepted aligned bases | 28 |
| target territory | 22 |
| near-target territory | 16 |
| on-target bases | 16 |
| near-target bases | 12 |
| off-target bases | 0 |
| target histogram | `0:10, 1:8, 2:4` |
| mean target depth | `0.727273` |
| covered target bases | 12 |
| uncovered target bases | 10 |
| bases ≥1× | 12 (`54.545455%`) |
| bases ≥2× | 4 (`18.181818%`) |
| dropout targets | 1 |
| target enrichment | `51948.051948` |
| D20 | 0 |
| uniformity penalty 80 | unavailable: `target_depth_20th_percentile_is_zero` |

The same complete targeted summary is asserted across chunk sizes 1, 7, 1024, and 65536 bases.

Per-source assertions include:

- target A mean depth `1.500000`;
- overlapping target B mean depth `1.000000`;
- target C mean depth `0.166667` with exact zero run `[65550,65560)` and longest dropout 10 bases;
- zero-length target denominator metrics unavailable.

## CLI fail-closed validation

Release integration tests prove:

- `--targets` publishes native targeted metrics through the existing atomic publication path;
- default/explicit near-distance handling is deterministic;
- `--near-distance` without `--targets` fails before output creation;
- missing target BED fails before publication;
- target path/SHA/size and normalization are preserved in provenance;
- targeted release still records exactly one alignment traversal.

Validated CLI integration run:

- run `31202016327`
- job `92944029589`
- result: success

## BAM/CRAM targeted equivalence

The existing v0.2 equivalent BAM/CRAM fixture was extended with the same target BED and target-aware release API.

The test requires equality of:

- counters;
- whole-genome coverage;
- complete canonical summary;
- native targeted summary;
- common target provenance/normalization actions.

Only already specified input-format/local-reference provenance differences are excluded from the common-plan comparison. Both formats assert one input traversal.

Validated run:

- run `31202581374`
- job `92945858259`
- result: success

## Pinned HG002 targeted differential

### Input projection

The existing deterministic HG002 chr20 coverage projection is reused. Its declared projected reference length is 1,100,000 bases.

Committed target definition:

`testdata/hg002/targeted-v0.3.bed`

Identity:

- SHA-256: `9eb4a3b4f9318521ac6f25ac3037d41a74133fdf215f9e281df3c6a801d408b4`
- size: 210 bytes
- source intervals: 6
- total unique zero-flank target territory: 6,000 bases
- default selected/near distance: 250 bases
- selected territory: 9,000 bases

### Reference tool

Pinned Samtools:

- version: `1.24`
- image: `quay.io/biocontainers/samtools@sha256:58c844089e2bd5114921b679be5956a0cd503140dcaed545caab37cd9947d64b`

The target-depth reference command runs in Docker with `--network none` and captures invocation, image, version, stdout, stderr, exit status, wall time, and `_SUCCESS`.

The effective depth filter is:

```text
samtools depth -a -q 0 -Q 0 \
  -G UNMAP,SECONDARY,QCFAIL,DUP,SUPPLEMENTARY \
  -b <TARGET_BED> <BAM>
```

This independently matches the comparable canonical coverage primitive: primary, mapped, QC-pass, nonduplicate records; MAPQ/base-quality floor 0; deletions excluded; no mate-overlap removal.

This is a differential oracle for comparable coverage primitives, **not** a named Picard or Samtools compatibility profile.

### Exact independently reduced fields

`tools/reference/samtools/compare-target-depth.py` independently reconstructs target arrays from Samtools position/depth output and requires exact equality for aggregate:

- target territory;
- on-target depth sum;
- complete target depth histogram;
- covered/uncovered target bases;
- configured threshold bases and percentages;
- target mean depth;
- D20;
- native uniformity penalty.

It also requires exact equality for every source target:

- source identity and coordinates;
- length;
- depth sum;
- mean depth;
- covered/uncovered counts;
- threshold counts/percentages;
- exact zero-coverage runs;
- longest zero run.

The comparator also checks exact BED SHA/size and dropout target count.

AlignGauge's canonical histogram representation intentionally preserves an explicit zero-depth bin even when its count is zero. The independent reducer preserves that representation after independently measuring the bin count.

### Observed HG002 targeted result

The exact validated run produced:

| Metric | Value |
| --- | ---: |
| genome territory | 1,100,000 |
| target territory | 6,000 |
| near-target territory | 3,000 |
| on-target bases | 186,436 |
| near-target bases | 94,243 |
| off-target bases | 30,714,729 |
| mean target depth | `31.072667` |
| covered target bases | 6,000 |
| uncovered target bases | 0 |
| bases ≥1× | 6,000 (`100.000000%`) |
| bases ≥10× | 6,000 (`100.000000%`) |
| bases ≥20× | 5,723 (`95.383333%`) |
| bases ≥30× | 3,585 (`59.750000%`) |
| dropout targets | 0 |
| target enrichment | `1.102742` |
| D20 | 26 |
| target uniformity penalty 80 | `1.195103` |

The exact aligned-base partition also passed:

```text
186436 + 94243 + 30714729 == total_accepted_aligned_bases
```

### Differential artifact

Permanent Targeted Validation on pre-evidence head `04ba604c120256b55c51a8a5e7871e4f9850181d`:

- run `31203909781`
- job `92950210864`
- result: success
- artifact ID: `9003972168`
- artifact digest: `sha256:8e28e8879793407c427614f3bf29e8bd55d7cb74f0b5d6a4d0cc7446c49bfa54`

The archived differential report has:

```text
schema = aligngauge-targeted-samtools-differential-v1
status = exact
compatibility_claim = null
```

No tolerance is applied.

## Compatibility boundary

v0.3 does not claim Picard `CollectHsMetrics`, Picard `FOLD_ENRICHMENT`, or Picard `FOLD_80_BASE_PENALTY` compatibility.

Reasons are explicit:

- v0.3 has one target BED rather than separate bait and target sets;
- canonical AlignGauge filtering differs from Picard defaults;
- mate-overlap treatment differs;
- the native D20 definition includes zero-depth target bases.

The exact Samtools differential validates independently comparable coverage primitives only. The project-native near/off partition and enrichment are validated by synthetic oracles, checked partition arithmetic, BAM/CRAM equivalence, and the full HG002 release path rather than being mislabeled as external-tool compatibility.

## Pre-evidence validation gates

All standing gates passed on exact head `04ba604c120256b55c51a8a5e7871e4f9850181d`:

| Gate | Run | Job | Result |
| --- | --- | --- | --- |
| Permanent CI | `31203909778` | `92950210259` | success |
| Full Runtime Validation | `31203909861` | `92950210705` | success |
| Reference Validation | `31203909785` | `92950210420` | success |
| Targeted Validation | `31203909781` | `92950210864` | success |

Permanent CI includes strict Clippy, the complete workspace test suite, JSON schema validation, permanent syntax validation for the targeted reference tooling, CRAM no-network isolation, documentation, and clean-tree verification.

Reference Validation explicitly projects schema 1.1 non-targeted coverage onto the historical v0.1 differential contract only after requiring `targeted = unavailable("target_bed_not_supplied")`; it does not silently ignore available target data.

## v0.3.0 release closure

Milestone 9 was merged by PR #3. The merged product/evidence commit was `ffa0994369eb9981821249207f2abc2c6202ef2f`, which passed Permanent CI, Full Runtime Validation, Reference Validation, Targeted Validation, and HG002 Preparation Validation before release-candidate preparation.

The final release candidate was `eccd45157d34ada00a3403a2b24d606956878b62`. All five release-relevant gates succeeded on that exact SHA:

| Gate | Run | Job | Result |
| --- | --- | --- | --- |
| Permanent CI | `31205398918` | `92955023998` | success |
| Full Runtime Validation | `31205397896` | `92955018188` | success |
| Reference Validation | `31205397729` | `92955080287` | success |
| Targeted Validation | `31205397861` | `92955082757` | success |
| HG002 Preparation Validation | `31205397734` | `92955018025` | success |

GitHub release `v0.3.0` (release ID `366930828`) was published on 2026-08-07 with tag target exactly `eccd45157d34ada00a3403a2b24d606956878b62`. The release is neither a draft nor a prerelease. One-time publisher run `31205792390`, job `92956442182`, succeeded and verified the tag target and release metadata before its temporary publisher branch was removed by cleanup run `31205917061`, job `92956811061`.

The release tag remains policy-pinned to the validated release SHA. Post-release documentation may advance `master` without moving the `v0.3.0` tag.
