# ADR-0010: Exact Picard overlap correction and authoritative streaming execution

- **Status:** Accepted for Milestone 13
- **Date:** 2026-08-08
- **Picard reference:** 3.4.0
- **HTSJDK reference:** 4.2.0 as pinned by Picard 3.4.0
- **Depends on:** ADR-0003, ADR-0006, ADR-0009
- **Supersedes:** no earlier metric semantics; this ADR closes the overlap design gap identified before v0.4

## Context

Milestone 12 selected Picard WGS and HsMetrics compatibility surfaces but deliberately did not emit them. Exact overlap handling is a prerequisite because overlap removal changes the WGS high-quality depth histogram, coverage statistics and exclusion denominator, and because `CollectHsMetrics` enables overlapping-read clipping by default.

The phrase "mate-overlap correction" is insufficiently precise. Picard 3.4.0 does not use one overlap algorithm for WGS and HsMetrics:

- default `CollectWgsMetrics` uses an HTSJDK `SamLocusIterator`. After record-level filters and base-quality/no-call filtering, it keeps a per-locus set of read names and excludes a high-quality observation when that raw query name has already contributed at that locus;
- default `CollectHsMetrics` sets `CLIP_OVERLAPPING_READS=true`. `TargetMetricsCollector` calls HTSJDK 4.2.0 `SAMUtils.getNumOverlappingAlignedBasesToClip` and clips the selected record using only that record's mate metadata and CIGAR.

Treating these as one generic clipping policy would be numerically wrong. Reconstructing mate pairs by query name for HsMetrics would also be wrong because the pinned HTSJDK method does not do that.

Indexed reference-partition execution creates a second correctness problem. Independent readers over disjoint partitions can split records that participate in the same overlap semantics, and they multiply reader buffers, file descriptors, decompression state, and merge state. The specification already requires a separate design and differential proof before exact overlap may be combined with indexed partition parallelism.

## Decision

### 1. Two named overlap profiles

Milestone 13 defines two separate exact policies:

1. `picard-wgs-3.4.0-default-overlap-v1`
2. `picard-hs-3.4.0-default-overlap-v1`

Neither name may be replaced by a generic `clip_overlaps=true` compatibility claim. A future profile that changes Picard options, HTSJDK behavior, or accumulation semantics requires a new profile name and differential evidence.

### 2. WGS record participation and ordering

The WGS overlap stage matches the pinned default `CollectWgsMetrics` ordering.

Before overlap de-duplication, the Picard record path:

1. excludes secondary alignments;
2. excludes adapter reads;
3. excludes reads below the default MAPQ threshold of 20;
4. excludes duplicates;
5. because `COUNT_UNPAIRED=false`, excludes unpaired reads and paired reads with an unmapped mate;
6. excludes non-PF reads through the locus iterator.

Supplementary records are not removed by HTSJDK 4.2.0 `SecondaryAlignmentFilter`; therefore they remain eligible if they pass the other filters. AlignGauge must not silently broaden the secondary filter to "secondary or supplementary" for this profile.

At each locus, Picard then applies base eligibility before overlap identity:

- base quality below 20 is excluded;
- a no-call base is excluded;
- only after those checks is the query name inserted into the per-locus set;
- if that query name is already present, the observation is an overlap exclusion.

This order is semantically important. A low-quality first observation must not suppress a later high-quality observation with the same query name at the same locus.

### 3. WGS pairing/identity key

The WGS overlap identity is the **raw query-name byte sequence** exactly as stored in the BAM record. It is not:

- a UTF-8-normalized string;
- read name plus `/1` or `/2` inference;
- read-group plus name;
- template length;
- mate coordinate;
- first/second-of-pair flags.

This matches Picard's per-locus `getReadName()` identity. AlignGauge must not use a probabilistic hash as the sole identity because a hash collision would create silent false overlap exclusions.

### 4. WGS bounded streaming state

Exact WGS correction uses one coordinate-ordered streaming state machine.

For each raw query name, AlignGauge stores only high-quality reference positions contributed by earlier records that can still overlap a later coordinate-sorted record. Before processing a record beginning at position `S`, a name state whose final eligible position is `< S` (half-open end `<= S`) is evicted. All overlap state is discarded at a forward reference transition.

This is sufficient because a future coordinate-sorted record cannot overlap an earlier eligible position once its alignment start is at or beyond that position's half-open end. No arbitrary mate-distance window is required.

The state is charged against an explicit caller-reserved memory budget using deterministic conservative charges. If adding exact state would exceed that reservation, execution fails with `resource_limit`. State is never dropped, truncated, summarized with a collision-prone identity, spilled through an undocumented path, or converted to approximate overlap handling.

The pinned default Picard `LOCUS_ACCUMULATION_CAP=100000` is also a compatibility boundary. AlignGauge detects an attempt to exceed the supported per-locus candidate bound and fails closed rather than silently discarding excess candidates. Therefore the exact compatibility claim covers inputs that remain within the pinned locus bound; an input beyond that bound is rejected rather than assigned plausible but truncated metrics.

### 5. WGS primary/supplementary semantics

Secondary alignments do not participate.

Supplementary alignments may participate when they survive the remaining Picard filters. Their observations use the same raw query-name identity as any other participating record. This intentionally follows the pinned Picard/HTSJDK behavior rather than inventing a cleaner template model.

A record carrying both secondary and supplementary flags is excluded because the secondary predicate wins.

### 6. HsMetrics exact clipping semantics

The HsMetrics profile reproduces HTSJDK 4.2.0 `SAMUtils.getNumOverlappingAlignedBasesToClip` rather than pairing records in AlignGauge state.

For a record:

- unpaired, unmapped, or mate-unmapped records clip zero bases;
- if the mate alignment start is before the record alignment start, the record is the right-most end and clips zero bases;
- if the starts are equal and the record is first-of-pair, it clips zero bases;
- otherwise the selected record is eligible to clip trailing aligned/read-consuming bases beginning at the mate alignment start according to the pinned HTSJDK CIGAR walk.

The equal-start rule therefore retains first-of-pair and makes the other end eligible for clipping.

HTSJDK's method uses record-local mate position and does **not** look up a mate by query name. It also does not require the mate coordinate to name the same reference before performing this calculation. AlignGauge preserves those pinned semantics. A later attempt to "fix" them requires a different compatibility profile.

The pinned CIGAR behavior is also preserved, including HTSJDK 4.2.0's treatment of `M`, insertion/deletion, skipped/padding/clipping operators, and extended `=`/`X` operations. Differential tests must cover these edge cases; a generalized interval-intersection implementation is not an acceptable substitute.

### 7. HsMetrics primary/supplementary semantics and state bound

`TargetMetricsCollector` ignores secondary alignments. Supplementary alignments are not treated as `mappedInPair` for pair-derived target metrics, but the pinned record-local overlap helper itself is still defined solely by the SAM flags/mate position/CIGAR checks above.

Hs overlap correction requires no cross-record pair cache. Its overlap state is O(1) per record plus CIGAR traversal already bounded by the validated record contract. Therefore there is no mate-cache overflow fallback to define: malformed/unrepresentable CIGAR arithmetic is fatal, and no approximate path exists.

### 8. Exact overlap forces authoritative streaming mode

The released exact-overlap execution mode is named:

`streaming-coordinate-order-v1`

The record stream remains globally coordinate ordered. Background decode/I/O threads are allowed only when they preserve that single logical record order and do not partition semantic ownership of records or loci.

Exact overlap requests must reject any future execution mode that cannot prove those semantics. The planner may not silently disable overlap correction to honor a thread request.

### 9. Indexed reference-partition parallelism is not admitted for v0.4

Milestone 13 does **not** implement indexed reference-partition parallelism.

The admission gate is closed for this release because:

- there is no measured repository evidence showing end-to-end value sufficient to justify the added semantic and resource complexity;
- exact overlap is the new correctness-critical path and the specification explicitly prohibits combining it with indexed partitioning without separate design and differential proof;
- the current streaming reader remains authoritative and already permits bounded HTSlib decode concurrency without changing record ownership.

Because no indexed implementation is admitted, there are no additional production readers, descriptors, partition buffers, decompression pools, partition merge rules, or storage-specific indexed benchmarks to include in the v0.4 planner. Those Milestone 13.2 implementation bullets are closed as **not applicable by admission decision**, not silently skipped.

A future indexed design must be introduced by a new ADR and must, before release:

1. model all readers, file descriptors, buffers and decompression pools under `--memory-limit`;
2. partition references deterministically;
3. prove serial equivalence at contig boundaries and merge boundaries;
4. either prove exact overlap across the partition model or reject exact-overlap profiles explicitly;
5. benchmark end-to-end behavior on at least local NVMe and a materially slower storage profile;
6. retain streaming mode as the authoritative fallback only when the user explicitly selected a profile for which such fallback is allowed. Exact-overlap profiles may never silently fall back.

### 10. No compatibility promotion from overlap code alone

Milestone 13 overlap primitives do not, by themselves, authorize `--format picard-wgs` or `--format picard-hs-metrics`.

ADR-0009's selected fields become compatibility claims only after their complete Picard record filtering, coverage/target reductions, renderers, exact differential fixtures, and pinned MultiQC parser checks pass. Missing fields remain absent. Native `target_uniformity_penalty_80` remains distinct from Picard `FOLD_80_BASE_PENALTY`.

## CI contract

Milestone 13 validation must prove at minimum:

- WGS first-eligible-observation behavior for repeated raw query names;
- base-quality filtering occurs before WGS overlap identity;
- supplementary-vs-secondary participation matches the pinned filter;
- state expires deterministically when overlap becomes impossible;
- state-budget exhaustion is fatal;
- locus-bound exhaustion is fatal rather than truncated;
- Hs left-most and equal-start tie breaks match HTSJDK 4.2.0;
- Hs insertion/deletion and extended-CIGAR cases match the pinned helper;
- unpaired and mate-unmapped Hs records clip zero bases;
- no source or workflow introduces an indexed-exact-overlap path or warning-only fallback.

The permanent Milestone 13 gate must fail on any mismatch. No assertion may be wrapped in `|| true`, converted to a warning, or replaced with a success marker created before validation completes.

## Consequences

- Exact overlap semantics are now explicit and profile-specific.
- WGS correction has a deterministic bounded-state strategy with fatal overflow behavior.
- HsMetrics avoids unnecessary mate caching and exactly follows the pinned record-local HTSJDK rule.
- Streaming execution is authoritative for exact overlap in v0.4.
- Indexed partition parallelism remains absent rather than being shipped without a correctness and performance case.
- ADR-0009's WGS/Hs metric surfaces remain selected-but-not-emitted until their complete differential compatibility implementations are finished.
