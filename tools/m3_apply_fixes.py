from pathlib import Path


def replace_exact(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if old not in text:
        raise SystemExit(f"expected text not found in {path}: {old!r}")
    path.write_text(text.replace(old, new), encoding="utf-8")


reader = Path("crates/aligngauge-hts/src/reader.rs")
replace_exact(reader, "Number of HTSlib background decode threads", "Number of `HTSlib` background decode threads")
replace_exact(reader, "Whether HTSlib expanded a BAM long-CIGAR representation", "Whether `HTSlib` expanded a BAM long-CIGAR representation")
replace_exact(reader, "invalid options, HTSlib\n", "invalid options, `HTSlib`\n")
replace_exact(
    reader,
    "self.record_index.checked_add(1).unwrap_or(u64::MAX)",
    "self.record_index.saturating_add(1)",
)
replace_exact(
    reader,
    "fn validate_cigar(\n",
    "#[allow(clippy::too_many_lines)]\nfn validate_cigar(\n",
)
replace_exact(
    reader,
    "fn validate_auxiliary(\n",
    "#[allow(clippy::too_many_lines)]\nfn validate_auxiliary(\n",
)
replace_exact(
    reader,
    "parse_nonnegative_integer(value, \"NM\", index, record)?",
    "parse_nonnegative_integer(&value, \"NM\", index, record)?",
)
replace_exact(reader, "    value: Aux<'_>,\n", "    value: &Aux<'_>,\n")
replace_exact(
    reader,
    "    let value = match value {\n        Aux::I8(value) => i64::from(value),\n        Aux::U8(value) => i64::from(value),\n        Aux::I16(value) => i64::from(value),\n        Aux::U16(value) => i64::from(value),\n        Aux::I32(value) => i64::from(value),\n        Aux::U32(value) => return Ok(u64::from(value)),\n",
    "    let value = match value {\n        Aux::I8(value) => i64::from(*value),\n        Aux::U8(value) => i64::from(*value),\n        Aux::I16(value) => i64::from(*value),\n        Aux::U16(value) => i64::from(*value),\n        Aux::I32(value) => i64::from(*value),\n        Aux::U32(value) => return Ok(u64::from(*value)),\n",
)

lib = Path("crates/aligngauge-hts/src/lib.rs")
replace_exact(
    lib,
    "/// HTSlib compatibility line supplied by the pinned rust-htslib release.",
    "/// `HTSlib` compatibility line supplied by the pinned rust-htslib release.",
)
