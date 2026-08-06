# Milestone 0 Repository Foundation Evidence

**Status:** Implementation complete; one external repository-metadata item remains open.

## Implementation commit

- Commit: `9fdba0a32a71376b040bb5ebb3ee128ae652386b`
- Message: `build: establish Milestone 0 Rust foundation`

## Permanent CI evidence

- Workflow: `Permanent CI`
- Run: `31091278207`
- Job: `92582617171` (`ci/permanent`)
- Conclusion: `success`

The successful job verified:

- Rust 1.97.1 from `rust-toolchain.toml`;
- formatting;
- Clippy with warnings denied;
- workspace tests;
- rustdoc with warnings denied;
- a clean repository after all gates.

## Repository foundation

The implementation commit added:

- the Cargo workspace and committed lockfile;
- the reserved `aligngauge` binary package;
- permanent least-privilege CI with immutable action pins;
- contribution, security, and conduct policies;
- ADR and milestone-evidence directories;
- an implementation-oriented README that states the product boundary.

## Repository identity

The canonical repository is `ekkus93/aligngauge`. The binary and crate namespace are `aligngauge` and `aligngauge-*`.

## Open external control

GitHub's About description was still `A DNA sequencer in Rust` when this evidence was prepared. The tracked README and specification are correct, but the repository metadata must be changed through GitHub settings to the description in SPEC §1.2 before every Milestone 0 checkbox can be marked complete.

## Evidence-commit rule

This document is authoritative at the commit containing it. That commit must pass the same permanent CI workflow before this evidence is accepted.
