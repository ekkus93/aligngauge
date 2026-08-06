# Milestone 0.5 Walking Skeleton Evidence

**Status:** Complete

## Final implementation commit

- Commit: `a57ec5e1c2986b8e48d064434270b404505a7788`
- Message: `test: clear unmapped flag on mapped fixture records`

This commit completes the vertical path:

```text
aligngauge qc --input sample.bam
  -> rust-htslib 1.0.1
  -> reused bam::Record buffer
  -> total/mapped/unmapped counters
  -> stdout on success or nonzero exit on failure
```

## Permanent CI evidence

- Workflow: `Permanent CI`
- Run: `31092280136`
- Job: `92585861368` (`ci/permanent`)
- Conclusion: `success`

The exact final implementation commit passed:

- locked dependency verification;
- `cargo fmt --all --check`;
- Clippy across all targets and features with warnings denied;
- unit and integration tests;
- rustdoc with warnings denied;
- clean-repository verification.

## Walking-skeleton tests

The integration suite generates BAM files through `rust-htslib` and verifies:

- one mapped plus one unmapped record produces exact counts;
- an empty valid BAM produces three explicit zero counts;
- a missing input exits nonzero and emits no plausible counts;
- a truncated BAM exits nonzero and emits no plausible counts.

## Failure history and correction

The first full test run exposed a fixture defect: `bam::Record::new()` carries the unmapped flag, and the synthetic mapped record did not clear it. The production classifier correctly reported both generated records as unmapped. The fixture now explicitly calls `record.set_flags(0)` for mapped records. No production fallback or semantic relaxation was introduced.

Earlier bootstrap failures were strict formatting and documentation-lint findings. They were corrected directly. The temporary workflow write permission used solely to commit the generated dependency lockfile was removed; permanent CI is read-only and does not persist checkout credentials.

## Backend ergonomics

`docs/adr/ADR-0001-HTSLIB_RECORD_BOUNDARY.md` records:

- caller-owned record reuse;
- record-borrowing constraints;
- CIGAR and tag-access implications;
- open-time versus read-time corruption propagation;
- deferred multithreaded decoding configuration;
- the explicit unresolved long-CIGAR/`CG` validation obligation;
- why a normalized record view is not yet justified.

## Deferred cases

A separate malformed-record fixture was not added because the current stable test path already exercises a truncated BGZF/BAM failure and the project has not yet selected a deterministic byte-level malformed-record generator. Long-CIGAR/`CG` behavior remains an explicit Milestone 3 validation item rather than an assumed capability.

## Evidence-commit rule

This evidence is accepted only after the commit containing this document passes the same permanent CI workflow.
