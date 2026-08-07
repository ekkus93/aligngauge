# ADR-0004: CRAM local-reference resolution

**Status:** Accepted for Milestone 7  
**Date:** 2026-08-07  
**Decision owners:** AlignGauge maintainers

## Context

CRAM decoding may require reference sequence data. HTSlib can resolve those
sequences through more than one provider, and older HTSlib releases historically
included remote lookup behavior when local lookup was insufficient. That is
incompatible with AlignGauge's fail-closed privacy and reproducibility contract:
ordinary analysis must not retrieve a reference implicitly from the network, and a
supplied but incorrect FASTA must not cause a fallback to some other reference.

The pinned production stack for this milestone is:

- `rust-htslib` `1.0.1`;
- `hts-sys` `2.2.1`;
- the HTSlib `1.19.1` source line vendored by that pinned `hts-sys` package.

The production `rust-htslib` dependency disables default features and enables only
`bzip2` and `lzma`, which are required for CRAM codec coverage. It does **not**
enable the `curl`, `s3`, or `gcs` features. Consequently the production HTSlib
build has no libcurl/S3/GCS network transport.

## Provider behavior that must be contained

HTSlib's reference-resolution model can use:

1. an explicitly configured FASTA;
2. `REF_CACHE`;
3. `REF_PATH`, which may contain local paths or URLs;
4. version-specific fallback behavior associated with reference MD5 lookup;
5. `@SQ UR`-style reference locations in CRAM/SAM metadata in relevant HTSlib
   workflows;
6. dynamically discovered HTSlib I/O plugins found through `HTS_PATH`.

For the older HTSlib behavior relevant to the pinned 1.19.1 line, an unset or
insufficient `REF_PATH` could lead to the EBI CRAM MD5 reference service. That
behavior is useful for general-purpose Samtools but is prohibited in AlignGauge.

`REF_CACHE` is also not an integrity authority for AlignGauge. A cache entry may be
useful to a generic HTSlib application, but AlignGauge must not silently substitute
it for the FASTA selected by the user.

## Decision

### 1. Network reference transport is removed at build time

AlignGauge compiles `rust-htslib` with default features disabled and enables only:

- `bzip2`;
- `lzma`.

The following are intentionally absent:

- `curl`;
- `s3`;
- `gcs`.

This is the primary HTTP/HTTPS prohibition. It does not depend on a runtime flag,
environment variable, DNS configuration, or the availability of a remote service.
Permanent CI shall fail if a network transport dependency such as `curl-sys`
appears in the production dependency graph.

### 2. CRAM requires an explicit local FASTA

The release path detects CRAM from local file magic. CRAM analysis requires
`--reference <FASTA>` (or the equivalent explicit API argument). The argument is
interpreted only as a local filesystem path.

No URI syntax is accepted as a reference provider. A missing/non-file reference is
`reference_required`.

Even a CRAM that HTSlib could decode from embedded material or an inherited cache
still requires the explicit FASTA. This keeps the user-visible contract uniform and
auditable.

### 3. AlignGauge validates the selected FASTA before record decoding

The CRAM header's `@SQ` records are converted into requirements containing:

- `SN`;
- `LN`;
- `M5`, when present.

The supplied FASTA is streamed independently of HTSlib reference lookup. For every
required contig AlignGauge verifies:

- the sequence name exists exactly once;
- the normalized sequence length equals `LN`;
- the SAM-style normalized sequence MD5 equals `M5` when `M5` is available.

For the M5 calculation, non-printable sequence bytes are ignored and printable
ASCII sequence characters are upper-cased before MD5, matching the maintained SAM
M5 convention used by the CRAM ecosystem.

Extra FASTA contigs are permitted but are not fallback candidates. A duplicate
FASTA name is a mismatch.

A supplied mismatch returns `reference_mismatch` **before** the reference is passed
to HTSlib for record decoding. There is no retry with `REF_CACHE`, `REF_PATH`,
`UR`, a plugin, or a remote provider.

### 4. HTSlib receives only the already-validated explicit FASTA

After AlignGauge validates the CRAM header requirements against the selected FASTA,
the reader calls `rust_htslib::bam::Reader::set_reference` with that exact local
path. Record traversal starts only after that succeeds.

A failure from `set_reference` is fatal. AlignGauge does not catch that failure and
try another provider.

### 5. Inherited reference environment is neutralized, not process-globally mutated

The specification originally described overriding inherited `REF_PATH` and
`REF_CACHE` before opening CRAM. The implementation uses a stronger scoped policy
rather than process-global environment mutation:

- remote reference transports are absent from the compiled HTSlib;
- CRAM without an explicit FASTA is rejected by AlignGauge;
- a supplied FASTA is validated before reference-dependent record decoding;
- only the validated path is passed through `set_reference`;
- mismatch is terminal and cannot fall through;
- tests intentionally supply hostile `REF_PATH`, `REF_CACHE`, and `HTS_PATH`
  values and require the explicit-reference path to remain authoritative.

This avoids unsafe/racy mutation of process-global environment state in a
multi-thread-capable Rust process while preserving the security requirement:
inherited provider configuration cannot select a different reference or create an
implicit network dependency.

Opening the local CRAM container is required to obtain its header and therefore the
`@SQ` requirements. The pinned build has no network transport at that point, and no
record is decoded until the explicit FASTA has passed AlignGauge validation and
`set_reference` has succeeded.

### 6. Reference identity is provenance, not just configuration

CRAM provenance records the actual selected reference identity:

- local FASTA path;
- exact file size;
- SHA-256 of the FASTA bytes;
- validated required contigs in CRAM-header order;
- normalized per-contig length;
- normalized per-contig MD5.

It also records:

- input format;
- `rust-htslib` version;
- `hts-sys` version;
- HTSlib source version;
- that HTSlib network transport is disabled;
- BAM/CRAM/alignment traversal counts.

## Failure policy

| Condition | Result |
| --- | --- |
| CRAM without explicit FASTA | `reference_required` |
| FASTA path missing/not a file | `reference_required` |
| Required contig absent | `reference_required` with contig and expected identity where available |
| Duplicate FASTA contig | `reference_mismatch` |
| `LN` mismatch | `reference_mismatch` |
| `M5` mismatch | `reference_mismatch` |
| HTSlib rejects validated FASTA | `reference_mismatch` |
| Corrupt/truncated CRAM | `input_corrupt` |
| Any reference mismatch followed by fallback | prohibited by design/test |

No completed output is published after these failures.

## Validation requirements

Milestone 7 is not complete until permanent tests demonstrate all of the following:

1. BAM and CRAM generated from the same deterministic records produce identical
   canonical counters and coverage.
2. Common provenance is identical after removing only explicitly format/reference-
   specific fields.
3. Missing reference fails with `reference_required`.
4. Wrong same-length reference fails with `reference_mismatch`.
5. Truncated/corrupt CRAM fails with `input_corrupt`.
6. Hostile inherited `REF_PATH`, `REF_CACHE`, and `HTS_PATH` cannot redirect a run
   that supplies the correct explicit FASTA.
7. A network-syscall observation test sees no IPv4/IPv6 socket attempt during the
   CRAM reference-isolation case.
8. The production dependency graph contains no enabled HTSlib network transport.

## Consequences

### Positive

- ordinary CRAM analysis is local-only and reproducible;
- a user-selected FASTA cannot silently be replaced;
- reference mismatch is visible and typed;
- the exact FASTA used is auditable from provenance;
- BAM and CRAM share the already-validated record/collector path rather than
  maintaining separate algorithms.

### Costs and limitations

- users must provide a FASTA for every CRAM run, even if generic HTSlib could find
  or reconstruct enough reference material elsewhere;
- AlignGauge does not provide refget/EBI/HTTP reference convenience in this mode;
- CRAMs whose required reference cannot be validated locally are rejected;
- absence of `M5` in an `@SQ` record means AlignGauge can validate name and length
  and record the computed FASTA MD5, but it cannot claim an M5 comparison that the
  input did not provide.

## Upstream references

- HTSlib reference-sequence lookup documentation: <https://www.htslib.org/doc/reference_seqs.html>
- Samtools CRAM workflow and `REF_PATH` / `REF_CACHE`: <https://www.htslib.org/workflow/cram.html>
- `rust-htslib` crate documentation and feature description: <https://docs.rs/rust-htslib/1.0.1/rust_htslib/>
- `rust-htslib::bam::Reader::set_reference`: <https://docs.rs/rust-htslib/1.0.1/rust_htslib/bam/struct.Reader.html>
