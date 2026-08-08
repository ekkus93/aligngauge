# AlignGauge v0.5 hardening evidence

**State:** M15 hardening implementation validated. The exact hardening evidence SHA `0e14af01c2f218aaca371c414133403e8e88c96d` and later integration head `d1e01b7d7064368179350471c050fce44cdbb377` both passed the complete permanent regression/hardening matrix. Signed/attested release material remains intentionally pending until the exact release-candidate/publication boundary. Milestone 14 full-scale HG002 qualification remains a separate release blocker.

## Hardening contract

ADR-0012 makes these failures release-blocking rather than warning-only:

1. BED parser fuzz failure;
2. raw BAM CIGAR coverage-boundary fuzz failure;
3. atomic-output fault-injection regression;
4. ASan/LeakSanitizer HTS/native-boundary failure;
5. release-blocking dependency advisory;
6. unapproved or unknown dependency license/source;
7. SBOM/license inventory generation or determinism failure;
8. release-artifact reproducibility mismatch;
9. missing schema migration/compatibility documentation;
10. missing signed/attested release checksum evidence at publication.

The permanent workflow does not use `continue-on-error`, `|| true`, or a best-effort fallback for these gates.

## Validated implementation candidate

Implementation candidate branch SHA:

`7d1264f4a1a553ff5d2c14315c51c91e0db05864`

V0.5 Hardening Validation run `31265386802` succeeded:

- preflight/fault injection job `93122570606`
- ASan/LeakSanitizer job `93122570568`
- fuzz job `93122570575`
- supply-chain/reproducibility job `93122570591`
- aggregate hardening job `93122984606`

Permanent CI run `31265386800`, job `93122570470`, also succeeded. Full Runtime, Reference, Targeted, Samtools Stats, Picard, MultiQC, and the existing v0.4 release gate were green on the same source candidate.

The supply-chain machine report records the PR merge execution SHA `f3c17499a7b5b05f058fa018ad15bee5737733dc`; normal pull-request Actions execute the merge ref while GitHub records the source branch head separately.

## Exact evidence-head validation

Evidence SHA:

`0e14af01c2f218aaca371c414133403e8e88c96d`

Every workflow triggered by that exact evidence state succeeded:

- Permanent CI run `31265637151`
- Full Runtime Validation run `31265637159`
- Reference Validation run `31265637146`
- Targeted Validation run `31265637160`
- Samtools Stats Validation run `31265637185`
- Picard Validation run `31265637142`
- MultiQC Validation run `31265637154`
- V0.4 Release Validation run `31265637145`
- V0.5 Hardening Validation run `31265637214`

The hardening run succeeded in all independent jobs:

- preflight/fault injection `93123185940`
- ASan/LeakSanitizer `93123185942`
- fuzz `93123185926`
- supply-chain/reproducibility `93123185929`
- aggregate gate `93123601204`

## Final integration-head validation

Integration head:

`d1e01b7d7064368179350471c050fce44cdbb377`

All ten triggered workflows succeeded before this evidence wording was closed:

- Permanent CI `31265902632`
- Full Runtime Validation `31265902601`
- Reference Validation `31265902606`
- Targeted Validation `31265902661`
- Samtools Stats Validation `31265902633`
- Picard Validation `31265902605`
- MultiQC Validation `31265902618`
- Exact Overlap Validation `31265902598`
- V0.4 Release Validation `31265902646`
- V0.5 Hardening Validation `31265902628`

Final-head hardening jobs were also all green: ASan `93123916083`, preflight `93123916092`, fuzz `93123916094`, supply-chain `93123916098`, aggregate `93124281845`.

## Fuzzing

The committed `fuzz/` package is outside the production Cargo workspace and contains two libFuzzer targets:

- `bed_parser` feeds arbitrary bytes to `aligngauge_formats::parse_bed_bytes` against a fixed valid sequence dictionary;
- `cigar_blocks` feeds arbitrary raw BAM CIGAR words plus arbitrary start/reference lengths to `aligngauge_coverage::cigar_to_coverage_blocks`.

Both completed 20,000 libFuzzer runs successfully. A panic, sanitizer finding, abort, or crash is fatal. The validated implementation-candidate fuzz artifact digest was `sha256:27fac43386bf3272322f99237591662663329e4ccba95500dcc8b809b2c538a7`.

## Atomic output fault injection

The hardening preflight reruns the production atomic-publication tests, including fail-closed cleanup at every pre-rename checkpoint, no observer-visible partial destination, `_FAILED` without `_SUCCESS` for preserved failures, destination non-overwrite, and reserved/path-like output-name rejection.

## ASan/LeakSanitizer finding and mitigation

The first sanitizer campaign found a real native-resource leak on the committed `truncated_bgzf.bam` fixture. `rust-htslib` 1.0.1 opened an `htsFile*`, failed `sam_hdr_read()`, and returned before a high-level `Reader` existed to close the raw file. LeakSanitizer reported 135,641 bytes retained in eight allocations.

Isolation proved that ordinary valid/header/coordinate/tag paths and four other corrupt/error fixtures were leak-clean; only the truncated-BGZF malformed-header path reproduced the leak. Current upstream retained the same ownership behavior.

AlignGauge therefore added the private `aligngauge-hts-ffi` crate as the single audited unsafe ownership boundary. Normal AlignGauge crates still inherit `unsafe_code = "forbid"`. The shim owns the temporary `htsFile*` and `sam_hdr_t*` with RAII guards behind a safe `preflight_header()` API, releasing every acquired native resource on both success and error before the unsafe-free `aligngauge-hts` crate constructs its ordinary high-level reader.

External dependency package identities did not move; `Cargo.lock` gained only the new local workspace package/dependency edge. The complete HTS suite is now green under the dedicated `x86_64-unknown-linux-gnuasan` target with LeakSanitizer still enabled, making the formerly leaking path a permanent regression check rather than an ignored defect.

## Dependency/security policy

`deny.toml` is enforced with pinned `cargo-deny 0.20.2`. Advisories, licenses, bans, and package sources all pass under the committed policy.

The pinned `rust-htslib` graph contains transitive `custom_derive 0.1.7`, associated with informational unmaintained advisory `RUSTSEC-2025-0058`. No advisory ID is ignored. Vulnerability/unsoundness findings remain fatal; an unmaintained direct workspace dependency is fatal; the transitive maintenance debt remains an explicit known limitation. Registry/git wildcard dependencies remain denied.

## SBOM, license inventory, and reproducibility

`tools/v0.5/generate-sbom.py` uses `cargo metadata --locked` to generate deterministic CycloneDX 1.5 JSON plus deterministic package/license/source inventory JSON. The workflow generates each twice and byte-compares them.

Validated implementation-candidate identities:

- CycloneDX SBOM SHA-256: `cf0d124384737bc2bd7a87c814dbde0db68037a24a0adebe06f0b23eff2ee1da`
- license inventory SHA-256: `cf1d2ad8c3ca78eb8d83fcdd0435fe9afa13384e5147ea440a959cbc23fa7400`
- Linux `aligngauge` binary SHA-256: `5d95f1a857dcdc1972bb839bae0fab297c9740aaa2533376f7cdcf326c7cd609`
- `aligngauge-v0.5.0-linux-x86_64.tar.gz` SHA-256: `67f255611947463a0b6d5b5b8a7688c23b00499ed122579a2c0013e0117c0218`
- supply-chain evidence artifact ZIP digest: `sha256:7cf2b827533ff8703455a69209fe08d46c21e758283d73e8bb00026e65e06fb4`

The first two-build assessment correctly failed because separate random staging-derived Cargo/native build roots made the native release binary/archive nondeterministic. The packager now uses a fixed `target/v0.5-release-build` root, cleans it before each sequential build, fixes `SOURCE_DATE_EPOCH`, remaps Rust and C/C++ source paths, and normalizes packaged timestamps, ownership, ordering, tar format, and gzip metadata. Two clean builds are now byte-identical.

This reproducibility claim is limited to the named Linux/pinned-toolchain build environment.

## Schema compatibility

`docs/SCHEMA_COMPATIBILITY.md` records the compatibility and migration policy. v0.5 hardening did not change canonical schema versions:

- summary schema `1.1.0`;
- provenance schema `1.0.0`.

## Signed release material

The hardening workflow generates reproducible artifacts and checksums but does not pretend that a pre-release CI artifact is a signed release artifact. The eventual exact release candidate must create cryptographic provenance/signature/attestation over the exact release artifact/checksum set, verify it, and record its identity before the signed-artifact checklist item can close.

This item remains **pending by design**.

## Remaining v0.5 blockers

M15 hardening implementation has no remaining fuzz/security/reproducibility blocker, but `v0.5.0` is not releasable yet:

1. Milestone 14 full ~30× whole-genome HG002 qualification has not been executed; `V0_5_FULL_HG002_REPORT.md` remains `BLOCKED`;
2. signed/attested release checksums/artifacts remain an exact-release-candidate/publication item;
3. after M14 closes, the eventual exact release candidate must repeat all required permanent gates before any tag exists.

No v0.5 tag or GitHub release has been created.

No pending item may be interpreted as passed.
