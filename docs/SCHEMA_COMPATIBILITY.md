# Canonical schema compatibility and migration

AlignGauge canonical JSON is a versioned contract. Consumers must inspect the declared schema version and must not infer a missing field as numeric zero, `false`, an empty collection, or another plausible value.

## Current schemas

As of the v0.5 production-beta qualification program:

- `summary.json` schema version: `1.1.0`
- `provenance.json` schema version: `1.0.0`

The authoritative JSON Schemas are committed as:

- `schemas/summary.schema.json`
- `schemas/provenance.schema.json`

The Rust constants in `aligngauge-core` are the implementation-side version identifiers and Permanent CI validates generated output against the committed schemas.

## Compatibility policy

Schema versions use semantic-versioning intent for the serialized contract:

- a **patch** change may clarify validation without changing the set or meaning of accepted data;
- a **minor** change may add explicitly versioned fields or capabilities while preserving the meaning of existing fields;
- a **major** change may remove, rename, reinterpret, or structurally replace existing fields.

A producer must emit the exact version it implements. A consumer may accept a newer minor version only when it has an explicit forward-compatibility policy; blindly ignoring unknown fields is not an AlignGauge compatibility guarantee.

Unavailable information remains explicit. Consumers must preserve the distinction between:

- an available value of zero; and
- an unavailable value carrying a reason.

No migration may manufacture a metric solely to satisfy a newer schema.

## Summary schema 1.0.x to 1.1.0

Summary schema 1.1.0 added the v0.3 targeted-analysis surface while preserving the earlier whole-input counter and coverage meanings. The targeted field is an explicit availability value. When no target BED was supplied, it is represented as unavailable with a reason; it is not represented as an all-zero targeted report.

For archived output using an earlier summary schema:

1. preserve the original file and schema version;
2. do not insert synthetic targeted values;
3. if a 1.1.0 document is required, rerun the corresponding AlignGauge release against the original alignment/configuration when those inputs remain available;
4. otherwise treat the targeted surface as unavailable outside the original document rather than rewriting historical evidence.

This is a reanalysis policy, not a lossy in-place migration.

## Provenance schema 1.0.0

The provenance schema remains at 1.0.0 through the start of v0.5 qualification. Runtime additions that fit existing versioned structures must preserve their established meaning. Any future incompatible provenance change requires a schema-version change and a migration note before release.

## v0.5 disposition

The v0.5 qualification/hardening work does not by itself require a canonical schema change. Full-scale reports, fuzz/security evidence, SBOMs, checksums, and release attestations are release evidence artifacts rather than new fields in `summary.json` or `provenance.json`.

If implementation work during v0.5 changes either canonical serialized contract, this document, the Rust version constants, both JSON Schemas, golden fixtures, and release evidence must be updated together before the v0.5 release gate can close.
