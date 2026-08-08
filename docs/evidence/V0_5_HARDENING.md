# AlignGauge v0.5 hardening evidence

**State:** validated implementation candidate; exact evidence-head validation pending. All permanent M15 hardening implementation gates are green. Signed/attested release material remains intentionally deferred to the exact release candidate/publication boundary, and Milestone 14 full-scale HG002 qualification remains a separate release blocker.

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

The workflow fails nonzero for these conditions. It does not turn them into `continue-on-error`, `|| true`, best-effort steps, or advisory-only output.

## Validated implementation candidate

Branch candidate SHA:

`7d1264f4a1a553ff5d2c14315c51c91e0db05864`

The pull-request workflow executed the synthetic PR merge commit `f3c17499a7b5b05f058fa018ad15bee5737733dc`, which is recorded by the machine-readable supply-chain report. The branch head itself is the source candidate; exact commit validation is repeated after merge on `master` before any v0.5 release can be qualified.

V0.5 Hardening Validation run `31265386802` completed successfully:

- `ci/v0.5-hardening-preflight` job `93122570606` — success
- `ci/v0.5-hardening-asan` job `93122570568` — success
- `ci/v0.5-hardening-fuzz` job `93122570575` — success
- `ci/v0.5-hardening-supply-chain` job `93122570591` — success
- aggregate `ci/v0.5-hardening` job `93122984606` — success

The same candidate also passed the complete existing regression surface:

- Permanent CI run `31265386800`, job `93122570470` — success
- Full Runtime Validation run `31265386793` — success
- Reference Validation run `31265386804` — success
- Targeted Validation run `31265386790` — success
- Samtools Stats Validation run `31265386801` — success
- Picard Validation run `31265386817` — success
- MultiQC Validation run `31265386813` — success
- V0.4 Release Validation run `31265386822` — success

No v0.5 tag or release exists from this evidence.

## Fuzzing

The committed `fuzz/` package is intentionally outside the production Cargo workspace and contains two libFuzzer targets:

- `bed_parser`: arbitrary bytes enter `aligngauge_formats::parse_bed_bytes` against a fixed valid sequence dictionary;
- `cigar_blocks`: arbitrary raw BAM CIGAR words plus arbitrary start/reference lengths enter `aligngauge_coverage::cigar_to_coverage_blocks`.

On the validated candidate:

- BED parser: 20,000 libFuzzer runs — success;
- raw CIGAR coverage boundary: 20,000 libFuzzer runs — success.

Expected typed parse/validation errors are allowed. A panic, sanitizer finding, abort, or crash is a campaign failure. The fuzz evidence artifact digest is `sha256:27fac43386bf3272322f99237591662663329e4ccba95500dcc8b809b2c538a7`.

## Output fault injection

`aligngauge-core` exposes deterministic publication checkpoints and tests every pre-rename checkpoint. The v0.5 hardening workflow reruns the complete atomic-publication test module, including:

- no partial destination visible to observers;
- fail-closed cleanup at every pre-rename checkpoint;
- preserved failures marked `_FAILED` with no `_SUCCESS`;
- existing destinations never overwritten;
- reserved/path-like output names rejected.

The exact preflight/fault-injection job succeeded. This reuses the production publication implementation rather than maintaining a separate fault-injection simulation.

## ASan/LeakSanitizer finding and mitigation

The first v0.5 sanitizer campaign exposed a real native-resource leak that prior correctness tests did not detect. The committed `truncated_bgzf.bam` fixture caused `rust-htslib` 1.0.1 to open an `htsFile*`, fail `sam_hdr_read()`, and return before a high-level `Reader` existed to close the raw file. LeakSanitizer reported 135,641 bytes retained in eight allocations.

Diagnostic isolation proved:

- all ordinary valid/header/coordinate/tag paths were leak-clean;
- four other corrupt/error fixtures were leak-clean;
- only the truncated-BGZF malformed-header path reproduced the leak.

Current upstream `rust-htslib` retained the same constructor ownership behavior, so the mitigation does not hide the leak or silently upgrade dependencies. Instead, AlignGauge added the private `aligngauge-hts-ffi` crate as the single audited unsafe ownership boundary. Normal AlignGauge crates continue to inherit `unsafe_code = "forbid"`.

The shim owns the temporary `htsFile*` and `sam_hdr_t*` with RAII guards behind a safe `preflight_header()` API. On malformed input it destroys/closes every acquired native resource before returning. On valid input it closes the temporary preflight and only then constructs the ordinary high-level reader.

The external dependency package identities in `Cargo.lock` did not move; only the new local workspace package/dependency edge was added.

On the validated candidate, the complete HTS test suite passed under the dedicated `x86_64-unknown-linux-gnuasan` target with LeakSanitizer still enabled. The previously leaking truncated-BGZF path is therefore a permanent sanitizer regression test rather than an ignored upstream defect.

## Dependency/security policy

The repository commits `deny.toml` and pins `cargo-deny 0.20.2` in the hardening workflow. On the validated candidate:

- advisories — pass;
- licenses — pass;
- bans — pass under the committed duplicate-version policy;
- sources — pass.

The pinned `rust-htslib` graph contains transitive `custom_derive 0.1.7`, associated with informational unmaintained advisory `RUSTSEC-2025-0058`. No advisory ID is ignored. The committed policy keeps vulnerability/unsoundness findings fatal and makes unmaintained direct workspace dependencies fatal while retaining this transitive maintenance debt as an explicit known limitation.

Private workspace path dependencies may omit versions; registry/git wildcard dependencies remain denied.

## SBOM and license inventory

`tools/v0.5/generate-sbom.py` uses `cargo metadata --locked` to generate:

- deterministic CycloneDX 1.5 JSON;
- deterministic package/license/source inventory JSON.

The workflow generated both twice and byte-compared the results successfully.

Exact generated identities from the validated candidate:

- CycloneDX SBOM SHA-256: `cf0d124384737bc2bd7a87c814dbde0db68037a24a0adebe06f0b23eff2ee1da`
- license inventory SHA-256: `cf1d2ad8c3ca78eb8d83fcdd0435fe9afa13384e5147ea440a959cbc23fa7400`
- supply-chain evidence artifact ZIP digest: `sha256:7cf2b827533ff8703455a69209fe08d46c21e758283d73e8bb00026e65e06fb4`

## Reproducible-build assessment

The initial assessment correctly failed because each release build used a different random staging-derived Cargo/native build directory. Although repository source paths were remapped, the native dependency build path could still affect the resulting binary/archive.

The release packager now:

1. uses one fixed deterministic absolute build root, `target/v0.5-release-build`;
2. removes that build root before each sequential clean build;
3. fixes `SOURCE_DATE_EPOCH` to the exact commit timestamp;
4. remaps repository paths for Rust and C/C++ compilation;
5. normalizes packaged mtimes, ownership, order, tar format, and gzip metadata;
6. emits binary/content/archive checksums.

With that correction, the two independent clean release builds were byte-identical and the reproducibility assessment passed.

Exact candidate artifact identities:

- Linux `aligngauge` binary SHA-256: `5d95f1a857dcdc1972bb839bae0fab297c9740aaa2533376f7cdcf326c7cd609`
- `aligngauge-v0.5.0-linux-x86_64.tar.gz` SHA-256: `67f255611947463a0b6d5b5b8a7688c23b00499ed122579a2c0013e0117c0218`

This is a reproducibility claim for the named Linux/pinned-toolchain build environment only. It is not a claim that arbitrary platforms or compilers produce identical binaries.

## Schema compatibility

`docs/SCHEMA_COMPATIBILITY.md` records the current canonical schema versions and migration policy. v0.5 qualification evidence remains outside canonical `summary.json`/`provenance.json` unless a separately reviewed schema change is made.

The validated candidate did not change the canonical schema versions:

- summary schema `1.1.0`;
- provenance schema `1.0.0`.

## Signed release material

Permanent hardening CI can generate reproducible artifacts and checksums but cannot truthfully pre-sign or attest a release artifact before the exact release commit/tag boundary exists. v0.5 publication must therefore add cryptographic provenance/signature/attestation over the exact release artifact/checksum set, verify it, and record its identity before the signed-artifact checklist item and v0.5 release gate can close.

This item is still **pending by design**. It is not waived by the green hardening workflow.

## Remaining v0.5 blockers

The hardening implementation is green, but v0.5 is not releasable yet:

1. Milestone 14 full ~30× whole-genome HG002 qualification has not been executed and `V0_5_FULL_HG002_REPORT.md` remains `BLOCKED`;
2. signed/attested release checksums/artifacts must be produced and verified on the eventual exact release candidate;
3. the exact evidence/merge/master release-candidate SHA must repeat all required permanent gates before a tag can exist.

No pending item may be interpreted as passed.
