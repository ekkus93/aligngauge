# Rust DNA Sequencer v0.1 Implementation TODO

**Repository:** `ekkus93/rust-dna-sequencer`  
**Companion specification:** `docs/DNA_QC_ENGINE_SPEC.md`  
**Status:** Initial implementation plan  
**Last updated:** 2026-08-05

## How to use this TODO

This document is the implementation contract for v0.1. Work should proceed in milestone order unless a later task is explicitly pulled forward to unblock validation.

A checkbox may be marked complete only when:

1. implementation is present;
2. required tests are present and passing;
3. documentation is updated;
4. required evidence is recorded;
5. no known silent fallback or swallowed failure remains;
6. the exact commit being claimed has passed the relevant permanent CI gates.

Do not mark a parent item complete while any required child item is incomplete.

## Global engineering rules

These apply to every milestone.

- [ ] Treat the CPU implementation as the correctness reference.
- [ ] Keep CUDA optional at compile time and runtime.
- [ ] Never silently substitute an approximate algorithm for an exact one.
- [ ] Never catch an error merely to emit a warning and continue with fabricated, empty, partial, or default metrics.
- [ ] Never emit zero as a substitute for unavailable data.
- [ ] Never mark an output directory successful unless all required outputs were completed and synchronized.
- [ ] Use checked arithmetic for coordinates, offsets, lengths, counters, and allocation sizes.
- [ ] Keep unsafe code and FFI inside narrowly scoped audited boundaries.
- [ ] Avoid per-record allocation and formatting in hot loops.
- [ ] Require tests for every bug fixed and every compatibility exception introduced.
- [ ] Record deliberate compatibility differences instead of hiding them with loose tolerances.
- [ ] Pin tool versions used to generate golden outputs.
- [ ] Preserve deterministic result ordering and deterministic parallel reduction.
- [ ] Do not commit large public genomics datasets to Git.
- [ ] Do not make clinical or diagnostic claims.

---

# Milestone 0 — Repository foundation and project controls

## 0.1 Workspace bootstrap

- [ ] Create the top-level Cargo workspace.
- [ ] Add initial crates:
  - [ ] `rds-cli`
  - [ ] `rds-core`
  - [ ] `rds-hts`
  - [ ] `rds-metrics`
  - [ ] `rds-coverage`
  - [ ] `rds-targets`
  - [ ] `rds-output`
  - [ ] `rds-test-support`
- [ ] Add `rds-gpu` as an optional crate or documented placeholder without making CUDA a default dependency.
- [ ] Add `Cargo.lock`.
- [ ] Add `rust-toolchain.toml` with a pinned channel or exact version after confirming dependency compatibility.
- [ ] Define the minimum supported Rust version in workspace metadata.
- [ ] Add workspace-wide package metadata, license, repository, authorship, and edition.
- [ ] Configure release profiles with measured settings only; do not add speculative unsafe compiler flags.

## 0.2 Code-quality baseline

- [ ] Add `rustfmt` configuration only where deviation from defaults is justified.
- [ ] Add workspace Clippy policy with warnings denied in CI.
- [ ] Ban or review high-risk lint suppressions.
- [ ] Add `cargo nextest` or standard test execution policy.
- [ ] Add dependency vulnerability scanning.
- [ ] Add dependency license checking.
- [ ] Add supply-chain policy for Git dependencies and unpinned actions.
- [ ] Document rules for `unsafe` blocks and FFI wrappers.

## 0.3 Permanent CI

- [ ] Add a permanent CI workflow for:
  - [ ] formatting;
  - [ ] strict Clippy;
  - [ ] debug build;
  - [ ] release build;
  - [ ] unit tests;
  - [ ] integration tests;
  - [ ] documentation tests;
  - [ ] JSON schema tests once available;
  - [ ] dependency audit;
  - [ ] license check;
  - [ ] fuzz-target compilation once available.
- [ ] Pin every GitHub Action by immutable commit SHA.
- [ ] Set least-privilege workflow permissions.
- [ ] Set `persist-credentials: false` for read-only checkouts.
- [ ] Prevent template injection through untrusted values in shell scripts.
- [ ] Add concurrency controls without cancelling release publication midway.
- [ ] Document required status checks.

## 0.4 Repository documentation

- [ ] Replace the starter README with:
  - [ ] precise project scope;
  - [ ] prominent statement that v0.1 is not physical sequencer control or basecalling;
  - [ ] build instructions;
  - [ ] initial CLI example;
  - [ ] link to specification and TODO;
  - [ ] development status and non-clinical disclaimer.
- [ ] Add `CONTRIBUTING.md`.
- [ ] Add architecture decision record template.
- [ ] Add security-reporting policy.
- [ ] Add changelog.

### Milestone 0 acceptance evidence

- [ ] Fresh checkout builds on the primary Linux development environment.
- [ ] Permanent CI passes on the exact milestone commit.
- [ ] No CUDA toolkit is required for the default build.
- [ ] Repository documentation accurately describes current capabilities rather than planned capabilities.

---

# Milestone 1 — Core types, configuration, and process lifecycle

## 1.1 Canonical error model

- [ ] Define typed error categories in `rds-core`.
- [ ] Include errors for:
  - [ ] input open/read/decode;
  - [ ] malformed header;
  - [ ] unsupported input feature;
  - [ ] unsorted input;
  - [ ] missing/incompatible index;
  - [ ] reference mismatch;
  - [ ] invalid target intervals;
  - [ ] arithmetic overflow;
  - [ ] memory/resource limit;
  - [ ] collector failure;
  - [ ] exporter failure;
  - [ ] output publication failure;
  - [ ] CUDA failures when enabled.
- [ ] Preserve source error chains without exposing sensitive read content by default.
- [ ] Assign stable process exit classes.
- [ ] Add unit tests proving no required error is converted to a successful exit.

## 1.2 Configuration model

- [ ] Define strongly typed configuration structures.
- [ ] Implement configuration precedence:
  - [ ] built-in defaults;
  - [ ] config file;
  - [ ] explicitly supported environment variables;
  - [ ] CLI arguments.
- [ ] Reject unknown configuration keys by default.
- [ ] Validate incompatible option combinations before processing.
- [ ] Add named profiles:
  - [ ] `wgs`
  - [ ] `targeted`
  - [ ] `custom`
- [ ] Add backend selection:
  - [ ] `auto`
  - [ ] `cpu`
  - [ ] `cuda`
- [ ] Add explicit memory budget configuration.
- [ ] Serialize the final resolved configuration for provenance.
- [ ] Add round-trip configuration tests.

## 1.3 Analysis and execution plans

- [ ] Define `AnalysisPlan`.
- [ ] Define `RequiredFields`.
- [ ] Define `RecordFilter`.
- [ ] Define `BaseFilter`.
- [ ] Define `MetricPlan`.
- [ ] Define `CoveragePlan`.
- [ ] Define `TargetPlan`.
- [ ] Define `ExecutionPlan`.
- [ ] Define `OutputPlan`.
- [ ] Make plans immutable after validation.
- [ ] Ensure compatibility exporters declare their required source metrics.
- [ ] Fail planning if an exporter cannot be satisfied.
- [ ] Add planner unit tests for representative WGS, WES, CPU, auto, and explicit-CUDA configurations.

## 1.4 Atomic process lifecycle

- [ ] Create output staging directory logic.
- [ ] Ensure staging and destination are on the same filesystem when atomic rename is required.
- [ ] Write outputs only through managed staging paths.
- [ ] Add `_SUCCESS` only after every required output is flushed and published.
- [ ] Remove temporary files on ordinary failure.
- [ ] Add explicit debug option to preserve failed staging output.
- [ ] Test exporter failure halfway through a run.
- [ ] Test disk-full behavior where practical.
- [ ] Test interruption cleanup.
- [ ] Confirm incomplete output never carries `_SUCCESS`.

### Milestone 1 acceptance evidence

- [ ] Invalid configurations fail before input traversal.
- [ ] Output failure cannot produce a successful process status.
- [ ] Resolved configuration and execution plan are serializable and deterministic.
- [ ] Permanent CI passes on the exact milestone commit.

---

# Milestone 2 — Test-data framework and reference baselines

This milestone intentionally precedes most analysis implementation. Correctness fixtures and differential tooling are specifications in executable form.

## 2.1 Test-data manifest

- [ ] Add `testdata/README.md`.
- [ ] Add a versioned `testdata/manifest.toml` schema.
- [ ] Record for every dataset:
  - [ ] identifier;
  - [ ] sample;
  - [ ] sequencing type;
  - [ ] reference build;
  - [ ] source URL/accession;
  - [ ] published checksum when available;
  - [ ] local SHA-256;
  - [ ] extraction coordinates;
  - [ ] downsampling seed and fraction;
  - [ ] generation commands;
  - [ ] tool versions;
  - [ ] redistribution status.
- [ ] Add manifest validation tests.
- [ ] Refuse to use a downloaded dataset whose checksum does not match the manifest.

## 2.2 Synthetic fixture generator

- [ ] Add auditable reference FASTA and SAM source fixtures.
- [ ] Generate deterministic BAM, BAI/CSI, CRAM, CRAI, and expected metadata.
- [ ] Cover:
  - [ ] match/equal/mismatch CIGAR operations;
  - [ ] insertions;
  - [ ] deletions;
  - [ ] reference skips;
  - [ ] soft clips;
  - [ ] hard clips;
  - [ ] pads where supported;
  - [ ] overlapping mates;
  - [ ] unmapped reads and mates;
  - [ ] secondary alignments;
  - [ ] supplementary alignments;
  - [ ] records with both secondary and supplementary flags;
  - [ ] duplicates;
  - [ ] QC-fail records;
  - [ ] multiple read groups/libraries;
  - [ ] missing and unknown read-group IDs;
  - [ ] missing NM/MD tags;
  - [ ] malformed optional tags where the file format permits creation;
  - [ ] extreme template lengths;
  - [ ] high-depth boundary cases;
  - [ ] coordinate regressions;
  - [ ] truncated compressed files;
  - [ ] reference mismatch;
  - [ ] BED overlap and invalid-coordinate cases.
- [ ] Commit the small generated fixtures only after confirming redistribution and size are appropriate.
- [ ] Add a command that regenerates every fixture from source.
- [ ] Verify regeneration is byte-stable where tools permit; otherwise verify semantic checksums.

## 2.3 Reference-tool containers

- [ ] Add pinned container or environment definitions for:
  - [ ] Samtools;
  - [ ] mosdepth;
  - [ ] Picard;
  - [ ] MultiQC;
  - [ ] optional Qualimap comparisons.
- [ ] Pin image digests, not floating tags.
- [ ] Record exact command lines used for every golden output.
- [ ] Add scripts that refuse to overwrite golden data without an explicit update flag.
- [ ] Require a generated difference report before golden outputs may be updated.

## 2.4 GIAB HG002 subset preparation

- [ ] Select and document the exact HG002 GRCh38 source alignment.
- [ ] Verify alignment header sequence names, lengths, and MD5 metadata.
- [ ] Select the exact matching reference FASTA.
- [ ] Create reproducible scripts for:
  - [ ] approximately 1 Mb chr20 subset;
  - [ ] 1× subset;
  - [ ] 10× subset;
  - [ ] 30× subset;
  - [ ] high-depth subset;
  - [ ] approximately 10 Mb 30× subset.
- [ ] Use fixed downsampling seeds.
- [ ] Validate generated BAM/CRAM with `samtools quickcheck` and explicit decoding.
- [ ] Do not commit large subsets; cache them through documented local paths or CI artifacts.

## 2.5 Exome/panel dataset

- [ ] Identify a public HG002 exome or target dataset.
- [ ] Locate the exact matching capture-target BED.
- [ ] Verify coordinate build and naming.
- [ ] Create a small reproducible subset for daily testing.
- [ ] Record target normalization and source provenance.
- [ ] Do not use a generic exome BED as a silent substitute.

### Milestone 2 acceptance evidence

- [ ] Synthetic corpus regenerates from documented source.
- [ ] HG002 small-data preparation is reproducible from a clean environment.
- [ ] Reference-tool versions and commands are pinned.
- [ ] Malformed fixtures are clearly separated from valid fixtures.
- [ ] Permanent CI validates the committed manifest and small fixture corpus.

---

# Milestone 3 — BAM/CRAM input and reference validation

## 3.1 HTSlib boundary

- [ ] Add `rust-htslib` behind `rds-hts`.
- [ ] Encapsulate raw record and pointer access.
- [ ] Define safe `AlignmentHeader` types.
- [ ] Define borrowed or batched record views.
- [ ] Reuse BAM record buffers.
- [ ] Ensure analysis crates do not depend directly on raw HTSlib pointers.
- [ ] Audit every unsafe operation and document invariants.

## 3.2 Header inspection

- [ ] Parse reference dictionary.
- [ ] Parse sort-order metadata.
- [ ] Parse read groups.
- [ ] Normalize sample/library/platform-unit identifiers into stable integer IDs.
- [ ] Detect duplicate or contradictory read-group definitions.
- [ ] Detect missing sample/library values without inventing them.
- [ ] Implement `inspect` output in human and JSON form.

## 3.3 Index handling

- [ ] Detect BAI, CSI, and CRAI.
- [ ] Distinguish workflows that require an index from streaming workflows that do not.
- [ ] Validate index compatibility before indexed execution.
- [ ] Detect stale or corrupt indexes where possible.
- [ ] Add tests for missing, incompatible, and corrupt index files.

## 3.4 Reference validation

- [ ] Read FASTA index.
- [ ] Compare sequence names and lengths.
- [ ] Compare sequence MD5 values where alignment metadata provides them.
- [ ] Validate CRAM decoding against the selected reference.
- [ ] Fail on mismatches capable of changing decoded sequence or coordinates.
- [ ] Produce actionable mismatch diagnostics.
- [ ] Avoid dumping full sequence data into errors.
- [ ] Implement `validate-reference` subcommand.

## 3.5 Coordinate-order validation

- [ ] Verify material coordinate monotonicity during traversal.
- [ ] Handle unmapped tail records correctly.
- [ ] Detect records assigned to invalid reference IDs.
- [ ] Fail on coordinate regression under v0.1 semantics.
- [ ] Add synthetic sorted/unsorted tests.

## 3.6 Batched input API

- [ ] Benchmark one-record-at-a-time versus batch normalization.
- [ ] Define bounded batches without copying unused fields.
- [ ] Make required-field selection visible to the input layer.
- [ ] Record decompression and decode timings separately.
- [ ] Add backpressure tests for bounded queues.

### Milestone 3 acceptance evidence

- [ ] Valid fixture BAM and CRAM traverse successfully.
- [ ] Truncated input fails with nonzero status and no success marker.
- [ ] CRAM reference mismatch fails closed.
- [ ] Unsorted input fails with exact offending coordinate context.
- [ ] Input traversal performs no unbounded allocation based solely on untrusted header values.

---

# Milestone 4 — Core alignment metrics

## 4.1 Record classification

- [ ] Define one authoritative flag-classification function.
- [ ] Explicitly define priority for primary, secondary, and supplementary categories.
- [ ] Correctly handle records with both secondary and supplementary bits.
- [ ] Partition QC-pass and QC-fail counts where compatibility requires it.
- [ ] Add exhaustive flag combination tests.
- [ ] Add invariants preventing double-counted mutually exclusive categories.

## 4.2 Flag counters

- [ ] Implement total records.
- [ ] Implement primary records.
- [ ] Implement secondary records.
- [ ] Implement supplementary records.
- [ ] Implement mapped/unmapped.
- [ ] Implement paired/properly paired.
- [ ] Implement read 1/read 2.
- [ ] Implement mate mapped/unmapped.
- [ ] Implement duplicates.
- [ ] Implement singleton counts.
- [ ] Implement per-reference mapped/unmapped counts.
- [ ] Implement read-group/library/sample partitions.
- [ ] Use checked counters.

## 4.3 Histograms

- [ ] Implement mapping-quality histogram.
- [ ] Implement read-length histogram.
- [ ] Implement aligned-query-length histogram.
- [ ] Implement reference-span histogram.
- [ ] Implement template-length/insert-size histogram.
- [ ] Implement clipping counts and distributions.
- [ ] Implement sequence GC histogram when sequence is requested.
- [ ] Implement base-quality histogram when quality is requested.
- [ ] Implement per-cycle base composition.
- [ ] Implement per-cycle quality distributions.
- [ ] Bound dynamic histogram expansion through validated limits.

## 4.4 Edit and mismatch metrics

- [ ] Define precedence among NM tag, MD tag, and reference-based derivation.
- [ ] Validate tag types before use.
- [ ] Never treat a missing tag as zero edits.
- [ ] Record derivation method in provenance.
- [ ] Add fixtures with absent, malformed, and inconsistent tags.
- [ ] Decide which metrics are unavailable when exact derivation cannot be performed.

## 4.5 Deterministic reduction

- [ ] Define merge methods for every accumulator.
- [ ] Ensure merge order is deterministic.
- [ ] Test serial versus multithreaded equivalence.
- [ ] Test different thread counts.
- [ ] Verify no integer overflow is hidden during merge.

## 4.6 Differential compatibility

- [ ] Implement first Samtools-like flagstat exporter.
- [ ] Implement first idxstats-like exporter.
- [ ] Implement selected Samtools stats sections.
- [ ] Pin comparison version.
- [ ] Compare every synthetic fixture.
- [ ] Compare HG002 small subsets.
- [ ] Document exact and non-exact fields.
- [ ] Do not loosen tolerances without a written root-cause analysis.

### Milestone 4 acceptance evidence

- [ ] All integer counters match the selected reference profile or have documented intentional differences.
- [ ] Dual secondary/supplementary fixture proves no accidental double count.
- [ ] Serial and multithreaded canonical JSON metrics are identical.
- [ ] Missing optional tags do not generate fabricated zeros.
- [ ] Permanent CI runs the small differential suite.

---

# Milestone 5 — Exact coverage engine

## 5.1 CIGAR reference-block emission

- [ ] Implement validated CIGAR traversal.
- [ ] Emit half-open reference blocks.
- [ ] Handle all supported CIGAR operations explicitly.
- [ ] Check coordinate addition for overflow.
- [ ] Reject blocks outside reference bounds.
- [ ] Add table-driven tests for every operation and mixed CIGAR.
- [ ] Add property tests for block-length conservation.

## 5.2 Difference-array accumulation

- [ ] Implement checked `delta[start] += 1` and `delta[end] -= 1` semantics.
- [ ] Choose signed delta representation with proven bounds.
- [ ] Implement whole-contig allocation strategy.
- [ ] Implement prefix scan to depth runs.
- [ ] Verify final delta balance.
- [ ] Fail on arithmetic overflow.
- [ ] Add exact handcrafted depth fixtures.

## 5.3 Chunked exact coverage

- [ ] Design chunk boundary carry state.
- [ ] Ensure reads crossing chunk boundaries are handled exactly.
- [ ] Implement bounded-memory chunk planner.
- [ ] Compare chunked and whole-contig output across every fixture.
- [ ] Test one-base and non-power-of-two chunk sizes.
- [ ] Test chunks smaller than long alignment spans.
- [ ] Record selected strategy and memory estimate in provenance.

## 5.4 Coverage filters

- [ ] Apply explicit flag filter.
- [ ] Apply minimum MAPQ.
- [ ] Implement duplicate-inclusive and duplicate-excluded profiles.
- [ ] Define deletion handling.
- [ ] Define reference-skip handling.
- [ ] Define secondary/supplementary handling.
- [ ] Define QC-fail handling.
- [ ] Add tests for every filter dimension.
- [ ] Include resolved semantics in canonical output.

## 5.5 Base-quality-filtered coverage

- [ ] Decide whether v0.1 requires base-level exact coverage or may defer it.
- [ ] If included, map reference positions to query positions through CIGAR exactly.
- [ ] Apply minimum base quality without counting deletions as bases.
- [ ] Validate clipped, inserted, deleted, and skipped segments.
- [ ] Benchmark cost independently.
- [ ] Never imply base-quality filtering was applied when only MAPQ filtering was performed.

## 5.6 Overlapping mate correction

- [ ] Define fragment overlap semantics.
- [ ] Implement pairing state with a bounded memory strategy.
- [ ] Handle supplementary/secondary records explicitly.
- [ ] Detect malformed or contradictory mate metadata.
- [ ] Compare against a pinned reference-tool profile.
- [ ] Fail or mark requested output unavailable when exact correction cannot be guaranteed.
- [ ] Do not silently count both mates under a requested overlap-corrected profile.

## 5.7 Coverage reductions

- [ ] Implement depth histogram.
- [ ] Implement cumulative threshold percentages.
- [ ] Implement mean depth.
- [ ] Implement median depth.
- [ ] Implement min/max depth.
- [ ] Implement covered and uncovered bases.
- [ ] Implement uncovered runs.
- [ ] Implement fixed-window means.
- [ ] Implement per-contig summaries.
- [ ] Use deterministic numeric reduction and formatting.

## 5.8 mosdepth-like compatibility

- [ ] Implement summary output.
- [ ] Implement global distribution output.
- [ ] Implement window/region output.
- [ ] Pin comparison version and semantics.
- [ ] Validate synthetic fixtures.
- [ ] Validate HG002 1 Mb and 10 Mb subsets.
- [ ] Document differences, especially overlap and filter semantics.

### Milestone 5 acceptance evidence

- [ ] Whole-contig and chunked modes are canonically identical.
- [ ] Exact depth matches handcrafted expectations at every base for synthetic fixtures.
- [ ] HG002 subset distributions match the pinned compatibility profile within documented rules.
- [ ] Memory-budget enforcement is tested.
- [ ] Overflow and impossible allocation requests fail cleanly.

---

# Milestone 6 — WES and targeted-panel metrics

## 6.1 BED parser and normalization

- [ ] Implement strict zero-based half-open BED parsing.
- [ ] Reject negative, reversed, overflowing, or non-numeric coordinates.
- [ ] Validate contig names.
- [ ] Sort deterministically.
- [ ] Merge intervals for territory calculations.
- [ ] Preserve original target identity for per-target output.
- [ ] Preserve optional target names.
- [ ] Report normalization actions.
- [ ] Add overlap, adjacency, duplicate, and malformed tests.

## 6.2 Target-focused coverage strategy

- [ ] Implement target-only memory planning.
- [ ] Map normalized target coordinates to compact arrays or run accumulators.
- [ ] Include configurable flanking territory.
- [ ] Handle reads spanning multiple targets.
- [ ] Avoid repeated interval-tree lookup when a sorted cursor can be used.
- [ ] Compare target-focused and whole-contig-derived metrics.

## 6.3 Core targeted metrics

- [ ] Calculate target territory.
- [ ] Calculate aligned bases.
- [ ] Calculate on-target bases.
- [ ] Calculate near-target bases.
- [ ] Calculate off-target bases.
- [ ] Calculate mean/median/min/max target depth.
- [ ] Calculate configured threshold percentages.
- [ ] Calculate zero-coverage bases and intervals.
- [ ] Calculate per-target summaries.
- [ ] Add optional per-gene aggregation only when a clear mapping file is supplied.
- [ ] Calculate duplicate-adjusted variants.

## 6.4 Enrichment and uniformity

- [ ] Define fold-enrichment formula and denominator.
- [ ] Define coverage-uniformity metric.
- [ ] Define fold-80 base penalty or documented equivalent.
- [ ] Ensure all formulas and units are written in documentation.
- [ ] Compare against Picard where semantics align.
- [ ] Record intentional differences where target normalization differs.

## 6.5 Target dropout reporting

- [ ] Emit zero-coverage target intervals.
- [ ] Emit targets below configured thresholds.
- [ ] Keep output ordering deterministic.
- [ ] Avoid loading unbounded human-readable labels into memory without limits.
- [ ] Include target source and reference identity in output.

## 6.6 Differential validation

- [ ] Pin Picard version and commands.
- [ ] Run synthetic targeted fixtures.
- [ ] Run exact HG002 exome/panel subset with matching target BED.
- [ ] Compare integer territories and base counts exactly where semantics align.
- [ ] Compare floating metrics with documented tolerances.
- [ ] Investigate every unexplained discrepancy.

### Milestone 6 acceptance evidence

- [ ] Target-only and whole-contig strategies agree.
- [ ] Matching capture BED is recorded and validated.
- [ ] Invalid coordinate conventions fail instead of being guessed.
- [ ] Picard comparison report documents every field.
- [ ] Target dropout outputs can be traced to canonical metrics.

---

# Milestone 7 — Canonical schema, provenance, and compatibility outputs

## 7.1 Canonical result model

- [ ] Define stable internal metric structures.
- [ ] Add `schema_version`.
- [ ] Add application version and Git commit.
- [ ] Add input/reference/target identities.
- [ ] Add filter definitions.
- [ ] Add metric definitions and units.
- [ ] Add sample/read-group/library hierarchy.
- [ ] Add execution plan.
- [ ] Add stage timings.
- [ ] Add warning and known-difference records.
- [ ] Ensure missing data is represented explicitly, not as zero.

## 7.2 JSON schema

- [ ] Generate or hand-maintain a JSON Schema.
- [ ] Version the schema.
- [ ] Add `schema` subcommand.
- [ ] Validate every end-to-end `summary.json` in tests.
- [ ] Add backward-compatibility tests once a second schema exists.
- [ ] Document schema evolution policy.

## 7.3 Provenance

- [ ] Record exact CLI arguments with sensitive path handling policy.
- [ ] Record resolved configuration.
- [ ] Record input size, metadata, and optional checksum mode.
- [ ] Record reference contigs and identity.
- [ ] Record target source and normalized territory.
- [ ] Record dependency/HTSlib versions.
- [ ] Record backend decisions and fallback reasons in `auto` mode.
- [ ] Record CPU model, thread counts, GPU device, driver/runtime versions when applicable.
- [ ] Record compatibility-profile versions.
- [ ] Record stage timings.
- [ ] Exclude raw sequence and read names.

## 7.4 Exporter framework

- [ ] Make exporters declare required metrics.
- [ ] Fail planning when required metrics are disabled.
- [ ] Prevent exporter-specific independent accumulation where canonical data suffices.
- [ ] Add golden-format tests.
- [ ] Add malformed-state tests proving exporters do not invent missing sections.

## 7.5 MultiQC compatibility

- [ ] Identify exact MultiQC parsers and filename patterns.
- [ ] Generate discoverable filenames.
- [ ] Run MultiQC over test outputs.
- [ ] Assert expected modules and sample names are discovered.
- [ ] Detect parser failure in CI rather than merely checking process exit.
- [ ] Document supported MultiQC version range or pinned validation version.

## 7.6 Deterministic files

- [ ] Stabilize map ordering.
- [ ] Stabilize numeric formatting.
- [ ] Separate volatile provenance from canonical metric equivalence.
- [ ] Add repeated-run byte comparison where appropriate.
- [ ] Add serial versus parallel output comparison.

### Milestone 7 acceptance evidence

- [ ] Canonical JSON validates against the checked-in schema.
- [ ] Compatibility outputs are generated solely when source metrics are available.
- [ ] MultiQC discovers expected reports.
- [ ] Repeated runs produce identical canonical content.
- [ ] Provenance explains exactly how every output was calculated.

---

# Milestone 8 — CPU performance engineering

Correctness baselines must be stable before this milestone changes hot-path structure.

## 8.1 Instrumentation

- [ ] Add stage timers.
- [ ] Separate storage read, decompression, record decode, normalization, collector, coverage, reduction, and output time.
- [ ] Add records/second and aligned-bases/second.
- [ ] Measure peak RSS.
- [ ] Add optional allocator statistics for benchmarks.
- [ ] Ensure instrumentation can be disabled or has negligible overhead.

## 8.2 Allocation cleanup

- [ ] Profile allocation count in representative runs.
- [ ] Remove per-record allocation.
- [ ] Reuse CIGAR and batch buffers.
- [ ] Replace hot-path strings with interned integer IDs.
- [ ] Replace hash maps with arrays or indexed vectors where domains are bounded.
- [ ] Confirm optimizations do not change canonical results.

## 8.3 Parallelism

- [ ] Benchmark HTSlib decompression thread counts.
- [ ] Benchmark streaming batch workers.
- [ ] Implement deterministic thread-local accumulators.
- [ ] Prototype indexed per-contig parallelism.
- [ ] Avoid multiple readers when storage becomes the bottleneck.
- [ ] Bound channels and prove backpressure.
- [ ] Add planner heuristics only after benchmark evidence.

## 8.4 Coverage optimization

- [ ] Benchmark whole-contig versus chunked arrays.
- [ ] Benchmark delta representation choices.
- [ ] Optimize prefix scan and reductions.
- [ ] Evaluate SIMD only after profiles identify relevant loops.
- [ ] Avoid architecture-specific assumptions without dispatch and tests.

## 8.5 Benchmark harness

- [ ] Add reproducible benchmark command.
- [ ] Capture hardware and software environment.
- [ ] Run warm and cold-cache variants where possible.
- [ ] Run multiple repetitions.
- [ ] Report median and spread.
- [ ] Compare:
  - [ ] serial Rust;
  - [ ] parallel Rust;
  - [ ] Samtools;
  - [ ] mosdepth;
  - [ ] Picard;
  - [ ] sequential reference workflow.
- [ ] Store reports under `docs/benchmarks/` without committing large raw outputs.

## 8.6 CPU release threshold

- [ ] Establish numeric regression thresholds after stable data exists.
- [ ] Add small benchmark smoke test to CI.
- [ ] Keep full performance runs outside ordinary shared CI if noise is excessive.
- [ ] Require investigation for material regressions.
- [ ] Never waive correctness to recover benchmark numbers.

### Milestone 8 acceptance evidence

- [ ] Profile shows where time is spent.
- [ ] Combined CPU workflow materially outperforms the sequential equivalent-tool workflow on representative data.
- [ ] Small files do not suffer unreasonable startup or threading overhead.
- [ ] All optimized paths remain canonically identical to the serial reference.

---

# Milestone 9 — Optional CUDA acceleration proof of value

This milestone is optional for a CPU-complete v0.1 release. Do not begin by offloading everything.

## 9.1 CUDA boundary and feature gating

- [ ] Add CUDA dependencies behind a non-default Cargo feature.
- [ ] Keep default build functional without CUDA headers, toolkit, driver, or GPU.
- [ ] Implement device discovery.
- [ ] Implement explicit device selection.
- [ ] Capture driver/runtime/device metadata.
- [ ] Make explicit `--backend cuda` fail if initialization fails.
- [ ] Make `auto` CPU fallback explicit in logs and provenance.

## 9.2 CPU/GPU contract

- [ ] Define canonical integer inputs and outputs for kernels.
- [ ] Define ownership and lifetime of device buffers.
- [ ] Define checked sizes before allocation or launch.
- [ ] Define synchronization and error propagation.
- [ ] Define deterministic reduction strategy.
- [ ] Add CPU oracle tests for every kernel.

## 9.3 Structure-of-arrays normalization

- [ ] Implement normalized record batches.
- [ ] Materialize only required fields.
- [ ] Reuse pinned host buffers after profiling.
- [ ] Add batch-size limits from memory budget.
- [ ] Test records and CIGAR streams crossing batch boundaries.
- [ ] Reject impossible size conversions.

## 9.4 First kernel: coverage scan/reduction

- [ ] Prototype GPU delta accumulation.
- [ ] Prototype prefix scan using vetted CUDA/CUB primitives.
- [ ] Prototype depth histogram reduction.
- [ ] Prototype threshold counts.
- [ ] Compare every integer result with CPU.
- [ ] Test high contention and high depth.
- [ ] Test contigs/chunks smaller than a block.
- [ ] Test device-memory exhaustion.
- [ ] Run compute-sanitizer.

## 9.5 Pipeline overlap

- [ ] Add multiple CUDA streams only after the single-stream path is correct.
- [ ] Overlap CPU decoding, transfer, and computation.
- [ ] Measure transfer cost separately.
- [ ] Bound in-flight batches.
- [ ] Ensure cancellation drains or safely abandons device work.

## 9.6 Additional candidate kernels

Evaluate separately; do not assume benefit.

- [ ] target aggregation;
- [ ] large fixed-bin histograms;
- [ ] per-cycle reductions;
- [ ] targeted SNP allele counting in a future scope;
- [ ] GPU BGZF decompression only as a research prototype after profiling proves CPU decompression dominates.

## 9.7 Auto-selection policy

- [ ] Gather workload-size benchmarks.
- [ ] Consider file type, read count, target territory, required fields, VRAM, transfer bandwidth, and CPU decode bottlenecks.
- [ ] Define a conservative decision threshold.
- [ ] Record decision inputs and result.
- [ ] Add tests that small workloads remain on CPU.
- [ ] Add override for reproducible CPU-only and CUDA-required runs.

## 9.8 CUDA acceptance gate

- [ ] CPU and CUDA canonical results match across all relevant synthetic fixtures.
- [ ] CPU and CUDA canonical results match across HG002 subsets.
- [ ] Repeated CUDA runs are deterministic.
- [ ] Kernel and transfer failures propagate to process failure.
- [ ] Explicit CUDA mode never silently falls back.
- [ ] Auto mode only selects CUDA where measured end-to-end benefit exceeds the documented threshold.
- [ ] CUDA CI passes on the exact claimed commit.
- [ ] No P0/P1 CUDA correctness defects remain.

### Milestone 9 acceptance evidence

- [ ] Publish an end-to-end CPU versus CUDA benchmark report.
- [ ] Report kernel time, transfer time, decode time, and total wall time.
- [ ] Report GPU model, driver, runtime, VRAM, and CPU/storage environment.
- [ ] Keep CUDA disabled by default if value is not yet demonstrated.

---

# Milestone 10 — Full HG002 validation and production hardening

## 10.1 Full-scale data preparation

- [ ] Select a full approximately 30× HG002 WGS dataset.
- [ ] Record source accession, reference, checksums, and preparation commands.
- [ ] Prepare both BAM and CRAM where practical.
- [ ] Verify local storage requirements and cleanup procedure.
- [ ] Keep raw data outside Git.

## 10.2 Full differential run

- [ ] Run Samtools baselines.
- [ ] Run mosdepth baselines.
- [ ] Run Picard baselines.
- [ ] Run Rust DNA Sequencer serial CPU.
- [ ] Run optimized CPU.
- [ ] Run CUDA hybrid if included.
- [ ] Compare all declared compatibility fields.
- [ ] Generate discrepancy report.
- [ ] Resolve or document every discrepancy.

## 10.3 WES/panel full validation

- [ ] Run complete selected target dataset.
- [ ] Validate exact capture BED.
- [ ] Compare target territories and coverage thresholds.
- [ ] Compare enrichment and fold-80 metrics where semantics align.
- [ ] Investigate target normalization differences.

## 10.4 Robustness campaign

- [ ] Run truncated/corrupt BAM and CRAM corpus.
- [ ] Run reference mismatch corpus.
- [ ] Run low-disk and memory-limit tests.
- [ ] Run high-depth stress test.
- [ ] Run unusual contig names and many-contig stress test.
- [ ] Run long-read-like CIGAR stress inputs even if long reads are not a supported profile.
- [ ] Run fuzzers for a documented minimum duration.
- [ ] Run sanitizers available for FFI boundaries.

## 10.5 Operational documentation

- [ ] Document installation.
- [ ] Document system dependencies.
- [ ] Document CPU-only deployment.
- [ ] Document optional CUDA deployment.
- [ ] Document WGS workflow.
- [ ] Document WES/panel workflow.
- [ ] Document output schema.
- [ ] Document compatibility profiles.
- [ ] Document memory sizing.
- [ ] Document test-data preparation.
- [ ] Document troubleshooting and failure meanings.
- [ ] Document known limitations.

## 10.6 Release packaging

- [ ] Produce Linux release artifacts.
- [ ] Decide static versus dynamic HTSlib linkage and document it.
- [ ] Produce checksums.
- [ ] Generate software bill of materials.
- [ ] Sign release artifacts where infrastructure permits.
- [ ] Verify clean-machine installation.
- [ ] Verify `--version` includes required build identity.
- [ ] Verify JSON schema version matches release notes.

### Milestone 10 acceptance evidence

- [ ] Full WGS validation report exists.
- [ ] Full target validation report exists.
- [ ] Performance report exists.
- [ ] Known differences document exists.
- [ ] Malformed-input and resource-failure tests pass.
- [ ] Release artifacts reproduce from the tagged commit.

---

# Milestone 11 — v0.1 release gate

v0.1 may be declared complete only when every required item below is satisfied.

## 11.1 Correctness

- [ ] BAM and CRAM support pass the declared platform matrix.
- [ ] CRAM reference validation is fail-closed.
- [ ] Core alignment metrics pass synthetic and HG002 differential validation.
- [ ] Whole-contig and chunked coverage match exactly.
- [ ] Targeted metrics pass the selected WES/panel corpus.
- [ ] Canonical JSON passes schema validation.
- [ ] Serial and parallel CPU results are identical.
- [ ] CUDA results are identical where CUDA is shipped.
- [ ] No unexplained compatibility discrepancy remains.

## 11.2 Failure integrity

- [ ] Truncated input fails.
- [ ] Unsorted input fails.
- [ ] Reference mismatch fails.
- [ ] Invalid BED fails.
- [ ] Arithmetic overflow fails.
- [ ] Memory-budget violation fails.
- [ ] Required exporter failure fails.
- [ ] Explicit CUDA failure fails without CPU fallback.
- [ ] Partial output never receives `_SUCCESS`.
- [ ] No silent or quiet collector failure exists.

## 11.3 Performance

- [ ] CPU benchmark meets the approved release threshold.
- [ ] Small-file overhead is documented and acceptable.
- [ ] Memory remains within documented limits.
- [ ] CUDA auto-selection is disabled unless its threshold is evidence-based.
- [ ] Performance claims include hardware, software, dataset, repetitions, and variance.

## 11.4 Reproducibility

- [ ] All committed fixtures regenerate.
- [ ] HG002 subset scripts reproduce verified checksums or semantic identities.
- [ ] Reference-tool versions are pinned.
- [ ] Release build is tied to an exact Git commit.
- [ ] CI passes on the exact release commit.
- [ ] Release artifacts have checksums and provenance.

## 11.5 Documentation

- [ ] README matches implemented behavior.
- [ ] Specification reflects approved changes.
- [ ] TODO accurately records completion status.
- [ ] CLI reference is current.
- [ ] Output/schema documentation is current.
- [ ] Compatibility differences are current.
- [ ] Non-clinical limitation is prominent.

## 11.6 Final evidence

Record in the release report:

- [ ] release commit SHA;
- [ ] CI workflow run IDs;
- [ ] exact test-data manifest revision;
- [ ] benchmark report path;
- [ ] validation report path;
- [ ] artifact checksums;
- [ ] unresolved non-blocking issues;
- [ ] CUDA status: not included, experimental, or supported.

---

# Deferred milestones after v0.1

These are intentionally excluded from the v0.1 completion gate unless explicitly promoted through a specification change.

## D1 — Targeted sample identity

- [ ] Load a small known-SNP panel.
- [ ] Perform CIGAR-aware allele extraction.
- [ ] Emit canonical allele counts.
- [ ] Optionally emit a minimal VCF.
- [ ] Validate against a pinned identity workflow.
- [ ] Keep genotype-threshold heuristics out of the core until scientifically validated.

## D2 — Native contamination estimation

- [ ] Define statistical model and scientific validation plan.
- [ ] Validate ancestry handling.
- [ ] Validate low-depth and tumor/normal behavior.
- [ ] Compare against established contamination tools.
- [ ] Do not implement as arbitrary allele-fraction thresholds.

## D3 — D4 and compact coverage tracks

- [ ] Evaluate D4 library maturity and format compatibility.
- [ ] Add canonical coverage-track metadata.
- [ ] Validate random access and interval summaries.
- [ ] Benchmark against bedGraph/bigWig output workflows.

## D4 — Remote and cloud I/O

- [ ] Add HTTP range access.
- [ ] Add S3/GCS only through authenticated, well-tested adapters.
- [ ] Add retry policy that cannot return partial successful data.
- [ ] Record ETag/version identity.
- [ ] Validate remote index/reference consistency.

## D5 — Alternative pure-Rust input backend

- [ ] Benchmark Noodles or another maintained Rust implementation.
- [ ] Compare BAM/CRAM correctness against HTSlib.
- [ ] Keep backend selection explicit.
- [ ] Do not replace HTSlib merely to claim a pure-Rust stack.

## D6 — Raw FASTQ and alignment workflow

- [ ] Define scope separately.
- [ ] Decide whether alignment is built in or orchestrated externally.
- [ ] Establish truth and performance baselines.
- [ ] Keep this independent from the v0.1 aligned-file QC contract.

## D7 — Nanopore/PacBio support

- [ ] Define long-read metrics.
- [ ] Add long-CIGAR and supplementary-chain semantics.
- [ ] Evaluate raw signal/basecalling integration separately.
- [ ] Do not reuse short-read assumptions silently.

---

# Required implementation reports

Create and maintain these reports as the project advances:

```text
docs/validation/
├── SYNTHETIC_FIXTURE_MANIFEST.md
├── SAMTOOLS_DIFFERENTIAL_REPORT.md
├── MOSDEPTH_DIFFERENTIAL_REPORT.md
├── PICARD_DIFFERENTIAL_REPORT.md
├── HG002_WGS_VALIDATION_REPORT.md
└── HG002_TARGET_VALIDATION_REPORT.md

docs/benchmarks/
├── CPU_BASELINE_REPORT.md
├── CPU_OPTIMIZATION_REPORT.md
└── CUDA_ACCELERATION_REPORT.md

docs/architecture/
├── ADR-0001-input-backend.md
├── ADR-0002-canonical-schema.md
├── ADR-0003-coverage-semantics.md
├── ADR-0004-parallel-execution.md
└── ADR-0005-cuda-selection-policy.md
```

Reports shall include exact commands, versions, commit SHAs, dataset identifiers, checksums, and unexplained discrepancies. A report that merely says “tests passed” is insufficient evidence.

# Immediate next work

The next implementation session should begin with Milestone 0 and stop before writing analysis algorithms unless Milestones 0–2 have enough structure to support trustworthy tests.

Recommended first sequence:

1. Create the Cargo workspace and initial crate skeletons.
2. Add permanent CI and strict quality gates.
3. Define core error/configuration/plan types.
4. Build the fixture generator and manifest.
5. Pin Samtools/mosdepth/Picard comparison environments.
6. Add the safe `rust-htslib` input boundary.
7. Implement record classification before broader metrics.

This ordering deliberately treats tests, validation data, and failure semantics as part of the product architecture rather than cleanup work added after optimization.
