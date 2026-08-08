# AlignGauge v0.4 release validation evidence

**State:** `v0.4.0` published and post-release closure complete. The release tag points to the exact pre-publication commit that passed the permanent release gates; later bookkeeping commits do not alter the tagged tree.

## Published release identity

- Tag: `v0.4.0`
- GitHub release ID: `367190259`
- Exact release SHA: `5be4aa4e5df3e8feb17fdde46c408683ac08bb53`
- PR #8 merge SHA: `c1ded07bad71b330aa712d65ec38850de009a218`
- Release is non-draft and non-prerelease.

The Git tag resolves directly to `5be4aa4e5df3e8feb17fdde46c408683ac08bb53`; it does not point to the temporary publisher commit or any later documentation commit.

## Exact release-SHA validation

The exact tag target `5be4aa4e5df3e8feb17fdde46c408683ac08bb53` passed every workflow triggered by the final release-evidence commit:

- Permanent CI run `31255804251`, job `93098870269` — success
- Reference Validation run `31255804281`, job `93098876451` — success
- V0.4 Release Validation run `31255804250`, job `93098876838` — success

The release tag was not created until all three were complete and successful.

The final v0.4 gate on that exact SHA successfully completed all of these executable assertions:

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
11. required evidence artifacts exist and the validation checkout remains clean.

Parser success alone was not accepted as compatibility proof. Unsupported fields remain absent rather than being synthesized as zero. Reference-tool or parser failure is fatal.

## Released compatibility scope

ADR-0011 freezes these v0.4 compatibility profiles:

- `samtools-stats-1.24-multiqc-1.35`
- `picard-alignment-summary-3.4.0-all-reads-subset-v1`
- `picard-insert-size-3.4.0-all-reads-v1`

Existing `samtools-flagstat` and `samtools-idxstats` projections remain available from earlier releases without semantic widening.

`docs/evidence/V0_4_COMPATIBILITY_REPORT.md` reconciles the complete claim boundary:

- all 39 ordinary Samtools 1.24 `SN` rows plus the default released `IS` surface;
- exactly the 13 reference-independent Picard AlignmentSummary fields from ADR-0008;
- the complete released Picard InsertSize default `ALL_READS` metrics row plus trimmed histogram surface;
- the exact MultiQC-generated-output claim boundary;
- every explicit unsupported/deferred surface.

Pinned MultiQC 1.35 compatibility is claimed only where generated output was independently parsed and compared to parsed reference output:

- Samtools Stats: parsed `multiqc_samtools_stats.txt` and `samtools_insert_size.txt` are byte-identical reference versus AlignGauge;
- Picard InsertSize: parsed Picard insert-size data are byte-identical reference versus AlignGauge.

The 13-field Picard AlignmentSummary subset remains a direct Picard 3.4.0 compatibility claim, not a MultiQC claim, because the pinned parser requires reference-dependent columns outside the released subset.

## WGS/Hs and fold-80 boundary

Picard WgsMetrics and HsMetrics are **not** v0.4 release profiles. The CLI rejects `picard-wgs` and `picard-hs-metrics`.

ADR-0009 selected those as future candidate surfaces, and Milestone 13 proved their exact overlap primitives against pinned Picard 3.4.0 / HTSJDK 4.2.0. Complete WGS/Hs filtering, denominators, coverage/target reductions, renderers, full metric differentials, and generated-output MultiQC equivalence remain outside v0.4.

Native `target_uniformity_penalty_80` remains distinct from Picard `FOLD_80_BASE_PENALTY`; no value is copied, aliased, zero-filled, or relabeled.

## Released execution-mode boundary

Collector/reduction execution remains deterministic and serial. Released bounded concurrency is HTSlib reader/decompression concurrency through `--io-threads` while preserving one logical ordered record stream.

- `--io-threads 0` normalizes to one effective reader thread.
- `--io-threads 2` uses two effective reader I/O threads in the release-gate proof.
- canonical summaries are byte-identical across those settings for whole-input and targeted paths.
- provenance intentionally differs where it truthfully records configured/effective I/O threads and timing values.
- indexed reference-partition execution remains unsupported by ADR-0010.

## Validation chain before the release SHA

### Release-gate implementation candidate

`191fd927c506d037dad57b8209d132f78a36d025`

- V0.4 Release Validation run `31254954734`, job `93096874003` — success
- Permanent CI run `31254954580`, job `93096873958` — success
- Reference Validation run `31254954662`, job `93096874325` — success

### Reconciled compatibility-report candidate

`39f464d7a54ed3b18fff0ea62e1fc47e71b7596f`

- V0.4 Release Validation run `31255113725`, job `93097266573` — success
- Permanent CI run `31255113745`, job `93097251861` — success
- Reference Validation run `31255113727`, job `93097266292` — success
- MultiQC Validation run `31255113729`, job `93097242866` — success
- Exact Overlap Validation run `31255113726`, job `93097242950` — success

### Broad release-surface candidate

`ba53a4dca8e06a653a3ad23c1f6a8711628a096d`

Every triggered PR workflow succeeded:

- Permanent CI run `31255308013`, job `93097721487`
- Full Runtime Validation run `31255308000`, job `93097699603`
- Reference Validation run `31255308023`, job `93097722713`
- Targeted Validation run `31255308011`, job `93097724071`
- Samtools Stats Validation run `31255308064`, job `93097699600`
- Picard Validation run `31255308045`, job `93097699494`
- MultiQC Validation run `31255308016`, job `93097699402`
- Exact Overlap Validation run `31255308010`, job `93097699391`
- V0.4 Release Validation run `31255308018`, job `93097723581`

### Final PR head

`51fa8651042222808569e1de4b502a7db13fe7ae`

All nine required PR workflows succeeded before merge, including Permanent CI, Full Runtime, Reference, Targeted, Samtools Stats, Picard, MultiQC, Exact Overlap, and V0.4 Release Validation.

PR #8 was merged only after that exact head was green.

### Merged master validation

PR #8 merged to `master` as `c1ded07bad71b330aa712d65ec38850de009a218`.

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

The merge therefore passed the complete v0.4 compatibility/runtime matrix plus HG002 preparation on actual `master`, not only on a pull-request merge ref.

## Release-evidence trigger hardening

Master commit `03e5cc3fe267c714d0f96830c5a47ebb396f8ece` added this file itself to the push and pull-request path filters for `.github/workflows/v0.4-release-validation.yml`.

That ensured the final release-evidence commit `5be4aa4e5df3e8feb17fdde46c408683ac08bb53` triggered `ci/v0.4-release` on its own exact SHA instead of inheriting proof from a parent or PR-wide diff.

## Publication and verification

A one-time publisher was added only after the exact release SHA was green:

- publisher child commit: `5b551ffd352fd920856e48006c44a7ae6bc30419`
- publisher run `31255923016`, job `93099134102` — success

The publisher:

1. proved its immediate parent was the exact release SHA;
2. revalidated the exact run identities for Permanent CI, Reference Validation, and V0.4 Release Validation;
3. refused an already-existing `v0.4.0` tag or release;
4. created GitHub release ID `367190259` with `target_commitish = 5be4aa4e5df3e8feb17fdde46c408683ac08bb53`;
5. fetched the resulting tag and release independently and required both to resolve to the same exact target.

The publisher completed successfully and was removed in commit `163ff40e1fedb38dced8c9535d6a4260959e33d8`.

## Post-release closure

A separate one-time documentation closer was used only after publication:

- closer setup commit: `355b2834c02672577250ca144e323384722f8d38`
- closer run `31256011737`, job `93099344963` — success
- guarded documentation commit: `4a0e6360ec57cb407350c786542ae799986d47fc`
- closer removal commit: `4923b853da0b68a5742ff1cc6d357d22685f4759`

The closer verified the already-published release identity before editing and then updated only post-release documentation:

- `docs/DNA_QC_ENGINE_TODO.md` now marks all four v0.4 release gates complete and records the exact release SHA/CI identities;
- root `README.md` identifies `v0.4.0` as the latest published release and links the release;
- `crates/aligngauge-cli/README.md` records the exact published release SHA and release-SHA gates;
- `docs/evidence/V0_4_COMPATIBILITY_REPORT.md` records the published release identity.

Both temporary write-enabled one-time workflows were removed after their single intended use. No standing publisher or post-release closer remains in the repository.

## Fail-closed boundaries

The permanent release gate fails rather than degrades when:

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

This post-release evidence file is part of the permanent v0.4 validation trigger surface. Its own commit must therefore pass Permanent CI, Reference Validation, and V0.4 Release Validation before repository closure is considered complete.
