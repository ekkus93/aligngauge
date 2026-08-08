# AlignGauge v0.4 release validation evidence

**State:** final v0.4 release-candidate evidence. No `v0.4.0` tag or GitHub release is assumed by this file; publication occurs only after the exact commit containing this evidence passes the permanent release gates.

## Release scope

ADR-0011 freezes these v0.4 compatibility profiles:

- `samtools-stats-1.24-multiqc-1.35`
- `picard-alignment-summary-3.4.0-all-reads-subset-v1`
- `picard-insert-size-3.4.0-all-reads-v1`

Existing `samtools-flagstat` and `samtools-idxstats` projections remain available from earlier releases without semantic widening.

Picard WgsMetrics and HsMetrics are **not** v0.4 release profiles. The CLI rejects `picard-wgs` and `picard-hs-metrics`. ADR-0009 selected those as future candidate surfaces and Milestone 13 proved their exact overlap primitives, but complete WGS/Hs filtering, reductions, renderers, metric differentials, and generated-output MultiQC equivalence remain unproved.

Native `target_uniformity_penalty_80` remains distinct from Picard `FOLD_80_BASE_PENALTY`; no value is copied, aliased, zero-filled, or relabeled.

## Release-gate implementation candidate

Candidate SHA:

`191fd927c506d037dad57b8209d132f78a36d025`

Successful workflows on that exact SHA:

- V0.4 Release Validation run `31254954734`, job `93096874003`
- Permanent CI run `31254954580`, job `93096873958`
- Reference Validation run `31254954662`, job `93096874325`

This candidate established the permanent `ci/v0.4-release` workflow and ADR-0011.

## Reconciled compatibility-report candidate

Evidence SHA:

`39f464d7a54ed3b18fff0ea62e1fc47e71b7596f`

Successful workflows on that exact SHA:

- V0.4 Release Validation run `31255113725`, job `93097266573`
- Permanent CI run `31255113745`, job `93097251861`
- Reference Validation run `31255113727`, job `93097266292`
- MultiQC Validation run `31255113729`, job `93097242866`
- Exact Overlap Validation run `31255113726`, job `93097242950`

`docs/evidence/V0_4_COMPATIBILITY_REPORT.md` on this state reconciles every v0.4 field claim: all 39 ordinary Samtools 1.24 `SN` rows plus the default `IS` surface, the exact 13 Picard AlignmentSummary fields, the complete released Picard InsertSize row/histogram surface, and all explicit unsupported/deferred boundaries.

## Broad release-surface candidate

Candidate SHA:

`ba53a4dca8e06a653a3ad23c1f6a8711628a096d`

Every PR workflow triggered by that exact SHA succeeded:

- Permanent CI run `31255308013`, job `93097721487`
- Full Runtime Validation run `31255308000`, job `93097699603`
- Reference Validation run `31255308023`, job `93097722713`
- Targeted Validation run `31255308011`, job `93097724071`
- Samtools Stats Validation run `31255308064`, job `93097699600`
- Picard Validation run `31255308045`, job `93097699494`
- MultiQC Validation run `31255308016`, job `93097699402`
- Exact Overlap Validation run `31255308010`, job `93097699391`
- V0.4 Release Validation run `31255308018`, job `93097723581`

This state also updated both public release-surface READMEs so they describe the actual v0.4 candidate instead of stale Milestone 12-next language.

## Final PR head validation

PR #8 final head:

`51fa8651042222808569e1de4b502a7db13fe7ae`

All nine required PR workflows succeeded on that exact head before merge:

- Permanent CI run `31255441265`, job `93098011516`
- Full Runtime Validation run `31255441202`
- Reference Validation run `31255441216`
- Targeted Validation run `31255441206`, job `93098011319`
- Samtools Stats Validation run `31255441209`, job `93098011253`
- Picard Validation run `31255441205`, job `93098011250`
- MultiQC Validation run `31255441219`, job `93098011260`
- Exact Overlap Validation run `31255441220`, job `93098011238`
- V0.4 Release Validation run `31255441229`, job `93098011455`

PR #8 was merged only after this exact head was green.

## Merged master validation

PR #8 merged to `master` as:

`c1ded07bad71b330aa712d65ec38850de009a218`

Every workflow triggered by that exact merge SHA completed successfully:

- Permanent CI run `31255545790`, job `93098258374`
- Full Runtime Validation run `31255545774`, job `93098258467`
- Reference Validation run `31255545825`, job `93098258508`
- Targeted Validation run `31255545812`, job `93098258451`
- Samtools Stats Validation run `31255545783`, job `93098258389`
- Picard Validation run `31255545796`, job `93098258432`
- MultiQC Validation run `31255545773`, job `93098258318`
- Exact Overlap Validation run `31255545792`, job `93098258383`
- V0.4 Release Validation run `31255545776`, job `93098258412`
- M2 HG002 Preparation Validation run `31255545808`, job `93098258437`

The merge therefore passed the complete v0.4 compatibility/runtime matrix plus the HG002 preparation/performance path on actual `master`, not only on a pull-request merge ref.

## Executable v0.4 proof

The permanent v0.4 gate proves all of the following on each candidate SHA it validates:

1. pinned tool identities are Samtools 1.24, Picard 3.4.0 / bundled HTSJDK 4.2.0, and MultiQC 1.35;
2. the released CLI exposes only the proved v0.4 compatibility profiles and rejects unproved WGS/Hs format names;
3. ordinary whole-input canonical `summary.json` is byte-identical between serial decoding and `--io-threads 2`;
4. targeted canonical `summary.json` is byte-identical between serial decoding and `--io-threads 2`;
5. `--threads 2` remains an explicitly serial collector configuration, reports `collector_threads_used = 1`, and emits `collector_threads_serial_v0_1`;
6. generated Samtools Stats matches pinned Samtools 1.24 exactly with no blanket numerical tolerance;
7. pinned MultiQC 1.35 independently parses generated Samtools reference and AlignGauge text and the parsed Samtools Stats / insert-size data are byte-identical;
8. generated Picard AlignmentSummary and InsertSize projections match pinned Picard 3.4.0 exactly;
9. pinned MultiQC 1.35 independently parses generated Picard InsertSize reference and AlignGauge text and parsed data are byte-identical;
10. WGS/Hs discovery fixtures remain discovery-only with `compatibility_claim: false`;
11. required evidence artifacts must exist and the validation checkout must remain clean.

Parser success alone is never accepted as compatibility proof. Unsupported fields are omitted rather than synthesized as zero. Reference-tool or parser failure is fatal.

## Released execution-mode boundary

Collector/reduction execution remains deterministic and serial. Released bounded concurrency is HTSlib reader/decompression concurrency through `--io-threads` while preserving one logical ordered record stream.

- `--io-threads 0` normalizes to one effective reader thread.
- `--io-threads 2` uses two effective reader I/O threads in the release-gate proof.
- canonical summaries are byte-identical across those settings for whole-input and targeted paths.
- provenance intentionally differs where it truthfully records configured/effective I/O threads and timing values.
- indexed reference-partition execution remains unsupported by ADR-0010.

## Fail-closed release boundaries

The release gate fails rather than degrades when:

- a pinned reference-tool contract changes;
- an unproved WGS/Hs compatibility format appears;
- serial and concurrent canonical outputs differ;
- execution settings are misreported;
- direct Samtools or Picard differential output is not exact;
- pinned MultiQC cannot parse a claimed generated surface;
- parsed MultiQC data differs between reference and AlignGauge;
- discovery-only fixtures acquire a compatibility claim;
- required artifacts are missing; or
- validation leaves repository state dirty.

There is no warning-only reference path, approximate compatibility fallback, zero-fill policy, or pre-success evidence marker.

## Exact release-candidate trigger hardening

Immediately after the fully green merge, master commit `03e5cc3fe267c714d0f96830c5a47ebb396f8ece` added this file itself to the push and pull-request path filters for `.github/workflows/v0.4-release-validation.yml`.

That closes a subtle release-process gap: an evidence-only final candidate can no longer change `V0_4_RELEASE_VALIDATION.md` without triggering `ci/v0.4-release` on that exact SHA.

The commit containing this final evidence text is therefore the intended `v0.4.0` tag target **only if** its own Permanent CI and V0.4 Release Validation runs, plus every other workflow it triggers, complete successfully.

## Publication rule

No tag is created speculatively.

After the exact commit containing this evidence is green, publication follows the repository's established one-time-publisher pattern:

1. create a temporary child commit containing a narrowly scoped publisher workflow;
2. require that publisher to verify its parent is the exact validated release SHA;
3. re-check the required release-SHA workflow conclusions;
4. refuse publication if `v0.4.0` already exists;
5. create the GitHub release with `target_commitish` equal to the validated parent SHA;
6. verify the resulting tag resolves to that exact SHA;
7. remove the temporary write-enabled publisher;
8. close the TODO and post-release documentation in later non-release commits.

The publisher child is never the release target. The tag must point to the already-validated parent candidate.
