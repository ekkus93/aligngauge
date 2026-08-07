# ADR-0008 — Picard alignment-summary and insert-size compatibility profile

**Status:** Accepted for Milestone 11

**Date:** 2026-08-07

## Context

Milestone 11 begins the Picard portion of the v0.4 compatibility boundary. The project rules require a named, version-pinned profile before implementation. Picard output is not one monolithic contract: `CollectAlignmentSummaryMetrics` contains both reference-independent and reference-dependent fields, while `CollectInsertSizeMetrics` has a separate orientation/histogram algorithm with median/MAD trimming and configurable category suppression.

A broad claim such as “Picard compatible” would therefore be inaccurate unless every field, filter, accumulation level, trimming rule, and renderer is matched. Milestone 11 intentionally adopts a smaller exact profile.

## Decision

### Reference implementation

Pin **Picard 3.4.0**. Differential execution must use a repository-pinned executable/container identity and must run without network access after the reference tool is available locally.

The compatibility profile identifiers are:

- `picard-alignment-summary-3.4.0-all-reads-subset-v1`
- `picard-insert-size-3.4.0-all-reads-v1`

`ALL_READS` is the only accumulation level claimed in Milestone 11. SAMPLE, LIBRARY, and READ_GROUP accumulation are deferred to the later v0.4 breakdown milestone.

### Alignment-summary subset

Milestone 11 claims exact compatibility only for the Picard 3.4.0 fields that are computed without reference-dependent alignment comparison:

- `CATEGORY`
- `TOTAL_READS`
- `PF_READS`
- `PCT_PF_READS`
- `PF_NOISE_READS`
- `PCT_ADAPTER`
- `MEAN_READ_LENGTH`
- `SD_READ_LENGTH`
- `MEDIAN_READ_LENGTH`
- `MAD_READ_LENGTH`
- `MIN_READ_LENGTH`
- `MAX_READ_LENGTH`
- `BAD_CYCLES`

The category rows follow Picard 3.4.0 exactly:

- paired inputs emit `FIRST_OF_PAIR`, `SECOND_OF_PAIR`, and `PAIR` when first-of-pair reads exist;
- unpaired data emit `UNPAIRED`;
- an empty input emits the Picard empty `UNPAIRED` row semantics;
- secondary and supplementary records are excluded from the alignment-summary read-count categories according to the pinned collector behavior.

The subset reproduces Picard's default adapter-sequence list and noise-read/no-call cycle semantics. Sequence materialization is permitted only for an explicitly planned Picard alignment-summary analysis; existing v0.1-v0.3 and Samtools-stats plans do not silently gain sequence decoding cost.

Reference-dependent fields are **not** claimed in Milestone 11. This includes alignment/mismatch/indel/error, strand-balance, chimera, pair-alignment, clipping percentages, and other fields whose Picard semantics depend on a reference sequence or reference-aware collector state. AlignGauge must not serialize those values as zero under the compatibility profile. They are omitted from the subset model/renderer and documented as unsupported.

### Insert-size profile

Milestone 11 targets exact Picard 3.4.0 `CollectInsertSizeMetrics` default ALL_READS semantics, except that generation of the PDF chart is not part of the compatibility claim.

Defaults frozen by this ADR:

- `DEVIATIONS = 10.0`;
- `HISTOGRAM_WIDTH = null`;
- `MIN_HISTOGRAM_WIDTH = null`;
- `MINIMUM_PCT = 0.05`;
- `INCLUDE_DUPLICATES = false`;
- accumulation level `ALL_READS` only.

Record eligibility follows the pinned collector:

- paired record required;
- current read mapped;
- mate mapped;
- first-of-pair records excluded, so the second-of-pair record supplies one observation per pair;
- secondary and supplementary records excluded;
- duplicate records excluded by default;
- `TLEN != 0` required;
- insert size is `abs(TLEN)` using checked arithmetic;
- orientation is Picard/HTSJDK FR, RF, or TANDEM semantics.

Each orientation has an independent histogram. An orientation category is emitted only when its count is at least `total_inserts * 0.05`, matching Picard's comparison rule.

For each emitted orientation, Milestone 11 reproduces:

- `READ_PAIRS`;
- `PAIR_ORIENTATION`;
- `MEDIAN_INSERT_SIZE`;
- `MODE_INSERT_SIZE`;
- `MEDIAN_ABSOLUTE_DEVIATION`;
- `MIN_INSERT_SIZE`;
- `MAX_INSERT_SIZE`;
- `MEAN_INSERT_SIZE`;
- `STANDARD_DEVIATION`;
- `WIDTH_OF_10_PERCENT` through `WIDTH_OF_99_PERCENT`;
- the trimmed histogram table consumed by downstream parsers.

### MAD trimming and rounding

This profile claims Picard behavior, not a similarly named native statistic.

The automatic trim width is the Java/Picard 3.4.0 expression:

`(int) (MEDIAN_INSERT_SIZE + DEVIATIONS * MEDIAN_ABSOLUTE_DEVIATION)`

with `DEVIATIONS = 10.0`. The conversion therefore truncates toward zero after the floating-point sum. Only histogram bins with insert size at or below that width remain for `MEAN_INSERT_SIZE`, `STANDARD_DEVIATION`, and rendered histogram output.

Median, mode, median absolute deviation, standard deviation, and centered width thresholds must match the pinned HTSJDK `Histogram` semantics exactly. Tie-breaking and decimal rendering are acceptance-test material, not implementation discretion.

### Compatibility versus similar metrics

Only fields covered by exact differential tests may use the Picard compatibility profile identifiers. Any future project-native variant must use a distinct metric/profile name and must not reuse a Picard field name merely because its formula is similar.

### Rendering and MultiQC

Compatibility text is derived from typed completed reports rather than accumulated independently. The renderer must contain the Picard class identifiers required for downstream discovery.

MultiQC parser validation may be used as an ecosystem acceptance test, but MultiQC agreement does not substitute for direct Picard differential testing because MultiQC consumes only a subset of Picard fields and may normalize values.

### Failure behavior

- Missing required planned fields are typed fatal errors; they are not converted to zero.
- Integer overflow is fatal; no saturating arithmetic is permitted.
- An insert-size `abs(TLEN)` overflow is fatal rather than wrapped.
- If no orientation survives `MINIMUM_PCT`, the compatibility result is explicitly empty/unavailable according to the Picard profile; AlignGauge must not invent an FR row.
- Unsupported reference-dependent alignment-summary fields remain absent, not zero-filled.

## Consequences

Milestone 11 can make two precise compatibility claims without pretending to reproduce all Picard metrics. Full reference-aware alignment-summary metrics, multilevel accumulation, WGS/hybrid-selection metrics, and exact mate-overlap correction remain later v0.4 work.
