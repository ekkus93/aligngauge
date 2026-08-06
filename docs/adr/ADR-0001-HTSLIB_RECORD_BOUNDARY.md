# ADR-0001: HTSlib Record Boundary for the Walking Skeleton

**Status:** Accepted for Milestone 0.5; subject to refinement in Milestone 3  
**Date:** 2026-08-06

## Context

The first vertical slice must establish how AlignGauge opens BAM input, reuses record storage, propagates corruption, and presents decoded records without freezing a broad multi-crate abstraction prematurely.

## Decision

Milestone 0.5 uses `rust-htslib` 1.0.1 with default features disabled. The CLI calls a small library function that owns a `bam::Reader`, allocates one `bam::Record`, and repeatedly invokes `Read::read(&mut record)` until end of input.

This avoids the per-record allocation performed by the convenience `records()` iterator. No normalized record type, collector trait, planner, JSON schema, staging directory, or GPU dimension is introduced.

## Findings

### Record borrowing and reuse

`Read::read` fills a caller-owned `Record`, permitting one allocation to be reused for the complete traversal. Data borrowed from the record cannot outlive the next read without being copied. The production boundary must make that lifetime explicit.

### CIGAR access

The high-level `Record::cigar()` API constructs an unpacked CIGAR view and is not needed by the walking skeleton. Coverage work must benchmark `raw_cigar`, `cigar`, and `cache_cigar` before selecting the hot-path representation.

### Auxiliary tags

Tag access is fallible and returns data tied to the current record. Missing tags must remain distinguishable from zero values. The walking skeleton does not read auxiliary tags.

### Truncation and decoder errors

Failure can occur while opening the file or while reading a later record. Both are preserved as separate error variants and cause a nonzero exit without printing plausible counts.

### Multithreaded decoding

HTSlib exposes `Read::set_threads`. Milestone 0.5 intentionally leaves it disabled so the vertical slice measures the simplest boundary. Thread configuration belongs to the production reader milestone.

### Oversized CIGAR / `CG`

The backend's exact expansion behavior has not yet been proven by a committed fixture. AlignGauge therefore makes no support claim at this milestone. Milestone 3 must either prove correct expansion or reject such records explicitly.

### Need for a normalized record view

The three counters require only `is_unmapped`. A normalized record view would add ceremony without evidence. The decision is deferred until flag counters and coverage expose the actual required field set.

## Consequences

- The walking skeleton has a real CLI-to-HTSlib-to-result path.
- Corruption cannot be mistaken for an empty or partially counted file.
- The production design remains free to choose borrowed, owned, or batched normalized records.
- Long-CIGAR support, tag semantics, and thread configuration remain explicit future validation obligations rather than assumptions.
