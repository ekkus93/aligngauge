# Milestone 4 Evidence — Flag and Per-Reference Counters

**Status:** Complete, subject to exact evidence-commit Permanent CI  
**Product SHA:** `54e7803835eb2a31207b36a44d44b178ae6b86ab`  
**Branch validation:** run `31112508841`, job `92653647084`, success  
**Exact-product Permanent CI:** run `31113177504`, job `92655944921`, success  
**Exact-product Reference Validation:** run `31113177567`, job `92655945424`, success  
**Exact-product HG002 validation:** run `31113177174`, job `92655943661`, success  
**Evidence date:** 2026-08-06

The commit containing this document is the Milestone 4 evidence candidate. The
Milestone 4 completion claim is valid only after Permanent CI succeeds on that
exact commit after it is published to `master`.

## 1. Delivered counter boundary

Milestone 4 adds `aligngauge-metrics` as the checked alignment-counter layer over
the validated Milestone 3 reader. One traversal produces:

- total, primary, secondary, supplementary, mapped, unmapped, duplicate, paired,
  properly paired, read-one, read-two, mate-mapped, mate-unmapped, and singleton
  counters;
- independent QC-pass and QC-fail partitions needed for exact Samtools rendering;
- mapped and unmapped counts for every declared reference in BAM-header order;
- the no-coordinate count used by the terminal `*` row in `idxstats` output;
- a canonical `Summary`, deterministic human text, Samtools-like `flagstat`, and
  Samtools-like `idxstats` projections.

All counter increments use checked `u64` arithmetic. Overflow is a typed fatal
error. No counter saturates, wraps, or silently stops changing.

## 2. Classification semantics

The compatibility profile is pinned to Samtools 1.24. Classification follows its
priority exactly:

1. a record with `SECONDARY` is secondary, including a record carrying both
   `SECONDARY` and `SUPPLEMENTARY`;
2. otherwise a record with `SUPPLEMENTARY` is supplementary;
3. otherwise the record is primary.

All-record counters such as total, mapped, and duplicate include secondary and
supplementary records where Samtools does. Pair, proper-pair, read-one, read-two,
singleton, both-mates-mapped, primary-mapped, and primary-duplicate metrics use the
pinned primary-record rules. Proper-pair counting also requires the current record
to be mapped. Different-reference counters retain the Samtools MAPQ-at-least-five
subpartition.

The dual-secondary/supplementary synthetic record is therefore counted once as
secondary and never double-counted.

## 3. Per-reference semantics

The collector allocates one checked counter entry per validated header reference and
preserves header order through canonical and compatibility output. Empty references
remain present with zero mapped and zero unmapped records. Invalid target IDs are
rejected by the reader before collection.

Mapped and unmapped records with a valid target ID are assigned to that target. The
terminal `*` row contains zero mapped records and the exact no-coordinate count,
consistent with the v0.1 reader contract that rejects mapped records without a
coordinate.

## 4. Canonical and compatibility separation

The canonical model stores aggregate alignment counters and header-ordered
per-reference counters. QC-pass/QC-fail partitions and Samtools-specific priority
rules remain collector/compatibility state rather than silently redefining the
canonical schema.

`summary.json` receives collected counter values while coverage remains explicitly
unavailable until Milestone 5. Provenance records the `aligngauge-v0.1` policy and
pinned Samtools compatibility version. Compatibility text is derived from the same
checked collector state; it is not reparsed or maintained as an independent source
of truth.

The existing three-line walking-skeleton CLI output remains the default. Explicit
formats add human, canonical JSON, Samtools `flagstat`, and Samtools `idxstats`
rendering. Unknown formats fail; there is no fallback to another renderer.

## 5. Determinism and arithmetic hardening

Strict Clippy rejected the initial human percentage renderer because converting
large `u64` counters to `f64` could lose precision. The final implementation computes
rounded hundredths of a percent with checked-width `u128` integer arithmetic and
formats the result deterministically.

Repeated analysis of the same fixture produces byte-identical canonical and
compatibility output. Header order is preserved rather than alphabetically resorted.
Missing coverage data remains unavailable rather than becoming zero.

## 6. Synthetic differential evidence

The branch gate ran the digest-pinned Samtools 1.24 image in a network-disabled,
read-only, capability-dropped container and compared AlignGauge output against:

- `basic.bam` for baseline `flagstat` and `idxstats` behavior;
- `flags_and_pairs.bam` for QC partitions, pair metrics, singleton behavior,
  duplicate handling, different-reference/MAPQ handling, and the dual-flag
  classification priority.

The differential compares every integer field and exact `idxstats` rows. It does not
use blanket tolerances. No unexplained integer discrepancy remains.

## 7. HG002 public-data evidence

The pinned GRCh38 GIAB HG002 chr20 10–11 Mb subset was prepared twice with the
committed source identities, seed, and fraction. The two preparation manifests were
identical. AlignGauge then matched pinned Samtools for both `flagstat` integer fields
and exact `idxstats` output. The reference and AlignGauge captures were uploaded as
workflow evidence.

The first HG002 attempt failed before comparison because the reference container's
default user could not traverse the preparation script's private staging-derived
output directory. This was not a counter discrepancy. The final reference runner
uses the invoking host UID/GID, matching the preparation sandbox without weakening
host permissions or making generated alignments world-readable.

## 8. Validation evidence

Branch validation run `31112508841`, job `92653647084`, required and passed:

1. formatting and workspace compilation;
2. locked dependency validation;
3. strict Clippy for all targets and features;
4. all unit and integration tests;
5. rustdoc with warnings denied;
6. byte-identical corpus regeneration;
7. exact pinned-Samtools synthetic counter differential;
8. clean-tree and temporary-gate removal checks.

On exact product SHA `54e7803835eb2a31207b36a44d44b178ae6b86ab`:

- Permanent CI run `31113177504`, job `92655944921`, passed every repository
  gate;
- Reference Validation run `31113177567`, job `92655945424`, passed the exact
  synthetic counter differential and retained coverage baseline;
- HG002 run `31113177174`, job `92655943661`, passed deterministic preparation,
  exact public-data counter comparison, and evidence upload.

## 9. Fail-closed properties

- No arithmetic overflow is saturated or ignored.
- No record is assigned to more than one primary/secondary/supplementary class.
- No dual-flag record is double-counted.
- No unknown target ID is coerced to the no-coordinate row.
- No missing counter or future coverage result is encoded as zero.
- No header reordering is hidden in canonical output.
- No compatibility mismatch is accepted under an unnamed tolerance.
- No unsupported output format falls back silently.
- No reference-tool network access is permitted during differential execution.

## 10. Deferred work

Exact chunked coverage, memory planning, histograms, thresholds, and coverage
differential validation remain Milestone 5. Final atomic output-directory CLI
integration remains later v0.1 work; Milestone 4 establishes the canonical and
compatibility counter projections consumed by that path.
