# AlignGauge Naming Decision

**Date:** 2026-08-06  
**Status:** Accepted and applied

## Decision

The project name is **AlignGauge**.

Repository:

```text
ekkus93/aligngauge
```

Binary name:

```text
aligngauge
```

Crate namespace:

```text
aligngauge
aligngauge-core
aligngauge-hts
aligngauge-metrics
aligngauge-coverage
aligngauge-formats
aligngauge-testkit
```

## Description

> A validation-first Rust engine for fast, single-pass alignment QC and coverage analysis across BAM and CRAM data for WGS, WES, and targeted sequencing.

## Rationale

“Align” identifies the durable product boundary: analysis of aligned sequencing records rather than one particular storage format. “Gauge” communicates measurement, quality control, and coverage assessment rather than alignment, basecalling, variant calling, or control of a physical sequencer.

The name remains accurate as the project grows from BAM support in v0.1 to CRAM, WES, targeted-panel, and compatibility features in later releases.

## Applied state

The GitHub repository was renamed from `ekkus93/rust-dna-sequencer` to `ekkus93/aligngauge` on 2026-08-06.
