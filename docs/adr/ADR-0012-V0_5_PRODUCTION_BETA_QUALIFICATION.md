# ADR-0012 — v0.5 production-beta qualification

**Status:** Accepted  
**Date:** 2026-08-08

## Context

AlignGauge v0.5 is the production-beta qualification release. Earlier releases prove correctness on deterministic synthetic fixtures and a small HG002 chr20 subset, but v0.5 additionally requires a real whole-genome workload at approximately 30× depth, repeated performance measurements, release hardening, dependency/security evidence, and signed release artifacts.

The existing HG002 source is the GIAB GRCh38-GIABv3 Element AVITI 81× BAM. Its whole-file BAM is approximately 118 GB and the repository's standing HG002 CI intentionally avoids downloading it by using indexed remote access for a one-megabase chr20 subset. That small subset is not acceptable evidence for Milestone 14.

## Decision

### Full-scale HG002 profile

Milestone 14 uses exactly this source profile:

- source: `HG002_GRCh38-GIABv3_Element-StdInsert_2X150_81x_20231018.bam`
- source BAM MD5: `f5360b7adbc798c90a78f290de928eca`
- source BAI MD5: `1d7fd88891eee203c02fb852cac95301`
- reference build: `GRCh38-GIABv3`
- source nominal depth: 81×
- target nominal depth: approximately 30×
- deterministic subsample seed: `42`
- deterministic subsample fraction: `0.37037037037037`
- region: whole alignment; no interval restriction is permitted

`testdata/hg002/prepare-full-wgs.sh` requires a locally provisioned copy of the complete source BAM and BAI and verifies the pinned upstream MD5 values before downsampling. It does not implicitly download the 118 GB source. The prepared 30× BAM, index, exact SHA-256 values, tool image, source identity, parameters, and byte sizes are captured in a manifest before `_SUCCESS` is written.

A missing source, checksum mismatch, insufficient free space, pre-existing destination, failed Samtools command, missing index, or failed quickcheck is fatal. There is no fallback to the small chr20 fixture, remote partial access, a different HG002 technology, another reference build, or a lower-depth sample.

### Qualification execution

The full campaign is a maintainer-operated qualification, not ordinary GitHub-hosted CI. Ordinary CI validates the scripts, hardening controls, and small deterministic smoke paths. Full-scale evidence is accepted only from the explicit whole-genome campaign.

The campaign must record:

- exact AlignGauge commit and clean-tree state;
- exact prepared BAM/index SHA-256 and manifest identity;
- CPU, RAM, OS, kernel, filesystem, storage mount, and free/total space;
- cold/warm-cache disposition;
- configured and effective thread counts;
- wall time, user CPU time, system CPU time, peak RSS, logical bytes read, physical bytes read where Linux exposes them, and output sizes;
- at least one warm-up plus at least three measured runs for each released execution configuration;
- serial reader mode (`--io-threads 0`);
- released I/O-parallel reader mode (`--io-threads 2` unless a later ADR changes the released contract);
- exact canonical `summary.json` equivalence across all measured runs and both modes;
- pinned reference-tool differential results for every released compatibility/canonical claim exercised by whole-genome BAM input.

Collector execution remains serial. `--threads >1` is not admitted as a second collector implementation and must not be represented as parallel execution.

### Full-scale differential boundary

The whole-genome campaign rechecks the released v0.5 claim surface against the same pinned references used by permanent CI:

- Samtools 1.24 `flagstat`;
- Samtools 1.24 `idxstats`;
- canonical coverage via Samtools 1.24 `depth -aa` under the `aligngauge-v0.1` policy, reduced as a stream rather than materializing whole-genome depth text;
- Samtools Stats profile `samtools-stats-1.24-multiqc-1.35`;
- Picard 3.4.0 AlignmentSummary released subset;
- Picard 3.4.0 InsertSize released profile;
- pinned MultiQC 1.35 for the generated profiles for which v0.4 already makes a MultiQC claim.

No new WgsMetrics, HsMetrics, fold-80, indexed-partition, or collector-parallel compatibility claim is created by v0.5 merely because full-scale qualification runs.

### Hardening boundary

Milestone 15 is independently executable in ordinary CI. It includes:

- libFuzzer campaigns for the BED parser and raw BAM CIGAR coverage boundary;
- deterministic atomic-output fault injection;
- AddressSanitizer and LeakSanitizer coverage of the Rust/native HTS boundary where the pinned runner supports it;
- dependency advisory and license checks;
- deterministic license inventory and CycloneDX SBOM generation;
- a two-build reproducibility assessment;
- schema compatibility/migration documentation;
- release artifact checksums plus cryptographic provenance/signature at publication.

A failed fuzz target, sanitizer failure, release-blocking advisory, disallowed/unknown license, SBOM generation failure, reproducibility mismatch, missing release signature/attestation, or release-blocking security finding blocks v0.5. It may not be downgraded to a warning to obtain a release.

### Audited native ownership shim

Normal AlignGauge crates continue to inherit `unsafe_code = "forbid"`. v0.5 does not relax that application-wide policy.

LeakSanitizer exposed one upstream `rust-htslib` 1.0.1 malformed-header ownership defect: when `sam_hdr_read()` returns null, its BAM reader constructor returns before a high-level `Reader` exists and therefore never closes the already opened `htsFile*`. The committed `truncated_bgzf.bam` fixture reproduces that path.

Because current upstream `rust-htslib` retains the same constructor behavior, AlignGauge isolates the mitigation in one private crate, `aligngauge-hts-ffi`. That crate is the only v0.5 exception to the workspace unsafe-code lint. Its entire purpose is to own the small raw HTSlib header-open boundary behind a safe `preflight_header()` API:

- an `htsFile*` returned by `hts_open()` is immediately owned by an RAII guard;
- a successful `sam_hdr_read()` result is immediately owned by a header RAII guard;
- every error path destroys/closes resources already acquired;
- successful preflight destroys the temporary header and requires `hts_close()` to return success;
- only after that succeeds does the unsafe-free `aligngauge-hts` crate construct the ordinary high-level reader.

The external dependency versions do not move for this mitigation. `Cargo.lock` adds only the new local workspace package and dependency edge. Permanent LeakSanitizer coverage remains enabled; the mitigation is accepted only if the previously leaking truncated-BGZF path and the complete HTS validation suite are leak-clean.

### Dependency advisory policy

`cargo-deny` remains fail-closed for vulnerability, unsoundness, and other release-blocking advisories. Unknown package sources and licenses outside the committed allowlist are also fatal.

The pinned `rust-htslib` dependency graph contains `custom_derive 0.1.7`, covered by `RUSTSEC-2025-0058`. That advisory is informational and classifies the crate as unmaintained; it does not publish a patched version. The committed policy therefore uses `unmaintained = "workspace"`:

- an unmaintained **direct workspace dependency** is release-blocking;
- transitive unmaintained maintenance debt remains visible and must be named in release evidence, but is not reclassified as a vulnerability;
- no advisory ID is blanket-ignored.

Private workspace path dependencies may omit versions (`allow-wildcard-paths = true`); registry and git wildcard dependencies remain denied.

## Consequences

v0.5 cannot be declared complete from standard GitHub-hosted CI alone. The full ~30× HG002 campaign requires explicitly provisioned storage and a local/maintainer execution environment capable of holding and repeatedly reading the prepared whole-genome BAM.

The production Rust code remains unsafe-free outside the single audited private FFI ownership shim. Sanitizer coverage is a permanent release gate, not a best-effort diagnostic.

This is intentional. Resource limitations and hardening findings are evidence boundaries, not reasons to silently replace the required workload or suppress a failing control.
