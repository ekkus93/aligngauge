# AlignGauge v0.4 compatibility report

**Report state:** Milestone 12 compatibility boundary and parser-validation evidence. This document is not a `v0.4.0` release declaration; Milestone 13 and the v0.4 release gate remain outstanding.

**Reference profiles**

- Samtools: 1.24
- Picard: 3.4.0
- MultiQC: 1.35, immutable image from `tools/reference/multiqc/image.lock`

## Compatibility matrix

| Surface | AlignGauge status | Numerical evidence | MultiQC 1.35 status |
|---|---|---|---|
| Samtools stats selected SN/IS sections | exact supported subset | Milestone 10 exact differential evidence | parsed and exact in Milestone 10 |
| Picard AlignmentSummaryMetrics reference-independent 13-column subset | exact supported differential projection | Milestone 11 exact synthetic + HG002 evidence | **not claimed compatible**; MultiQC directly requires fields outside the M11 subset |
| Picard InsertSizeMetrics default `ALL_READS` metrics + trimmed histogram | exact supported differential projection | Milestone 11 exact synthetic + HG002 evidence | parsed Picard-vs-AlignGauge data byte-identical in Milestone 12 |
| Picard WgsMetrics selected default-MultiQC surface | selected candidate, not emitted | blocked on exact overlap correction | discovery/parser fixture only; no compatibility claim |
| Picard HsMetrics selected capture/coverage surface | selected candidate, not emitted | blocked on exact overlap correction and Picard Hs semantics | discovery/parser fixture only; no compatibility claim |
| Native `aligngauge-targeted-v0.3` | supported native profile | v0.3 validation evidence | no Picard HsMetrics claim |

## Milestone 10 carry-forward

Milestone 10 established the exact Samtools 1.24 compatibility subset consumed by the pinned MultiQC 1.35 Samtools parser. Its network-isolated parser validation compares MultiQC's parsed data from Samtools reference text and AlignGauge text rather than treating parser exit alone as sufficient evidence.

That evidence remains authoritative and is not duplicated under a new profile name in Milestone 12.

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

This profile is **not emitted in Milestone 12**. Picard 3.4.0 removes overlapping mate observations before final high-quality depth is recorded, and MultiQC directly consumes `PCT_EXC_OVERLAP`. A WGS renderer before exact overlap correction would either disagree numerically or hide the disagreement. Both are prohibited.

## Selected Picard hybrid-selection surface

ADR-0009 selects the first HsMetrics compatibility candidate around territory, PF accounting, bait/target placement, enrichment, usable-base fractions, target-depth statistics, fold-80, and the default target coverage thresholds used by MultiQC.

The initial profile excludes GC/AT dropout, heterozygous-SNP sensitivity, library-size estimation, `HS_PENALTY_*`, sidecar coverage files, and non-default accumulation levels.

This profile is also **not emitted in Milestone 12**. Picard `CollectHsMetrics` defaults to overlapping-read clipping, and the exact targeted denominator/filtering semantics are not the native v0.3 AlignGauge semantics.

## Fold-80 reconciliation

The two names are intentionally different metrics:

- AlignGauge native: `target_uniformity_penalty_80`
- Picard compatibility: `FOLD_80_BASE_PENALTY`

The native metric remains defined over the full normalized target territory including zero-depth bases. It is unavailable when its nearest-rank 20th-percentile denominator is zero.

Picard documents `FOLD_80_BASE_PENALTY` in terms of non-zero-coverage targets and computes it after the pinned Hs filtering/overlap behavior. Milestone 12 therefore forbids relabeling the native value as Picard fold-80. The Picard field must remain absent until an exact HsMetrics collector passes differential validation.

## MultiQC 1.35 executable validation

Milestone 12 adds:

- `tools/reference/multiqc/fixtures/picard-wgs-discovery.metrics.txt`
- `tools/reference/multiqc/fixtures/picard-hs-discovery.metrics.txt`
- `tools/reference/multiqc/validate-picard.sh`
- `.github/workflows/multiqc-validation.yml`

The two static WGS/Hs files are explicitly **discovery-only fixtures**. They exercise the exact pinned upstream file discovery and parser-required field contract but are not generated by AlignGauge and are not numerical compatibility evidence.

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

## Fail-closed evidence

The first parser-gate execution deliberately remained red when the parsed Picard and AlignGauge insert-size TSVs differed in sample identity:

- MultiQC Validation run `31246661302`
- job `93076297421`
- result: failure in `Run pinned MultiQC Picard parser`
- direct Picard insert-size differential step: success before the parser failure

The failure was not suppressed. Investigation showed that MultiQC extracted the Picard reference sample name from Picard's embedded command line but fell back to the filename for the AlignGauge projection. The validator was then changed to use MultiQC's explicit `--fn_as_s_name` sample-handling mode with identical copied filenames. No metric column, parser assertion, or compatibility boundary was relaxed.

## Successful implementation candidate

Candidate SHA `9200358708650a1b0a462f3395ab24c133b3b0b5` passed every path-triggered branch gate:

- MultiQC Validation run `31246815851`, job `93076687337` — success
- Permanent CI run `31246815856`, job `93076687424` — success
- Reference Validation run `31246815859`, job `93076687386` — success
- Samtools Stats Validation run `31246815862`, job `93076687403` — success

The MultiQC job confirms:

- direct Picard 3.4.0 insert-size differential: exact, no tolerance;
- pinned MultiQC 1.35 version/digest check: success;
- Picard reference and AlignGauge insert-size parsed data: byte-identical under filename sample identity;
- WgsMetrics discovery fixture: discovered and parsed with `compatibility_claim: false`;
- HsMetrics discovery fixture: discovered and parsed with `compatibility_claim: false`;
- evidence artifact upload: success.

## Validated evidence SHA

Evidence SHA `5b7f6d15970918862d1006ea4c6add6937479ea6` also passed every path-triggered branch gate after the evidence was committed:

- MultiQC Validation run `31246937565`, job `93076990568` — success
- Permanent CI run `31246937609`, job `93076990623` — success
- Reference Validation run `31246937557`, job `93076990412` — success
- Samtools Stats Validation run `31246937566`, job `93076990392` — success

That separates implementation proof from evidence-commit proof. The remaining repository-closure step is to mark the TODO, validate that closure commit, merge PR #6, and validate the merged `master` SHA. None of those steps changes the selected WGS/Hs compatibility semantics.

## Remaining v0.4 work

Milestone 13 remains required before a `v0.4.0` release can satisfy the release gate. In particular, exact overlap correction and its interaction with execution/parallelism must be resolved before the selected WGS/Hs compatibility profiles can become emitted exact outputs.

This report must be updated rather than silently reinterpreted if Milestone 13 changes the selected Picard surfaces or demonstrates that a selected field cannot be matched exactly.
