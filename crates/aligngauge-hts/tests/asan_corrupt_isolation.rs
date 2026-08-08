use std::path::{Path, PathBuf};

use aligngauge_core::ErrorCategory;
use aligngauge_hts::{BamReader, FieldPlan, ReaderOptions};

#[test]
fn coordinate_regression_is_leak_clean() {
    assert_category("coordinate_regression.bam", ErrorCategory::InputUnsorted);
}

#[test]
fn malformed_optional_tag_is_leak_clean() {
    assert_category("malformed_optional_tag.bam", ErrorCategory::InputCorrupt);
}

#[test]
fn malformed_record_length_is_leak_clean() {
    assert_category("malformed_record_length.bam", ErrorCategory::InputCorrupt);
}

#[test]
fn truncated_bgzf_is_leak_clean() {
    assert_category("truncated_bgzf.bam", ErrorCategory::InputCorrupt);
}

#[test]
fn unknown_reference_id_is_leak_clean() {
    assert_category("unknown_reference_id.bam", ErrorCategory::InputCorrupt);
}

fn assert_category(name: &str, category: ErrorCategory) {
    let error = stream_count(&fixture_path(name), combined_plan()).expect_err("fixture must fail");
    assert_eq!(error.category(), category, "fixture {name}: {error}");
}

fn combined_plan() -> FieldPlan {
    FieldPlan::counters()
        .union(&FieldPlan::coverage())
        .with_optional_tags()
}

fn stream_count(path: &Path, plan: FieldPlan) -> Result<u64, aligngauge_core::AlignGaugeError> {
    let mut reader = BamReader::open(path, plan, ReaderOptions::default())?;
    let mut count = 0_u64;
    while reader.next_record()?.is_some() {
        count = count.checked_add(1).ok_or_else(|| {
            aligngauge_core::AlignGaugeError::new(
                ErrorCategory::InternalInvariant,
                "test traversal count overflowed",
            )
        })?;
    }
    Ok(count)
}

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("testdata/fixtures")
        .join(name)
}
