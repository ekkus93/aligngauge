# Rust DNA Sequencer v0.1 Specification

**Repository:** `ekkus93/rust-dna-sequencer`  
**Status:** Initial architecture specification  
**Target release:** v0.1  
**Last updated:** 2026-08-05

## 1. Executive summary

Rust DNA Sequencer is a high-performance DNA sequencing quality-control and alignment-analysis engine for whole-genome sequencing (WGS), whole-exome sequencing (WES), and targeted sequencing data.

Despite the repository name, v0.1 is **not** software for controlling a physical sequencing instrument and is **not** a basecaller. It processes existing coordinate-sorted BAM or CRAM alignment files. Its primary architectural goal is to decode each alignment record once, share that decoded record among all enabled analyses, and produce a coherent set of alignment, coverage, target, and quality metrics without repeatedly scanning the same large file.

The system shall be implemented in Rust. A complete CPU implementation is mandatory and authoritative. NVIDIA CUDA acceleration is optional and shall only be used for workloads where measured end-to-end performance improves. GPU availability must never be required for correctness or basic operation.

The initial implementation shall prioritize:

- exact and documented analysis semantics;
- deterministic results;
- fail-closed input and reference validation;
- no silent fallback to approximate algorithms;
- a stable native JSON result model;
- compatibility exports for common bioinformatics tooling;
- differential validation against established tools;
- realistic validation using Genome in a Bottle (GIAB) HG002 data;
- reproducible CPU and GPU benchmarks.

## 2. Problem statement

A typical DNA sequencing QC workflow may invoke several tools independently over the same BAM or CRAM file. Common operations include alignment statistics, flag counts, insert-size metrics, mapping-quality distributions, genome coverage, target coverage, GC bias, quality histograms, and sample-identity allele extraction.

Each independent invocation may perform another expensive round of:

1. storage reads;
2. BGZF or CRAM decompression;
3. record decoding;
4. CIGAR parsing;
5. filtering;
6. output formatting.

For large WGS files, repeated decompression and traversal can dominate the total cost. Rust DNA Sequencer shall consolidate compatible analyses into one coordinated pass and reuse work among collectors.

The project is successful when its total runtime approaches the cost of reading and decoding the input once, plus the unavoidable cost of the enabled analyses.

## 3. Goals

### 3.1 Functional goals

v0.1 shall:

1. Read coordinate-sorted BAM and CRAM files.
2. Validate alignment headers, coordinate order, indexes where required, and CRAM reference compatibility.
3. Produce `flagstat`-style alignment counters.
4. Produce `idxstats`-style per-reference counts.
5. Produce core alignment statistics comparable to the commonly consumed sections of `samtools stats` and Picard alignment metrics.
6. Calculate exact genome, window, and interval coverage.
7. Calculate WES and targeted-panel coverage metrics from BED intervals.
8. Calculate read length, mapping quality, insert size, clipping, sequence GC, and base-quality histograms where the necessary fields are enabled.
9. Break down metrics by sample, read group, library, and platform unit when present.
10. Emit a canonical versioned JSON result.
11. Emit versioned provenance describing all inputs, filters, algorithms, backends, and compatibility profiles.
12. Emit selected compatibility files for MultiQC and established command-line ecosystems.
13. Support deterministic multithreaded CPU execution.
14. Support optional CUDA acceleration through a separately testable backend.
15. Validate results against synthetic edge cases and public GIAB HG002 data.

### 3.2 Performance goals

The implementation shall:

- avoid per-record heap allocation in the steady-state hot path;
- reuse record and batch buffers;
- decode only fields required by the selected analysis plan where the input backend permits it;
- use thread-local accumulators and deterministic reduction;
- batch work sent to the GPU;
- overlap CPU decoding, host/device transfer, and GPU execution when CUDA is enabled;
- measure end-to-end performance rather than kernel-only performance;
- preserve a complete CPU path as the reference implementation.

### 3.3 Correctness goals

The implementation shall:

- define every filter and metric mathematically or procedurally;
- reject malformed or incompatible inputs rather than guessing;
- distinguish unsupported input from empty valid input;
- never silently change exact algorithms into approximate algorithms;
- never silently ignore failed collectors;
- avoid saturating counters without reporting an error;
- produce deterministic output ordering and deterministic reductions;
- identify all expected compatibility differences in machine-readable metadata and documentation.

## 4. Non-goals for v0.1

The following are explicitly outside v0.1:

- physical sequencer control;
- Illumina image processing;
- Oxford Nanopore signal basecalling;
- PacBio signal processing;
- FASTQ alignment;
- de novo assembly;
- duplicate marking or removal;
- base-quality score recalibration;
- small-variant, structural-variant, or copy-number calling;
- phasing;
- ancestry inference;
- a native contamination estimator equivalent to VerifyBamID2;
- a clinical diagnostic claim or regulated clinical validation;
- a new BAM or CRAM codec implementation;
- remote object-store support as a release requirement;
- perfect byte-for-byte emulation of every historical option in Samtools, Picard, mosdepth, or Qualimap.

Future versions may add raw-read workflows, targeted allele extraction, contamination estimation, D4 output, remote I/O, and additional sequencing technologies.

## 5. Terminology

- **Alignment record:** A decoded SAM/BAM/CRAM record.
- **Primary alignment:** A record that is neither secondary nor supplementary.
- **Coverage:** The number of accepted aligned read bases covering a reference position under an explicitly named filter profile.
- **Reference block:** A contiguous reference interval covered by a CIGAR operation that consumes aligned query and reference sequence.
- **Target interval:** A half-open genomic interval supplied through BED or a normalized equivalent.
- **Collector:** A component that consumes decoded records or coverage runs and accumulates a metric family.
- **Analysis plan:** An immutable plan created before record processing that defines required fields, filters, collectors, output profiles, memory strategy, and execution backends.
- **Compatibility profile:** A named set of semantics intended to reproduce or closely match an established tool's output.
- **Exact mode:** A mode that follows the specified CIGAR, filtering, overlap, and base-quality semantics.
- **Approximate mode:** Any mode that deliberately trades semantic fidelity for speed. No approximate mode is part of the default v0.1 path.

## 6. Primary user workflows

### 6.1 WGS QC

```bash
rust-dna-sequencer qc \
  --input sample.bam \
  --reference GRCh38.fa \
  --outdir results \
  --profile wgs \
  --backend auto \
  --threads 16
```

Expected outputs include alignment counters, per-contig statistics, coverage distributions, threshold summaries, insert-size and quality metrics, canonical JSON, and provenance.

### 6.2 WES or panel QC

```bash
rust-dna-sequencer qc \
  --input sample.cram \
  --reference GRCh38.fa \
  --targets capture_targets.bed \
  --outdir results \
  --profile targeted \
  --coverage-thresholds 1,10,20,30,50,100
```

Expected outputs include WGS-style core metrics plus on-target/off-target metrics, per-target coverage, uncovered intervals, threshold percentages, uniformity, and enrichment metrics.

### 6.3 CPU-only reproducibility run

```bash
rust-dna-sequencer qc \
  --input sample.bam \
  --reference GRCh38.fa \
  --outdir results-cpu \
  --backend cpu
```

### 6.4 Required-CUDA run

```bash
rust-dna-sequencer qc \
  --input sample.bam \
  --reference GRCh38.fa \
  --outdir results-cuda \
  --backend cuda \
  --cuda-device 0
```

If CUDA is requested explicitly and cannot be initialized, the command shall fail. It shall not silently continue on the CPU.

### 6.5 Input inspection

```bash
rust-dna-sequencer inspect \
  --input sample.cram \
  --reference GRCh38.fa
```

This shall validate metadata without running the full analysis and shall report contigs, sort order, read groups, index availability, reference matching, and estimated workload.

## 7. Command-line interface

The installed binary name shall initially be `rust-dna-sequencer`. A shorter alias may be added later without changing the canonical name.

### 7.1 Subcommands

- `qc`: run one complete analysis;
- `inspect`: inspect and validate alignment metadata;
- `validate-reference`: validate an input alignment against a FASTA and indexes;
- `schema`: print the native JSON schema version and optionally export the schema;
- `testdata`: list or prepare documented public test-data subsets; this may be deferred until after the core engine.

### 7.2 Common options

- `--input <PATH>`
- `--reference <PATH>`
- `--targets <PATH>`
- `--outdir <PATH>`
- `--profile <wgs|targeted|custom>`
- `--backend <auto|cpu|cuda>`
- `--threads <N>`
- `--io-threads <N>`
- `--cuda-device <N>`
- `--memory-limit <SIZE>`
- `--coverage-thresholds <LIST>`
- `--config <PATH>`
- `--log-format <human|json>`
- `--quiet`
- `--verbose`

Configuration precedence shall be documented and deterministic:

1. built-in defaults;
2. configuration file;
3. environment variables, only for explicitly supported values;
4. CLI arguments.

The final resolved configuration shall be recorded in provenance.

## 8. Input contracts

### 8.1 BAM and CRAM

v0.1 shall support:

- BAM conforming to the maintained SAM/BAM specification;
- CRAM through HTSlib via `rust-htslib`;
- coordinate-sorted input;
- BAI, CSI, or CRAI where indexed region access is used;
- local filesystem paths.

A streaming whole-file pass may operate without an index when the enabled analyses do not require seeking. Indexed parallel mode shall require a compatible index.

### 8.2 Reference FASTA

CRAM shall require a matching reference FASTA unless the file is fully self-contained and the backend can prove that no external reference is needed. The implementation shall not guess between references.

Reference validation shall compare, when available:

- sequence names;
- sequence lengths;
- header MD5 values;
- FASTA index entries;
- dictionary metadata.

Any mismatch that can change decoded sequence or coordinates shall be fatal.

### 8.3 BED targets

BED input shall be interpreted as zero-based, half-open intervals. The parser shall:

- reject negative, reversed, or overflowing coordinates;
- validate contig names against the alignment header;
- optionally reject unknown contigs by default;
- sort intervals deterministically;
- merge overlaps for aggregate calculations while retaining original interval identity for per-target reporting;
- preserve optional target names;
- record normalization actions in provenance.

The program shall never infer whether an interval file is one-based.

### 8.4 Coordinate order

The header sort-order declaration is insufficient by itself. During processing, the engine shall detect material coordinate regressions. An unsorted input shall fail with a clear diagnostic unless a future explicitly named unsorted mode is implemented.

## 9. Output contracts

### 9.1 Atomic output behavior

All outputs shall be written to a staging directory inside the selected output filesystem. The completed output set shall be published atomically where the platform permits.

On failure:

- no completed-success marker shall be emitted;
- incomplete compatibility outputs shall not be presented as valid;
- temporary files shall be removed by default;
- an explicit diagnostic option may preserve temporary files for debugging.

### 9.2 Canonical native outputs

Required files:

- `summary.json`: canonical versioned metrics;
- `provenance.json`: resolved configuration, algorithms, versions, backends, filters, input identities, warnings, and timings;
- `run.log` or structured log stream when requested;
- `_SUCCESS`: created only after all required outputs are complete and synchronized.

The JSON schema shall include:

- `schema_version`;
- application version and Git commit;
- analysis profile;
- input and reference identity;
- filter definitions;
- metric definitions and units;
- per-sample/read-group/library breakdowns;
- execution plan;
- CPU and GPU device information;
- stage timings;
- compatibility differences;
- warning and error summaries.

### 9.3 Compatibility outputs

Compatibility output shall be generated from the canonical internal model, not accumulated independently where avoidable.

Initial targets:

- Samtools-like flag statistics;
- Samtools-like per-contig counts;
- selected Samtools statistics sections required by common MultiQC reports;
- mosdepth-like summary, global distribution, and region tables;
- selected Picard-like alignment, insert-size, WGS, and hybrid-selection metrics;
- MultiQC-discoverable filenames and metadata.

Every compatibility format shall have:

- a named profile;
- a pinned reference-tool version during validation;
- documented exact matches;
- documented tolerated numeric differences;
- documented unsupported fields;
- golden fixtures.

Compatibility exporters shall fail if required source metrics were not collected. They shall not emit zeros or empty sections as substitutes for missing analysis.

## 10. Metrics

### 10.1 Alignment and flag counters

At minimum:

- total records;
- primary records;
- secondary records;
- supplementary records;
- mapped and unmapped records;
- paired records;
- proper pairs;
- read 1 and read 2;
- mate mapped and mate unmapped;
- duplicates;
- QC-fail records;
- singleton records;
- inward-, outward-, and other-orientation pairs where defined;
- records by mapping-quality bin;
- records by reference sequence;
- passing and failing quality-control partitions where compatibility requires them.

Classification order shall be explicit. In particular, records carrying both secondary and supplementary bits must follow the selected compatibility profile's priority rather than being accidentally double-counted.

### 10.2 Histograms and distributions

When enabled and available:

- read length;
- aligned query length;
- reference span;
- mapping quality;
- insert size/template length;
- clipping by type and cycle;
- sequence GC percentage;
- base quality;
- per-cycle base composition;
- per-cycle quality;
- mismatch/edit-distance metrics from validated tags or reference comparison.

Missing optional tags shall not be silently interpreted as zero. The metric shall either use a documented alternate derivation or report unavailable data.

### 10.3 Coverage

Coverage shall support:

- whole-genome per-base accumulation;
- fixed windows;
- arbitrary normalized target intervals;
- coverage histograms;
- cumulative threshold percentages;
- per-contig summaries;
- uncovered runs;
- callable coverage under a named policy;
- optional duplicate-inclusive and duplicate-excluded tracks;
- optional base-quality and mapping-quality filters.

Each coverage result shall state:

- accepted flag mask;
- minimum mapping quality;
- minimum base quality, if any;
- treatment of deletions;
- treatment of reference skips;
- treatment of overlapping mates;
- treatment of secondary and supplementary records;
- duplicate policy;
- QC-fail policy.

### 10.4 Targeted metrics

For WES and panels:

- target territory;
- aligned bases;
- on-target bases;
- near-target bases when flanks are configured;
- off-target bases;
- mean and median target depth;
- minimum and maximum target depth;
- percentages at configured thresholds;
- zero-coverage target bases and intervals;
- per-target and optionally per-gene summaries;
- fold enrichment;
- coverage uniformity;
- fold-80 base penalty or an explicitly documented equivalent;
- duplicate-adjusted metrics;
- target dropout lists.

Target aggregation must retain enough information to explain how overlapping input intervals affected the result.

### 10.5 Read-group hierarchy

Metrics shall be available at applicable levels:

- complete input;
- sample;
- library;
- read group;
- platform unit.

Malformed or contradictory read-group metadata shall produce explicit diagnostics. Unknown read-group identifiers in records shall not be silently assigned to a synthetic group unless the selected policy explicitly requests it.

## 11. Filtering model

Filtering shall be represented by immutable named structures rather than scattered booleans.

```rust
pub struct RecordFilter {
    pub include_unmapped: bool,
    pub include_secondary: bool,
    pub include_supplementary: bool,
    pub include_duplicates: bool,
    pub include_qc_fail: bool,
    pub min_mapq: u8,
}

pub struct BaseFilter {
    pub min_base_quality: Option<u8>,
    pub count_deletions: bool,
    pub count_reference_skips: bool,
    pub correct_overlapping_mates: bool,
}
```

Compatibility profiles may instantiate these structures, but resolved values shall always be visible in output provenance.

## 12. Core architecture

### 12.1 Workspace layout

The initial workspace should use narrowly scoped crates:

```text
crates/
├── rds-cli/               # CLI and process lifecycle
├── rds-core/              # shared types, plans, errors, canonical results
├── rds-hts/               # rust-htslib boundary and record views
├── rds-metrics/           # alignment and histogram collectors
├── rds-coverage/          # coverage events, scans, windows, intervals
├── rds-targets/           # BED normalization and targeted metrics
├── rds-output/            # JSON and compatibility exporters
├── rds-gpu/               # optional CUDA runtime and kernels
└── rds-test-support/      # fixture builders and differential helpers
```

Crate boundaries may be consolidated during early implementation if compile-time or API overhead outweighs the benefit. Public library APIs shall remain intentionally small until semantics stabilize.

### 12.2 Input boundary

The initial input backend shall use `rust-htslib` and HTSlib. This provides mature BAM/CRAM support while allowing the project to focus on analysis rather than codec implementation.

All unsafe or FFI behavior shall be contained in the input and CUDA boundary crates. Safe borrowed views shall be exposed to the analysis layer.

```rust
pub trait AlignmentSource {
    fn header(&self) -> &AlignmentHeader;
    fn next_record(&mut self) -> Result<Option<RecordView<'_>>, InputError>;
}
```

The exact trait may change to support batched reading, but analysis code shall not depend directly on raw HTSlib pointers.

### 12.3 Analysis plan

Before processing, the system shall compile the resolved configuration into an immutable `AnalysisPlan`.

```rust
pub struct AnalysisPlan {
    pub required_fields: RequiredFields,
    pub record_filter: RecordFilter,
    pub coverage_plan: Option<CoveragePlan>,
    pub target_plan: Option<TargetPlan>,
    pub metric_plan: MetricPlan,
    pub execution_plan: ExecutionPlan,
    pub output_plan: OutputPlan,
}
```

The plan shall determine:

- which record fields are required;
- which collectors are active;
- the coverage memory strategy;
- CPU thread counts;
- CUDA eligibility;
- batching thresholds;
- compatibility profiles;
- expected output files.

Invalid combinations shall fail before reading the full input.

### 12.4 Hot-path processing

The hot path shall:

1. reuse decoded record storage;
2. perform common flag classification once;
3. normalize frequently used integer identifiers;
4. avoid formatting and logging;
5. update fixed arrays or preallocated vectors;
6. avoid hash lookups for common bins;
7. emit coverage events from CIGAR blocks;
8. dispatch only to enabled analysis code;
9. accumulate into thread-local or batch-local state;
10. reduce deterministically.

Dynamic trait-object calls per record should be avoided. A statically composed collector set, generated enum, or explicit analysis loop is preferred.

### 12.5 Parallel CPU modes

#### Streaming mode

- one ordered input stream;
- HTSlib decompression threads;
- one or more batched analysis workers where ordering permits;
- bounded queues;
- deterministic merge;
- suitable for CRAM and slower storage.

#### Indexed parallel mode

- independent readers over disjoint reference partitions;
- requires compatible index;
- thread-local collectors;
- deterministic reduction by reference order;
- enabled only when profiling predicts benefit.

`auto` may choose between modes, but the decision and evidence inputs must be recorded.

## 13. Coverage algorithms

### 13.1 Difference-array model

For each accepted aligned reference block `[start, end)`:

```text
delta[start] += 1
delta[end]   -= 1
```

An inclusive prefix scan over `delta` produces depth runs. Separate reductions then calculate histograms, thresholds, windows, and target summaries.

Only CIGAR operations that meet the selected coverage semantics shall generate blocks. Insertions and clipping do not consume reference positions. Deletions and reference skips shall be treated according to explicit policy.

### 13.2 Memory strategies

The engine shall support at least:

1. **Whole-contig arrays** for maximum speed when memory permits.
2. **Chunked arrays** for bounded memory.
3. **Target-focused arrays** for WES and small panels.

The planner shall estimate required memory before processing and fail or choose an exact lower-memory strategy. It shall not silently lower precision.

Coverage counters shall use checked arithmetic. If a configured representation cannot hold observed depth, the engine shall fail with an actionable diagnostic or restart only when an explicit restart policy permits it. Saturation is forbidden.

### 13.3 Overlapping mates

Overlap correction is required for exact fragment-oriented coverage profiles. Because it requires pairing information and CIGAR-aware overlap handling, it shall be implemented as a distinct tested stage.

If overlap correction is requested but cannot be performed reliably because the necessary pairing constraints are violated, the engine shall fail or mark the requested output unavailable. It shall not quietly count both mates.

### 13.4 Coverage output precision

Internally accumulated integer counts shall remain integers. Floating-point means and percentages shall be calculated during deterministic reduction using documented precision and rounding. Compatibility exporters may apply profile-specific formatting without modifying canonical values.

## 14. Optional NVIDIA CUDA acceleration

### 14.1 Principles

- CUDA is optional.
- CPU results are authoritative.
- Explicit `--backend cuda` requires a usable CUDA device or fails.
- `--backend auto` may select CPU or CUDA, but must report the decision.
- CPU fallback in `auto` is allowed only when semantics remain identical.
- No GPU code may introduce an approximate algorithm under an exact profile.
- GPU acceleration is accepted only after end-to-end benchmarks demonstrate benefit.

### 14.2 Initial GPU candidates

The first CUDA milestone should evaluate:

- coverage event accumulation;
- prefix scans;
- coverage histogram and threshold reductions;
- interval/target aggregation;
- large fixed-bin histograms;
- later, targeted SNP allele counting.

BAM/CRAM parsing, header processing, reference validation, BED parsing, error handling, and compatibility output shall remain on the CPU initially.

### 14.3 Batch representation

Raw HTSlib records shall not be copied directly to the GPU. The CPU shall normalize enabled fields into structure-of-arrays batches.

```rust
pub struct GpuRecordBatch {
    pub reference_ids: Vec<i32>,
    pub starts: Vec<i64>,
    pub flags: Vec<u16>,
    pub mapqs: Vec<u8>,
    pub template_lengths: Vec<i64>,
    pub cigar_offsets: Vec<u32>,
    pub cigar_lengths: Vec<u16>,
    pub cigar_ops: Vec<u32>,
    pub sequence_offsets: Option<Vec<u32>>,
    pub packed_sequences: Option<Vec<u8>>,
    pub qualities: Option<Vec<u8>>,
}
```

Only required arrays shall be materialized. Pinned host buffers and multiple CUDA streams should be used after profiling confirms benefit.

### 14.4 Execution plan

```rust
pub enum Backend {
    Cpu,
    Cuda { device: usize },
    Auto,
}

pub struct ExecutionPlan {
    pub input_backend: Backend,
    pub alignment_metrics_backend: Backend,
    pub coverage_backend: Backend,
    pub target_backend: Backend,
    pub histogram_backend: Backend,
}
```

The actual plan shall be printed before analysis unless quiet mode is requested and shall always be written to provenance.

### 14.5 GPU acceptance gate

A CUDA implementation shall not become the default for a workload until:

- it matches CPU canonical results across the full differential corpus;
- it passes repeated determinism tests;
- it passes compute-sanitizer or equivalent checks;
- it handles device-memory exhaustion without corrupting output;
- it improves end-to-end wall-clock time by a documented minimum threshold on at least one supported workload class;
- it does not cause unacceptable regressions on smaller workloads;
- its selection threshold is derived from benchmark data rather than guessed.

## 15. Error handling and failure policy

Errors shall use typed categories, including:

- input open/read/decode errors;
- malformed header;
- unsupported format or feature;
- sort-order violation;
- missing or incompatible index;
- reference mismatch;
- invalid target intervals;
- arithmetic overflow;
- resource-limit violation;
- CUDA initialization, transfer, launch, or synchronization failure;
- collector failure;
- exporter failure;
- output publication failure.

Rules:

1. Required collector failure fails the run.
2. Unsupported requested output fails before expensive processing where possible.
3. No empty file may stand in for failed output.
4. Warnings shall never claim successful calculation of unavailable metrics.
5. `auto` backend fallback shall be recorded with the original reason.
6. Explicit CUDA mode shall not fall back to CPU.
7. Partial output is never marked successful.
8. Error messages shall identify the input, stage, and actionable cause without exposing raw sensitive sequence data unnecessarily.

## 16. Determinism

Given identical:

- application version;
- configuration;
- input bytes;
- reference bytes;
- target bytes;
- compatibility profile;

canonical metric values and ordering shall be identical across repeated runs. CPU and CUDA backends shall produce identical integer results. Floating-point reductions shall use a deterministic order or compensated deterministic method.

Timestamps, hostnames, device identifiers, and timings belong in provenance and are excluded from canonical metrics equivalence.

## 17. Test-data strategy

### 17.1 Committed synthetic fixtures

The repository shall contain small redistributable synthetic BAM/CRAM fixtures covering:

- all major CIGAR operations;
- overlapping mates;
- secondary alignments;
- supplementary alignments;
- records carrying both secondary and supplementary flags;
- duplicate and QC-fail records;
- mapped read with unmapped mate;
- unmapped read with mapped mate;
- missing and malformed optional tags;
- multiple read groups and libraries;
- unknown read-group identifiers;
- extreme template lengths;
- zero-length references where legal;
- high-depth overflow boundaries;
- contig naming mismatches;
- unsorted records;
- truncated BAM and CRAM;
- reference mismatch;
- BED overlaps and invalid coordinates.

Fixtures should be generated from auditable SAM/reference source files, not opaque binaries alone.

### 17.2 GIAB HG002 development subsets

The project shall provide reproducible scripts and manifests to prepare small HG002 subsets, initially:

- approximately 1 Mb on GRCh38 chromosome 20 at several downsampled depths;
- a larger approximately 10 Mb region for scaling;
- an HG002 exome or target subset with the exact matching capture intervals;
- later, a full approximately 30× WGS validation sample.

Large data shall not be committed to Git. Each manifest entry shall record:

- source accession or URL;
- source checksum when published;
- local SHA-256;
- reference identity;
- extraction region;
- downsampling seed and fraction;
- generation commands;
- tool versions;
- redistribution restrictions.

### 17.3 Differential reference tools

The validation harness shall compare against pinned versions of:

- Samtools;
- mosdepth;
- Picard;
- optionally Qualimap where a metric is directly comparable;
- MultiQC parsing for compatibility outputs.

Pinned versions and containers shall be recorded in the test manifest.

## 18. Validation strategy

### 18.1 Unit tests

Required areas:

- flag classification;
- CIGAR parsing and block emission;
- interval normalization;
- filter evaluation;
- histogram updates;
- deterministic merges;
- JSON schema serialization;
- compatibility formatting;
- execution planner decisions;
- checked arithmetic and resource limits.

### 18.2 Property tests

Properties shall include:

- emitted reference blocks never exceed declared reference bounds;
- coverage deltas sum to zero at the end of a complete contig;
- total covered-base contribution equals the sum of accepted CIGAR block lengths before overlap correction;
- primary/secondary/supplementary categories obey the chosen priority and invariants;
- interval normalization preserves union territory;
- chunked and whole-contig coverage produce identical results;
- parallel merges equal serial results;
- CPU and CUDA integer outputs match.

### 18.3 Fuzzing

Fuzz targets shall cover:

- SAM header interpretation;
- CIGAR decoding;
- optional-tag access;
- BED parsing;
- interval normalization;
- canonical JSON deserialization where supported;
- compatibility exporters;
- batch normalization for CUDA.

### 18.4 Differential tests

For each fixture and real-data subset:

- run the reference tool;
- run Rust DNA Sequencer with the matching compatibility profile;
- compare exact integers;
- compare floating values with metric-specific tolerances;
- compare output schema and required sections;
- reject unexplained differences;
- store concise expected artifacts, not large source data.

### 18.5 End-to-end tests

An end-to-end test must verify:

1. input and reference validation;
2. completed analysis;
3. atomic output publication;
4. `_SUCCESS` creation;
5. canonical JSON schema validation;
6. compatibility output generation;
7. MultiQC discovery where enabled;
8. deterministic rerun equivalence;
9. absence of hidden partial failures.

## 19. Benchmark strategy

### 19.1 Required measurements

- wall-clock time;
- CPU time;
- peak resident memory;
- bytes read and written;
- records per second;
- aligned bases per second;
- decompression time;
- record normalization time;
- collector time;
- coverage scan/reduction time;
- output time;
- GPU transfer and kernel time;
- GPU utilization and peak VRAM;
- backend-planner decision.

### 19.2 Dataset classes

- tiny synthetic fixtures;
- 1 Mb HG002 subsets at 1×, 10×, 30×, and high depth;
- 10 Mb HG002 subset;
- representative WES/panel subset;
- full approximately 30× HG002 WGS;
- BAM and CRAM equivalents where practical.

### 19.3 Baselines

Compare:

- serial Rust implementation;
- optimized multithreaded CPU implementation;
- CUDA hybrid implementation;
- sequential reference-tool workflow;
- each major reference tool individually.

### 19.4 Performance release criteria

Correctness has priority over speed. Subject to correctness:

- the combined CPU workflow should materially reduce wall-clock time relative to the sequential sum of equivalent reference tools on representative WGS and WES workloads;
- the optimized CPU path must not regress more than a documented tolerance against the serial baseline on small inputs;
- CUDA shall only be selected automatically where it provides a repeatable end-to-end improvement;
- benchmark variance and storage-cache state shall be reported;
- no performance claim shall be made from a single run.

Exact numeric thresholds shall be finalized after the first benchmark harness is operational and before GPU auto-selection is enabled.

## 20. Security and robustness

Alignment, reference, BED, configuration, and index files shall be treated as untrusted input.

Requirements:

- bounds-check all coordinates and lengths;
- use checked arithmetic for offsets and counts;
- cap allocations through a configured memory budget;
- reject absurd header declarations before allocation;
- avoid following unsafe output-directory symlinks where practical;
- create temporary files with restrictive permissions;
- avoid including sequence or read names in routine errors and telemetry;
- audit every `unsafe` block and explain its invariant;
- isolate FFI and CUDA pointer handling;
- run dependency vulnerability and license checks;
- fuzz parsers and boundary adapters;
- test truncated and corrupted compressed input;
- never execute content from input files.

## 21. Observability

Human logs shall be concise. Structured logs shall include stage, severity, input identity, and error category.

Progress reporting may include:

- bytes or records processed;
- current reference;
- throughput;
- estimated completion only when based on reliable input size/index information;
- current CPU/GPU execution stage.

Progress UI shall not alter output semantics and shall be disabled automatically when inappropriate.

Stage timings and planner decisions shall always be captured in provenance even when console progress is disabled.

## 22. Dependency and toolchain policy

- Pin a minimum supported Rust version once the initial workspace compiles.
- Commit `Cargo.lock` for the application workspace.
- Use `rust-htslib` for initial BAM/CRAM support.
- Keep CUDA dependencies optional behind a feature and runtime capability check.
- Prefer mature crates with clear maintenance and licensing.
- Avoid introducing a framework into the record hot path without benchmarks.
- Run `cargo fmt`, strict Clippy, unit tests, integration tests, docs tests, dependency audit, and license checks in CI.
- Pin GitHub Actions by immutable commit SHA.

The repository license is authoritative for project code. Third-party compatibility formats and copied reference fixtures require separate provenance review.

## 23. CI and release gates

Permanent CI shall include:

- formatting;
- strict Clippy with warnings denied;
- build on supported Linux targets;
- unit and integration tests;
- documentation tests;
- JSON schema validation;
- synthetic differential tests;
- malformed-input tests;
- deterministic rerun tests;
- dependency and license audit;
- fuzz target compilation;
- CPU benchmark smoke test with non-regression thresholds only after stable baselines exist.

CUDA CI may run on a separate trusted runner. CUDA absence on ordinary CI must not weaken CPU coverage.

A release shall require:

- all permanent gates passing on the exact release commit;
- completed release checklist;
- versioned schema and changelog;
- benchmark report for supported workloads;
- validation report with known differences;
- signed checksums for release artifacts;
- no unresolved P0 correctness or data-integrity defects.

## 24. Compatibility and stability policy

Until 1.0:

- internal APIs may change freely;
- public library APIs shall be minimal and explicitly marked unstable;
- the CLI may evolve, but breaking changes require changelog entries;
- canonical JSON schema changes require a schema-version change and migration notes;
- compatibility profiles shall include the validated reference-tool version;
- output files shall never change semantics under an unchanged profile name.

## 25. Initial repository layout

```text
.
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── README.md
├── docs/
│   ├── DNA_QC_ENGINE_SPEC.md
│   ├── DNA_QC_ENGINE_TODO.md
│   ├── architecture/
│   ├── validation/
│   └── benchmarks/
├── crates/
│   ├── rds-cli/
│   ├── rds-core/
│   ├── rds-hts/
│   ├── rds-metrics/
│   ├── rds-coverage/
│   ├── rds-targets/
│   ├── rds-output/
│   ├── rds-gpu/
│   └── rds-test-support/
├── testdata/
│   ├── README.md
│   ├── manifest.toml
│   ├── fixtures/
│   ├── generators/
│   └── expected/
├── benches/
└── .github/workflows/
```

## 26. v0.1 acceptance criteria

v0.1 is complete only when all of the following are true:

1. A fresh checkout builds using documented commands.
2. Coordinate-sorted BAM and CRAM are supported on the primary Linux target.
3. CRAM reference mismatch fails before valid-looking metrics are emitted.
4. Core alignment counters pass synthetic and HG002 differential tests.
5. Whole-contig and chunked exact coverage match on the same data.
6. Targeted metrics pass validated WES/panel fixtures.
7. Canonical JSON validates against a versioned schema.
8. Required compatibility outputs are parsed successfully by their intended consumers.
9. CPU parallel output is deterministic and equals serial output.
10. Malformed, truncated, unsorted, and incompatible inputs fail closed.
11. Required outputs are published atomically and `_SUCCESS` is trustworthy.
12. No silent collector failure or approximate fallback exists.
13. HG002 development subsets can be reproduced from documented manifests.
14. A full approximately 30× WGS benchmark and validation report exists.
15. All permanent CI gates pass on the exact release commit.
16. Known differences and unsupported features are documented.
17. CUDA, if shipped in v0.1, passes CPU equivalence and performance gates; otherwise the CPU release remains complete without it.

## 27. Deferred design decisions

The following decisions are intentionally deferred until measurement or implementation evidence exists:

- whether Noodles becomes an additional or replacement input backend;
- whether D4 output is included in v0.1 or v0.2;
- exact CPU indexed-parallel scheduling policy;
- exact CUDA batch-size and auto-selection thresholds;
- whether targeted identity SNP extraction enters v0.1;
- whether remote HTTP/S3/GCS input is supported before 1.0;
- which full set of Picard-compatible fields is practical;
- final short CLI alias;
- minimum supported NVIDIA compute capability.

Deferred decisions must not be resolved through undocumented behavior. Each shall be recorded in an architecture decision record before implementation becomes permanent.

## 28. Reference standards and comparison projects

Implementation and validation should consult and pin the relevant maintained primary sources, including:

- GA4GH/Samtools SAM, BAM, CRAM, VCF, BED-related, and index specifications;
- HTSlib and Samtools documentation and source behavior;
- mosdepth documentation and source behavior;
- Picard metric definitions and source behavior;
- MultiQC parser expectations;
- NIST Genome in a Bottle documentation and HG002 manifests;
- NVIDIA CUDA, CUB, and compute-sanitizer documentation for optional acceleration.

Reference-tool behavior is evidence, but this specification's named semantics and canonical schema remain authoritative for Rust DNA Sequencer.
