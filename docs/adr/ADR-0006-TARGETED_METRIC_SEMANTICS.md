# ADR-0006: Targeted Metric Semantics and Compatibility Boundary

- **Status:** Accepted
- **Date:** 2026-08-07
- **Decision owners:** AlignGauge maintainers
- **Applies to:** v0.3 targeted/WES metrics

## Context

Milestone 9 adds targeted-sequencing metrics on top of the Milestone 8 BED model and the existing exact chunked coverage engine. The roadmap names target territory, on-/near-/off-target bases, per-target coverage, dropout reporting, enrichment, and a fold-80-style uniformity metric, but the original specification did not define the required arithmetic precisely enough to implement them safely.

Picard `CollectHsMetrics` is an important reference point, but direct Picard compatibility is not appropriate for v0.3. Picard distinguishes bait intervals from target intervals and, by default, applies mapping-quality, base-quality, overlapping-mate, and other filters that differ from AlignGauge's canonical `aligngauge-v0.1` coverage profile. AlignGauge v0.3 accepts one BED target set and intentionally reuses the existing canonical coverage track.

The project therefore needs one deterministic native profile whose formulas are explicit and testable, while retaining enough differential coverage to detect implementation errors.

## Decision

### Profile identity

The targeted profile is named:

```text
aligngauge-targeted-v0.3
```

It is a reduction over the exact canonical coverage profile `aligngauge-v0.1`; it is not an independent read-filtering or coverage algorithm.

The canonical track includes mapped, primary, QC-pass, non-duplicate records, counts `M`/`=`/`X`, uses minimum MAPQ 0, applies no base-quality filter, and does not correct overlapping mates. Targeted metrics inherit those semantics exactly.

Because duplicates are already excluded by the canonical coverage profile, the v0.3 targeted result is explicitly duplicate-adjusted by exclusion. No second hidden duplicate-filtered track is created.

### Target and near-target territory

The source BED is normalized twice from the same validated parse result:

1. **target set:** zero flank;
2. **selected set:** symmetric `near_distance_bases` flank.

The default `near_distance_bases` is **250**. It is configurable explicitly and recorded in provenance.

Definitions use unique union territory:

- **target territory** = union length of non-empty zero-flank target intervals;
- **selected territory** = union length after expanding each non-empty target by `near_distance_bases` and clipping expansion at known contig boundaries;
- **near-target territory** = selected territory minus target territory.

The target BED remains authoritative. The near-target expansion never repairs an invalid source interval.

### Aligned-base partition

Every accepted aligned reference base from the canonical coverage track belongs to exactly one aggregate class:

1. **on-target** if its reference position lies in target territory;
2. **near-target** if it lies in selected territory but not target territory;
3. **off-target** otherwise.

The counts are depth-weighted reference-base observations, not unique loci. Therefore:

```text
on_target_bases + near_target_bases + off_target_bases
    == total_accepted_aligned_bases
```

This invariant is checked. No base may be dropped, double-counted, or assigned by read start alone.

### Aggregate target coverage

Aggregate target coverage is evaluated over unique target territory only.

The targeted reduction records:

- target territory bases;
- target depth histogram;
- covered and uncovered target bases;
- target mean depth;
- cumulative target-base counts and percentages at the configured coverage thresholds.

The target depth sum must equal `on_target_bases`.

Percentages are rendered with the same deterministic six-decimal convention used by canonical coverage.

### Per-source-target coverage

Per-target metrics retain Milestone 8 source interval identity rather than using merged intervals.

For every non-empty source target, record:

- source index and source line number;
- contig, start, end, and optional BED name;
- length;
- exact depth sum;
- mean depth;
- covered bases;
- uncovered bases;
- configured threshold base counts and percentages;
- maximal zero-coverage half-open runs;
- longest zero-coverage run length.

If source BED intervals overlap, the shared genomic bases may contribute to each corresponding per-source-target result. This is intentional. Aggregate target metrics continue to use the unique merged union and therefore do not double-count overlaps.

A valid zero-length source interval is retained in the report, but mean depth and threshold percentages are explicitly unavailable because their denominator is zero. It does not create target territory or a synthetic dropout run.

### Dropout reporting

A zero-coverage run is a maximal half-open subinterval of one non-empty source target whose canonical depth is exactly zero.

A source target is considered a **dropout target** when at least one target base has zero coverage. The per-target report records the exact zero-base count and runs rather than reducing dropout to only a boolean.

### Enrichment

AlignGauge v0.3 defines project-native **target enrichment** as observed on-target fraction divided by the random-placement fraction implied by target territory:

```text
target_enrichment
  = (on_target_bases / total_accepted_aligned_bases)
    / (target_territory_bases / genome_territory_bases)
```

Equivalently, reduction may use exact integer products before final decimal rendering:

```text
(on_target_bases * genome_territory_bases)
/ (total_accepted_aligned_bases * target_territory_bases)
```

`genome_territory_bases` is the checked sum of declared reference lengths in the validated alignment header.

The value is explicitly unavailable when any required denominator is zero. This is **not** labeled Picard `FOLD_ENRICHMENT` because v0.3 has no separate bait set and uses different filtering semantics.

### Uniformity penalty 80

Instead of claiming Picard `FOLD_80_BASE_PENALTY`, v0.3 exposes the ADR-approved native metric:

```text
target_uniformity_penalty_80
```

Let `D20` be the nearest-rank 20th-percentile depth over **all bases in unique target territory**, including zero-depth bases. Let `M` be mean target depth.

```text
target_uniformity_penalty_80 = M / D20
```

This answers the same operational uniformity question—how much the mean exceeds the depth reached by at least 80% of target bases—without importing Picard's different filtering and non-zero-target conventions.

The metric is explicitly unavailable when target territory is empty or `D20 == 0`; AlignGauge does not serialize infinity, silently exclude zero-depth bases, or substitute zero.

The exact percentile selection rule is deterministic nearest-rank over the integer target-depth histogram. For `N > 0` target bases, the ascending rank is:

```text
ceil(0.20 * N)
```

with ranks one-based.

### Compatibility boundary

v0.3 does **not** claim `CollectHsMetrics` compatibility.

Differential validation may compare independently comparable primitives, especially per-region means and threshold counts, against a pinned external tool such as mosdepth. Differences caused by filtering, bait/target separation, mate-overlap treatment, or metric naming must not be hidden behind a compatibility label.

## Failure behavior

Targeted analysis fails closed for:

- malformed or mismatched BED input under ADR-0005;
- checked arithmetic overflow;
- impossible target/reference mapping;
- aggregate partition mismatch;
- target histogram territory mismatch;
- target depth-sum mismatch;
- resource limits that cannot safely represent the requested targeted state.

Unavailable denominator-based metrics remain explicit `Availability::Unavailable` values; they are not fatal when all underlying integer measurements are otherwise correct.

## Consequences

- Milestone 9 can reuse the exact chunked depth sweep rather than create a second coverage algorithm.
- Aggregate target metrics remain independent of BED source ordering and overlap duplication.
- Per-source-target metrics preserve vendor interval identity and can intentionally overlap.
- The canonical duplicate policy remains single-sourced.
- `target_uniformity_penalty_80` is honest about semantics and avoids a false Picard compatibility claim.
- A future release may add separate bait intervals and a named Picard compatibility profile without changing the v0.3 native metric meanings.

## Validation obligations

Milestone 9 must test at least:

- exact on/near/off partitioning at interval boundaries;
- blocks crossing target and near-target boundaries;
- overlapping and adjacent source targets;
- targets at contig boundaries;
- zero-length targets;
- target depth histogram and threshold reductions;
- overlapping-source per-target double participation without aggregate double counting;
- zero-coverage run coalescing and longest-run calculation;
- enrichment denominator edge cases;
- uniformity percentile/rank edge cases and `D20 == 0` unavailability;
- deterministic results across chunk sizes and BED source order;
- BAM/CRAM targeted equivalence;
- pinned differential comparisons for externally comparable target coverage primitives;
- exact target identity and near-distance provenance;
- one input traversal for counters, whole-genome coverage, and targeted reductions.
