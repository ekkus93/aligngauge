from pathlib import Path

IMPLEMENTATION_SHA = "45211236419e5bebc7c0d09d5cb35d65174cc11a"
PERMANENT_RUN = "31100841806"
PERMANENT_JOB = "92613749893"
REFERENCE_RUN = "31100842135"
REFERENCE_JOB = "92613751393"
HG002_RUN = "31100844104"
HG002_JOB = "92613759310"


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    return text.replace(old, new)


todo_path = Path("docs/DNA_QC_ENGINE_TODO.md")
todo = todo_path.read_text(encoding="utf-8")
todo = replace_once(
    todo,
    "**Status:** Ralph Loop active — Milestone 2 implementation in progress",
    "**Status:** Ralph Loop active — Milestone 2 complete; Milestone 3 next",
    "document status",
)

start_marker = "## Milestone 2 — Test corpus and differential harness\n"
end_marker = "\n---\n\n## Milestone 3 — Production BAM reader and validation"
start = todo.find(start_marker)
if start < 0:
    raise SystemExit("Milestone 2 start marker not found")
end = todo.find(end_marker, start)
if end < 0:
    raise SystemExit("Milestone 2 end marker not found")
section = todo[start:end]
section = replace_once(
    section,
    "**Status:** Implementation in progress.",
    (
        "**Status:** Complete — implementation SHA `"
        + IMPLEMENTATION_SHA
        + "`; Permanent CI run `"
        + PERMANENT_RUN
        + "`, job `"
        + PERMANENT_JOB
        + "`; Reference Validation run `"
        + REFERENCE_RUN
        + "`, job `"
        + REFERENCE_JOB
        + "`; HG002 Preparation run `"
        + HG002_RUN
        + "`, job `"
        + HG002_JOB
        + "`; all successful."
    ),
    "Milestone 2 status",
)
section = section.replace("- [ ]", "- [x]")
if "- [ ]" in section:
    raise SystemExit("unchecked Milestone 2 task remains")
todo = todo[:start] + section + todo[end:]
todo_path.write_text(todo, encoding="utf-8")

evidence = f"""# Milestone 2 Evidence — Test Corpus and Differential Harness

**Status:** Complete  
**Implementation SHA:** `{IMPLEMENTATION_SHA}`  
**Evidence date:** 2026-08-06

The commit containing this document is the Milestone 2 evidence commit. Completion
is valid only after Permanent CI, Reference Validation, and HG002 Preparation all
succeed on that exact commit.

## 1. Validated source evidence

The implementation source was validated on `{IMPLEMENTATION_SHA}` by:

- Permanent CI run `{PERMANENT_RUN}`, job `{PERMANENT_JOB}` — success;
- Reference Validation run `{REFERENCE_RUN}`, job `{REFERENCE_JOB}` — success;
- HG002 Preparation run `{HG002_RUN}`, job `{HG002_JOB}` — success.

The user-reported job `92612115283` in run `31100344848` was an obsolete candidate
run on SHA `9645b429bb9e874fc55ac04494241155919a1827`. It failed only because the
bootstrap-generated Rust files had not yet been committed in rustfmt form. The
validated source and all later exact-SHA runs include the formatted files. That
failure was not ignored, retried as success, or used as milestone evidence.

## 2. Delivered implementation

Milestone 2 adds the `aligngauge-testkit` crate with:

- deterministic raw BAM and BGZF serialization;
- deterministic BAI generation through pinned `rust-htslib`/HTSlib;
- strict local test-data manifest parsing and SHA-256 verification;
- exact integer, explicit decimal-rounding, and byte-exact text comparison;
- deterministic machine-readable discrepancy reports;
- a CLI for corpus generation, local verification, and differential comparison.

The synthetic corpus covers:

- empty, basic mapped/unmapped, and full CIGAR-operation BAMs;
- long CIGAR restoration through `CG:B,I`;
- paired, secondary, supplementary, dual-flag, duplicate, QC-fail, singleton,
  and discordant records;
- optional tags, missing tags, declared and unknown read groups;
- coordinate regression and unmapped-tail cases;
- unknown reference IDs, integer boundaries, and zero-length references;
- chunk-boundary and multi-track memory cases;
- malformed optional data, malformed record lengths, and truncated BGZF data.

Every committed fixture has an immutable manifest identity. Ordinary tests perform
local filesystem reads only and cannot resolve external URLs or download data.

## 3. Reference-tool contract

Samtools/HTSlib is pinned to version 1.24 at:

```text
quay.io/biocontainers/samtools@sha256:a130447589651ed09252aa95a5e4f4132942cdb54d835d81a04a9a930d656561
```

The permanent reference runner:

- executes with networking disabled;
- uses a read-only root filesystem;
- drops all Linux capabilities;
- enables `no-new-privileges`;
- mounts the repository read-only;
- captures command, image, version, stdout, stderr, status, and wall time;
- publishes `_SUCCESS` only after a zero exit status and complete capture.

`docs/adr/ADR-0003-COVERAGE_BASELINE.md` owns the v0.1 coverage baseline. It uses
`samtools depth -aa -q 0 -Q 0 -G UNMAP,SECONDARY,QCFAIL,DUP,SUPPLEMENTARY` with
no deletion counting, mate-overlap correction, or target restriction.

## 4. HG002 preparation contract

The explicit preparation path uses the GIAB HG002 Element AVITI GRCh38-GIABv3
alignment and records:

- source BAM MD5 `f5360b7adbc798c90a78f290de928eca`;
- source BAI MD5 `1d7fd88891eee203c02fb852cac95301`;
- region `chr20:10000000-11000000`;
- subsampling seed `42`;
- subsampling fraction `0.37037037037037`;
- exact Samtools container digest;
- local subset and index SHA-256 values.

The preparation workflow runs the operation twice and requires identical prepared
manifests. Containers remain non-root, capability-free, and read-only at the root;
they use the runner UID/GID solely so the owner-only staging directory remains
writable without making it world-writable.

## 5. Fail-closed properties

- No implicit test-data download exists in ordinary validation.
- External manifest entries cannot claim unprepared local identities.
- Missing or mismatched committed checksums are fatal.
- Partial index identities are rejected.
- Existing reference-output destinations are rejected.
- Failed or incomplete reference commands do not publish `_SUCCESS`.
- Integer comparisons are exact.
- Decimal comparisons require an explicit per-field rounding rule.
- No blanket epsilon or undocumented compatibility tolerance exists.
- Every accepted semantic difference requires a named compatibility note.
- Large HG002 source data and prepared subsets remain outside Git.

## 6. Permanent gates

The evidence commit must pass:

1. `ci/permanent`: lockfile, formatting, schema parsing, local manifest verification,
   byte-identical corpus regeneration, shell syntax, strict Clippy, tests, rustdoc,
   and clean-tree verification;
2. `ci/reference`: digest-pinned, network-isolated Samtools `flagstat`, `idxstats`,
   and `depth` captures;
3. `ci/hg002-preparation`: two successful preparations with identical manifests
   and uploaded evidence.

## 7. Deferred work

Milestone 2 establishes fixtures, oracle execution, and comparison machinery. It
does not claim that AlignGauge already implements production BAM validation,
Samtools-equivalent counters, or canonical coverage. Those remain Milestones 3,
4, and 5. CRAM support and reference resolution remain v0.2 work under the future
`ADR-0004-CRAM_REFERENCE_RESOLUTION.md`.
"""
Path("docs/evidence/M2_TEST_CORPUS.md").write_text(evidence, encoding="utf-8")

hg002_readme_path = Path("testdata/hg002/README.md")
hg002_readme = hg002_readme_path.read_text(encoding="utf-8")
addition = f"""

## Milestone 2 validation evidence

Two-pass preparation succeeded on source SHA `{IMPLEMENTATION_SHA}` in workflow run
`{HG002_RUN}`, job `{HG002_JOB}`. This documentation change intentionally retriggers
the preparation workflow so the final Milestone 2 evidence commit receives its own
exact-SHA validation.
"""
if "## Milestone 2 validation evidence" in hg002_readme:
    raise SystemExit("HG002 validation evidence section already exists")
hg002_readme_path.write_text(hg002_readme.rstrip() + addition, encoding="utf-8")

Path("tools/m2_finalize.py").unlink()
Path(".github/workflows/m2-finalize.yml").unlink()
