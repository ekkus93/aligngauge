# ADR-0005: BED Unknown-Contig and Coordinate Policy

- **Status:** Accepted
- **Date:** 2026-08-07
- **Decision owners:** AlignGauge maintainers
- **Applies to:** v0.3 BED parsing and target normalization

## Context

SPEC §9 requires an explicit unknown-contig policy before BED parsing is released, and SPEC §20 requires that policy to be captured in an ADR before Milestone 8 begins.

BED target files are untrusted input. Real vendor files commonly contain three or more BED fields, comments, UCSC `track`/`browser` directives, mixed line endings, and chromosome naming conventions that may not match an alignment header. Silently dropping intervals, guessing `chr` aliases, or clipping invalid coordinates without recording the action would change target territory and downstream metrics invisibly.

The parser therefore needs one deterministic policy that is strict enough for correctness while remaining compatible with ordinary BED3–BED12 input.

## Decision

### Sequence dictionary is authoritative

Target parsing/normalization is performed against an explicit sequence dictionary containing contig names and lengths. For production analysis this dictionary is derived from the already validated alignment header/reference context.

A BED interval contig must match one dictionary contig name exactly. AlignGauge does not:

- infer `1` ↔ `chr1` or any other alias;
- normalize case;
- consult an external alias database;
- silently drop an unknown contig;
- create an invented contig.

An unknown BED contig is fatal with stable error category `target_contig`. The diagnostic includes the source line and contig name but never substitutes another sequence.

A future explicit alias-map feature may supersede this policy, but aliasing is not part of Milestone 8 or v0.3.

### Coordinate validation

BED coordinates remain zero-based and half-open. AlignGauge never infers one-based coordinates.

For every interval:

- start and end must parse as non-negative integers representable as `u64`;
- `start > end` is fatal with `target_format`;
- `start == end` is valid and represents an empty interval, because SPEC §9 forbids only start greater than end;
- `end` must not exceed the authoritative contig length; out-of-bounds intervals are fatal rather than silently clipped;
- arithmetic used by normalization and flanking is checked.

Flank expansion is a normalization operation, not input repair. A requested flank may be clipped at coordinate zero or the known contig length because the unclipped source interval has already been validated. Every such flank clipping action is counted in normalization provenance.

### BED surface accepted by Milestone 8

AlignGauge accepts standard BED3 through BED12 records. The first three fields are required; field 4, when present, is preserved as the optional interval name. Fields 5–12 are accepted and preserved as uninterpreted extra fields for future consumers; Milestone 8 does not assign semantic meaning to them.

Field separators may be ASCII horizontal whitespace as permitted by the UCSC BED custom-track format. Blank lines, `#` comments, and UCSC `track`/`browser` directive lines are skipped as required by SPEC §9. CRLF is normalized. Trailing whitespace is ignored.

Records with fewer than three or more than twelve fields are fatal `target_format` errors. Quoted-field or BED-detail extensions are not inferred implicitly.

### Deterministic normalization

Source intervals retain a stable source identity consisting of original accepted-record index and source line number, plus the optional BED name.

Normalization:

1. validates all source intervals against the authoritative dictionary;
2. applies the configured flank with explicit boundary clipping where necessary;
3. sorts deterministically by dictionary contig order, start, end, and source identity;
4. merges overlapping or directly adjacent non-empty normalized intervals for aggregate territory;
5. retains the complete mapping from each merged interval back to contributing source interval identities;
6. leaves empty source intervals represented in the source set but excludes them from positive aggregate territory;
7. records normalization counts and target-file identity in provenance-ready data.

Direct adjacency is merged because half-open intervals `[a,b)` and `[b,c)` have no uncovered base between them and their aggregate territory is exactly `[a,c)`. Source-level metrics in Milestone 9 continue to use the retained source mapping.

### Target identity

The target input identity records:

- path supplied by the caller where available;
- exact byte size;
- SHA-256 of the original BED bytes;
- accepted source-interval count.

The checksum is over original bytes before line-ending or whitespace normalization, so provenance identifies the actual file supplied rather than a reconstructed representation.

## Alternatives rejected

### Silently ignore unknown contigs

Rejected. It can reduce target territory and alter on/off-target metrics without an obvious failure.

### Automatically translate chromosome aliases

Rejected. Alias rules are assembly- and vendor-specific and can be ambiguous. An explicit future alias map is safer than heuristics.

### Clip out-of-range input intervals

Rejected. Unlike flank expansion, an out-of-range source interval indicates a mismatch or malformed target definition. Silent clipping would conceal that defect.

### Require tabs only

Rejected for Milestone 8. UCSC BED custom tracks permit whitespace-delimited fields, and accepting horizontal whitespace improves compatibility without changing coordinate semantics.

## Consequences

- Milestone 8 receives one fail-closed, deterministic contig policy.
- BED files using `chr` aliases that do not match the alignment dictionary fail until the caller supplies a matching target file or a future explicit alias-map feature exists.
- Aggregate territory never depends on input ordering.
- Normalization can safely support configurable flanks without treating malformed source coordinates as repairable.
- Milestone 9 can compute source-target and merged-territory metrics from the same normalized target model without reparsing BED.

## Validation obligations

Milestone 8 tests must cover at least:

- BED3 and BED4–BED12 acceptance;
- whitespace and tab separation;
- CRLF and trailing whitespace;
- blank/comment/`track`/`browser` skipping;
- negative, non-numeric, overflow, reversed, and out-of-bounds coordinates;
- unknown contig rejection and no alias inference;
- deterministic ordering independent of source order;
- overlap and adjacency merging;
- source-to-merged mapping;
- zero-length intervals;
- left/right flank clipping;
- exact target SHA-256/size identity;
- deterministic normalization provenance.
