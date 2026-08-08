# AlignGauge v0.5 hardening evidence

**State:** implementation candidate — this report becomes complete only after the permanent v0.5 hardening workflow succeeds on the exact evidence commit and no release-blocking finding remains.

## Hardening contract

ADR-0012 makes the following v0.5 release blockers rather than warning-only checks:

1. BED parser fuzz campaign failure;
2. raw BAM CIGAR coverage-boundary fuzz campaign failure;
3. atomic-output fault-injection regression;
4. sanitizer-compatible HTS/native-boundary test failure where the pinned runner supports the sanitizer configuration;
5. unresolved dependency advisory under the committed policy;
6. unapproved or unknown dependency license;
7. unknown package source;
8. SBOM/license inventory generation failure;
9. nondeterministic generated SBOM/license inventory;
10. release-binary reproducibility mismatch under the documented controlled build inputs;
11. missing schema migration/compatibility documentation;
12. missing signed/attested release checksum evidence at publication.

The workflow must fail nonzero for these conditions. It must not turn them into `continue-on-error`, `|| true`, best-effort steps, or advisory-only output.

## Fuzzing

The committed `fuzz/` package is intentionally outside the production Cargo workspace and contains two libFuzzer targets:

- `bed_parser`: arbitrary bytes enter `aligngauge_formats::parse_bed_bytes` against a fixed valid sequence dictionary;
- `cigar_blocks`: arbitrary raw BAM CIGAR words plus arbitrary start/reference lengths enter `aligngauge_coverage::cigar_to_coverage_blocks`.

Expected typed parse/validation errors are allowed. A panic, sanitizer finding, abort, or crash is a campaign failure.

Permanent CI uses a pinned nightly toolchain and pinned `cargo-fuzz`/`libfuzzer-sys` versions. The standing CI campaign is bounded for maintainability; release evidence must also record any additional pre-release campaign duration/runs if the permanent budget is expanded.

## Output fault injection

`aligngauge-core` already exposes deterministic publication checkpoints and tests every pre-rename checkpoint. The v0.5 hardening workflow reruns the complete atomic-publication test module, including:

- no partial destination visible to observers;
- fail-closed cleanup at every pre-rename checkpoint;
- preserved failures marked `_FAILED` with no `_SUCCESS`;
- existing destinations never overwritten;
- reserved/path-like output names rejected.

This reuses the production publication implementation rather than maintaining a separate fault-injection simulation.

## Dependency/security policy

The repository commits `deny.toml` and pins the `cargo-deny` executable in the hardening workflow. Advisory, license, and source checks are fatal.

The exact policy intentionally allows only the licenses required by the resolved non-dev dependency graph. New dependency licenses or package sources must be explicitly reviewed and committed; they are not accepted automatically.

## SBOM and license inventory

`tools/v0.5/generate-sbom.py` uses `cargo metadata --locked` to generate:

- deterministic CycloneDX 1.5 JSON;
- deterministic package/license/source inventory JSON.

The workflow generates both twice and byte-compares the outputs. The artifact contains the generated files and checksums.

## Reproducible-build assessment

The hardening workflow performs two clean release builds in separate target directories with a fixed `SOURCE_DATE_EPOCH` derived from the exact commit and path remapping. The resulting `aligngauge` binaries must be byte-identical for the assessment to pass.

A passing assessment is evidence for the named Linux/toolchain build environment only; it is not a claim that every platform/toolchain produces identical binaries.

## Schema compatibility

`docs/SCHEMA_COMPATIBILITY.md` records the current canonical schema versions and migration policy. v0.5 qualification evidence is kept outside canonical `summary.json`/`provenance.json` unless a separately reviewed schema change is made.

## Signed release material

Permanent hardening CI can generate checksums but cannot truthfully pre-sign a release artifact before the release commit/tag exists. v0.5 publication must therefore add cryptographic provenance/signature/attestation over the exact release artifact/checksum set, verify it, and record its identity before the v0.5 release gate is closed.

## Evidence identities

Populate after the exact candidate is validated:

- implementation/evidence SHA: **pending**
- V0.5 Hardening run/job: **pending**
- Permanent CI run/job: **pending**
- any additional release-gate runs: **pending**
- generated SBOM SHA-256: **pending**
- generated license inventory SHA-256: **pending**
- reproducible release binary SHA-256: **pending**
- fuzz campaign result: **pending**
- sanitizer result: **pending**
- advisory/license/source result: **pending**

No pending item may be interpreted as passed.
