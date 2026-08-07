# Milestone 8 BED normalization evidence

**Milestone:** 8 — BED and target normalization  
**Evidence date:** 2026-08-07  
**Validated implementation source SHA:** `8bc7f6df8154f2f3162e8d498781873364b4c1ea`  
**Pre-evidence PR head:** `5441775343f6a84b25490c997be32270e351745d`  
**Disposition:** Implementation and required Milestone 8 tests are complete. The milestone acceptance gate remains open until Permanent CI succeeds on the exact evidence/TODO candidate commit.

## 1. Scope

Milestone 8 establishes the reusable BED parsing and deterministic target-normalization layer required by the v0.3 targeted-sequencing work. It deliberately does **not** compute targeted QC metrics and does not silently activate `qc --targets`; those consumers belong to Milestone 9.

The implementation adds the `aligngauge-formats` crate and keeps target-file semantics separate from HTS decoding, coverage accumulation, and CLI publication logic.

## 2. Normative policy

`docs/adr/ADR-0005-BED_UNKNOWN_CONTIG_POLICY.md` was accepted before parser implementation and is authoritative for Milestone 8 behavior.

The sequence dictionary supplied by the caller is authoritative:

- BED contig names must match exactly;
- `1` is not silently translated to `chr1` or vice versa;
- case is not normalized;
- external alias databases are not consulted;
- unknown contigs are fatal `target_contig` errors;
- out-of-range source coordinates are fatal rather than clipped.

This intentionally chooses deterministic failure over assembly-specific alias heuristics.

The BED surface follows the UCSC BED definition relevant to this milestone: three required fields, up to nine optional fields, consistent field count within one data set, zero-based starts, half-open ends, and zero-length features where start equals end. AlignGauge additionally rejects empty tab-delimited columns instead of collapsing them, so later optional fields cannot silently shift position.

Reference: UCSC Genome Browser FAQ, BED format: `https://www.genome.ucsc.edu/FAQ/FAQformat.html#format1`.

## 3. Parser behavior

`crates/aligngauge-formats/src/bed.rs` provides byte- and path-based BED parsing against an explicit `SequenceDictionary`.

The parser proves the SPEC §9 requirements:

- blank lines are skipped;
- `#` comment lines are skipped;
- UCSC `track` and `browser` directive lines are skipped;
- CRLF is accepted and normalization is counted;
- trailing ASCII space/tab is ignored;
- BED3 through BED12 records are accepted;
- one consistent record width is required throughout a data set;
- empty tab-delimited fields are rejected;
- required coordinates must be non-negative integers representable as `u64`;
- `start > end` is fatal;
- `start == end` remains a valid zero-length source feature;
- `end` beyond the authoritative contig length is fatal;
- unknown contigs are fatal and no alias is inferred;
- source record index, original one-based line number, optional BED name, and remaining optional fields are retained.

All parser failures use stable typed `AlignGaugeError` categories rather than best-effort recovery.

## 4. Target identity

`TargetFileIdentity` identifies the actual supplied BED bytes, not a reconstructed normalized representation. It records:

- caller-supplied path when parsing from a file;
- exact byte size;
- SHA-256 over the original bytes before CRLF/whitespace handling;
- accepted source-interval count.

The committed representative vendor-style BED6 fixture is:

`crates/aligngauge-formats/tests/fixtures/vendor_style_capture_panel.bed`

Pinned fixture identity:

- exact size: `275` bytes;
- SHA-256: `ebab4ad34dc4b17fbc418a3d3274003aa2d1927dae47f18f4cef6761c54b9094`;
- accepted source intervals: `4`;
- original source lines: `4`, `5`, `6`, `7` after `track`, `browser`, and comment records.

The fixture is a committed representative vendor-style capture-panel BED used to exercise the format contract; it is not represented as a proprietary vendor distribution.

## 5. Deterministic normalization

`normalize_targets` consumes only validated source intervals.

Normalization is deterministic:

1. zero-length source intervals remain present in the source set but contribute no aggregate territory;
2. configurable symmetric flanks are applied with checked arithmetic;
3. requested flank bases that cannot fit at coordinate zero or the contig end are clipped and counted explicitly;
4. intervals are sorted by authoritative dictionary contig order, start, end, then source identity;
5. overlapping positive intervals are merged;
6. directly adjacent half-open intervals are also merged because there is no uncovered base between them;
7. every merged interval retains the full set of contributing source interval indices;
8. aggregate territory uses checked `u64` arithmetic.

The implementation distinguishes reaching a contig boundary from clipping: if the full requested flank exactly reaches coordinate zero or the contig end, no clipping event is recorded. A clipping event is recorded only when fewer than the requested flank bases can be applied.

## 6. Representative normalization result

For the committed vendor-style fixture with a symmetric flank of five bases, normalization produces exactly three aggregate intervals:

| Contig | Start | End | Source indices |
| --- | ---: | ---: | --- |
| `chr1` | 5 | 40 | `0,1` |
| `chr1` | 85 | 105 | `2` |
| `chr2` | 0 | 17 | `3` |

Expected aggregate territory is exactly `72` bases. The first two overlapping `chr1` source records produce one overlap merge. The `chr2` record begins at coordinate 5, so a five-base left flank reaches coordinate zero exactly and is **not** counted as a clip.

Separate unit coverage proves direct-adjacency merging independently from overlap merging.

## 7. Source mapping and order independence

Merged intervals retain stable source interval indices. The source records themselves remain in source-file order with their original line numbers and names.

Integration coverage parses two different source permutations representing the same interval geometry and requires identical normalized aggregate geometry and identical aggregate territory. Input ordering therefore cannot change aggregate target territory.

## 8. Provenance

`TargetNormalizationProvenance` records deterministic normalization information including:

- profile `aligngauge-bed-v0.3`;
- target SHA-256 and byte size;
- source/positive/empty interval counts;
- configured flank;
- deterministic reorder count;
- overlap and adjacency merge counts;
- left/right flank clipping counts;
- merged interval count;
- aggregate territory;
- blank/comment/track/browser skip counts;
- CRLF normalization count.

`TargetSet::provenance_actions()` renders these as stable `targets:*` actions suitable for later integration into canonical run provenance by Milestone 9.

## 9. Validation coverage

Milestone 8 unit/integration tests cover:

- BED3 and BED12 independently;
- vendor-style BED6 input;
- blank/comment/`track`/`browser` handling;
- CRLF normalization and trailing whitespace;
- ASCII whitespace and tab separation;
- empty tab-delimited field rejection;
- inconsistent BED width rejection;
- negative, non-numeric, overflowing, reversed, and out-of-bounds coordinates;
- unknown contig rejection with no alias inference;
- duplicate sequence-dictionary contig rejection;
- missing target path typing;
- exact byte-size/SHA identity;
- deterministic sorting and source-order independence;
- overlap merging;
- adjacency merging;
- merged-to-source mapping;
- zero-length source intervals;
- flank clipping at both boundaries;
- deterministic provenance;
- deterministic mutation fuzzing over arbitrary byte inputs without panic.

Existing BAM, CRAM, coverage, release, reference, and testkit suites remain in the same full-workspace test run.

## 10. Validation result

The hardened implementation working tree was validated before product commit in Milestone 8 validation run `31197147271`, job `92928071302`:

- workspace check: success;
- strict Clippy with `-D warnings`: success;
- complete workspace test suite: success;
- generated-diff scope validation: success;
- validated product source commit: `8bc7f6df8154f2f3162e8d498781873364b4c1ea`.

The temporary builder and hardening helper were removed afterward and are absent from the permanent branch diff.

PR #2 then began the repository's standing Permanent CI, Full Runtime Validation, and Reference Validation workflows on pre-evidence head `5441775343f6a84b25490c997be32270e351745d`.

## 11. Known discrepancies and scope boundary

No known Milestone 8 parser/normalization discrepancy is being hidden.

The following are intentionally deferred rather than silently partially implemented:

- targeted `qc --targets` metric computation;
- target/near-target/off-target base accounting;
- per-target depth/dropout metrics;
- fold-enrichment and fold-80-style metrics;
- HG002 exome/target differential validation.

Those are Milestone 9 responsibilities and will consume the normalized `TargetSet` established here.

## 12. Acceptance state

Implementation tasks 8.1 and 8.2 are complete and have direct tests/evidence. Milestone 8 is not yet declared complete by this document because the repository's milestone rule still requires Permanent CI success on the exact evidence/TODO candidate commit.