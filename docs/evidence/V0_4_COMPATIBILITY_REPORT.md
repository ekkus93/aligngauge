# AlignGauge v0.4 compatibility report

**Report state:** Milestones 12 and 13 compatibility-boundary evidence. This document is not a `v0.4.0` release declaration; the separate v0.4 release gate remains outstanding.

**Reference profiles**

- Samtools: 1.24
- Picard: 3.4.0
- HTSJDK: 4.2.0 as bundled by pinned Picard 3.4.0
- MultiQC: 1.35, immutable image from `tools/reference/multiqc/image.lock`

## Compatibility matrix

| Surface | AlignGauge status | Numerical evidence | MultiQC 1.35 status |
|---|---|---|---|
| Samtools stats selected SN/IS sections | exact supported subset | Milestone 10 exact differential evidence | parsed and exact in Milestone 10 |
| Picard AlignmentSummaryMetrics reference-independent 13-column subset | exact supported differential projection | Milestone 11 exact synthetic + HG002 evidence | **not claimed compatible**; MultiQC directly requires fields outside the M11 subset |
| Picard InsertSizeMetrics default `ALL_READS` metrics + trimmed histogram | exact supported differential projection | Milestone 11 exact synthetic + HG002 evidence | parsed Picard-vs-AlignGauge data byte-identical in Milestone 12 |
| Picard WgsMetrics selected default-MultiQC surface | selected candidate, not emitted | exact overlap primitive validated in Milestone 13; complete WGS collector/output differential still outstanding | discovery/parser fixture only; no compatibility claim |
| Picard HsMetrics selected capture/coverage surface | selected candidate, not emitted | exact pinned Hs overlap primitive validated in Milestone 13; complete Hs collector/output differential still outstanding | discovery/parser fixture only; no compatibility claim |
| Native `aligngauge-targeted-v0.3` | supported native profile | v0.3 validation evidence | no Picard HsMetrics claim |

## Milestone 10 carry-forward

Milestone 10 established the exact Samtools 1.24 compatibility subset consumed by the pinned MultiQC 1.35 Samtools parser. Its network-isolated parser validation compares MultiQC's parsed data from Samtools reference text and AlignGauge text rather than treating parser exit alone as sufficient evidence.

That evidence remains authoritative and is not duplicated under a new profile name.

## Milestone 11 carry-forward

Milestone 11 established two Picard 3.4.0 profiles:

1. `picard-alignment-summary-3.4.0-all-reads-subset-v1`
2. `picard-insert-size-3.4.0-all-reads-v1`

The alignment-summary profile is a deliberately reference-independent 13-column exact subset. It must remain that subset. Pinned MultiQC 1.35 directly indexes `PF_READS_ALIGNED` and `PF_ALIGNED_BASES` in its Picard alignment plot path, and those reference-dependent fields are outside the M11 compatibility claim. Milestone 12 therefore records the output as **not MultiQC-compatible** rather than adding fake zeros or approximate values.

The insert-size profile already contains the default `ALL_READS` fields and histogram required by MultiQC. Milestone 12 adds an end-to-end parser gate that requires the parsed MultiQC data produced from Picard reference output and AlignGauge output to be byte-identical.

## Selected Picard WGS surface

ADR-0009 selects the first WGS compatibility candidate around the default MultiQC 1.35 presentation:

- genome territory;
- mean, standard-deviation, and median coverage;
- default 30X threshold fraction;
- MAPQ, duplicate, unpaired, base-quality, overlap, and cap exclusion fractions;
- high-quality coverage histogram.

Milestone 13 closes the earlier overlap-design gap with the exact named policy `picard-wgs-3.4.0-default-overlap-v1`. The policy matches the pinned per-locus ordering: base-quality/no-call rejection occurs before raw-query-name de-duplication, secondary records do not participate, supplementary records remain eligible when they survive the other Picard filters, and repeated eligible query names at a locus become overlap exclusions.

The WGS surface is still **not emitted**. Exact overlap is a necessary primitive, not a substitute for proving the complete Picard WGS record filtering, depth reduction, cap semantics, metric denominators, renderer, and final differential output.

## Selected Picard hybrid-selection surface

ADR-0009 selects the first HsMetrics compatibility candidate around territory, PF accounting, bait/target placement, enrichment, usable-base fractions, target-depth statistics, fold-80, and the default target coverage thresholds used by MultiQC.

The initial profile excludes GC/AT dropout, heterozygous-SNP sensitivity, library-size estimation, `HS_PENALTY_*`, sidecar coverage files, and non-default accumulation levels.

Milestone 13 closes the overlap-design gap with `picard-hs-3.4.0-default-overlap-v1`, reproducing pinned HTSJDK 4.2.0 `SAMUtils.getNumOverlappingAlignedBasesToClip` behavior. That helper is record-local; AlignGauge therefore does not invent a mate cache or template-reconstruction algorithm for Hs overlap.

The HsMetrics surface is still **not emitted**. The exact targeted Picard filtering, denominator, bait/target accounting, fold-80, complete metric reduction, renderer, and differential output remain release-gate work.

## Fold-80 reconciliation

The two names remain intentionally different metrics:

- AlignGauge native: `target_uniformity_penalty_80`
- Picard compatibility: `FOLD_80_BASE_PENALTY`

The native metric remains defined over the full normalized target territory including zero-depth bases. It is unavailable when its nearest-rank 20th-percentile denominator is zero.

Picard documents `FOLD_80_BASE_PENALTY` in terms of non-zero-coverage targets and computes it after the pinned Hs filtering/overlap behavior. Milestone 13 does not change the Milestone 12 rule: the native value must never be relabeled as Picard fold-80. The Picard field remains absent until a complete exact HsMetrics collector passes differential validation.

## MultiQC 1.35 executable validation

Milestone 12 added:

- `tools/reference/multiqc/fixtures/picard-wgs-discovery.metrics.txt`
- `tools/reference/multiqc/fixtures/picard-hs-discovery.metrics.txt`
- `tools/reference/multiqc/validate-picard.sh`
- `.github/workflows/multiqc-validation.yml`

The two static WGS/Hs files remain explicitly **discovery-only fixtures**. They exercise the exact pinned upstream file discovery and parser-required field contract but are not generated by AlignGauge and are not numerical compatibility evidence.

The permanent validator uses real generated output for Picard insert-size compatibility. It:

1. starts from an exact Picard-vs-AlignGauge insert-size differential pair;
2. runs the pinned MultiQC container with networking disabled;
3. forces filename-based sample identity with pinned MultiQC's `--fn_as_s_name` option so Picard's embedded `INPUT=` command line cannot create an artificial sample-name mismatch against an AlignGauge projection that correctly does not impersonate a Picard command invocation;
4. requires the Picard module to discover both inputs;
5. requires the parsed insert-size data files to exist;
6. byte-compares the parsed reference and AlignGauge data;
7. separately requires WGS and HsMetrics discovery fixtures to produce their expected parsed data files;
8. verifies parser-required fields are present;
9. records `compatibility_claim: false` for both discovery-only surfaces;
10. fails on any nonzero parser exit or missing expected output.

There is no `|| true`, warning-only parser path, zero-fill fallback, or success marker written before every assertion succeeds.

## Milestone 12 fail-closed parser evidence

The first parser-gate execution deliberately remained red when the parsed Picard and AlignGauge insert-size TSVs differed in sample identity:

- MultiQC Validation run `31246661302`
- job `93076297421`
- result: failure in `Run pinned MultiQC Picard parser`
- direct Picard insert-size differential step: success before the parser failure

The failure was not suppressed. Investigation showed that MultiQC extracted the Picard reference sample name from Picard's embedded command line but fell back to the filename for the AlignGauge projection. The validator was then changed to use MultiQC's explicit `--fn_as_s_name` sample-handling mode with identical copied filenames. No metric column, parser assertion, or compatibility boundary was relaxed.

## Milestone 12 validated closure

Milestone 12's merged implementation and evidence remain recorded by the following master history:

- implementation candidate `9200358708650a1b0a462f3395ab24c133b3b0b5` — path-triggered gates green;
- evidence SHA `5b7f6d15970918862d1006ea4c6add6937479ea6` — path-triggered gates green;
- TODO-closure SHA `0e81a63d06a524f58667e10d0bc3fa8c44999197` — full seven-gate PR suite green;
- PR #6 merge SHA `733e052534ec9fc2fe6a4dd2b6d7f790f8e2a5c7` — all eight triggered master workflows green.

The final Milestone 12 documentation closeout is commit `e0e8c0e0fa71815409d437db098e79dd4f58d298`, whose Permanent CI, Reference Validation, and MultiQC Validation workflows all succeeded.

## Milestone 13 exact-overlap architecture

ADR-0010 freezes two different exact profiles rather than a generic clipping switch:

- `picard-wgs-3.4.0-default-overlap-v1`
- `picard-hs-3.4.0-default-overlap-v1`

The only released exact-overlap execution mode is:

`streaming-coordinate-order-v1`

WGS exact state is bounded and fail-closed. It keeps raw query-name identity without lossy normalization or collision-prone hash-only storage, evicts state when future coordinate-sorted records can no longer overlap it, and fails with `resource_limit` rather than dropping or approximating state. The pinned `LOCUS_ACCUMULATION_CAP=100000` is also treated as a hard compatibility boundary.

Hs overlap does not have a cross-record mate cache because the pinned HTSJDK helper does not use one.

## Milestone 13 pinned executable differential

`.github/workflows/overlap-validation.yml` creates a deterministic BAM and compares AlignGauge's overlap primitives to a narrow Java oracle executed from inside the immutable pinned Picard 3.4.0 image. The Java path uses the Picard-bundled HTSJDK 4.2.0 classes and runs with networking disabled.

The candidate's Rust and reference TSVs are byte-identical with these exact counters:

| Counter | Exact value |
|---|---:|
| `wgs_retained_bases` | 135 |
| `wgs_baseq_excluded_bases` | 10 |
| `wgs_overlap_excluded_bases` | 35 |
| `hs_overlap_clipped_read_bases` | 64 |

The fixture and unit suite cover ordinary paired overlap, low-quality-first ordering, secondary/supplementary behavior, equal-start read1/read2 ties, insertion CIGAR, extended `=`/`X` CIGAR, state expiry, memory-budget failure, locus-cap failure, and unpaired/mate-unmapped Hs behavior.

Implementation candidate `3f3237ab34c43d826a2332134d3dc1462955bbf8` passed the full eight-gate pull-request matrix:

- Permanent CI run `31249073395`, job `93082408210` — success
- Full Runtime Validation run `31249073379`, job `93082432961` — success
- Reference Validation run `31249073370`, job `93082432239` — success
- Targeted Validation run `31249073352`, job `93082431519` — success
- Samtools Stats Validation run `31249073367`, job `93082408071` — success
- Picard Validation run `31249073348`, job `93082408038` — success
- MultiQC Validation run `31249073373`, job `93082407967` — success
- Exact Overlap Validation run `31249073464`, job `93082408299` — success

Detailed Milestone 13 evidence is recorded in `docs/evidence/M13_EXACT_OVERLAP.md`.

## Indexed parallelism disposition

Milestone 13 does not admit indexed reference-partition parallelism for v0.4. The TODO conditions implementation on measured value sufficient to justify the additional semantic/resource complexity, and no repository evidence establishes that admission case. Separately, the specification forbids combining indexed partition execution with exact overlap until separately designed and differentially proved.

Therefore no additional production readers, descriptors, partition buffers, decompression pools, merge rules, or indexed-exact-overlap fallback were introduced. The globally coordinate-ordered streaming path remains authoritative. Decoder/I/O concurrency is allowed only where it preserves that one logical ordered stream.

This is an explicit design disposition, not an unimplemented feature silently represented as complete.

## Remaining v0.4 work

Milestone 13 removes the overlap-semantics blocker but does not itself satisfy the separate `v0.4.0` release gate.

Before WGS/Hs compatibility can be promoted, the release path must still prove the complete selected collectors and outputs end-to-end: all remaining Picard record filters and denominators, depth/target reductions, WGS cap/exclusion semantics, Hs bait/target and fold-80 semantics, renderers, exact differential metrics, pinned MultiQC ingestion, determinism, memory behavior, and release documentation.

The selected WGS/Hs surfaces remain **not emitted** until that proof exists. This report must be updated rather than silently reinterpreted if the release-gate work demonstrates that a selected field cannot be matched exactly.
