# ADR-0002: Output destination and atomic publication policy

- **Status:** Accepted
- **Date:** 2026-08-06
- **Decision owners:** AlignGauge maintainers
- **Applies to:** v0.1 canonical output publication

## Context

AlignGauge must never expose a partially built output directory as a completed run.
The v0.1 interface accepts an output directory, and retry behavior must be explicit.
Silently deleting, merging into, or replacing an existing directory could destroy
valuable genomic-analysis results or combine incompatible runs.

## Decision

1. An existing destination is a fatal `output_exists` error.
2. v0.1 has no overwrite, merge, resume, or automatic-suffix fallback.
3. Output files are built in a restrictive-permission staging directory beside the
   destination, ensuring that staging and destination are on the same filesystem.
4. Required files are written and synchronized before `_SUCCESS` is created.
5. `_SUCCESS` is the final staging file and is synchronized before staging metadata.
6. Staging is atomically renamed to the destination only after all checks pass.
7. Any pre-rename failure removes staging by default.
8. `--preserve-failed-staging` preserves it under a `.staging.failed` name, removes
   `_SUCCESS`, and adds `_FAILED`.
9. A platform without the required directory synchronization and atomic-rename
   behavior is unsupported until a separately named policy is specified and tested.

## Consequences

- Users must remove or choose a different destination before retrying.
- A race that creates the destination before rename fails rather than overwriting it.
- Callers can treat a destination containing `_SUCCESS` as an indivisible completed
  publication.
- Preserved diagnostic staging directories cannot be mistaken for successful runs.

## Rejected alternatives

- **Delete and replace:** risks irreversible data loss.
- **Merge into an existing directory:** can create mixed-run output.
- **Automatically append a numeric suffix:** hides configuration mistakes and makes
  downstream path selection nondeterministic.
- **Write `_SUCCESS` after rename:** creates a visible interval in which a complete
  directory lacks its completion marker.
