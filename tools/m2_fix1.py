from pathlib import Path


def replace_once(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    path.write_text(text.replace(old, new), encoding="utf-8")


replace_once(
    Path("crates/aligngauge-testkit/src/differential.rs"),
    '''            (MetricType::Decimal, value) if value != "-" => {
                let parsed = value.parse::<u32>().map_err(|error| {
                    TestkitError::differential(format!(
                        "line {line}: invalid rounding_decimals {value:?}: {error}"
                    ))
                })?;
                if parsed > 18 {
                    return Err(TestkitError::differential(format!(
                        "line {line}: rounding_decimals exceeds 18"
                    )));
                }
                Some(parsed)
            }
            (MetricType::Decimal, "-") => {
                return Err(TestkitError::differential(format!(
                    "line {line}: decimal metric requires rounding_decimals"
                )));
            }
''',
    '''            (MetricType::Decimal, "-") => {
                return Err(TestkitError::differential(format!(
                    "line {line}: decimal metric requires rounding_decimals"
                )));
            }
            (MetricType::Decimal, value) => {
                let parsed = value.parse::<u32>().map_err(|error| {
                    TestkitError::differential(format!(
                        "line {line}: invalid rounding_decimals {value:?}: {error}"
                    ))
                })?;
                if parsed > 18 {
                    return Err(TestkitError::differential(format!(
                        "line {line}: rounding_decimals exceeds 18"
                    )));
                }
                Some(parsed)
            }
''',
    "decimal match",
)

replace_once(
    Path("crates/aligngauge-testkit/src/bam.rs"),
    '''    let query_length = cigar_query_length(&record.cigar)?;
    if query_length != record.sequence.len() {
        return Err(TestkitError::generation(format!(
            "record {} CIGAR consumes {query_length} query bases but sequence has {}",
            record.name,
            record.sequence.len()
        )));
    }
''',
    '''    let query_length = cigar_query_length(&record.cigar)?;
    if !record.cigar.is_empty() && query_length != record.sequence.len() {
        return Err(TestkitError::generation(format!(
            "record {} CIGAR consumes {query_length} query bases but sequence has {}",
            record.name,
            record.sequence.len()
        )));
    }
''',
    "unmapped CIGAR validation",
)

Path(__file__).unlink()
