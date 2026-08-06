# Milestone 1 Core-Contract Evidence

**Milestone:** 1 — Core model, errors, and atomic output  
**Date:** 2026-08-06  
**Repository:** `ekkus93/aligngauge`  
**Authoritative specification:** `docs/DNA_QC_ENGINE_SPEC.md`  
**Implementation signoff SHA:** `538ba902de793bd706c58bf1cb449bcf2740142a`

## Scope delivered

Milestone 1 establishes the contracts used by later BAM collectors without adding
coverage or compatibility semantics prematurely.

Implemented in `aligngauge-core`:

- the complete stable error-category set from SPEC §14;
- unique nonzero exit codes, human rendering, deterministic JSON rendering, source
  chains, and default redaction of sensitive read-level details;
- strict typed v0.1 configuration resolution with precedence
  `built-ins < config file < environment < CLI overrides`;
- unknown-key rejection, checked memory-unit parsing, normalized positive coverage
  thresholds, contradiction checks, and resolved-config provenance serialization;
- deterministic canonical JSON values and rendering;
- typed `summary.json` and `provenance.json` models with independent schema versions;
- tagged available/unavailable values, preventing unavailable metrics from appearing
  as numeric zero;
- committed JSON schemas and golden serialization fixtures;
- same-filesystem staging, required-file synchronization, `_SUCCESS` written last,
  staging metadata synchronization, and atomic destination rename;
- fail-closed cleanup and clearly marked preserved failed staging;
- a fatal existing-destination policy documented by
  `docs/adr/ADR-0002-OUTPUT_DESTINATION_POLICY.md`.

The walking-skeleton CLI now maps missing, invalid, and corrupt BAM failures to the
stable core error taxonomy. Its corruption tests assert category identity rather
than brittle prose.

## Test evidence

Permanent CI executed:

```text
cargo update --workspace --locked --dry-run
cargo fmt --all --check
JSON schema parse validation
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked
git status --porcelain clean-tree check
```

The exact implementation signoff SHA passed:

- **Workflow:** Permanent CI
- **Run:** `31095663673`
- **Job:** `92596959816`
- **Conclusion:** success
- **Exact SHA:** `538ba902de793bd706c58bf1cb449bcf2740142a`

Test inventory on that SHA:

- 4 walking-skeleton BAM integration tests;
- 14 `aligngauge-core` unit tests;
- 3 canonical-model/golden-contract integration tests;
- 21 tests total, all passing;
- rustdoc completed with warnings denied;
- the repository remained clean after validation.

## Error-contract evidence

Tests prove that:

- all 16 specification categories have unique stable names and unique nonzero exit
  codes;
- every category renders in both human and JSON form;
- source chains are retained;
- sensitive read-level details are absent by default and appear only when explicitly
  requested;
- missing input reports `input_not_found`;
- truncated BAM input reports `input_corrupt` and emits no plausible-looking counts.

## Configuration-contract evidence

Tests prove that:

- CLI overrides environment, environment overrides the config file, and the config
  file overrides built-ins;
- unknown and duplicate config keys fail;
- unsupported config schema versions fail;
- zero threads, zero memory, invalid units, invalid thresholds, and contradictory
  quiet/verbose modes fail;
- memory multiplication is checked for overflow;
- coverage thresholds are sorted and deduplicated deterministically;
- the fully resolved configuration has deterministic provenance JSON.

## Canonical-model evidence

Golden tests cover both canonical files:

- `crates/aligngauge-core/tests/golden/summary.json`
- `crates/aligngauge-core/tests/golden/provenance.json`

The explicit availability representation is either:

```json
{"status":"available","value":...}
```

or:

```json
{"status":"unavailable","reason":"..."}
```

An unavailable metric has no `value` field and cannot be serialized as zero.
Objects use deterministic key ordering; reference and warning lists are normalized
before serialization.

## Atomic-publication evidence

Fault injection covers every pre-rename checkpoint:

1. staging created;
2. required files written;
3. required files synchronized;
4. `_SUCCESS` written and synchronized;
5. staging metadata synchronized;
6. immediately before rename.

For every injected failure, tests prove that the final destination never appears.
Default cleanup removes staging. Preserved failures remove `_SUCCESS`, add `_FAILED`,
and use a `.staging.failed` name. A separate observer test proves the destination is
absent at every checkpoint and appears only after atomic rename. Existing
 destinations are never overwritten.

## Repair and validation history

The first Milestone 1 run correctly failed strict Clippy. Structural repairs were
made instead of adding lint allowances. A later run exposed two stale
walking-skeleton assertions after the new typed-error contract; the assertions were
updated to stable categories without weakening corruption checks.

A validated source commit was published as:

- `bbe0c3456612ec9f96a888d26b391e4ffb00b0e0`

Temporary write-enabled repair workflow state and its repair script were then
removed atomically. The permanent read-only workflow was restored in the exact
implementation signoff commit listed above.

## Deliberate deferrals

- The public CLI remains the Milestone 0.5 counting slice. Full configuration and
  canonical-output wiring occur in later milestones.
- Atomic directory publication is implemented and tested for Unix-like platforms.
  Unsupported platforms fail explicitly; no non-atomic fallback exists.
- v0.1 has no overwrite, merge, resume, automatic suffix, or partial-success mode.
- BAM production validation, differential fixtures, counters, and coverage remain
  owned by Milestones 2–5.

## Milestone conclusion

The Milestone 1 implementation contracts pass on the exact implementation signoff
SHA. Milestone closure additionally requires Permanent CI to pass on the evidence
commit that introduces this document; that final evidence-run identity is recorded
in the subsequent TODO signoff update.
