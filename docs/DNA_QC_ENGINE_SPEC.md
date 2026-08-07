# AlignGauge Product and Architecture Specification

**Current repository:** `ekkus93/rust-dna-sequencer`  
**Recommended repository name:** `ekkus93/aligngauge`  
**Binary name:** `aligngauge`  
**Status:** Revised planning specification  
**Last updated:** 2026-08-06  
**Supersedes:** Initial `DNA_QC_ENGINE_SPEC.md` dated 2026-08-05

## 1. Product identity

### 1.1 Name

The project name is **AlignGauge**.

“Align” identifies the durable product boundary: analysis of aligned sequencing
records rather than one particular file format. “Gauge” communicates measurement,
quality control, and coverage assessment rather than alignment, variant calling,
basecalling, or physical sequencer control. The name remains accurate as the
project grows from BAM support in v0.1 to CRAM, WES, targeted-panel, and
compatibility features in later releases.

Recommended repository rename:

```text
ekkus93/rust-dna-sequencer
    ↓
ekkus93/aligngauge
```

Recommended crate namespace:

```text
aligngauge
aligngauge-core
aligngauge-hts
aligngauge-metrics
aligngauge-coverage
aligngauge-formats
aligngauge-testkit
```

### 1.2 Project description

> **A validation-first Rust engine for fast, single-pass alignment QC and coverage
> analysis across BAM and CRAM data for WGS, WES, and targeted sequencing.**

A slightly longer README description may use:

> AlignGauge reads each alignment record once and shares it across deterministic
> quality-control collectors, producing canonical JSON, provenance, alignment
> counters, coverage metrics, and compatibility exports without repeatedly scanning
> the same BAM or CRAM file.

### 1.3 Product boundary

AlignGauge is not:

- software for controlling a physical DNA sequencer;
- a basecaller;
- a FASTQ aligner;
- a duplicate marker;
- a variant caller;
- a genome assembler;
- a clinical diagnostic product.

It analyzes existing aligned sequencing data.

## 2. Problem statement

DNA sequencing QC pipelines commonly run several tools independently over the same
large BAM or CRAM file. Each invocation may repeat:

1. storage reads;
2. BGZF or CRAM decompression;
3. alignment-record decoding;
4. CIGAR parsing;
5. filtering;
6. metric accumulation;
7. output formatting.

For WGS data, repeated decoding and traversal can dominate the total QC cost.
AlignGauge shall coordinate compatible analyses around a shared record stream so
that each decoded record is reused by all enabled collectors.

The performance target is not “Rust is faster than C.” The target is:

> Approach the cost of reading and decoding the input once, plus the unavoidable
> work of the selected metrics.

## 3. Design principles

The following requirements apply to every release.

### 3.1 Correctness before breadth

A metric is not complete until its semantics, fixtures, reference comparison, and
failure behavior are defined. A smaller verified release is preferred over a broad
release with unresolved compatibility gaps.

### 3.2 CPU-authoritative implementation

Every released metric shall have a complete CPU implementation. Experimental
hardware acceleration shall never be required to obtain correct results.

### 3.3 Fail closed

AlignGauge shall not:

- substitute zero for missing data;
- silently skip a failed collector;
- silently accept malformed or unsorted input;
- silently change an exact algorithm into an approximation;
- silently fetch an input or reference from the network;
- silently emit partial compatibility files;
- silently saturate counters;
- silently assign unknown read groups to invented groups.

Unsupported or unverifiable input shall produce an actionable error.

### 3.4 Canonical model first

The native, versioned result model is authoritative. Compatibility files are derived
from that model rather than accumulated independently, except where a format
requires additional explicitly named state.

### 3.5 Deterministic reduction

Integer counts remain integers through collection and reduction. Means, percentages,
and other floating-point values are computed in one deterministic CPU-side final
reduction over canonical integer inputs. Output ordering is stable.

### 3.6 Evidence-based optimization

Optimization follows profiling. The project shall not add permanent configuration,
public CLI flags, crates, or result-schema fields for a hypothetical backend before
a prototype demonstrates reproducible end-to-end value.

## 4. Version roadmap

The version boundaries are product contracts, not aspirational labels.

### 4.1 v0.1 — BAM CPU foundation

v0.1 is a coherent, releasable BAM QC engine with:

- coordinate-sorted BAM input;
- one streaming reader;
- CPU execution only;
- input/header/coordinate validation;
- `flagstat`-equivalent record classification and counters;
- `idxstats`-equivalent per-reference mapped and unmapped counters;
- exact project-defined genome coverage;
- coverage histograms and configurable thresholds;
- canonical JSON;
- provenance JSON;
- atomic output publication;
- synthetic differential fixtures;
- validation on a small real HG002 region.

v0.1 does not include CRAM, WES/panel metrics, Picard compatibility, CUDA, remote
I/O, full-genome validation, or indexed partition parallelism.

### 4.2 v0.2 — CRAM and reference integrity

v0.2 adds:

- CRAM input through a pinned HTSlib/rust-htslib version;
- mandatory explicit local-reference resolution for CRAM analysis;
- remote reference retrieval disabled unconditionally;
- reference identity and MD5 validation;
- BAM/CRAM equivalence testing;
- CRAM corruption and missing-reference fixtures.

Standalone `inspect` and `validate-reference` commands are **not** v0.2
release requirements. The v0.2 integrity contract is enforced through the
released `qc --reference <FASTA>` path and the shared reference-validation
API. Dedicated inspection/validation workflows are deferred until their CLI,
output-schema, and error contracts are specified and tested as independent
product surfaces.

### 4.3 v0.3 — WES and targeted panels

v0.3 adds:

- BED target input;
- strict but vendor-compatible BED parsing;
- normalized target intervals;
- on-target, near-target, and off-target measurements;
- per-target coverage and threshold summaries;
- uncovered target runs;
- target territory and enrichment metrics;
- an explicitly defined fold-80 metric or named alternative;
- HG002 exome/target validation.

### 4.4 v0.4 — Expanded metric and ecosystem compatibility

v0.4 adds selected, explicitly scoped compatibility with:

- commonly consumed `samtools stats` sections;
- Picard alignment-summary metrics;
- Picard insert-size metrics, including exact trimming/rounding where claimed;
- Picard WGS and hybrid-selection metrics where adopted;
- MultiQC parser validation;
- read-group, library, sample, and platform-unit breakdowns;
- exact mate-overlap correction under a supported execution mode.

### 4.5 v0.5 — Full-scale qualification

v0.5 adds:

- approximately 30× HG002 WGS validation;
- repeatable benchmark reports;
- serial-versus-parallel equivalence where parallel execution is released;
- resource-bound testing;
- fuzzing and sanitizer campaigns sufficient for a production beta;
- signed release artifacts and SBOMs if distribution automation is ready.

### 4.6 Post-v0.5 candidates

Potential later capabilities include:

- D4 coverage output;
- targeted sample-identity allele extraction;
- contamination-estimation integration;
- remote object-store input;
- additional compatibility profiles;
- long-read-specific policies;
- native or alternative alignment backends.

### 4.7 Hardware-acceleration research track

GPU acceleration is not assigned to a release.

Research spikes may investigate:

- GPU BGZF decompression;
- coverage prefix scans;
- target reductions;
- pileup and allele counting;
- compression or other profiled bottlenecks.

A GPU feature may enter the product only after an ADR records:

1. the measured bottleneck;
2. the prototype design;
3. CPU and GPU hardware;
4. storage and PCIe topology;
5. warm and cold runs;
6. end-to-end wall-clock improvement;
7. correctness equivalence;
8. resource and maintenance costs.

Kernel-only speedups do not qualify. No `--backend` option or GPU crate is part of
the product until a feature passes this gate.

## 5. Terminology

- **Alignment record:** a decoded SAM/BAM/CRAM record.
- **Primary record:** neither secondary nor supplementary.
- **Reference block:** a half-open reference interval represented by a CIGAR
  operation that the selected coverage policy treats as covered.
- **Coverage track:** one coverage parameterization, including flag filters, MAPQ,
  base-quality, duplicate, overlap, deletion, and skip policies.
- **Collector:** a component that consumes validated records or coverage runs and
  accumulates one metric family.
- **Analysis plan:** an immutable plan created before processing that specifies
  required fields, collectors, filters, resource limits, and outputs.
- **Compatibility profile:** a pinned set of semantics intended to match a named
  version of an established tool.
- **Canonical result:** the versioned native metric model from which other outputs
  are generated.
- **Walking skeleton:** the first disposable vertical slice proving the complete
  CLI-to-result path.

## 6. v0.1 command-line contract

### 6.1 Walking-skeleton interface

The first code milestone may expose:

```bash
aligngauge qc --input sample.bam
```

It shall count total, mapped, and unmapped records, print a human-readable result,
and fail nonzero on a truncated BAM.

The walking skeleton is intentionally disposable. It does not establish the final
schema or output contract.

### 6.2 v0.1 release interface

```bash
aligngauge qc \
  --input sample.bam \
  --outdir results \
  --threads 8 \
  --memory-limit 4GiB \
  --coverage-thresholds 1,10,20,30
```

Required v0.1 options:

- `--input <PATH>`
- `--outdir <PATH>`

Optional v0.1 options:

- `--threads <N>`
- `--io-threads <N>`
- `--memory-limit <SIZE>`
- `--coverage-thresholds <LIST>`
- `--config <PATH>`
- `--log-format <human|json>`
- `--quiet`
- `--verbose`
- `--preserve-failed-staging`

The following options are not part of v0.1:

- `--reference`
- `--targets`
- `--profile targeted`
- `--backend`
- `--cuda-device`

### 6.3 Configuration precedence

Configuration precedence is:

1. built-in defaults;
2. configuration file;
3. explicitly documented environment variables;
4. CLI arguments.

The fully resolved configuration is written to provenance.

Unknown configuration keys are fatal by default. A future migration tool may
provide controlled schema upgrades.

## 7. v0.1 BAM input contract

### 7.1 Format and access

v0.1 accepts local BAM files conforming to the maintained SAM/BAM specification.

A whole-file streaming pass does not require an index. If an index is present it
may be inspected, but v0.1 correctness shall not depend on indexed partitioning.

### 7.2 Coordinate order

The header sort declaration is not sufficient. During traversal, AlignGauge shall
detect coordinate regressions among mapped records.

Policy:

- mapped records must be nondecreasing by reference ID and position;
- unmapped-tail placement shall follow the supported SAM/BAM ordering policy;
- a material regression is fatal;
- the diagnostic shall identify the prior and current record coordinates without
  logging sensitive read names by default.

### 7.3 Record corruption

Truncation, malformed record lengths, invalid CIGAR encodings, overflowing
coordinates, and decoder errors are fatal.

No completed output directory may be published after such an error.

### 7.4 Oversized CIGAR policy

BAM records that use the `CG` tag representation for more than 65,535 CIGAR
operations shall be either:

- expanded correctly by the pinned backend and processed; or
- rejected explicitly as unsupported.

Undefined behavior is prohibited. The pinned backend behavior shall be tested with
a named fixture before v0.1 is released.

### 7.5 Untrusted input

Alignment fields, tags, read-group values, contig names, and paths are untrusted.

The implementation shall:

- enforce checked integer conversions;
- bound allocations derived from file values;
- avoid using record content as a format string or path;
- avoid implicit subprocess execution;
- avoid implicit network access;
- redact read names from routine errors unless diagnostic mode is explicitly
  enabled.

## 8. v0.2 CRAM reference contract

This section is normative for v0.2 and shall influence the v0.1 I/O boundary.

Before reference-dependent CRAM record decoding, AlignGauge shall establish a
fail-closed local-only reference policy. The implementation shall not rely on
process-global environment mutation when inherited provider state can instead be
made non-authoritative by construction.

Opening a local CRAM container far enough to read its header and obtain `@SQ`
reference requirements is permitted before FASTA validation, provided the pinned
production HTSlib build has no remote reference transport and record traversal has
not begun.

Requirements:

- pin the exact HTSlib/rust-htslib version;
- verify version-specific `REF_PATH`, `REF_CACHE`, `HTS_PATH`, metadata/provider,
  and MD5-lookup behavior that could otherwise influence reference selection;
- compile the production HTSlib stack without HTTP/HTTPS or other remote reference
  transport features;
- require an explicit local FASTA for CRAM analysis;
- validate required contig names, lengths, and `M5` identities against that FASTA
  before reference-dependent record traversal;
- ensure inherited provider state cannot select an alternate reference after the
  explicit FASTA is supplied;
- prohibit fallback from a supplied but mismatched FASTA to any other local or
  remote reference;
- run the hostile-provider CRAM case in a network-disabled sandbox and observe
  network syscalls where the platform supports it;
- fail if a required sequence cannot be resolved locally;
- name the missing contig and expected MD5 in the diagnostic where available;
- record the actual FASTA identity and per-contig validation in provenance.

Process-global mutation of `REF_PATH`, `REF_CACHE`, or `HTS_PATH` is not required
and should be avoided when it would introduce race-prone global state. The release
evidence shall instead prove that hostile inherited values are non-authoritative
and that the CRAM reference-resolution path cannot access the network.

## 9. BED contract for v0.3

BED coordinates are zero-based and half-open.

The parser shall skip:

- blank lines;
- lines beginning with `#`;
- UCSC `track` lines;
- UCSC `browser` lines.

It shall accept CRLF and trailing whitespace.

An interval line is fatal when it contains:

- non-numeric required coordinates;
- negative coordinates;
- start greater than end;
- arithmetic overflow;
- a contig not permitted by the selected unknown-contig policy.

The parser shall:

- normalize line endings;
- preserve original interval identity and optional name;
- sort deterministically;
- merge overlaps for aggregate metrics while retaining the mapping back to source
  intervals;
- record every normalization action in provenance.

AlignGauge shall never infer one-based coordinates.

## 10. Canonical output contract

### 10.1 Required files

A completed v0.1 output directory contains:

- `summary.json`
- `provenance.json`
- optional requested compatibility files
- `_SUCCESS`

### 10.2 Atomic publication

Outputs are built in a staging directory on the same filesystem as the final
destination.

Publication order:

1. write all required files into staging;
2. flush and synchronize required file contents;
3. write `_SUCCESS` into staging as the final file;
4. synchronize staging metadata where supported;
5. atomically rename staging to the destination.

`_SUCCESS` is retained for ecosystem compatibility. It is never written after
publication.

If the platform cannot provide the required atomic rename semantics, AlignGauge
shall fail before processing unless a future explicitly named non-atomic mode is
selected.

On failure:

- the destination must not expose a partial completed run;
- `_SUCCESS` must not exist at the destination;
- staging is removed by default;
- `--preserve-failed-staging` may preserve it under a clearly incomplete name.

### 10.3 Canonical JSON

`summary.json` shall contain:

- `schema_version`;
- application version and Git commit;
- metric definitions and units;
- alignment counters;
- per-reference counters;
- coverage policy;
- coverage histogram and thresholds;
- unavailable metrics represented explicitly, never as zero;
- warning summary.

`provenance.json` shall contain:

- resolved configuration;
- input path and identity;
- input size and checksum policy;
- header identity;
- backend library versions;
- analysis plan;
- resource limits;
- stage timings;
- normalization actions;
- compatibility profiles;
- warnings and errors;
- operating-system and CPU information needed for reproducibility.

No GPU fields are required before a released GPU feature exists.

### 10.4 Schema evolution

The schema version is independent of the application version.

Rules:

- additive optional fields may retain the schema major version;
- semantic changes require a schema version change;
- unknown required fields are not ignored by strict consumers;
- golden schemas are committed and tested.

## 11. v0.1 alignment classification

### 11.1 Counter partitions

At minimum, v0.1 records:

- total records;
- QC-pass and QC-fail totals;
- primary records;
- secondary records;
- supplementary records;
- mapped and unmapped records;
- paired records;
- proper pairs;
- read 1 and read 2;
- mate mapped and mate unmapped;
- duplicates;
- singletons;
- per-reference mapped counts;
- per-reference unmapped counts where the compatibility definition supports them.

### 11.2 Classification priority

A record carrying both secondary and supplementary bits shall follow the pinned
`samtools flagstat` compatibility profile’s classification priority. It shall not
be independently added to both mutually exclusive top-level categories.

The exact pinned Samtools version and expected fixture outputs are owned by the test
manifest.

### 11.3 Counter types

Counters use checked `u64` accumulation. Overflow is fatal.

No counter saturates silently.

## 12. v0.1 coverage semantics

### 12.1 Named default profile

The v0.1 canonical coverage profile is `aligngauge-v0.1`.

It includes records that are:

- mapped;
- primary;
- QC-pass;
- not marked duplicate.

It excludes:

- unmapped records;
- secondary records;
- supplementary records;
- QC-fail records;
- duplicate records.

The profile has:

- minimum MAPQ: `0`;
- no base-quality filter;
- no mate-overlap correction;
- no implicit clipping adjustment beyond CIGAR semantics.

This is a project-defined canonical profile. Compatibility with a particular
mosdepth or Picard profile is not claimed until separately validated and named.

### 12.2 CIGAR semantics

Covered reference bases are emitted for:

- `M`
- `=`
- `X`

Not covered:

- `I`
- `D`
- `N`
- `S`
- `H`
- `P`

CIGAR arithmetic uses checked coordinates. An operation extending past the declared
reference length is fatal.

### 12.3 Chunked sweep

v0.1 shall implement one exact coverage accumulator: a parameterized chunked sweep
over coordinate-sorted records.

Whole-contig processing is the case where the selected chunk spans the contig.
Target-focused processing in v0.3 is a scheduling parameterization over normalized
target regions, not an independent coverage algorithm.

The implementation shall preserve exactness across chunk boundaries, including:

- blocks ending in a future chunk;
- very long deletions or reference skips;
- supplementary records under profiles that later include them;
- empty regions;
- contig transitions.

Chunk size is selected by the planner under the memory limit and recorded in
provenance.

### 12.4 Multiple tracks

Every distinct coverage policy requires its own logical track.

The planner’s memory estimate shall include:

- number of active tracks;
- bytes per delta entry;
- chunk length;
- pending cross-chunk events;
- histogram/reduction state;
- reader buffers;
- output buffers;
- safety margin.

A plan exceeding `--memory-limit` is rejected before traversal.

### 12.5 Coverage outputs

v0.1 produces:

- total accepted aligned bases;
- per-reference covered bases;
- per-reference mean depth;
- whole-run depth histogram;
- cumulative percentages at configured thresholds;
- uncovered reference bases where reference lengths are known.

Median and percentile calculations shall use integer histogram counts and documented
rounding.

### 12.6 Mate-overlap correction

Mate-overlap correction is not part of v0.1.

When added, it shall have its own named policy. Exact overlap correction shall force
a supported execution mode that can guarantee pair semantics. Indexed partition
parallelism shall not be combined with exact overlap correction until a design and
differential proof exist.

The specification shall name the reference tool being matched; “reference-tool
profile” without a named tool and version is insufficient.

## 13. Architecture

### 13.1 Initial workspace

The initial workspace should remain small:

```text
crates/
├── aligngauge-cli/
├── aligngauge-core/
├── aligngauge-hts/
├── aligngauge-metrics/
├── aligngauge-coverage/
├── aligngauge-formats/
└── aligngauge-testkit/
```

Crates may be merged if the boundaries add more ceremony than value. The walking
skeleton should be implemented before these boundaries are treated as stable.

### 13.2 I/O boundary

The I/O layer wraps rust-htslib and exposes only the validated fields required by
the active plan.

A conceptual interface is:

```rust
pub trait AlignmentSource {
    fn header(&self) -> &AlignmentHeader;
    fn next_record(&mut self) -> Result<Option<RecordView<'_>>>;
}
```

This is illustrative, not frozen API. The walking skeleton shall determine whether
borrowed views, owned normalized records, or batched records provide the safest
ergonomics.

### 13.3 Collector dispatch

The hot path shall avoid per-record heap allocation and avoid unnecessary dynamic
dispatch.

Acceptable designs include:

- a statically composed collector tuple;
- an enum-based plan compiled before traversal;
- generated or macro-assisted dispatch;
- batched normalized records where profiling justifies copying.

The project shall not commit to a complex planner abstraction until the walking
skeleton and first real collectors expose actual requirements.

### 13.4 Execution model by release

v0.1:

- one streaming alignment reader;
- HTSlib I/O/decompression threads where safe;
- deterministic collector accumulation;
- deterministic final reduction.

Later indexed parallel mode may use independent readers over disjoint partitions,
but its planner must account for:

- one file descriptor per reader;
- one set of HTSlib buffers per reader;
- decompression thread pools;
- duplicated index/reference state;
- memory-limit impact;
- overlap-correction incompatibilities.

## 14. Error taxonomy

The specification owns the error taxonomy. The TODO references this section.

Required categories:

- `usage`
- `configuration`
- `input_not_found`
- `input_format`
- `input_corrupt`
- `input_unsorted`
- `unsupported_record`
- `reference_required`
- `reference_mismatch`
- `target_format`
- `target_contig`
- `resource_limit`
- `output_exists`
- `output_io`
- `compatibility_unavailable`
- `internal_invariant`

Errors shall provide:

- stable category;
- human-readable message;
- causal context;
- nonzero exit code;
- optional structured details;
- no misleading fallback advice.

Warnings are reserved for conditions where all requested outputs remain correct and
fully defined.

## 15. Test corpus

The specification owns the required fixture set.

### 15.1 Synthetic fixtures

The fixture generator shall cover:

- empty valid BAM;
- mapped and unmapped records;
- every ordinary CIGAR operation;
- long CIGAR via `CG`;
- clipping combinations;
- insertions, deletions, and long reference skips;
- secondary records;
- supplementary records;
- records carrying both bits;
- duplicate and QC-fail records;
- paired, singleton, orphan, and discordant records;
- missing `NM` and `MD` tags;
- malformed optional tags;
- missing or contradictory read groups;
- unknown read-group IDs;
- coordinate regressions;
- unmapped tails;
- contig-name mismatches;
- zero-length references where legal;
- pads where supported;
- integer-boundary cases;
- truncated BGZF blocks;
- malformed BAM record lengths;
- chunk-boundary coverage events;
- multiple simultaneous coverage tracks.

Each fixture has:

- generation source;
- expected validity;
- expected error category or canonical result;
- pinned reference-tool outputs where applicable.

### 15.2 Small real dataset

v0.1 shall validate on a documented HG002 GRCh38 subset, initially approximately
one megabase of chromosome 20 at representative depth.

The manifest records:

- source URL or accession;
- source checksum;
- region;
- downsampling seed and fraction;
- local SHA-256;
- reference build;
- generation commands;
- tool versions;
- redistribution policy.

Large data are downloaded or generated, not committed.

### 15.3 Differential tools

v0.1 differential baselines:

- pinned Samtools `flagstat`;
- pinned Samtools `idxstats`;
- pinned coverage baseline selected and documented by ADR.

Later versions add Picard, mosdepth, and MultiQC baselines as their profiles enter
scope.

Differential tests run in a network-disabled environment.

## 16. Document ownership

To prevent divergence:

### 16.1 This specification owns

- product scope;
- version boundaries;
- metric semantics;
- input/output contracts;
- error taxonomy;
- fixture requirements;
- compatibility claims;
- release acceptance criteria.

### 16.2 The TODO owns

- task decomposition;
- implementation sequence;
- milestone gates;
- evidence files;
- status checkboxes.

The TODO shall reference specification sections instead of duplicating lists or
definitions.

## 17. Performance methodology

Performance claims require:

- exact input identity;
- tool and commit identity;
- CPU, RAM, storage, OS, and filesystem;
- cold/warm-cache distinction;
- thread counts;
- minimum three measured runs after warm-up where practical;
- wall-clock time;
- CPU time;
- peak RSS;
- bytes read where measurable;
- output equivalence status.

v0.1 performance gates are modest:

- no pathological regression versus a simple rust-htslib traversal;
- memory remains within the configured limit;
- the single-pass implementation demonstrates that enabling counters plus coverage
  does not trigger a second input traversal.

No specific speedup factor is promised before measurements exist.

## 18. Security and privacy

AlignGauge processes potentially identifying genomic data.

Requirements:

- local processing by default;
- no telemetry by default;
- no implicit network access;
- no read names in routine logs;
- no sample identifiers in crash reports unless explicitly enabled;
- restrictive temporary-file permissions;
- deterministic cleanup;
- dependency auditing;
- fuzzing of untrusted parsers and record boundaries;
- clear warning that outputs may contain sensitive genomic summaries.

## 19. Release acceptance criteria

### 19.1 v0.1

v0.1 is complete only when:

1. the walking skeleton has been replaced by production code;
2. coordinate-sorted BAM input is validated;
3. truncated/corrupt/unsorted fixtures fail with correct categories;
4. flag classification matches the pinned Samtools profile on all applicable
   fixtures;
5. per-reference counts match the pinned profile;
6. canonical coverage matches the defined `aligngauge-v0.1` semantics;
7. chunk boundaries do not alter results;
8. multi-track memory planning is enforced;
9. canonical JSON and provenance validate against committed schemas;
10. missing metrics are never represented as zero;
11. output publication is atomic and `_SUCCESS` ordering is tested;
12. repeated runs produce identical canonical output after excluding explicitly
    volatile timing fields;
13. the small HG002 subset has a reconciled validation report;
14. permanent CI passes on the exact release commit;
15. documentation states all non-goals and known limitations.

### 19.2 v0.2

v0.2 additionally requires:

- CRAM reference retrieval cannot access the network;
- missing/mismatched references fail;
- BAM and equivalent CRAM canonical results agree;
- CRAM corruption fixtures pass;
- reference provenance identifies actual local data used.

### 19.3 v0.3 and later

Each later release has an evidence report mapping every added compatibility or
metric claim to:

- a specification section;
- fixtures;
- differential output;
- unresolved differences;
- performance impact;
- release commit and CI result.

## 20. Open ADRs

The following decisions shall be captured as ADRs before their milestone begins:

1. pinned rust-htslib/HTSlib versions;
2. v0.1 coverage baseline tool;
3. canonical checksum strategy for large inputs;
4. output-directory overwrite policy;
5. exact BED unknown-contig policy;
6. fold-80 compatibility versus named equivalent;
7. indexed partition execution design;
8. exact mate-overlap correction profile;
9. long-read/oversized-CIGAR support policy;
10. any hardware-acceleration admission decision.
