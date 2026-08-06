# Pinned Samtools Reference Environment

The v0.1 reference tool is Samtools/HTSlib 1.24 at the immutable container
digest in `image.lock`.

Run all three baselines against a committed fixture:

```bash
tools/reference/samtools/run-baselines.sh \
  testdata/fixtures/basic.bam \
  target/reference/basic
```

The runner pulls the exact digest before execution, then starts the analysis
container with:

- `--network none`;
- read-only root filesystem;
- all Linux capabilities dropped;
- `no-new-privileges`;
- bounded PIDs, memory, and CPUs;
- repository mounted read-only.

Each result directory contains invocation, image, version, stdout, stderr, exit
status, wall time, and `_SUCCESS`. Existing destinations are rejected. A failed
or incomplete command is never published as complete.
