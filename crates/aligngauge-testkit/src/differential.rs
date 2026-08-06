//! Exact field-level differential comparison.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use aligngauge_core::JsonValue;

use crate::error::{Result, TestkitError};

const EXPECTED_HEADER: &str =
    "metric\ttype\texpected\trounding_decimals\tcompatibility_note";
const ACTUAL_HEADER: &str = "metric\ttype\tactual";

/// Metric comparison type.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MetricType {
    /// Signed integer compared exactly.
    Integer,
    /// Decimal compared after an explicitly declared decimal-place rounding.
    Decimal,
    /// Text compared byte-for-byte.
    Text,
}

impl MetricType {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "integer" => Ok(Self::Integer),
            "decimal" => Ok(Self::Decimal),
            "text" => Ok(Self::Text),
            _ => Err(TestkitError::differential(format!(
                "unsupported metric type {value:?}"
            ))),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Integer => "integer",
            Self::Decimal => "decimal",
            Self::Text => "text",
        }
    }
}

/// One expected metric.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExpectedMetric {
    /// Stable metric key.
    pub metric: String,
    /// Comparison type.
    pub metric_type: MetricType,
    /// Expected value text.
    pub expected: String,
    /// Explicit decimal places for decimal metrics.
    pub rounding_decimals: Option<u32>,
    /// Named compatibility note for an accepted semantic difference.
    pub compatibility_note: Option<String>,
}

/// One observed metric.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ActualMetric {
    /// Stable metric key.
    pub metric: String,
    /// Comparison type.
    pub metric_type: MetricType,
    /// Observed value text.
    pub actual: String,
}

/// One discrepancy.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Discrepancy {
    /// Metric key.
    pub metric: String,
    /// Comparison type.
    pub metric_type: MetricType,
    /// Expected value.
    pub expected: Option<String>,
    /// Actual value.
    pub actual: Option<String>,
    /// Human-readable reason.
    pub reason: String,
    /// Optional named compatibility note.
    pub compatibility_note: Option<String>,
}

/// Machine-readable comparison report.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DifferentialReport {
    /// Total expected metrics.
    pub expected_count: usize,
    /// Total observed metrics.
    pub actual_count: usize,
    /// Deterministically ordered discrepancies.
    pub discrepancies: Vec<Discrepancy>,
}

impl DifferentialReport {
    /// Whether the comparison is exact under every per-field rule.
    #[must_use]
    pub fn is_match(&self) -> bool {
        self.discrepancies.is_empty()
    }

    /// Render deterministic compact JSON.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mismatches = self
            .discrepancies
            .iter()
            .map(|item| {
                let mut object = BTreeMap::from([
                    (
                        String::from("metric"),
                        JsonValue::String(item.metric.clone()),
                    ),
                    (
                        String::from("type"),
                        JsonValue::String(item.metric_type.as_str().to_owned()),
                    ),
                    (
                        String::from("reason"),
                        JsonValue::String(item.reason.clone()),
                    ),
                ]);
                object.insert(
                    String::from("expected"),
                    item.expected
                        .clone()
                        .map_or(JsonValue::Null, JsonValue::String),
                );
                object.insert(
                    String::from("actual"),
                    item.actual
                        .clone()
                        .map_or(JsonValue::Null, JsonValue::String),
                );
                object.insert(
                    String::from("compatibility_note"),
                    item.compatibility_note
                        .clone()
                        .map_or(JsonValue::Null, JsonValue::String),
                );
                JsonValue::Object(object)
            })
            .collect();

        JsonValue::Object(BTreeMap::from([
            (
                String::from("actual_count"),
                JsonValue::Unsigned(self.actual_count as u64),
            ),
            (
                String::from("expected_count"),
                JsonValue::Unsigned(self.expected_count as u64),
            ),
            (
                String::from("match"),
                JsonValue::Bool(self.is_match()),
            ),
            (String::from("discrepancies"), JsonValue::Array(mismatches)),
        ]))
        .to_compact_string()
    }
}

/// Parse expected-result TSV.
pub fn parse_expected(text: &str) -> Result<Vec<ExpectedMetric>> {
    let rows = parse_rows(text, EXPECTED_HEADER, 5)?;
    let mut metrics = Vec::with_capacity(rows.len());
    let mut keys = BTreeSet::new();

    for (line, fields) in rows {
        let metric = required(&fields[0], line, "metric")?.to_owned();
        if !keys.insert(metric.clone()) {
            return Err(TestkitError::differential(format!(
                "line {line}: duplicate expected metric {metric:?}"
            )));
        }
        let metric_type = MetricType::parse(&fields[1])?;
        let expected = required(&fields[2], line, "expected")?.to_owned();
        let rounding_decimals = match (metric_type, fields[3].as_str()) {
            (MetricType::Decimal, value) if value != "-" => {
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
            (MetricType::Integer | MetricType::Text, "-") => None,
            (MetricType::Integer | MetricType::Text, _) => {
                return Err(TestkitError::differential(format!(
                    "line {line}: only decimal metrics may declare rounding"
                )));
            }
        };
        let compatibility_note = optional(&fields[4]);
        validate_value(metric_type, &expected, rounding_decimals)?;

        metrics.push(ExpectedMetric {
            metric,
            metric_type,
            expected,
            rounding_decimals,
            compatibility_note,
        });
    }

    Ok(metrics)
}

/// Parse actual-result TSV.
pub fn parse_actual(text: &str) -> Result<Vec<ActualMetric>> {
    let rows = parse_rows(text, ACTUAL_HEADER, 3)?;
    let mut metrics = Vec::with_capacity(rows.len());
    let mut keys = BTreeSet::new();

    for (line, fields) in rows {
        let metric = required(&fields[0], line, "metric")?.to_owned();
        if !keys.insert(metric.clone()) {
            return Err(TestkitError::differential(format!(
                "line {line}: duplicate actual metric {metric:?}"
            )));
        }
        let metric_type = MetricType::parse(&fields[1])?;
        let actual = required(&fields[2], line, "actual")?.to_owned();
        metrics.push(ActualMetric {
            metric,
            metric_type,
            actual,
        });
    }

    Ok(metrics)
}

/// Compare expected and actual metric vectors.
pub fn compare(expected: &[ExpectedMetric], actual: &[ActualMetric]) -> Result<DifferentialReport> {
    let expected_by_key: BTreeMap<&str, &ExpectedMetric> = expected
        .iter()
        .map(|metric| (metric.metric.as_str(), metric))
        .collect();
    let actual_by_key: BTreeMap<&str, &ActualMetric> = actual
        .iter()
        .map(|metric| (metric.metric.as_str(), metric))
        .collect();
    let keys: BTreeSet<&str> = expected_by_key
        .keys()
        .chain(actual_by_key.keys())
        .copied()
        .collect();

    let mut discrepancies = Vec::new();
    for key in keys {
        match (expected_by_key.get(key), actual_by_key.get(key)) {
            (Some(expected_metric), Some(actual_metric)) => {
                compare_one(expected_metric, actual_metric, &mut discrepancies)?;
            }
            (Some(expected_metric), None) => discrepancies.push(Discrepancy {
                metric: key.to_owned(),
                metric_type: expected_metric.metric_type,
                expected: Some(expected_metric.expected.clone()),
                actual: None,
                reason: String::from("missing actual metric"),
                compatibility_note: expected_metric.compatibility_note.clone(),
            }),
            (None, Some(actual_metric)) => discrepancies.push(Discrepancy {
                metric: key.to_owned(),
                metric_type: actual_metric.metric_type,
                expected: None,
                actual: Some(actual_metric.actual.clone()),
                reason: String::from("unexpected actual metric"),
                compatibility_note: None,
            }),
            (None, None) => {
                return Err(TestkitError::differential(
                    "internal key-union invariant failed",
                ));
            }
        }
    }

    Ok(DifferentialReport {
        expected_count: expected.len(),
        actual_count: actual.len(),
        discrepancies,
    })
}

/// Compare two TSV files and write a deterministic JSON report.
pub fn compare_files(expected: &Path, actual: &Path, report: &Path) -> Result<DifferentialReport> {
    let expected_text = fs::read_to_string(expected)
        .map_err(|source| TestkitError::io("read expected metrics", expected, source))?;
    let actual_text = fs::read_to_string(actual)
        .map_err(|source| TestkitError::io("read actual metrics", actual, source))?;
    let comparison = compare(
        &parse_expected(&expected_text)?,
        &parse_actual(&actual_text)?,
    )?;
    fs::write(report, format!("{}\n", comparison.to_json()))
        .map_err(|source| TestkitError::io("write discrepancy report", report, source))?;
    Ok(comparison)
}

fn compare_one(
    expected: &ExpectedMetric,
    actual: &ActualMetric,
    discrepancies: &mut Vec<Discrepancy>,
) -> Result<()> {
    if expected.metric_type != actual.metric_type {
        discrepancies.push(Discrepancy {
            metric: expected.metric.clone(),
            metric_type: expected.metric_type,
            expected: Some(expected.expected.clone()),
            actual: Some(actual.actual.clone()),
            reason: format!(
                "type mismatch: expected {}, observed {}",
                expected.metric_type.as_str(),
                actual.metric_type.as_str()
            ),
            compatibility_note: expected.compatibility_note.clone(),
        });
        return Ok(());
    }

    let equal = match expected.metric_type {
        MetricType::Integer => {
            let expected_value = parse_integer(&expected.expected)?;
            let actual_value = parse_integer(&actual.actual)?;
            expected_value == actual_value
        }
        MetricType::Decimal => {
            let decimals = expected.rounding_decimals.ok_or_else(|| {
                TestkitError::differential("decimal metric lacks explicit rounding")
            })?;
            round_decimal(&expected.expected, decimals)?
                == round_decimal(&actual.actual, decimals)?
        }
        MetricType::Text => expected.expected == actual.actual,
    };

    if !equal {
        discrepancies.push(Discrepancy {
            metric: expected.metric.clone(),
            metric_type: expected.metric_type,
            expected: Some(expected.expected.clone()),
            actual: Some(actual.actual.clone()),
            reason: String::from("value mismatch"),
            compatibility_note: expected.compatibility_note.clone(),
        });
    }

    Ok(())
}

fn parse_rows(text: &str, header: &str, field_count: usize) -> Result<Vec<(usize, Vec<String>)>> {
    let normalized = text.replace("\r\n", "\n");
    let mut lines = normalized.lines();
    let observed = lines
        .next()
        .ok_or_else(|| TestkitError::differential("result file is empty"))?;
    if observed != header {
        return Err(TestkitError::differential(format!(
            "unexpected header {observed:?}; expected {header:?}"
        )));
    }

    let mut rows = Vec::new();
    for (zero_index, line) in lines.enumerate() {
        let line_number = zero_index + 2;
        if line.is_empty() {
            return Err(TestkitError::differential(format!(
                "line {line_number}: blank lines are not allowed"
            )));
        }
        let fields: Vec<String> = line.split('\t').map(str::to_owned).collect();
        if fields.len() != field_count {
            return Err(TestkitError::differential(format!(
                "line {line_number}: expected {field_count} fields, found {}",
                fields.len()
            )));
        }
        rows.push((line_number, fields));
    }
    Ok(rows)
}

fn required<'a>(value: &'a str, line: usize, name: &str) -> Result<&'a str> {
    if value.is_empty() || value == "-" {
        Err(TestkitError::differential(format!(
            "line {line}: {name} is required"
        )))
    } else {
        Ok(value)
    }
}

fn optional(value: &str) -> Option<String> {
    (value != "-").then(|| value.to_owned())
}

fn validate_value(
    metric_type: MetricType,
    value: &str,
    rounding_decimals: Option<u32>,
) -> Result<()> {
    match metric_type {
        MetricType::Integer => {
            parse_integer(value)?;
        }
        MetricType::Decimal => {
            round_decimal(
                value,
                rounding_decimals.ok_or_else(|| {
                    TestkitError::differential("decimal metric lacks rounding")
                })?,
            )?;
        }
        MetricType::Text => {}
    }
    Ok(())
}

fn parse_integer(value: &str) -> Result<i128> {
    value.parse::<i128>().map_err(|error| {
        TestkitError::differential(format!("invalid integer {value:?}: {error}"))
    })
}

fn round_decimal(value: &str, decimals: u32) -> Result<i128> {
    let (negative, unsigned) = value.strip_prefix('-').map_or((false, value), |rest| {
        (true, rest)
    });
    let unsigned = unsigned.strip_prefix('+').unwrap_or(unsigned);
    let (whole_text, fractional_text) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    if whole_text.is_empty()
        || !whole_text.bytes().all(|byte| byte.is_ascii_digit())
        || !fractional_text.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(TestkitError::differential(format!(
            "invalid fixed decimal {value:?}"
        )));
    }

    let whole = whole_text.parse::<i128>().map_err(|error| {
        TestkitError::differential(format!("invalid decimal whole part {value:?}: {error}"))
    })?;
    let scale = 10_i128
        .checked_pow(decimals)
        .ok_or_else(|| TestkitError::differential("decimal scale overflow"))?;
    let decimals_usize = usize::try_from(decimals)
        .map_err(|_| TestkitError::differential("decimal precision does not fit usize"))?;
    let kept_text: String = fractional_text
        .chars()
        .take(decimals_usize)
        .chain(std::iter::repeat('0'))
        .take(decimals_usize)
        .collect();
    let kept = if kept_text.is_empty() {
        0
    } else {
        kept_text.parse::<i128>().map_err(|error| {
            TestkitError::differential(format!("invalid decimal fraction {value:?}: {error}"))
        })?
    };
    let mut scaled = whole
        .checked_mul(scale)
        .and_then(|base| base.checked_add(kept))
        .ok_or_else(|| TestkitError::differential("decimal scaling overflow"))?;

    let round_up = fractional_text
        .as_bytes()
        .get(decimals_usize)
        .is_some_and(|digit| *digit >= b'5');
    if round_up {
        scaled = scaled
            .checked_add(1)
            .ok_or_else(|| TestkitError::differential("decimal rounding overflow"))?;
    }

    if negative {
        scaled
            .checked_neg()
            .ok_or_else(|| TestkitError::differential("decimal sign overflow"))
    } else {
        Ok(scaled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_are_exact_and_decimals_use_declared_rounding() {
        let expected = parse_expected(concat!(
            "metric\ttype\texpected\trounding_decimals\tcompatibility_note\n",
            "count\tinteger\t7\t-\t-\n",
            "mean\tdecimal\t1.2344\t3\t-\n"
        ))
        .expect("parse expected");
        let actual = parse_actual(concat!(
            "metric\ttype\tactual\n",
            "count\tinteger\t7\n",
            "mean\tdecimal\t1.23449\n"
        ))
        .expect("parse actual");

        let report = compare(&expected, &actual).expect("compare");
        assert!(report.is_match());
    }

    #[test]
    fn no_blanket_epsilon_is_applied() {
        let expected = parse_expected(concat!(
            "metric\ttype\texpected\trounding_decimals\tcompatibility_note\n",
            "mean\tdecimal\t1.234\t3\t-\n"
        ))
        .expect("parse expected");
        let actual = parse_actual("metric\ttype\tactual\nmean\tdecimal\t1.235\n")
            .expect("parse actual");

        let report = compare(&expected, &actual).expect("compare");
        assert!(!report.is_match());
        assert!(report.to_json().contains("\"value mismatch\""));
    }

    #[test]
    fn missing_and_unexpected_metrics_are_reported() {
        let expected = parse_expected(concat!(
            "metric\ttype\texpected\trounding_decimals\tcompatibility_note\n",
            "expected_only\tinteger\t1\t-\tprofile-note\n"
        ))
        .expect("parse expected");
        let actual = parse_actual("metric\ttype\tactual\nactual_only\tinteger\t2\n")
            .expect("parse actual");

        let report = compare(&expected, &actual).expect("compare");
        assert_eq!(report.discrepancies.len(), 2);
        assert!(report.to_json().contains("profile-note"));
    }
}
