# Milestone 11 — Picard Alignment Summary / Insert Size Validation Evidence

**Milestone:** 11 — Picard alignment and insert-size profiles  
**Date:** 2026-08-07  
**Pre-evidence validated source SHA:** `46b8a330cc26fd2b0f472bcc72322c01fd15311f`  
**Reference implementation:** Picard 3.4.0  
**Alignment profile:** `picard-alignment-summary-3.4.0-all-reads-subset-v1`  
**Insert-size profile:** `picard-insert-size-3.4.0-all-reads-v1`  
**Disposition:** Implementation and direct differential validation complete; final evidence-commit acceptance remains gated on exact-SHA permanent CI. Milestone 11 does not publish or imply `v0.4.0`.

## Scope

Milestone 11 implements two deliberately narrow Picard 3.4.0 `ALL_READS` compatibility profiles. It does **not** claim general Picard compatibility.

The public compatibility probes are:

```text
aligngauge qc --input <BAM> --format picard-alignment-summary
aligngauge qc --input <BAM> --format picard-insert-size
```

Normal v0.1-v0.3 QC and the Milestone 10 Samtools-stats profile do not silently acquire the additional sequence/XN/CIGAR/mate/TLEN decoding work required by these profiles.

## Pinned Picard identity

Repository lock:

`tools/reference/picard/image.lock`

- version: `3.4.0`
- tag: `broadinstitute/picard:3.4.0`
- immutable image: `broadinstitute/picard@sha256:f11df229b5b49ea28a872c04fc5d33e76bd14890754079390f36897c16194b28`
- jar: `/usr/picard/picard.jar`
- Java: OpenJDK `17.0.14`
- R and Rscript are present in the pinned image.

All differential Picard executions run after the image is locally available and use Docker `--network none`. The reference runners reject an unpinned image identity.

## Normative semantic boundary

ADR-0008 and SPEC §12.9 were committed before product implementation.

### Alignment-summary exact subset

Milestone 11 claims exact Picard 3.4.0 behavior for these columns only:

1. `CATEGORY`
2. `TOTAL_READS`
3. `PF_READS`
4. `PCT_PF_READS`
5. `PF_NOISE_READS`
6. `PCT_ADAPTER`
7. `MEAN_READ_LENGTH`
8. `SD_READ_LENGTH`
9. `MEDIAN_READ_LENGTH`
10. `MAD_READ_LENGTH`
11. `MIN_READ_LENGTH`
12. `MAX_READ_LENGTH`
13. `BAD_CYCLES`

The compatibility renderer is required to contain exactly those columns. Reference-dependent Picard fields such as aligned-base, mismatch/error/indel, chimera, strand-balance, clipping, and pair-alignment fields are unsupported in Milestone 11 and remain absent rather than being serialized as invented zeros.

Category semantics match Picard 3.4.0:

- paired inputs with first-of-pair reads emit `FIRST_OF_PAIR`, `SECOND_OF_PAIR`, and `PAIR`;
- unpaired data emit `UNPAIRED`;
- empty input uses Picard's empty `UNPAIRED` semantics;
- both secondary and supplementary records are rejected by the top-level Picard alignment-summary collector before category dispatch, so they contribute neither read counts nor `BAD_CYCLES`.

The subset also reproduces Picard's default adapter list, 16-base/one-error adapter matching boundary, MAPQ-0 mapped-adapter rule, `XN` noise-read semantics, reverse-cycle handling, and the exact 80% bad-cycle threshold.

### Insert-size exact profile

Milestone 11 claims the Picard 3.4.0 default `CollectInsertSizeMetrics` `ALL_READS` metrics row(s) and trimmed histogram. The PDF chart itself is not part of the claim.

The frozen defaults are:

- `DEVIATIONS = 10.0`;
- `HISTOGRAM_WIDTH = null`;
- `MIN_HISTOGRAM_WIDTH = null`;
- `MINIMUM_PCT = 0.05f` as a Java `float`, promoted to the collector's `double`;
- `INCLUDE_DUPLICATES = false`;
- `ALL_READS` accumulation only.

Eligibility and orientation semantics match the pinned Picard/HTSJDK path: mapped paired records with mapped mates, second-of-pair observation only, no secondary/supplementary/duplicate records, nonzero TLEN, checked `abs(TLEN)`, and exact FR/RF/TANDEM orientation.

The profile covers:

- median;
- mode and tie-breaking;
- median absolute deviation;
- min/max;
- mean and sample standard deviation after trimming;
- read-pair count;
- pair orientation;
- `WIDTH_OF_10_PERCENT` through `WIDTH_OF_99_PERCENT`;
- complete trimmed histogram text.

Automatic trimming reproduces Picard's Java expression `(int) (median + 10 * MAD)`, including truncation behavior.

## Executable discrepancies discovered and resolved during implementation

Milestone 11 intentionally treated the pinned executable as the final compatibility oracle where source reading or transcription was ambiguous.

### Supplementary alignment exclusion

An initial implementation allowed supplementary records to affect no-call bad-cycle counts. The adversarial alignment fixture disagreed with Picard. Re-reading the top-level Picard 3.4.0 collector showed that both secondary and supplementary records are rejected before category dispatch. AlignGauge, ADR-0008, and SPEC §12.9 were corrected together.

Validated correction:

- run `31230236430`
- job `93032521746`
- result: success

### Java `float` `MINIMUM_PCT`

An initial implementation used mathematical binary64 `0.05`, which retained an orientation with 2 observations out of 40. Picard 3.4.0 suppressed that category despite the collector source using a `>=` comparison.

A dedicated pinned-executable diagnostic established the reason: the command field is Java `float` `0.05f`, and that binary32 value is promoted to `double` before the comparison. The promoted value is slightly greater than mathematical 0.05. Therefore a mathematically exact 5% category can be below Picard's actual threshold. AlignGauge now stores the constant as `f32` and promotes it for the comparison, reproducing the same boundary.

Validated correction:

- run `31230581559`
- job `93033482801`
- result: success

### Empty insert-size output

Picard 3.4.0 exits successfully but does not create its `OUTPUT` metrics file when there are no eligible insert observations. The reference runner records this explicitly as `output_state.txt = absent` and creates an empty capture placeholder only for stable differential tooling. AlignGauge's compatibility renderer emits no text for the corresponding empty report.

## Deterministic adversarial fixtures

The M11 fixtures are generated through the repository's normal deterministic synthetic-corpus machinery and are pinned in `testdata/manifest.v1.tsv`.

### `picard_alignment_edge`

- BAM SHA-256: `9956c88b79f048cf791fda3f239e611b48493eac7d56e10dde9b55316f029223`
- BAI SHA-256: `05b99f5553264f49624ba4778a7b90d0ff54869a686980da08409de204dcfcf8`

Exercises:

- unpaired and paired category dispatch;
- first-/second-of-pair aggregation;
- exact default adapter matching including reverse-strand handling;
- mapped MAPQ-0 adapter eligibility;
- `XN=1` noise semantics;
- no-call cycle orientation;
- exact 80% bad-cycle threshold;
- secondary/supplementary exclusion.

### `picard_insert_edge`

- BAM SHA-256: `568e5d4fab910a7a958c6ee95622cb949c247bc0768657792130fd4d6aa8105c`
- BAI SHA-256: `b4d73ff0e3d1503a6b548d272ef3594d9abf6e80e0873081f19a71573b56074a`

The eligible distribution includes 37 FR observations with a deliberately extreme `1000` outlier, 2 RF observations, and 1 TANDEM observation, plus records that must be excluded by duplicate, secondary, supplementary, mate-unmapped, zero-TLEN, and first-of-pair rules.

It proves:

- exact orientation filtering;
- mode tie behavior;
- even/odd robust statistics;
- median/MAD trimming;
- centered percentage widths;
- suppression at the Java-float 5% boundary;
- no accidental participation by excluded records.

Deterministic fixture validation:

- run `31229888397`
- job `93031492304`
- result: success

## Core implementation validation

The core field-plan/reader/collector/CLI slice was accepted only after one run passed all of:

- workspace compilation;
- strict Clippy with warnings denied;
- complete workspace tests;
- public CLI format probes;
- restricted generated-diff verification;
- temporary-builder cleanup.

Run/job:

- run `31229027252`
- job `93028898472`
- result: success

No lint suppression or saturating arithmetic was used to get the slice through validation.

## Direct pinned Picard oracle

Before promotion into permanent CI, a temporary oracle compared both compatibility profiles directly against the immutable Picard 3.4.0 image under network isolation.

Final direct-oracle run:

- run `31230661110`
- job `93033715869`
- result: success

Exact alignment-summary comparisons passed for:

- `basic.bam`;
- `flags_and_pairs.bam`;
- `empty.bam`;
- `picard_alignment_edge.bam`.

Exact insert-size comparisons passed for:

- `basic.bam` empty-output semantics;
- `flags_and_pairs.bam`;
- `empty.bam`;
- `picard_insert_edge.bam`.

The temporary oracle and threshold diagnostic workflows were removed before the PR acceptance candidate.

## Permanent differential tooling

Reference runners:

- `tools/reference/picard/run-alignment-summary.sh`
- `tools/reference/picard/run-insert-size.sh`

Exact comparators:

- `tools/reference/picard/compare-alignment-summary.py`
- `tools/reference/picard/compare-insert-size.py`

Differential schemas:

- `aligngauge-picard-alignment-summary-differential-v1`
- `aligngauge-picard-insert-size-differential-v1`

Successful reports require:

```text
status = exact
tolerance = null
```

There is no blanket or field-specific tolerance for either claimed surface.

## Permanent `ci/picard` validation

Permanent workflow:

`.github/workflows/picard-validation.yml`

Job name:

`ci/picard`

On the pre-evidence source SHA `46b8a330cc26fd2b0f472bcc72322c01fd15311f`, the permanent gate passed:

- Picard lock and reference-tool validation;
- deterministic M11 fixture identities;
- synthetic exact alignment-summary differential;
- synthetic exact insert-size differential;
- deterministic HG002 subset preparation;
- **exact HG002 alignment-summary differential**;
- **exact HG002 insert-size differential**;
- explicit proof that unsupported alignment-summary columns remain absent;
- evidence artifact upload.

Run/job:

- run `31230920624`
- job `93034458877`
- result: success

Artifact:

- artifact ID: `9013783385`
- size: `19,240` bytes
- digest: `sha256:a6cf71dc20f7a2cba27cfc386cdb200da1d086518360b9d19affe23b197f5670`
- expired: false at capture time

GitHub names this artifact `picard-validation-85a3fd1382f29ab5b15958ca18a152a4aa23c891` because `${{ github.sha }}` in a pull-request workflow refers to GitHub's synthetic PR merge commit. The workflow-run metadata independently records the authoritative source `head_sha` as `46b8a330cc26fd2b0f472bcc72322c01fd15311f`. This evidence therefore does not confuse the synthetic PR merge SHA with the source commit being validated.

## Pre-evidence exact-SHA regression matrix

All standing PR gates completed successfully on source SHA `46b8a330cc26fd2b0f472bcc72322c01fd15311f`:

| Gate | Run | Job | Result |
| --- | ---: | ---: | --- |
| Permanent CI | `31230920655` | `93034459006` | success |
| Full Runtime Validation | `31230920636` | `93034459029` | success |
| Reference Validation | `31230920649` | `93034480726` | success |
| Targeted Validation | `31230920638` | `93034490971` | success |
| Samtools Stats Validation | `31230920646` | `93034458952` | success |
| Picard Validation | `31230920624` | `93034458877` | success |

This matrix proves that the planned sequence/XN and Picard-specific reader extensions did not regress the v0.1-v0.3 canonical paths, targeted differential, or the Milestone 10 Samtools/MultiQC compatibility profile.

## Expected differences and deferred Picard surface

These are explicit non-claims, not hidden discrepancies:

- reference-dependent `CollectAlignmentSummaryMetrics` fields are omitted;
- PDF histogram chart compatibility is not claimed;
- accumulation is `ALL_READS` only;
- SAMPLE, LIBRARY, and READ_GROUP breakdowns are deferred to Milestone 12;
- WGS and hybrid-selection Picard metrics are deferred to Milestone 12;
- MultiQC parser acceptance for Picard output is deferred to Milestone 12;
- v0.4 release closure is deferred until Milestones 12 and 13 satisfy the v0.4 release gate.

## Clean evidence candidate acceptance

The clean evidence candidate `ad212b839d3054aae4c1206c5c451f4c6b098b2d` passed the complete six-gate PR matrix before merge:

| Gate | Run | Job | Result |
| --- | ---: | ---: | --- |
| Permanent CI | `31231342595` | `93035556380` | success |
| Full Runtime Validation | `31231342597` | `93035556543` | success |
| Reference Validation | `31231342616` | `93035556657` | success |
| Targeted Validation | `31231342605` | `93035556643` | success |
| Samtools Stats Validation | `31231342607` | `93035556622` | success |
| Picard Validation | `31231342594` | `93035556665` | success |

This exact SHA therefore satisfies the Milestone 11 evidence-commit acceptance gate.

## Exact merged-master validation

PR #5 merged only from the validated evidence head. The exact merge commit is `b5ec36f05110a458fbc70a1b38debeefa2a195cd`.

All seven push workflows attached to that exact merge commit completed successfully:

| Gate | Run | Job | Result |
| --- | ---: | ---: | --- |
| Permanent CI | `31231472869` | `93036022316` | success |
| Full Runtime Validation | `31231472841` | `93036022164` | success |
| Reference Validation | `31231472844` | `93036022206` | success |
| Targeted Validation | `31231472845` | `93036022139` | success |
| Samtools Stats Validation | `31231472859` | `93036022163` | success |
| Picard Validation | `31231472871` | `93036022279` | success |
| HG002 Preparation Validation | `31231472840` | `93036022356` | success |

The merge-side `ci/picard` job again passed exact synthetic alignment-summary and insert-size comparisons, deterministic HG002 preparation, exact HG002 alignment-summary and insert-size differentials, unsupported-column exclusion, and evidence upload.

## Acceptance state

Milestone 11 is complete. The implementation, direct Picard differential, adversarial fixtures, exact HG002 differentials, clean evidence candidate, and exact merged `master` commit all satisfy the required fail-closed validation boundary. Milestone 12 is next. Milestone 11 does not publish or imply a `v0.4.0` release.
