# Review: DNA QC Engine specification and implementation TODO

**Reviewer:** Claude (Opus 5)
**Date:** 2026-08-06
**Documents reviewed:**
- `docs/DNA_QC_ENGINE_SPEC.md` (rev. 2026-08-05)
- `docs/DNA_QC_ENGINE_TODO.md` (rev. 2026-08-05)
**Repository state at review time:** `master` @ docs-only — no `Cargo.toml`, no `crates/`, no source.

This document records review comments only. It is not a specification change.
Nothing here has been applied to either source document.

---

## 0. How to read this

Findings are grouped by severity:

- **S1 — Blocking:** should be resolved before implementation begins, because the
  decision shapes code that would otherwise be written twice.
- **S2 — Significant:** a real defect, gap, or contradiction in the documents.
  Fixable during the relevant milestone, but must not be discovered by the
  implementation.
- **S3 — Minor:** gaps, omissions, and hygiene items.

Each finding names the affected section(s) so it can be traced back.

---

## 1. Overall assessment

The design quality is above the norm for this stage. Several choices are ones
that comparable projects typically get wrong and pay for later:

- Fail-closed behavior stated as an enforceable contract rather than an aspiration
  (SPEC §15, TODO "Global engineering rules").
- An explicit prohibition on zero-as-missing-data (SPEC §10.2, TODO global rules).
  This single rule prevents a large class of silent QC corruption.
- Compatibility outputs derived from the canonical model rather than accumulated
  independently (SPEC §9.3). Independent accumulation is how compatibility layers
  drift out of agreement with their own tool's native output.
- Test-data manifest and differential harness sequenced *before* analysis code
  (TODO Milestone 2). Correct, and rarely done.
- The CUDA acceptance gate requiring measured **end-to-end** improvement rather
  than kernel-only speedup (SPEC §14.5). This is the specific trap most GPU
  bioinformatics work falls into.
- Provenance treated as a first-class required output rather than a log (SPEC §9.2, §21).

The problems are concentrated in **scope**, **sequencing**, and a small number of
specific technical landmines. The semantics themselves are largely sound.

---

## 2. S1 — Blocking findings

### 2.1 v0.1 as specified is not a v0.1

**Affects:** SPEC §26, TODO Milestones 10–11

The v0.1 acceptance criteria require, in a single release:

- BAM **and** CRAM support
- correctness-matched subsets of `samtools stats`, `samtools flagstat`,
  `samtools idxstats`, mosdepth, Picard `CollectAlignmentSummaryMetrics`,
  `CollectInsertSizeMetrics`, `CollectWgsMetrics`, and `CollectHsMetrics`
- WES/panel targeted metrics with enrichment and fold-80
- deterministic multithreaded execution with serial equivalence
- a full ~30× HG002 WGS differential run with every discrepancy resolved
- MultiQC parser validation
- SBOM, signed artifacts, fuzzing campaigns, sanitizer runs

This is a multi-person-year scope carrying a 0.1 label. Milestone 10 alone is
months of calendar time: a 30× HG002 WGS alignment is ~100 GB+, Picard
`CollectWgsMetrics` at that scale runs for hours per invocation, and the plan
requires baselines from three separate tools plus three of your own
configurations (serial CPU, optimized CPU, CUDA), then reconciliation of every
field.

**Recommended change.** Redefine v0.1 as:

- BAM only (CRAM → v0.2)
- CPU only (see §2.3)
- flagstat-equivalent counters, idxstats-equivalent counters, exact genome coverage
- validated against the synthetic corpus and the 1 Mb chr20 HG002 subset
- canonical JSON + provenance + atomic publish

Then: CRAM in v0.2, targeted/WES metrics in v0.3, Picard compatibility in v0.4,
full-scale HG002 validation in v0.5. No architectural change is required — this
only moves the release line to something reachable, and lets the fail-closed and
determinism guarantees be proven on a small surface before the surface grows.

The current structure risks the common failure mode where a project is 70%
complete against an unreachable definition for two years and never ships.

### 2.2 There is no vertical slice

**Affects:** TODO Milestones 0–3, "Immediate next work"

The recommended first sequence produces no user-visible output. Workspace
bootstrap, CI, quality gates, error taxonomy, configuration model, plan types,
fixture generator, manifest schema, pinned reference-tool containers, and the
htslib boundary all land before the tool prints a single number.

That is a large amount of scaffolding built against untested assumptions —
specifically about the lifetime and shape of `RecordView<'_>`, what
`RequiredFields` actually needs to express, and how the collector dispatch
composes without per-record dynamic calls (SPEC §12.2, §12.3, §12.4). These are
exactly the APIs that change once real records flow through them.

**Recommended change.** Insert a **Milestone 0.5 — walking skeleton**, sized at
roughly one week, deliberately unpolished and explicitly disposable:

1. `qc --input x.bam` opens a BAM through `rust-htslib`.
2. Counts total / mapped / unmapped records.
3. Prints them to stdout.
4. Exits nonzero on a truncated file.

No plan types, no JSON schema, no staging directory. The goal is to prove the
CLI → plan → htslib → collector → output shape end-to-end and to discover the
`RecordView` ergonomics before they are frozen into eight crates. Everything
after Milestone 0.5 becomes filling in a skeleton that is known to work, rather
than assembling parts that have never been connected.

### 2.3 CUDA is likely a dead end, and it is taxing the whole design

**Affects:** SPEC §1, §7.2, §14 (all), §12.3, §27; TODO Milestone 9

The discipline around CUDA is good — optional, CPU-authoritative, gated on
measured benefit, no silent fallback. The problem is not the guardrails. It is
that the guardrails are almost certainly going to reject the feature, and in the
meantime it is imposing cost across the entire design.

**The arithmetic argument.** For a 30× WGS BAM the workload is:

- read ~100 GB from storage
- BGZF-decompress to ~300 GB
- per record: parse a CIGAR, and increment two positions in a delta array

That is approximately zero floating-point work per byte moved. The named first
candidates in SPEC §14.2 — coverage event accumulation, prefix scan, histogram
and threshold reduction, interval aggregation — are all in this regime. Building
the structure-of-arrays batch (SPEC §14.3) and moving it across PCIe will cost
more than the CPU spends performing the increments. The honest prediction is that
none of these clear the §14.5 gate.

**What it costs meanwhile.** `--backend` in the CLI (§7.2), `Backend` fields on
every collector in `ExecutionPlan` (§14.4), the `rds-gpu` crate (§12.1), the
auto-selection policy and its benchmark corpus (§14.1, TODO 9.7), plus provenance
fields, configuration surface, planner branches, and test matrix entries. All of
that is being maintained for a code path that will always resolve to `cpu`.

**Recommended change.** Remove `--backend` from the v0.1 CLI. Remove the
per-collector `Backend` fields from `ExecutionPlan`. Keep `rds-gpu` out of the
workspace. Convert Milestone 9 into an ADR-documented research spike with no
release obligation. Adding a backend flag later, once something has actually
demonstrated benefit, is a trivial additive change; a flag that only ever takes
one value is permanent dead weight in config parsing, provenance schema, docs,
and tests.

**The candidate ranking is inverted.** SPEC §14.2 lists coverage accumulation
first and places GPU BGZF decompression last, as "a research prototype after
profiling proves CPU decompression dominates" (TODO 9.6). But SPEC §2 already
asserts that decompression dominates — that is the stated premise of the entire
project. And decompression is the *only* candidate in the list with meaningful
arithmetic intensity per byte transferred: it does real work on the compressed
bytes rather than trivial work on already-decoded fields. Production GPU deflate
implementations exist (nvCOMP).

If any GPU work is prototyped, prototype decompression first. It targets the
actual bottleneck; the current list targets the cheapest part of the pipeline.

---

## 3. S2 — Significant findings

### 3.1 htslib can silently fetch CRAM reference sequences over the network

**Affects:** SPEC §8.2, §20, §19; TODO 3.4, 2.3

This is the highest-priority correctness item in the document set, because it
silently violates principles the specification states explicitly.

htslib's CRAM reference resolution consults the `REF_PATH` and `REF_CACHE`
environment variables. The compiled-in default for `REF_PATH` has historically
included a remote lookup against the EBI CRAM reference registry
(`https://www.ebi.ac.uk/ena/cram/md5/%s`). When a required sequence cannot be
resolved locally — including when the supplied `--reference` does not match a
contig's MD5 — htslib may reach out over the network and retrieve it, without any
signal at the API level.

Consequences, each of which contradicts a stated requirement:

- SPEC §8.2: "The implementation shall not guess between references." A silent
  remote fetch is exactly that, and it can succeed, producing plausible-looking
  metrics computed against a reference the operator never supplied.
- SPEC §20: alignment and reference files are to be treated as untrusted input,
  and content is never to be executed or fetched implicitly.
- SPEC §19 / §16: benchmarks and determinism claims become network-dependent and
  non-reproducible, with wall-clock times that vary with EBI availability.
- SPEC §9.2: provenance would record a reference identity that does not describe
  what was actually used.

**Recommended change.** Add an explicit requirement in §8.2 that the process
pins `REF_PATH` and `REF_CACHE` to local-only values before any CRAM handle is
opened, that remote reference resolution is disabled unconditionally, and that
failure to resolve a sequence locally is fatal with an actionable diagnostic
naming the contig and its expected MD5. Add a TODO 3.4 item to verify the actual
behavior against the pinned htslib version, and a TODO 2.3 item to assert the
sandbox has no network access during differential runs so a regression here is
caught rather than hidden.

Verify the specific default against the htslib version you pin — the details
have shifted across releases — but design as though it is present.

### 3.2 Overlap correction and indexed parallel mode do not compose

**Affects:** SPEC §12.5, §13.3; TODO 5.6, 8.3

SPEC §13.3 specifies mate-overlap correction with "a bounded memory strategy" for
pairing state. SPEC §12.5 separately specifies indexed parallel mode with
"independent readers over disjoint reference partitions." The two are described
independently, as though they compose. They do not.

Under partitioned parallel execution, a read's mate may be:

- in a different partition, owned by a different thread with its own accumulators;
- on a different contig entirely;
- beyond the bounded pairing window, at any distance permitted by the library;
- present as a supplementary record whose overlap semantics differ from the primary.

This is the hardest correctness problem in the project, and it currently occupies
one paragraph in the spec and seven undifferentiated checkboxes in the TODO. It
needs its own design section stating: how pairing state is partitioned or shared,
what the bound is and what happens at the bound, and whether requesting
overlap-corrected coverage forces streaming mode.

The existing escape hatch — fail or mark the output unavailable when exact
correction cannot be guaranteed (SPEC §13.3) — is the right default, but it
should be a *designed* outcome, not a runtime surprise discovered on real data.

**Related:** SPEC §13.3 and TODO 5.6 say to compare against "a pinned
reference-tool profile." Samtools and mosdepth do not agree with each other on
overlap handling. The specification must name which one is being matched and
document the delta against the other, or the compatibility claim is
underdetermined.

### 3.3 Three coverage memory strategies where one would do

**Affects:** SPEC §13.2, §26.5; TODO 5.2, 5.3, 6.2

SPEC §13.2 mandates three exact strategies — whole-contig arrays, chunked arrays,
target-focused arrays — and then §26.5, TODO 5.3, and TODO 6.2 require proving
they all produce identical results across the full fixture corpus. That is three
implementations plus a cross-validation obligation, for one algorithm.

But the input contract already guarantees coordinate-sorted records (SPEC §8.4).
Under that guarantee a chunked sweep with chunk size ≈ maximum read span *is* a
streaming algorithm with memory bounded by read span rather than contig length.
The whole-contig strategy is the degenerate case where chunk size equals contig
length. The target-focused strategy is the case where chunk boundaries are drawn
at target intervals.

**Recommended change.** Make chunked accumulation the single strategy, with chunk
size as a planner-selected parameter, and describe whole-contig and
target-focused as parameterizations rather than separate implementations. The
memory ceiling is preserved, the planner logic is preserved, and two
implementations plus a cross-validation requirement disappear from the critical
path.

If the three are kept deliberately for performance reasons, the spec should say
so explicitly and cite the expected gain, because as written the reader cannot
tell whether the multiplicity is intentional.

### 3.4 Coverage memory estimate does not account for simultaneous tracks

**Affects:** SPEC §10.3, §13.2

SPEC §10.3 permits "optional duplicate-inclusive and duplicate-excluded tracks,"
and coverage results are parameterized by flag mask, MAPQ threshold, base-quality
threshold, deletion policy, skip policy, and overlap policy. Each distinct
parameterization requires its own accumulator.

SPEC §13.2's planner estimate is written as though there is one array. On
chromosome 1 (~250 Mb) with a 32-bit delta representation that is ~1 GB per
track; two tracks is ~2 GB before any other allocation.

**Recommended change.** State in §13.2 that the memory estimate is computed over
the full set of active coverage parameterizations, and that the planner counts
tracks explicitly. Add a TODO 5.2 item for a multi-track memory-budget test.

### 3.5 `_SUCCESS` and atomic publication are redundant, and the ordering contradicts

**Affects:** SPEC §9.1, §9.2; TODO 1.4

SPEC §9.1 requires building output in a staging directory and publishing the
completed set atomically. SPEC §9.2 additionally requires a `_SUCCESS` file
"created only after all required outputs are complete and synchronized."

If the staging directory is renamed into place atomically, the existence of the
destination directory *is* the completion marker. `_SUCCESS` is a convention
inherited from systems that build output in place and therefore cannot express
atomic publication — it is solving a problem that §9.1 has already solved.

More importantly, the two requirements conflict on ordering. `_SUCCESS` must be
written *inside* staging, before the rename, or it cannot be part of the
atomically published set. But §9.2 read literally ("after all required outputs
are complete") suggests it is written last, which if interpreted as
post-publication reintroduces exactly the non-atomic window the design eliminates.

**Recommended change.** Either drop `_SUCCESS` and document that directory
existence is the contract, or keep it for ecosystem compatibility and state
explicitly that it is written into staging as the final step *before* the atomic
rename. Add a TODO 1.4 test asserting that no partially built staging directory
is ever observable at the destination path.

### 3.6 §16 overclaims floating-point determinism relative to §13.4

**Affects:** SPEC §13.4, §16

SPEC §16 asserts that canonical metric values are identical across repeated runs
and that "CPU and CUDA backends shall produce identical integer results," while
also requiring deterministic floating-point reduction. Bit-identical
floating-point results across CPU and GPU are difficult to guarantee in general:
FMA contraction differs, libm implementations differ, and reduction trees differ.

SPEC §13.4 already contains the escape: integer counts stay integers internally,
and floating-point means and percentages are computed during a deterministic
reduction. But §16 does not connect to it, so the claim reads stronger than the
mechanism that supports it.

**Recommended change.** State explicitly in §16 that all floating-point
computation occurs CPU-side in a single final reduction over integer inputs, and
that therefore backend equivalence is an integer-equivalence claim. This makes
the guarantee both weaker on paper and actually keepable — which is the correct
trade.

### 3.7 Specification and TODO have already begun to diverge

**Affects:** SPEC §15 / TODO 1.1; SPEC §17.1 / TODO 2.2; SPEC §10 / TODO 4.2–4.4;
SPEC §26 / TODO 11

Both documents independently enumerate the same content: error categories,
synthetic fixture cases, metric lists, and acceptance criteria. At roughly 40 KB
each, divergence is inevitable — and it has already occurred at the first commit.

Concretely, the fixture lists do not match. SPEC §17.1 includes "zero-length
references where legal," "contig naming mismatches," and "unsorted records."
TODO 2.2 includes "pads where supported," "missing NM/MD tags," and "coordinate
regressions." Neither list is a superset of the other, and it is not stated which
is authoritative.

**Recommended change.** Assign ownership: the specification owns semantics,
metric definitions, error taxonomy, and fixture requirements; the TODO owns
sequencing, decomposition, and evidence. The TODO should reference
"per SPEC §17.1" rather than restate. Reconcile the two fixture lists into one in
the spec as part of this change. One source of truth per fact, or these will be
contradicting each other within a month of implementation starting.

---

## 4. S3 — Minor findings and gaps

### 4.1 BAM records with more than 65,535 CIGAR operations

**Affects:** SPEC §4 (non-goals), §13.1, §17.1; TODO 5.1, 2.2

Long-read support is an explicit non-goal, but such records can still be
*encountered* in an input file. BAM stores oversized CIGARs via the `CG` tag
workaround, with a placeholder in the fixed-length field. The specification does
not say whether these are handled, rejected, or undefined.

Given the fail-closed posture, undefined is not acceptable. Add an explicit
policy and a named fixture. Confirm how the pinned `rust-htslib` surfaces the
expansion.

### 4.2 Picard insert-size compatibility is understated

**Affects:** SPEC §9.3, §10.2; TODO 4.3, 6.4

Picard's insert-size metrics are computed over a histogram trimmed using a
median-absolute-deviation cutoff (the `DEVIATIONS` parameter). Reproducing
`MEDIAN_INSERT_SIZE`, `MEDIAN_ABSOLUTE_DEVIATION`, and the width percentiles
means reimplementing that trimming exactly, including its tie-breaking and
rounding.

This is currently one bullet among many. It deserves its own TODO item with its
own differential fixture, because it is a place where "close enough" silently
produces numbers that differ from Picard in the third significant figure and are
then rationalized as a tolerance.

The hedge on fold-80 ("or an explicitly documented equivalent," SPEC §10.4) is
well-judged and should be applied to the insert-size metrics too.

### 4.3 BED parser will reject real vendor capture files

**Affects:** SPEC §8.3; TODO 6.1

Production capture-target BEDs from vendors routinely contain `track` and
`browser` lines, `#` comments, blank lines, trailing whitespace, and CRLF line
endings. TODO 6.1 says to "reject negative, reversed, overflowing, or non-numeric
coordinates" — a `track` line will be classified as non-numeric and rejected,
failing the run on a perfectly valid file.

Specify which non-interval lines are skipped versus fatal. Keep the strict stance
on coordinate interpretation (SPEC §8.3's refusal to infer one-based is correct
and should not be softened), but distinguish "line is not an interval" from
"interval is malformed."

### 4.4 Indexed parallel mode implies N readers, N thread pools, N descriptors

**Affects:** SPEC §12.5; TODO 3.6, 8.3

Independent readers over disjoint partitions means one `IndexedReader` per worker,
each with its own htslib decompression thread pool, file descriptor, and internal
buffers. At high thread counts this multiplies both resident memory and FD usage
in ways the memory budget (SPEC §7.2 `--memory-limit`) currently does not model.

Add the reader count to the planner's memory estimate and note the descriptor
implication in §12.5.

### 4.5 The per-checkbox CI rule is unenforceable

**Affects:** TODO "How to use this TODO", item 6

"The exact commit being claimed has passed the relevant permanent CI gates" cannot
be verified by hand across roughly six hundred checkboxes, and will be ignored
within a month. The milestone-level acceptance evidence sections already carry
this weight properly.

Either drop the per-checkbox rule and rely on milestone gates, or automate it —
but do not leave a stated rule that is known not to be followed, since that
erodes the credibility of the other global rules, which are good and should be
followed.

### 4.6 Checklist noise

**Affects:** TODO 0.1

`Add Cargo.lock` is a byproduct of the first build, not a task. A handful of items
at this level dilute the signal of the surrounding checklist. Worth a pass to
remove anything that happens automatically.

### 4.7 Repository name

**Affects:** SPEC §1, §7; TODO 0.4

The specification opens by explaining that the repository name is wrong — v0.1 is
not sequencer control and not a basecaller — and TODO 0.4 requires a prominent
README disclaimer to that effect. The disclaimer is a permanent tax being paid to
avoid a rename.

Nothing external references the repository yet, so renaming is free right now and
gets more expensive every week. `rust-bam-qc`, `rust-align-qc`, or similar would
eliminate the disclaimer requirement entirely. If the name is being kept
deliberately, say so in the spec so the question is closed.

Minor related note: the `rds-` crate prefix collides with a widely used
abbreviation for a cloud database service, which will affect searchability.

### 4.8 Acceptance criterion 14 is a practical blocker

**Affects:** SPEC §26 item 14; TODO 10.1

"A full approximately 30× WGS benchmark and validation report exists" requires
sustained local storage for a ~100 GB alignment plus a reference, plus outputs
from three baseline tools, plus multiple runs of the tool under test. For a
solo-maintained project this is a hard resource constraint, not a scheduling one.

If §2.1's staging is adopted, this moves out of v0.1 naturally. If not, consider
reducing it to a single chromosome at full depth, which exercises the same code
paths at a fraction of the cost.

---

## 5. Suggested order of changes

If the recommendations above are accepted, the cheapest order to apply them is:

1. Rename the repository (§4.7) — free now, never cheaper.
2. Cut SPEC §26 to a BAM / CPU / counters-and-coverage release (§2.1).
3. Add the `REF_PATH` / `REF_CACHE` requirement to SPEC §8.2 (§3.1).
4. Add a design subsection for overlap correction under parallel execution, or
   state that overlap-corrected coverage forces streaming mode (§3.2).
5. Collapse SPEC §13.2 to a single parameterized strategy (§3.3).
6. Remove `--backend` and the `Backend` fields from the v0.1 surface; move
   Milestone 9 to an ADR spike (§2.3).
7. Reconcile the two fixture lists and establish document ownership (§3.7).
8. Resolve the `_SUCCESS` ordering (§3.5) and the §16 float claim (§3.6).
9. Write the Milestone 0.5 walking skeleton (§2.2) before returning to the
   Milestone 0 CI checklist.

Items 1–8 are edits to two documents and can be done in a single sitting. Item 9
is the first code.

---

## 6. Questions for the next reviewer

Points where a second opinion would be most useful, phrased so they can be
addressed directly:

1. Is the arithmetic-intensity argument in §2.3 correct for this workload, and
   does GPU BGZF decompression genuinely have better prospects than the currently
   listed first candidates?
2. Is the claim in §3.1 accurate for the htslib version likely to be pinned, and
   is pinning `REF_PATH` / `REF_CACHE` sufficient to close it, or are there other
   implicit-fetch paths in CRAM handling?
3. Does the streaming-sweep argument in §3.3 hold given the coordinate-sorted
   input contract, or is there a case — unmapped tails, very long reference skips,
   supplementary chains — where chunked accumulation cannot reproduce whole-contig
   results exactly?
4. For §3.2, is there a published approach to mate-overlap correction under
   partitioned parallel execution that preserves exactness, or is forcing
   streaming mode the only sound option?
5. Is the reduced v0.1 in §2.1 still coherent as a releasable artifact, or does
   dropping CRAM and targeted metrics remove too much to be useful to anyone?
