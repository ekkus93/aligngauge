# AlignGauge Test-Data Manifest v1

`testdata/manifest.v1.tsv` is a strict UTF-8, tab-separated manifest. Blank
lines, comments, duplicate IDs, unknown schemas, and missing columns are errors.

The exact header is:

```text
schema	id	kind	path	sha256	index_path	index_sha256	source_url	source_checksum	reference_build	generation	license	expected_validity	expected_error	expected_metrics
```

## Invariants

- `schema` is always `aligngauge-testdata-v1`.
- `kind` is `committed` or `external`.
- A committed entry requires a repository-relative `path` and lowercase
  SHA-256.
- An external entry must use `-` for local path and SHA-256 until an explicit
  preparation command creates local data.
- Index path and index SHA-256 are supplied together or both use `-`.
- Source checksums use `algorithm:value`.
- Valid entries use `valid` and `-` for `expected_error`.
- Invalid entries use `error` and a stable AlignGauge error category.
- Optional fields use the literal `-`.
- Fields cannot contain tabs, carriage returns, or newlines.

`aligngauge-testkit verify-manifest --root .` verifies committed files using
local filesystem reads only. It records external URLs but never resolves them.
No ordinary unit, integration, or CI test may download test data.
