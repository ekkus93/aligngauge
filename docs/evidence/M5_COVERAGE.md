# Milestone 5 Evidence — Exact Chunked Coverage

**Status:** Complete, subject to exact final evidence-commit validation  
**Implementation source SHA:** `27b056e5766354a63ab6a81e69cf02e8f991170b`  
**Permanent CI:** run `31147742128`, job `92770657494`, success  
**Reference Validation:** run `31147742142`, job `92770657754`, success  
**Release/runtime validation source:** SHA `d0abbda9bf77632b54cc1f54c47f27c5540b65b6`, run `31147602155`, job `92770239455`, success  
**HG002 validation source:** SHA `d0abbda9bf77632b54cc1f54c47f27c5540b65b6`, run `31147602166`, job `92770224253`, success  
**Evidence date:** 2026-08-06

The commit containing the completed TODO is the Milestone 5 evidence candidate. The
Milestone 5 completion claim is valid only after Permanent CI, Full Runtime
Validation, Reference Validation, and HG002 Preparation Validation all succeed on
that exact final evidence-trigger SHA.

## 1. Delivered coverage boundary

Milestone 5 adds `aligngauge-coverage`, an exact CPU coverage collector over the
validated Milestone 3 BAM reader. It uses `FieldPlan::coverage()` and the existing
single reusable BAM record boundary; it does not add a second BAM parser or an
independent input traversal abstraction.

The released M5 collector implements the named `aligngauge-v0.1` coverage profile:

- MAPQ threshold 0;
- exclude unmapped, secondary, QC-fail, duplicate, and supplementary records;
- count both mates independently; no mate-overlap correction;
- count only `M`, `=`, and `X` reference blocks;
- advance reference coordinates for `D` and `N` without adding depth;
- ignore `I`, `S`, `H`, and `P` for reference depth.

All reference coordinate arithmetic is checked. Zero-length/unknown CIGAR operations,
coordinate overflow, and blocks outside the declared reference fail with typed errors;
there is no truncation or clipping fallback.

## 2. One parameterized chunked algorithm

Coverage uses one `parameterized-chunked-delta-v1` implementation for every chunk
size. The algorithm maintains:

- a fixed-size signed delta vector for the active chunk;
- exact carried depth at the chunk boundary;
- a bounded ordered map of future end events;
- deterministic chunk flushing and sparse skipping across constant-depth regions;
- exact reference-finalization invariants at contig transitions and empty contigs.

Very long deletions/skips do not cause iteration over every empty chunk. When no local
delta is active, constant-depth territory is reduced in runs up to the next pending
event or required record position. There is no separate whole-contig algorithm and
no target-specific coverage algorithm in v0.1.

Tests run the same canonical fixture with chunk sizes 1, 7, 64, 1,024, and 65,536
bases and require byte-identical canonical coverage output.

## 3. Memory planning and fail-closed resource behavior

`CoverageMemoryPlan` is computed before opening/traversing the BAM. It explicitly
accounts for:

- active coverage tracks;
- delta-vector entries;
- pending cross-chunk event budget;
- histogram/reduction-map budget;
- reader buffer budget;
- output buffer budget;
- fixed reduction state;
- an explicit safety margin.

The planner supports multi-track estimates even though the canonical v0.1 whole-genome
collector currently activates one track. It refuses zero tracks, zero memory,
impossible minimum plans, oversized explicit chunk sizes, and checked arithmetic
failures. Runtime exhaustion of the bounded pending-event or histogram budgets is a
fatal `resource_limit`; it does not silently approximate, drop events, or switch
algorithms.

Full Runtime Validation measured the sparse `integer_boundary.bam` case with GNU
`time`: observed maximum RSS was 3,543,040 bytes against a 533,725,200-byte planned
peak. The CI check permits a documented fixed 64 MiB measurement tolerance for
runner/runtime accounting, while the product plan itself must remain below the hard
configured limit. A 256 MiB memory-limit probe fails before traversal with
`[resource_limit] memory limit cannot support the minimum exact coverage plan` and
emits no plausible coverage result.

## 4. Exact reductions

The collector produces checked integer reductions for:

- whole-territory depth histogram;
- accepted aligned-base count;
- covered and uncovered reference bases;
- per-reference accepted aligned bases;
- per-reference covered/uncovered bases;
- configurable cumulative depth thresholds.

Per-reference mean depth and threshold percentages are rendered with deterministic
six-decimal, half-up integer arithmetic. No binary floating-point operation is used
for canonical finalization. Median and percentile metrics are not implemented in
Milestone 5, so no median/percentile rounding policy applies yet.

Two invariants are enforced both by tests and collector finalization:

1. the sum of histogram counts equals evaluated reference territory;
2. the weighted sum `depth × bases` equals accepted aligned bases.

Violation is an `internal_invariant` failure, never a warning or adjusted result.

## 5. CIGAR and property validation

Unit/property coverage includes:

- all BAM CIGAR operation codes used by the v0.1 semantics, including combined
  `M/I/D/N/S/H/P/=/X` cases;
- very long `D` and `N` operations;
- reference-bound violations;
- committed chunk-boundary fixtures;
- empty contigs and contig transitions;
- excluded-record invariance;
- deterministic pseudo-fuzz generation of 2,000 CIGAR programs compared with a
  per-base oracle;
- one-track and multi-track planner estimates;
- low-memory failure;
- multiple chunk sizes producing identical canonical results.

The chunk-size property changes only the parameter passed to the same accumulator;
it does not compare different implementations.

## 6. Pinned Samtools differential

The authoritative coverage baseline is `docs/adr/ADR-0003-COVERAGE_BASELINE.md`
(the original TODO reference to ADR-0002 was stale and is corrected at M5 closure).
The pinned baseline is Samtools 1.24 in the immutable container:

`quay.io/biocontainers/samtools@sha256:a130447589651ed09252aa95a5e4f4132942cdb54d835d81a04a9a930d656561`

Reference execution remains network-disabled, read-only, capability-dropped, and
`no-new-privileges`. The exact command semantics are:

```text
samtools depth -aa -q 0 -Q 0 -G UNMAP,SECONDARY,QCFAIL,DUP,SUPPLEMENTARY
```

`tools/reference/samtools/compare-coverage.sh` compares AlignGauge against exact
Samtools reductions for `basic`, `cigar_ops`, `flags_and_pairs`, `chunk_boundary`,
and `multi_track`. Comparison is structural and field-exact; there is no blanket
floating-point epsilon and no accepted unnamed discrepancy. Reference Validation run
`31147742142`, job `92770657754`, completed successfully on implementation source SHA
`27b056e5766354a63ab6a81e69cf02e8f991170b`.

## 7. HG002 public-data validation

The existing reproducible GIAB HG002 GRCh38 chr20 10–11 Mb preparation is retained.
Milestone 5 adds a deterministic coverage-only projection so exact `samtools depth
-aa` validation does not require emitting tens of millions of zero-depth lines for
the full GRCh38 chr20 header territory.

The projection:

- is derived only from the already prepared pinned HG002 subset;
- shifts chr20 coordinates by a fixed recorded offset;
- uses a 1,100,000-base projected chr20 reference that contains the selected source
  interval plus deterministic margins;
- normalizes mate-coordinate/proper-pair metadata only to preserve a valid standalone
  coordinate-sorted projected BAM; these fields are outside the v0.1 coverage
  inclusion/depth semantics;
- is generated twice and requires identical preparation manifests;
- is validated by pinned Samtools and indexed before comparison.

HG002 run `31147602166`, job `92770224253`, passed deterministic source preparation,
exact M4 counter comparison, two identical coverage projections, exact pinned-Samtools
coverage comparison, and evidence upload.

The workflow also runs AlignGauge coverage against the original full-header subset,
which exercises sparse reduction over the real chr20 declared length before the
projection is used for practical exact `depth -aa` differential output.

## 8. Provenance and deterministic surface

Coverage reports record the canonical profile and strategy. `apply_provenance()`
records:

- coverage profile;
- coverage strategy;
- selected chunk size;
- complete memory-plan JSON;
- coverage memory limit;
- planned peak bytes.

Strict Clippy identified panic-capable platform-size conversions in the first draft.
The final implementation propagates those as typed errors instead of using `expect`
or suppressing the lint. Missing/unrepresentable provenance cannot silently produce a
partial record.

## 9. Validation history worth retaining

Several intermediate failures were deliberately not treated as evidence:

- an initial Git-tree assembly accidentally omitted the repository root; it was
  repaired before compiler diagnosis and never used as a validation claim;
- the first compiled draft exposed a child-module visibility error for coverage event
  insertion;
- strict Clippy then found structural quality issues, including an overlong planner
  function and panic-capable provenance conversions; these were refactored rather
  than suppressed;
- Permanent CI later failed only because `python -m py_compile` created untracked
  `__pycache__` files, contaminating the checkout. CI now compiles Python source in
  memory and remains read-only/clean.

These failures are not semantic exceptions and no fallback was added to make them
pass.

## 10. Fail-closed properties

- No excluded record silently contributes depth.
- No `D`/`N` operation is counted as covered reference territory.
- No out-of-range CIGAR block is clipped.
- No arithmetic overflow saturates or wraps.
- No pending event or histogram entry is dropped when a budget is exhausted.
- No impossible memory plan begins BAM traversal.
- No low-memory condition switches from exact to approximate coverage.
- No second whole-contig/target algorithm can drift from the canonical accumulator.
- No reference differential uses network access or an unpinned Samtools image.
- No reference discrepancy is accepted under a blanket tolerance.
- No Python validation step is permitted to dirty the permanent CI checkout.

## 11. Deferred work

Milestone 6 owns final v0.1 CLI/config integration, one-pass counters-plus-coverage
orchestration, final JSON/provenance publication, human/compatibility output assembly,
atomic run-directory publication, end-to-end failure injection, and v0.1 performance
baselines. Milestone 5 establishes the exact coverage engine and evidence consumed by
that release-integration work.
