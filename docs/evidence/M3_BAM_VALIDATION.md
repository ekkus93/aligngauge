# Milestone 3 Evidence — Production BAM Reader and Validation

**Status:** Complete, subject to exact evidence-commit Permanent CI  
**Implementation SHA:** `8d5cbb1d69764a8ba0d96ef96c03f24af1a77fe5`  
**Branch validation:** run `31106115769`, job `92631610668`, success  
**Evidence date:** 2026-08-06

The commit containing this document is the Milestone 3 evidence candidate. The
Milestone 3 completion claim is valid only after Permanent CI succeeds on that
exact commit after it is published to `master`.

## 1. Delivered boundary

Milestone 3 adds the `aligngauge-hts` crate as the sole v0.1 production BAM input
boundary. It:

- accepts local BGZF-compressed BAM and verifies compressed and decompressed magic;
- owns one reusable `rust_htslib::bam::Record` buffer;
- returns a validated record borrowed only until the next reader call;
- wraps HTSlib failures with stable AlignGauge categories and preserved causes;
- bounds header bytes, header fields, reference/read-group counts, record bytes,
  query-name storage, sequence length, CIGAR operations, auxiliary fields, and
  decode threads;
- exposes only fields selected by an immutable required-field plan.

The CLI counting path now consumes this boundary and emits counts only after the
entire stream validates. A reader failure therefore cannot leave plausible
completed counts on stdout.

## 2. Header contract

The reader parses the raw SAM header and cross-checks textual `@SQ` declarations
against the BAM binary reference table in order, name, and length. Duplicate or
contradictory references, invalid lengths, oversized header structures, invalid
UTF-8, and inconsistent binary/text tables are fatal.

Sort-order metadata is retained but not trusted as proof. Read-group declarations
remain untrusted and resolve explicitly as known, unknown, or ambiguous. A
stable, domain-separated SHA-256 header identity is available for provenance.

## 3. Record and ordering contract

Every record is checked before collector access:

- BAM variable-data layout is validated before any wrapper slice accessor;
- reserved or contradictory flag combinations fail explicitly;
- coordinate sentinels, target IDs, and positions are checked;
- CIGAR query/reference spans use checked arithmetic;
- mapped records require coordinates and a nonempty CIGAR;
- CIGAR query span must match sequence length;
- reference-consuming spans may not exceed the declared reference;
- auxiliary data is fully parsed so malformed trailing fields cannot be ignored;
- requested `NM`, `MD`, and `RG` values preserve missing/unknown states.

Actual coordinates must be nondecreasing regardless of `@HD SO`. Once a
no-coordinate tail starts, a later coordinate-bearing record is fatal. Diagnostics
include prior/current coordinates while read names remain redacted by default.

## 4. Oversized CIGAR and field planning

The committed 66,000-operation fixture proves that the pinned backend expands BAM
`CG:B,I` long-CIGAR representation. A remaining `CG` tag with only a short
placeholder CIGAR is rejected as unsupported rather than silently truncated.

The counters plan requests flags and coordinates. The coverage plan adds CIGAR.
Optional `NM`, `MD`, and `RG` access is explicit. Sequence and qualities are not
exposed by v0.1 plans, and the plan contains no backend, GPU, or CUDA dimension.
The resolved plan has deterministic JSON suitable for provenance.

## 5. Safety defect found during validation

The first malformed-record test demonstrated that calling rust-htslib query-name or
CIGAR accessors before validating the decoded BAM variable-data layout could panic
inside the wrapper. The final implementation checks `l_data`, query-name storage,
CIGAR storage, packed sequence, and quality storage with checked arithmetic before
any such accessor. The malformed fixture now returns `input_corrupt` without a
panic or partial result.

A later test showed that a truncated stream whose BAM magic was already verified
was categorized as `input_format` when HTSlib could not open it. The final boundary
classifies that state as `input_corrupt`, preserving the distinction between an
unsupported format and a damaged BAM.

## 6. Validation evidence

Branch validation run `31106115769`, job `92631610668`, succeeded and required:

1. `cargo fmt --all`;
2. workspace compilation and locked dependency resolution;
3. strict Clippy across all targets and features;
4. all workspace tests, including the Milestone 3 validation suite;
5. rustdoc with warnings denied;
6. byte-identical regeneration of fixtures, expected outputs, and manifest;
7. `git diff --check`;
8. removal of all temporary validation/fixup machinery before publishing
   implementation SHA `8d5cbb1d69764a8ba0d96ef96c03f24af1a77fe5`.

The validation suite covers valid committed v0.1 fixtures, coordinate regression,
no-coordinate-tail violations, malformed optional data, malformed record lengths,
truncated BGZF, unknown target IDs, duplicate/contradictory references, invalid
reference lengths, reference-bound violations, reserved flag bits, long CIGAR,
missing/unknown/ambiguous read groups, redaction, and deterministic identities and
field plans.

## 7. Fail-closed properties

- No record is skipped after a validation failure.
- No malformed auxiliary tail is ignored.
- No missing `NM` or `MD` becomes zero.
- No unknown or duplicate read group is silently normalized.
- No header sort-order claim bypasses actual coordinate checks.
- No mapped record after the no-coordinate tail is accepted.
- No CIGAR/reference overflow is saturated or wrapped.
- No explicit unsupported record representation falls back to a weaker path.
- No completed CLI counts are emitted after a reader failure.

## 8. Deferred work

Milestone 3 does not implement final flag/per-reference counters, canonical output
integration, or Samtools differential matching; those are Milestone 4. Exact
chunked coverage remains Milestone 5. CRAM and fail-closed reference resolution
remain v0.2 work.
