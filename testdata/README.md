# AlignGauge Test Data

This directory separates small deterministic fixtures committed to Git from
large public datasets that require explicit preparation.

## Ordinary validation

```bash
cargo run -p aligngauge-testkit --locked -- verify-manifest --root .
```

This command performs local checksum verification only. It does not contain a
network client and does not follow `source_url` fields.

## Regenerating synthetic fixtures

```bash
cargo run -p aligngauge-testkit --locked -- generate-corpus --root .
cargo run -p aligngauge-testkit --locked -- verify-manifest --root .
```

Generation is deterministic. CI regenerates the corpus in a temporary
directory and requires byte-identical output.

## Reference-tool outputs

The Samtools reference environment is under `tools/reference/samtools/`. Its
container is pinned by digest. Tool execution is network-isolated.

## HG002

The HG002 source BAM is not committed. `testdata/hg002/prepare.sh` is the only
supported preparation entry point. It records source identity, region,
subsampling parameters, and local SHA-256 values.
