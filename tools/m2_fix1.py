from pathlib import Path

path = Path("crates/aligngauge-testkit/src/differential.rs")
text = path.read_text(encoding="utf-8")
old = '''            (MetricType::Decimal, value) if value != "-" => {
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
'''
new = '''            (MetricType::Decimal, "-") => {
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
'''
count = text.count(old)
if count != 1:
    raise SystemExit(f"expected one decimal match block, found {count}")
path.write_text(text.replace(old, new), encoding="utf-8")
Path(__file__).unlink()
