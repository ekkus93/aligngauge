# ADR-0001: HTSlib Record Boundary

**Status:** Accepted for Milestone 3  
**Date:** 2026-08-06

## Context

The walking skeleton established that `rust-htslib::bam::Read::read` can reuse one
caller-owned record and preserve decoder failures. Milestone 3 must turn that probe
into the production v0.1 BAM boundary without freezing Milestone 4 counters or
Milestone 5 coverage collectors into the I/O layer.

The boundary must also resolve the deferred questions about long CIGARs, header
trust, coordinate order, optional tags, read groups, resource limits, and which
record fields consumers may access.

## Decision

AlignGauge uses a dedicated `aligngauge-hts` crate over pinned `rust-htslib` 1.0.1
with default features disabled. The crate owns the HTSlib reader and one reusable
`bam::Record`. `BamReader::next_record` returns a validated record borrowed until
the next call, so consumers cannot retain pointers into storage that HTSlib will
reuse.

The v0.1 boundary accepts local BGZF-compressed BAM only. It verifies compressed
and decompressed magic before opening HTSlib. SAM, CRAM, non-BGZF input, malformed
headers, decoder failures, and unsupported record representations fail with stable
typed errors.

## Header contract

The reader parses and bounds the raw SAM header, then cross-checks textual `@SQ`
declarations against the BAM binary reference table in order, name, and length.
Duplicate or contradictory reference declarations are fatal. Sort-order metadata
is retained but never trusted as proof of actual ordering.

Read-group declarations are retained as untrusted values. A unique ID is `known`,
a missing ID is `unknown`, and duplicate IDs are `ambiguous`; AlignGauge never
selects one duplicate declaration or invents a replacement group.

A domain-separated SHA-256 over the raw header and binary reference table is
recorded as the header identity for provenance.

## Record and ordering contract

Every record is validated before it reaches a collector:

- record, query-name, sequence, CIGAR, auxiliary-field, and thread counts are
  bounded;
- flags use only the standard BAM mask and required pair-bit relationships are
  checked;
- target IDs and positions use explicit sentinel rules;
- CIGAR query/reference spans use checked integer arithmetic;
- query span must equal sequence length when a CIGAR is present;
- mapped records require a CIGAR and coordinate;
- reference-consuming spans may not cross the declared reference length;
- every auxiliary field is parsed so malformed trailing data cannot be ignored;
- requested `NM`, `MD`, and `RG` values preserve missing and unknown states.

Actual coordinate order is enforced independently of `@HD SO`. Coordinate-bearing
records must be nondecreasing by target ID and position. Once a no-coordinate tail
begins, a later coordinate-bearing record is fatal. Diagnostics include prior and
current coordinates while read names remain sensitive details.

## Oversized CIGAR result

The committed `long_cigar` fixture contains 66,000 operations in BAM `CG:B,I`
representation. The pinned HTSlib backend expands it into the record CIGAR before
AlignGauge validation. The reader requires the expanded operation count to exceed
65,535; a remaining `CG` tag with a short placeholder CIGAR is rejected as
`unsupported_record`. Undefined or silently truncated behavior is not accepted.

## Required-field planning

`FieldPlan` is immutable and deterministic. The counters plan exposes flags and
coordinates; the coverage plan adds CIGAR data; optional-tag access is explicit.
Sequence and qualities are not materialized by v0.1 plans. The resolved plan has a
stable JSON provenance form and contains no backend, GPU, or CUDA dimension.

Validation may inspect fields that are not exposed because corruption and bounds
checks are not optional collector work.

## Consequences

- All v0.1 collectors share one validated record stream.
- Corruption, unsupported records, and coordinate regressions cannot produce
  plausible completed counts.
- Missing tags cannot become metric zero.
- Unknown and contradictory read groups remain visible rather than silently
  normalized.
- No owned normalized batch is introduced before collector requirements justify
  one.
- CRAM reference resolution remains a v0.2 decision and will receive a separate
  ADR.
