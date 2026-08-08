# ADR-0011: v0.4 release scope and final gate

- **Status:** Accepted for v0.4 release preparation
- **Date:** 2026-08-08
- **Depends on:** ADR-0007, ADR-0008, ADR-0009, ADR-0010
- **Reference tools:** Samtools 1.24, Picard 3.4.0, HTSJDK 4.2.0, MultiQC 1.35

## Context

Milestones 10 through 13 created several different kinds of compatibility work:

1. exact released compatibility projections with executable differential evidence;
2. exact overlap primitives needed by future Picard whole-genome and hybrid-selection work;
3. discovery-only MultiQC fixtures; and
4. selected future Picard WGS/HsMetrics candidate surfaces.

The v0.4 release gate must not collapse those categories into one broad compatibility claim. A selected candidate surface is not a released profile merely because its prerequisite overlap primitive now exists.

The normative v0.4 surfaces already defined by the specification are the Samtools Stats / MultiQC profile from ADR-0007 and the two Picard profiles from ADR-0008. ADR-0009 deliberately described WGS and HsMetrics as selected candidate surfaces and explicitly prohibited approximate or zero-filled promotion.

## Decision

### 1. Released v0.4 compatibility profiles

v0.4 releases exactly these compatibility projections:

- `samtools-stats-1.24-multiqc-1.35`;
- `picard-alignment-summary-3.4.0-all-reads-subset-v1`;
- `picard-insert-size-3.4.0-all-reads-v1`.

Existing `samtools-flagstat` and `samtools-idxstats` compatibility projections remain available from earlier releases and are not semantically widened by v0.4.

The Samtools Stats profile covers the complete ordinary non-target 39-row `SN` surface and the default `IS` section frozen by ADR-0007. The Picard alignment-summary profile covers exactly the 13 reference-independent fields frozen by ADR-0008. The Picard insert-size profile covers the complete default `ALL_READS` row plus the trimmed histogram surface frozen by ADR-0008.

No other Samtools or Picard field is implied by the v0.4 version number.

### 2. MultiQC compatibility boundary

The v0.4 MultiQC compatibility claim applies only to generated outputs that have been proved consumable by pinned MultiQC 1.35:

- `samtools-stats-1.24-multiqc-1.35`; and
- `picard-insert-size-3.4.0-all-reads-v1`.

The Picard alignment-summary subset remains exact against Picard 3.4.0 but is not a MultiQC 1.35 profile because the pinned parser requires reference-dependent columns outside the released 13-field subset. Those missing columns remain absent; they are not synthesized as zero.

The WGS and HsMetrics files under `tools/reference/multiqc/fixtures/` remain discovery-only fixtures with `compatibility_claim: false`. They are not AlignGauge-generated compatibility outputs.

### 3. WGS and HsMetrics disposition

The candidate surfaces selected by ADR-0009 are **not promoted in v0.4**.

v0.4 does not expose `--format picard-wgs` or `--format picard-hs-metrics`, and documentation shall not claim that native `aligngauge-targeted-v0.3` metrics are Picard HsMetrics values.

Milestone 13 closed the exact-overlap primitive gap, but complete WGS/Hs compatibility still requires independent proof of the remaining record filters, denominators, WGS exclusion/capping behavior, bait/target reductions, Picard `FOLD_80_BASE_PENALTY`, renderers, full differential metrics, and generated-output MultiQC equivalence.

Deferral is preferable to a plausible-looking incomplete profile. No candidate field may be copied from a native metric, zero-filled, or emitted under a Picard name without that proof.

### 4. Released execution modes

The authoritative collector/reduction path remains deterministic and serial.

`--threads > 1` is accepted only as a configured limit and continues to report the explicit `collector_threads_serial_v0_1` warning; it does not create a released parallel collector mode.

The released concurrency mechanism is HTSlib reader/decompression concurrency through `--io-threads`. `--io-threads 0` normalizes to the serial reader setting and `--io-threads N` for `N > 1` may use multiple HTSlib I/O workers while preserving one logical coordinate-ordered record stream.

The v0.4 release gate therefore requires byte-identical canonical `summary.json` output between serial decoding and a multi-worker `--io-threads 2` run for both ordinary and targeted release paths. Provenance is expected to differ in the configured/effective I/O-thread fields and timing values; it must truthfully record those differences.

Indexed reference-partition execution remains unsupported as frozen by ADR-0010.

### 5. Final v0.4 release gate

A commit is eligible to become the `v0.4.0` release commit only after all of the following are true on that exact source state:

1. `docs/evidence/V0_4_COMPATIBILITY_REPORT.md` reconciles every field and explicitly names unsupported/deferred surfaces;
2. direct pinned Samtools 1.24 and Picard 3.4.0 differential gates are green;
3. pinned MultiQC 1.35 parses generated Samtools Stats and Picard InsertSize outputs and the parsed data agrees with the corresponding reference output;
4. serial and `--io-threads 2` release runs produce byte-identical canonical summaries for ordinary and targeted paths;
5. WGS/Hs compatibility formats remain unavailable unless a later change supplies the complete proof required by ADR-0009;
6. Permanent CI and every permanent compatibility/runtime workflow triggered by the exact release commit succeeds.

The tag must point to that validated commit. A tag is not created first and used as evidence retroactively.

## Consequences

- v0.4 is an ecosystem-compatibility release without overstating WGS/Hs support.
- Existing exact Samtools/Picard work becomes an explicit released product surface.
- MultiQC compatibility remains tied to generated, parser-proved output rather than text resemblance.
- The project keeps one ordered execution model while allowing bounded HTSlib I/O concurrency.
- Future WGS/Hs work can proceed under ADR-0009 and ADR-0010 without a backward-compatibility obligation to an incomplete v0.4 format.
