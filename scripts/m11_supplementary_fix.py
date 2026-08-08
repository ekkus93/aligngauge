from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old!r}")
    p.write_text(text.replace(old, new, 1))

replace_once(
    "crates/aligngauge-metrics/src/picard.rs",
    "        if record.flags() & FLAG_SECONDARY != 0 {\n            return Ok(());\n        }\n",
    "        if record.flags() & (FLAG_SECONDARY | FLAG_SUPPLEMENTARY) != 0 {\n            return Ok(());\n        }\n",
)

replace_once(
    "docs/adr/ADR-0008-PICARD_ALIGNMENT_INSERT_SIZE_PROFILE.md",
    "- secondary and supplementary records are excluded from the alignment-summary read-count categories according to the pinned collector behavior.\n",
    "- secondary and supplementary records are rejected by Picard's top-level alignment-summary collector before category dispatch, so they contribute neither read counts nor `BAD_CYCLES`.\n",
)

replace_once(
    "docs/DNA_QC_ENGINE_SPEC.md",
    "silently acquire sequence materialization cost.\n\nThe insert-size profile matches Picard 3.4.0 CollectInsertSizeMetrics defaults at\n",
    "silently acquire sequence materialization cost. Secondary and supplementary records are\nrejected before alignment-summary category dispatch and therefore contribute neither read\ncounts nor `BAD_CYCLES`.\n\nThe insert-size profile matches Picard 3.4.0 CollectInsertSizeMetrics defaults at\n",
)
