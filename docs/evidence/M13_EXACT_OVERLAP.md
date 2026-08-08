# Milestone 13 exact overlap evidence

**Evidence state:** validated Milestone 13 evidence. This document does not declare a `v0.4.0` release and does not promote the selected Picard WGS or HsMetrics surfaces to emitted compatibility profiles.

## Validated implementation candidate

Candidate SHA:

`3f3237ab34c43d826a2332134d3dc1462955bbf8`

Every pull-request workflow triggered by that exact SHA completed successfully:

- Permanent CI run `31249073395`, job `93082408210` — success
- Full Runtime Validation run `31249073379`, job `93082432961` — success
- Reference Validation run `31249073370`, job `93082432239` — success
- Targeted Validation run `31249073352`, job `93082431519` — success
- Samtools Stats Validation run `31249073367`, job `93082408071` — success
- Picard Validation run `31249073348`, job `93082408038` — success
- MultiQC Validation run `31249073373`, job `93082407967` — success
- Exact Overlap Validation run `31249073464`, job `93082408299` — success

This separates overlap-specific proof from the existing permanent, runtime, reference, targeted, Samtools, Picard, and MultiQC compatibility gates. No existing gate was replaced or weakened.

## Validated evidence SHA

Evidence/report SHA:

`8edc7563afe271357e3d0215c4d3a44c36646f68`

After `M13_EXACT_OVERLAP.md` and the updated v0.4 compatibility report were committed, that exact SHA also passed every triggered pull-request workflow:

- Permanent CI run `31249215972`, job `93082789859` — success
- Full Runtime Validation run `31249215964` — success
- Reference Validation run `31249216006` — success
- Targeted Validation run `31249215965` — success
- Samtools Stats Validation run `31249215993` — success
- Picard Validation run `31249215960` — success
- MultiQC Validation run `31249215985` — success
- Exact Overlap Validation run `31249215961`, job `93082777275` — success

The exact-overlap oracle therefore remained green after the evidence and compatibility-boundary text existed in the repository. The next repository-closure step is to update the milestone TODO and validate that closure SHA before merge.

## Frozen exact-overlap profiles

ADR-0010 freezes two different algorithms because pinned Picard 3.4.0 does not use one generic mate-overlap operation:

- WGS: `picard-wgs-3.4.0-default-overlap-v1`
- hybrid selection: `picard-hs-3.4.0-default-overlap-v1`
- authoritative exact-overlap execution mode: `streaming-coordinate-order-v1`

`INDEXED_PARTITION_EXACT_OVERLAP_SUPPORTED` is `false` for v0.4.

## WGS semantics proved

The WGS primitive implements the pinned default `CollectWgsMetrics` overlap ordering after its record-level filters:

1. traverse eligible aligned observations in global coordinate order;
2. reject base quality below 20 and no-call bases first;
3. use the raw BAM query-name byte sequence as the exact per-locus identity;
4. retain the first eligible observation for that name/locus;
5. count subsequent eligible observations for the same name/locus as overlap exclusions.

Secondary alignments are not candidates. Supplementary alignments remain candidates when they survive the other Picard filters, matching the pinned secondary filter rather than silently treating supplementary as secondary.

The streaming state keeps only prior eligible positions that can still overlap a later coordinate-sorted record. State is evicted when overlap becomes impossible and cleared on a forward reference transition.

## HsMetrics semantics proved

The Hs primitive reproduces pinned HTSJDK 4.2.0 `SAMUtils.getNumOverlappingAlignedBasesToClip` semantics rather than reconstructing mate pairs in an AlignGauge cache.

The deterministic differential fixture covers:

- ordinary overlapping paired reads;
- equal-start first/second-of-pair tie handling;
- insertion CIGAR behavior;
- extended `=` / `X` CIGAR behavior;
- unpaired and mate-unmapped zero-clipping behavior in unit tests;
- secondary versus supplementary participation boundaries.

## Pinned executable differential oracle

The permanent `.github/workflows/overlap-validation.yml` gate generates one deterministic coordinate-sorted BAM and evaluates it through two independent paths:

1. the Rust Milestone 13 primitives;
2. `tools/reference/picard/M13OverlapOracle.java` executed inside the immutable pinned Picard 3.4.0 image, using the Picard-bundled HTSJDK 4.2.0 classes.

The reference execution runs with Docker networking disabled. `tools/reference/picard/run-m13-overlap-oracle.sh` validates the pinned image/version/jar, treats a nonzero oracle exit as fatal, validates the exact four-key output contract, and creates its success marker only after every assertion succeeds.

The successful candidate produced byte-identical Rust and pinned-reference TSVs with these counters:

| Counter | Exact value |
|---|---:|
| `wgs_retained_bases` | 135 |
| `wgs_baseq_excluded_bases` | 10 |
| `wgs_overlap_excluded_bases` | 35 |
| `hs_overlap_clipped_read_bases` | 64 |

There is no tolerance, approximate comparison, warning-only reference path, or fallback value.

## Fail-closed resource behavior

Exact WGS overlap state has an explicit caller-reserved hard budget with deterministic conservative charges. Unit tests prove that exhausting this budget returns `resource_limit`; state is not dropped, truncated, spilled through an undocumented path, or replaced by a probabilistic identity.

The pinned default Picard `LOCUS_ACCUMULATION_CAP=100000` is also enforced as a compatibility boundary. A would-be overflow is fatal rather than silently truncating observations.

The Hs overlap helper uses O(1) cross-record state because the pinned HTSJDK algorithm is record-local. There is therefore no hidden mate-cache fallback.

## Indexed parallelism disposition

Milestone 13 does not admit indexed reference-partition parallelism for v0.4.

The TODO explicitly conditions implementation on measured value sufficient to justify its additional correctness and resource complexity. No repository evidence establishes that admission case, while the specification separately prohibits combining indexed partitions with exact overlap until their semantics are designed and differentially proved.

Accordingly:

- no additional production readers, descriptors, per-partition buffers, decompression pools, or merge rules were introduced;
- no indexed-exact-overlap mode exists;
- no benchmark claim for an unimplemented indexed path is fabricated;
- the existing globally ordered streaming path remains authoritative;
- bounded decoder/I/O concurrency remains allowed only where it preserves the single logical record order.

The implementation-only sub-bullets under Milestone 13.2 are therefore not applicable for v0.4 by explicit admission decision, rather than silently skipped.

## Compatibility boundary after Milestone 13

Milestone 13 closes the exact-overlap design and primitive-validation gap from ADR-0009. It does **not** by itself mean AlignGauge emits exact Picard `WgsMetrics` or `HsMetrics` files.

Those selected surfaces still require their complete Picard record filtering, coverage/target reductions, renderer integration, differential metric validation, and pinned MultiQC proof before they may be promoted to emitted compatibility profiles. In particular, native `target_uniformity_penalty_80` remains distinct from Picard `FOLD_80_BASE_PENALTY`.

The separate v0.4 release gate remains outstanding after Milestone 13 closure.
