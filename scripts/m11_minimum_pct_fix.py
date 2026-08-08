from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:100]!r}")
    p.write_text(text.replace(old, new, 1))

replace_once(
    "crates/aligngauge-metrics/src/picard.rs",
    "const MINIMUM_ORIENTATION_PCT: f64 = 0.05;\n",
    "const MINIMUM_ORIENTATION_PCT: f32 = 0.05;\n",
)
replace_once(
    "crates/aligngauge-metrics/src/picard.rs",
    "            if u64_to_f64(count) < u64_to_f64(total_inserts) * MINIMUM_ORIENTATION_PCT {\n",
    "            if u64_to_f64(count)\n                < u64_to_f64(total_inserts) * f64::from(MINIMUM_ORIENTATION_PCT)\n            {\n",
)

adr = "docs/adr/ADR-0008-PICARD_ALIGNMENT_INSERT_SIZE_PROFILE.md"
replace_once(
    adr,
    "- `MINIMUM_PCT = 0.05`;\n",
    "- `MINIMUM_PCT = 0.05f` as a Java `float`, promoted to `double` by the collector;\n",
)
replace_once(
    adr,
    "Each orientation has an independent histogram. An orientation category is emitted only when its count is at least `total_inserts * 0.05`, matching Picard's comparison rule.\n",
    "Each orientation has an independent histogram. Picard declares `MINIMUM_PCT` as Java `float` `0.05f`, passes that value to a `double` collector field, and then retains a category when `count >= total_inserts * promoted_minimum_pct`. The promoted binary32 value is slightly greater than mathematical `0.05`, so a category at mathematically exact 5% (for example 2 of 40 observations) is suppressed by Picard 3.4.0. AlignGauge reproduces that binary32-to-binary64 boundary exactly.\n",
)

spec = "docs/DNA_QC_ENGINE_SPEC.md"
replace_once(
    spec,
    "ALL_READS: DEVIATIONS=10.0, HISTOGRAM_WIDTH unset, MIN_HISTOGRAM_WIDTH unset,\nMINIMUM_PCT=0.05, INCLUDE_DUPLICATES=false. It accepts only mapped paired records\n",
    "ALL_READS: DEVIATIONS=10.0, HISTOGRAM_WIDTH unset, MIN_HISTOGRAM_WIDTH unset,\nMINIMUM_PCT=Java float 0.05f promoted to the collector's double, INCLUDE_DUPLICATES=false.\nThe orientation inclusion comparison must reproduce that promoted binary32 value exactly,\nincluding suppression of a mathematically exact 5% category. It accepts only mapped paired records\n",
)
