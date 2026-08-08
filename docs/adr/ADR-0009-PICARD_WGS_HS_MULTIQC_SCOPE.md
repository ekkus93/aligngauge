# ADR-0009: Picard WGS/HsMetrics scope and MultiQC 1.35 validation

- **Status:** Accepted for Milestone 12
- **Date:** 2026-08-08
- **Picard reference:** 3.4.0
- **MultiQC reference:** 1.35
- **Depends on:** ADR-0006, ADR-0008
- **Blocks exact Picard WGS/Hs compatibility until:** Milestone 13 exact overlap correction

## Context

Milestone 12 must select the Picard whole-genome and hybrid-selection compatibility surfaces, define fold-80 behavior, and make the pinned MultiQC parser an executable CI contract. It must not create a false compatibility label merely because a text file can be discovered by MultiQC.

The existing native targeted profile (`aligngauge-targeted-v0.3`) deliberately has different semantics from Picard `CollectHsMetrics`. In particular, native `target_uniformity_penalty_80` is computed over the complete normalized target territory, including zero-depth bases, and is unavailable when its 20th-percentile depth denominator is zero. It is not Picard `FOLD_80_BASE_PENALTY`.

Picard 3.4.0 `CollectWgsMetrics` and `CollectHsMetrics` also remove overlapping mate observations from their strict coverage calculations. `CollectHsMetrics` enables overlapping-read clipping by default. Exact overlap correction is the next milestone, not a behavior that Milestone 12 may approximate or silently skip.

MultiQC 1.35 creates another hard boundary. Its Picard WGS parser directly consumes the exclusion fields used for the filtered-base plot, including `PCT_EXC_OVERLAP`, and its HsMetrics module exposes Picard fold-80 and target-coverage fields. MultiQC discovery therefore cannot be treated as proof of numerical equivalence.

## Decision

### 1. Pin the compatibility consumers

Milestone 12 retains:

- Picard `3.4.0` as the Picard semantic reference;
- MultiQC `1.35` using the immutable container digest already recorded in `tools/reference/multiqc/image.lock`.

All MultiQC validation runs with container networking disabled after the pinned image is pulled. A missing module output, parser error, version mismatch, or parsed-output mismatch is fatal.

### 2. Select the Picard WGS v0.4 candidate surface

The first exact `CollectWgsMetrics` compatibility profile will target the fields that drive the default MultiQC 1.35 WGS presentation plus the genome denominator:

- `GENOME_TERRITORY`
- `MEAN_COVERAGE`
- `SD_COVERAGE`
- `MEDIAN_COVERAGE`
- `PCT_30X`
- `PCT_EXC_MAPQ`
- `PCT_EXC_DUPE`
- `PCT_EXC_UNPAIRED`
- `PCT_EXC_BASEQ`
- `PCT_EXC_OVERLAP`
- `PCT_EXC_CAPPED`
- the high-quality coverage histogram consumed by MultiQC when present

This is a **selected candidate surface**, not an emitted AlignGauge compatibility profile in Milestone 12. Exact implementation and differential evidence are gated on Milestone 13 because overlap removal changes the coverage histogram, derived coverage statistics, threshold fractions, and the exclusion denominator.

The following Picard WGS surfaces are outside this first profile unless a later ADR adds them:

- theoretical heterozygous sensitivity outputs;
- base-quality histogram output;
- arbitrary user-selected Picard coverage-threshold columns beyond the selected default `PCT_30X` surface;
- Picard options whose semantics differ from the pinned default profile.

No missing WGS field may be zero-filled, copied from the native coverage profile, or approximated under a Picard field name.

### 3. Select the Picard HsMetrics v0.4 candidate surface

The first exact hybrid-selection profile will target the MultiQC-default capture/coverage surface that can be reconciled deterministically against Picard 3.4.0:

- identity/territory: `BAIT_SET`, `BAIT_TERRITORY`, `TARGET_TERRITORY`, `BAIT_DESIGN_EFFICIENCY`;
- read/base accounting: `TOTAL_READS`, `PF_READS`, `PF_UNIQUE_READS`, `PF_UQ_READS_ALIGNED`, `PF_BASES`, `PF_BASES_ALIGNED`, `PF_UQ_BASES_ALIGNED`;
- bait/target placement: `ON_BAIT_BASES`, `NEAR_BAIT_BASES`, `OFF_BAIT_BASES`, `ON_TARGET_BASES`, `PCT_SELECTED_BASES`, `ON_BAIT_VS_SELECTED`;
- usable/enrichment: `PCT_USABLE_BASES_ON_BAIT`, `PCT_USABLE_BASES_ON_TARGET`, `FOLD_ENRICHMENT`;
- target coverage: `MEAN_BAIT_COVERAGE`, `MEAN_TARGET_COVERAGE`, `MEDIAN_TARGET_COVERAGE`, `MAX_TARGET_COVERAGE`, `ZERO_CVG_TARGETS_PCT`, `FOLD_80_BASE_PENALTY`;
- target thresholds: `PCT_TARGET_BASES_1X`, `2X`, `10X`, `20X`, `30X`, `40X`, `50X`, and `100X`.

Derived percentages that MultiQC can display from those exact Picard rows may be retained when they are emitted by Picard 3.4.0 and covered by the same differential fixture.

The initial profile explicitly excludes:

- `AT_DROPOUT` and `GC_DROPOUT`;
- `HET_SNP_SENSITIVITY` and `HET_SNP_Q`;
- `HS_LIBRARY_SIZE`;
- `HS_PENALTY_*` fields;
- per-target/per-base Picard sidecar outputs;
- accumulation levels other than the single all-reads/default aggregate selected by the eventual profile.

As with WGS, this is a selected future exact profile. Milestone 12 does not expose `--format picard-wgs` or `--format picard-hs-metrics` and does not claim that native v0.3 targeted values are Picard values.

### 4. Fold-80 is a semantic boundary, not a rename

`target_uniformity_penalty_80` remains the native AlignGauge metric defined by ADR-0006 and the specification. It must never be serialized as Picard `FOLD_80_BASE_PENALTY`.

Picard `FOLD_80_BASE_PENALTY` becomes available only when all of the following are true:

1. the Picard 3.4.0 target-selection/filtering profile is implemented exactly;
2. overlapping mate observations are handled exactly according to the pinned Picard profile;
3. the Picard non-zero-coverage target behavior and denominator are reproduced exactly;
4. deterministic fixtures compare the field against Picard with no unexplained tolerance;
5. the resulting HsMetrics text is accepted by pinned MultiQC 1.35.

If any prerequisite is absent, the Picard field is absent. There is no fallback to the native metric and no sentinel numeric value.

### 5. MultiQC validation distinguishes compatibility from discovery

Milestone 12 validates three different contracts:

1. **Generated compatible output:** the existing exact Picard 3.4.0 `ALL_READS` insert-size projection must be discovered by MultiQC 1.35, and MultiQC's parsed data from Picard reference output and AlignGauge output must be byte-identical.
2. **Discovery-only WGS fixture:** a static synthetic Picard-shaped WGS file exercises MultiQC's pinned discovery and parser-required columns. It carries no AlignGauge compatibility claim.
3. **Discovery-only HsMetrics fixture:** a static synthetic Picard-shaped HsMetrics file exercises the same discovery/parser contract and likewise carries no compatibility claim.

The discovery fixtures live under `tools/reference/multiqc/fixtures/` and contain an explicit documentation boundary. They may never be used as differential metric evidence.

### 6. M11 alignment-summary remains exact but is not a MultiQC 1.35 profile

ADR-0008's 13-column reference-independent alignment-summary subset remains valid and exact against Picard 3.4.0. It is **not** promoted to a MultiQC-compatible profile in Milestone 12.

MultiQC 1.35's Picard alignment module directly needs reference-dependent `PF_READS_ALIGNED` and `PF_ALIGNED_BASES` while constructing its plots. Those fields are intentionally outside the M11 subset. Milestone 12 therefore records the boundary rather than zero-filling them or widening ADR-0008 without new differential evidence.

The CLI may continue exposing the M11 output as a differential-validation compatibility projection, but documentation must not claim that pinned MultiQC can consume that projection successfully.

## CI contract

`.github/workflows/multiqc-validation.yml` is the permanent Milestone 12 parser gate. It must:

1. verify the immutable Picard/MultiQC pins;
2. build the current AlignGauge CLI;
3. produce exact Picard and AlignGauge insert-size files from the deterministic `picard_insert_edge` fixture;
4. reconfirm their direct Picard differential result is exact;
5. run MultiQC 1.35 with `--network none` over both files and byte-compare its parsed insert-size data;
6. run the same pinned parser over the WGS/Hs discovery fixtures;
7. fail when an expected parser data file or required parsed field is absent;
8. record WGS/Hs fixture results with `compatibility_claim: false`;
9. upload the parser evidence artifact.

No parser invocation is wrapped in a best-effort fallback, `|| true`, warning-only path, or success-on-missing-output branch.

## Consequences

- Milestone 12 freezes the WGS/Hs compatibility target before implementing it.
- MultiQC compatibility is now executable rather than inferred from text resemblance.
- Exact Picard WGS/Hs output remains blocked until the overlap semantics are implemented and validated in Milestone 13.
- Native targeted metrics retain their stable v0.3 names and semantics.
- The project avoids a dangerous failure mode in which plausible numbers are emitted under Picard labels despite different filtering or denominator rules.
