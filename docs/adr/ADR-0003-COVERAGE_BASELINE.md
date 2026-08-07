# ADR-0003: v0.1 Coverage Differential Baseline

- **Status:** Accepted
- **Date:** 2026-08-06
- **Decision owners:** AlignGauge maintainers
- **Applies to:** v0.1 coverage differential evidence

## Context

AlignGauge needs one named external baseline for coverage development and
differential testing. A vague claim of “mosdepth-like” or “Samtools-compatible”
would hide filter, deletion, overlap, and zero-depth differences.

ADR-0002 is already the output-destination policy. This ADR therefore owns the
coverage baseline; the future CRAM reference-resolution ADR is ADR-0004.

## Decision

The v0.1 development baseline is Samtools/HTSlib 1.24 executed from:

```text
quay.io/biocontainers/samtools@sha256:a130447589651ed09252aa95a5e4f4132942cdb54d835d81a04a9a930d656561
```

The canonical baseline command is:

```bash
samtools depth \
  -aa \
  -q 0 \
  -Q 0 \
  -G UNMAP,SECONDARY,QCFAIL,DUP,SUPPLEMENTARY \
  INPUT.bam
```

The profile deliberately:

- emits zero-depth positions for every declared reference (`-aa`);
- accepts bases and alignments at quality zero (`-q 0`, `-Q 0`);
- excludes unmapped, secondary, QC-fail, duplicate, and supplementary records;
- does not use `-J`, so deletions do not count as covered bases;
- does not use `-s`, so mate-overlap correction is disabled;
- does not apply a BED target;
- does not imply byte-for-byte compatibility with Samtools output formatting.

Every baseline invocation runs in a digest-pinned container with
`--network none`. The runner captures command text, tool version, stdout,
stderr, exit status, and wall time. Nonzero status or an incomplete capture is
fatal.

## Rationale

Samtools is already the v0.1 counter oracle and provides a small, independently
implemented coverage path. Explicit flags make its semantics reviewable. Using a
single baseline avoids maintaining several subtly different meanings of depth
before AlignGauge’s canonical coverage engine exists.

The external baseline is an oracle for differential investigation, not the
product specification. AlignGauge’s specification remains authoritative where
semantics intentionally differ.

## Consequences

- Milestone 2 can establish reproducible baseline artifacts before coverage is
  implemented.
- Milestone 5 must reconcile every integer result against this profile or attach
  a named compatibility note.
- No blanket numerical tolerance is permitted.
- Exact mate-overlap correction remains deferred.
- Changing the version, digest, or command is a semantic change requiring a new
  ADR or an explicit amendment with regenerated evidence.
