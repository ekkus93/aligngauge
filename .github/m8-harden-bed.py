#!/usr/bin/env python3
"""Apply the temporary Milestone 8 BED parser hardening edits idempotently."""

from pathlib import Path
import re

PATH = Path("crates/aligngauge-formats/src/bed.rs")
text = PATH.read_text()

old_state = "    let mut stats = BedParseStats::default();\n    let mut intervals = Vec::new();"
new_state = (
    "    let mut stats = BedParseStats::default();\n"
    "    let mut intervals = Vec::new();\n"
    "    let mut expected_field_count = None;"
)
if text.count(old_state) == 1:
    text = text.replace(old_state, new_state, 1)
elif text.count("    let mut expected_field_count = None;") != 1:
    raise SystemExit("parser-state initialization is neither original nor hardened")

old_parse = (
    "        let fields = split_fields(line, line_number)?;\n"
    "        let interval = parse_interval(&fields, line_number, intervals.len(), dictionary)?;"
)
new_parse = (
    "        let fields = split_fields(line, line_number)?;\n"
    "        validate_field_count_consistency(&fields, line_number, &mut expected_field_count)?;\n"
    "        let interval = parse_interval(&fields, line_number, intervals.len(), dictionary)?;"
)
if text.count(old_parse) == 1:
    text = text.replace(old_parse, new_parse, 1)
elif text.count(
    "validate_field_count_consistency(&fields, line_number, &mut expected_field_count)?;"
) != 1:
    raise SystemExit("parser interval path is neither original nor hardened")

field_parser = re.compile(
    r"fn split_fields\(line: &str, line_number: u64\) -> Result<Vec<&str>, AlignGaugeError> \{"
    r".*?\n\}\n\n(?:fn validate_field_count_consistency\(.*?\n\}\n\n)?fn parse_interval\(",
    re.S,
)
field_replacement = r'''fn split_fields(line: &str, line_number: u64) -> Result<Vec<&str>, AlignGaugeError> {
    if line.contains('\t')
        && line
            .split('\t')
            .any(|segment| segment.trim_matches(' ').is_empty())
    {
        return Err(target_format_error(
            line_number,
            "BED interval contains an empty tab-delimited field",
        ));
    }
    let fields: Vec<&str> = line.split_ascii_whitespace().collect();
    if !(3..=12).contains(&fields.len()) {
        return Err(target_format_error(
            line_number,
            "BED interval must contain 3 through 12 fields",
        )
        .with_detail(
            "field_count",
            u64::try_from(fields.len()).unwrap_or(u64::MAX),
        ));
    }
    Ok(fields)
}

fn validate_field_count_consistency(
    fields: &[&str],
    line_number: u64,
    expected_field_count: &mut Option<usize>,
) -> Result<(), AlignGaugeError> {
    if let Some(expected) = *expected_field_count {
        if fields.len() != expected {
            return Err(target_format_error(
                line_number,
                "BED field count is inconsistent within the target dataset",
            )
            .with_detail(
                "expected_field_count",
                u64::try_from(expected).unwrap_or(u64::MAX),
            )
            .with_detail(
                "actual_field_count",
                u64::try_from(fields.len()).unwrap_or(u64::MAX),
            ));
        }
    } else {
        *expected_field_count = Some(fields.len());
    }
    Ok(())
}

fn parse_interval('''
text, count = field_parser.subn(lambda _match: field_replacement, text, count=1)
if count != 1:
    raise SystemExit(f"expected one BED field parser block, found {count}")

if 'BED3 should parse' not in text:
    unit_test = re.compile(
        r"    #\[test\]\n    fn accepts_bed3_through_bed12_and_preserves_optional_fields\(\) \{"
        r".*?\n    \}\n",
        re.S,
    )
    unit_replacement = r'''    #[test]
    fn accepts_bed3_through_bed12_and_preserves_optional_fields() {
        let bed3 = parse_bed_bytes(b"chr1\t0\t1\n", &dictionary())
            .expect("BED3 should parse");
        assert_eq!(bed3.intervals[0].name, None);

        let bed12 = parse_bed_bytes(
            b"chr1\t1\t2\tname\t1\t+\t1\t2\t255,0,0\t1\t1\t0\n",
            &dictionary(),
        )
        .expect("BED12 should parse");
        assert_eq!(bed12.intervals[0].name.as_deref(), Some("name"));
        assert_eq!(bed12.intervals[0].extra_fields.len(), 8);
    }
'''
    text, count = unit_test.subn(lambda _match: unit_replacement, text, count=1)
    if count != 1:
        raise SystemExit(f"expected one BED3/BED12 unit test, found {count}")
elif text.count('BED3 should parse') != 1:
    raise SystemExit("hardened BED3/BED12 test is duplicated")

PATH.write_text(text)
