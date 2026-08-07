# AlignGauge v0.1 Performance Report

## Scope

This report is a reproducible **baseline**, not a speedup claim. Measurements were collected by the HG002 workflow on product SHA `f93001cf22a2315f01e6b857c295720d99e392ca`, run `31162819757`, using artifact `8987812158` (`sha256:8058c031c80dc6bc736ce744b3349c2a669c30c43212ab7449114ef8c03d2a06`).

The benchmark compares four internal execution paths on the same prepared HG002 BAM:

1. reusable-record rust-htslib traversal only;
2. counters only;
3. exact coverage only;
4. combined counters plus coverage through the v0.1 release analysis path.

The harness first verifies semantic equivalence and requires combined mode to report exactly one BAM traversal.

## Input

- Records: `207,996`.
- Prepared BAM size: `15,245,338` bytes.
- Prepared BAM SHA-256: `9b5b0cf54ca98fdae9c703ae62e616fe7e1a69370de652183ef4b80e41903819`.
- Coverage accepted aligned bases: `30,995,408` in both standalone coverage and combined modes.
- Benchmark memory limit: `1,073,741,824` bytes (1 GiB).

## Method

For each mode the harness performs one warmup followed by three measured process invocations. Process startup is included. GNU `/usr/bin/time` records elapsed/user/system time and maximum RSS; a Python controller independently records monotonic wall time.

Cache state is deliberately documented as:

> warm-cache after one warmup per mode; cold cache not measured because the hosted runner does not grant cache-drop privilege

No cold-cache comparison or external-tool speed comparison is inferred from these measurements.

## Environment

- GitHub runner OS: Ubuntu 24 (`ubuntu24`).
- Runner image version: `20260720.247.2`.
- Kernel: `6.17.0-1020-azure`.
- Architecture: `x86_64`.
- CPU: AMD EPYC 7763 64-Core Processor.
- Logical CPUs visible: `4`.
- Total memory: `16,373,456 KiB`.
- Storage: ext4 on `/dev/root`; approximately 151,263,856 KiB total during the run.
- Rust: `rustc 1.97.1 (8bab26f4f 2026-07-14)`.
- Cargo: `cargo 1.97.1 (c980f4866 2026-06-30)`.

## Measurements

Controller wall time is the primary comparison because it has finer resolution than the GNU-time output retained by this small benchmark.

| Mode | Min wall (s) | Median wall (s) | Max wall (s) | Range / median | Median max RSS (KiB) |
| --- | ---: | ---: | ---: | ---: | ---: |
| rust-htslib traversal | 0.187432237 | 0.187763906 | 0.188944619 | 0.81% | 3,348 |
| counters only | 0.246925139 | 0.247986390 | 0.253829368 | 2.78% | 3,544 |
| coverage only | 0.274491002 | 0.274984713 | 0.276336234 | 0.67% | 4,288 |
| counters + coverage | 0.280458253 | 0.280592844 | 0.280827673 | 0.13% | 4,712 |

GNU-time median elapsed values were 0.18 s, 0.24 s, 0.27 s, and 0.27 s respectively.

## Semantic cross-checks

The harness rejects a benchmark run unless:

- reader record count = counters record count = combined record count;
- coverage accepted aligned bases = combined accepted aligned bases;
- combined reports `input_traversals = 1`;
- warmup and all measured runs for a mode produce identical semantic output.

Observed semantic values:

- reader records: `207,996`;
- counters records: `207,996`;
- combined records: `207,996`;
- coverage accepted bases: `30,995,408`;
- combined accepted bases: `30,995,408`;
- combined BAM traversals: `1`.

## Interpretation

The data establishes a v0.1 regression baseline and demonstrates that combined counters plus coverage use a single input traversal. It does **not** establish a speedup over Samtools, mosdepth, Picard, another AlignGauge release, or any other implementation. The sample is deliberately small and warm-cache; process-startup overhead is material at these durations.

The measured maximum RSS is also not a replacement for the coverage planner's conservative memory accounting. The planner includes bounded future-event, reader/output, safety-margin, and active-track allowances that are intentionally larger than the observed resident set on this small subset.
