# v0.2 CRAM validation evidence

**Milestone:** 7 — CRAM local-reference design  
**Evidence date:** 2026-08-07  
**Validated implementation SHA:** `cf288efe8ffdc3542abedc95404bc6602515da4a`  
**Disposition:** Milestone 7 implementation tasks 7.1–7.4 are complete. This document does **not** declare the v0.2 release gate closed.

## 1. Validation summary

The validated implementation adds CRAM to the existing release analysis path while preserving the v0.1 BAM parser and publication structure. CRAM is accepted only with an explicit local FASTA. AlignGauge independently validates that FASTA against CRAM `@SQ` identity before record traversal and passes only that validated path to the pinned HTSlib reader.

The exact implementation SHA passed all three standing pull-request suites:

| Gate | Run | Job | Result |
| --- | --- | --- | --- |
| Permanent CI | `31184689945` | `92886187241` | success |
| Full Runtime Validation | `31184688346` | `92886217801` | success |
| Reference Validation | `31184688367` | `92886248460` | success |

Permanent CI passed formatting, strict Clippy, the complete workspace test suite, documentation, clean-tree verification, HTSlib network-feature exclusion, and the network-disabled CRAM reference-isolation proof.

## 2. Pinned backend and provider audit

Production CRAM support is pinned to:

- `rust-htslib` `1.0.1`;
- `hts-sys` `2.2.1`;
- vendored HTSlib source line `1.19.1`.

`crates/aligngauge-hts/Cargo.toml` disables `rust-htslib` default features and enables only `bzip2` and `lzma` for CRAM codec coverage. The production dependency graph does not enable `curl`, `s3`, or `gcs`. Permanent CI fails if `curl-sys` appears or if the relevant HTSlib network features are enabled.

`docs/ADR-0004-CRAM_REFERENCE_RESOLUTION.md` records the audited provider surface: explicit FASTA, `REF_CACHE`, `REF_PATH`, version-specific MD5 lookup fallback behavior, `@SQ UR`-style locations where relevant, and plugin discovery through `HTS_PATH`. It also records the older HTSlib behavior under which insufficient reference-path configuration could lead to remote EBI CRAM reference lookup.

Build-time removal of network transports is the primary HTTP/HTTPS prohibition. It does not rely on DNS failure, a remote service being unavailable, or a best-effort runtime flag.

## 3. Explicit local-reference resolver

The CRAM release path requires an explicit `--reference <FASTA>` or equivalent API reference argument. CRAM without a reference fails with `reference_required`, even if generic HTSlib behavior could otherwise find sequence material through inherited state.

Before reference-dependent record decoding, `crates/aligngauge-hts/src/reference.rs` streams the selected FASTA and validates CRAM-header requirements:

- exact contig name (`SN`);
- exact normalized sequence length (`LN`);
- SAM-style normalized sequence MD5 against `M5` when present;
- duplicate FASTA names are rejected;
- missing required sequence is `reference_required`;
- length or MD5 mismatch is `reference_mismatch`.

Mismatch is terminal. AlignGauge does not retry with `REF_CACHE`, `REF_PATH`, `UR`, plugins, or a remote provider. Only after independent validation succeeds does `BamReader::open_cram` call `Reader::set_reference` with the already-validated local path.

## 4. Inherited provider-state neutralization

The original TODO wording said to override inherited reference environment before opening CRAM. ADR-0004 deliberately avoids process-global environment mutation because it is unsafe and race-prone in a multi-thread-capable Rust process.

The implemented scoped policy makes inherited provider state non-authoritative instead:

1. HTSlib network transports are absent from the production build.
2. CRAM requires an explicit local FASTA.
3. AlignGauge validates that FASTA independently before traversal.
4. Only the validated path is supplied through `set_reference`.
5. Any mismatch is terminal.
6. Tests inject hostile `REF_PATH`, `REF_CACHE`, and `HTS_PATH` values and require the explicit local reference to remain authoritative.

This satisfies the security intent without mutating process-global state. The literal wording difference from SPEC §8 is recorded as an open release-scope reconciliation item rather than hidden.

## 5. Reference provenance

CRAM provenance records the actual selected local reference identity:

- local FASTA path;
- exact FASTA byte size;
- SHA-256 of the FASTA bytes;
- required contigs in CRAM-header order;
- normalized per-contig length;
- normalized per-contig MD5.

Provenance also records input format, pinned backend versions, BAM/CRAM/alignment traversal counts, and `htslib_network_transport_enabled = false`.

## 6. BAM/CRAM equivalence

`crates/aligngauge-cli/tests/cram_v0_2.rs` constructs one deterministic record set, serializes it as BAM, and writes an equivalent CRAM with the explicit local reference. The equivalence test requires:

- BAM/CRAM format detection;
- identical canonical counters;
- identical canonical coverage;
- identical complete canonical summary;
- equality of common header/backend/resource/normalization/warning/system provenance;
- analysis-plan equality after removing only explicitly format/reference-specific keys.

No blanket tolerance or format-specific alternate collector is used. BAM and CRAM share the validated record and collector path.

## 7. Failure behavior

The CRAM integration suite proves:

- no explicit FASTA -> `reference_required`;
- required contig absent -> `reference_required`, with contig and expected M5 in the diagnostic;
- wrong same-length FASTA -> `reference_mismatch`;
- half-truncated CRAM -> `input_corrupt`;
- separately malformed CRAM version -> `input_corrupt`.

The malformed-version case is distinct from the truncation case, so the TODO requirement for both truncation and corruption is covered by two independent failure modes.

## 8. Network-isolation proof

Permanent CI first proves that production HTSlib network transport features are absent. It then builds the exact `cram_v0_2` integration-test executable, obtains its path from Cargo JSON artifact metadata, and requires exactly one matching executable.

The hostile-reference test runs under `sudo unshare --net` and `strace -f -e trace=network`. CI rejects the run if the trace contains an `AF_INET` or `AF_INET6` syscall. The validated implementation SHA passed this gate in Permanent CI run `31184689945`, job `92886187241`.

The test supplies hostile inherited values including `REF_PATH=http://127.0.0.1:9/%s`, a nonexistent `REF_CACHE`, and a nonexistent `HTS_PATH`. The correct explicit FASTA still succeeds, while the network namespace and syscall trace prove the case does not attempt IPv4/IPv6 reference retrieval.

## 9. Milestone 7 reconciliation

### 7.1 Backend behavior

Complete. Versions are pinned, provider behavior is documented, remote transports are removed, and ADR-0004 records the decision.

### 7.2 Local-only resolution

Complete under ADR-0004's scoped-neutralization design. Explicit FASTA, SN/LN/M5 validation, missing/mismatch failures, no alternate fallback, and actual FASTA provenance are implemented and tested.

### 7.3 Network isolation

Complete. The hostile-M5 case runs in a network-disabled namespace with syscall observation, while missing/mismatch/correct-reference behavior is covered by the permanent CRAM integration suite.

### 7.4 BAM/CRAM equivalence

Complete. Equivalent fixtures, exact canonical counter/coverage comparison, constrained provenance comparison, truncation, and a distinct corruption fixture are covered.

## 10. Release-scope items intentionally left open

Milestone 7 implementation evidence is complete, but the v0.2 release is **not** declared complete for two reasons:

1. SPEC §4.2 says v0.2 adds `inspect` and `validate-reference` workflows. Those workflows are not implemented by Milestone 7 and are not listed in SPEC §19.2's v0.2 acceptance bullets. Their disposition must be explicitly reconciled before a v0.2 release claim.
2. SPEC §8 literally says to override inherited reference environment state before opening CRAM. ADR-0004 implements the safer scoped-neutralization design instead. The specification should be amended or explicitly reconciled before release so normative text and implementation do not disagree.

The final TODO release-gate item requiring Permanent CI on the exact release commit therefore remains unchecked. No tag, release, merge-to-master, or v0.2 completion claim is made by this evidence document.
