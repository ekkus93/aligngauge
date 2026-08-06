use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

use aligngauge_core::ErrorCategory;
use aligngauge_hts::{
    BamReader, FieldPlan, FieldValue, ReadGroupValue, ReaderOptions, SortOrder,
};
use aligngauge_testkit::bam::{
    CigarOp, RecordSpec, ReferenceSpec, aux_string, write_bam,
};

static NEXT_WORKSPACE_ID: AtomicU64 = AtomicU64::new(0);

struct TestWorkspace {
    root: PathBuf,
}

impl TestWorkspace {
    fn create() -> Self {
        let id = NEXT_WORKSPACE_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "aligngauge-m3-validation-{}-{id}",
            process::id()
        ));
        fs::create_dir_all(&root).expect("create test workspace");
        Self { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn every_committed_valid_v01_fixture_streams() {
    for fixture in [
        "basic",
        "chunk_boundary",
        "cigar_ops",
        "empty",
        "flags_and_pairs",
        "integer_boundary",
        "long_cigar",
        "multi_track",
        "tags_and_read_groups",
        "unmapped_tail",
        "zero_length_reference",
    ] {
        let path = fixture_path(&format!("{fixture}.bam"));
        let count = stream_count(&path, combined_plan())
            .unwrap_or_else(|error| panic!("fixture {fixture} failed: {error}"));
        if fixture == "empty" || fixture == "zero_length_reference" {
            assert_eq!(count, 0, "fixture {fixture}");
        } else {
            assert!(count > 0, "fixture {fixture}");
        }
    }
}

#[test]
fn corrupt_and_unsorted_fixtures_fail_closed_with_expected_categories() {
    for (fixture, category) in [
        ("coordinate_regression.bam", ErrorCategory::InputUnsorted),
        ("malformed_optional_tag.bam", ErrorCategory::InputCorrupt),
        ("malformed_record_length.bam", ErrorCategory::InputCorrupt),
        ("truncated_bgzf.bam", ErrorCategory::InputCorrupt),
        ("unknown_reference_id.bam", ErrorCategory::InputCorrupt),
    ] {
        let error = stream_count(&fixture_path(fixture), combined_plan())
            .expect_err("invalid fixture must fail");
        assert_eq!(error.category(), category, "fixture {fixture}: {error}");
    }
}

#[test]
fn long_cigar_is_expanded_by_the_pinned_backend() {
    let mut reader = open_fixture("long_cigar.bam", FieldPlan::coverage());
    let record = reader
        .next_record()
        .expect("read long CIGAR")
        .expect("long CIGAR record");
    let FieldValue::Value(cigar) = record.cigar() else {
        panic!("CIGAR was not exposed");
    };

    assert_eq!(cigar.operation_count, 66_000);
    assert_eq!(cigar.query_span, 66_000);
    assert_eq!(cigar.reference_span, 66_000);
    assert!(cigar.long_cigar_expanded);
    assert_eq!(record.raw_cigar().expect("raw CIGAR").len(), 66_000);
}

#[test]
fn optional_tags_remain_explicit_and_read_groups_are_not_invented() {
    let mut reader = open_fixture(
        "tags_and_read_groups.bam",
        FieldPlan::counters().with_optional_tags(),
    );

    let complete = reader
        .next_record()
        .expect("read complete tag record")
        .expect("complete tag record");
    assert_eq!(complete.edit_distance(), &FieldValue::Value(1));
    assert_eq!(
        complete.mismatch_descriptor(),
        &FieldValue::Value(String::from("2A2"))
    );
    assert_eq!(
        complete.read_group(),
        &ReadGroupValue::Known(String::from("known"))
    );

    let missing = reader
        .next_record()
        .expect("read missing tag record")
        .expect("missing tag record");
    assert_eq!(missing.edit_distance(), &FieldValue::Missing);
    assert_eq!(missing.mismatch_descriptor(), &FieldValue::Missing);
    assert_eq!(missing.read_group(), &ReadGroupValue::Missing);

    let unknown = reader
        .next_record()
        .expect("read unknown read-group record")
        .expect("unknown read-group record");
    assert_eq!(
        unknown.read_group(),
        &ReadGroupValue::Unknown(String::from("not-declared"))
    );
}

#[test]
fn duplicate_read_group_declarations_are_ambiguous_not_silently_selected() {
    let workspace = TestWorkspace::create();
    let path = workspace.path("ambiguous-rg.bam");
    let references = standard_references();
    let header = concat!(
        "@HD\tVN:1.6\tSO:coordinate\n",
        "@SQ\tSN:chr1\tLN:1000\n",
        "@RG\tID:duplicate\tSM:first\n",
        "@RG\tID:duplicate\tSM:second\n"
    );
    let mut record = mapped_record("ambiguous", 0, 10, 4);
    record.auxiliary = aux_string(*b"RG", "duplicate");
    write_bam(&path, header, &references, &[record]).expect("write BAM");

    let mut reader = BamReader::open(
        &path,
        FieldPlan::counters().with_optional_tags(),
        ReaderOptions::default(),
    )
    .expect("open BAM");
    let record = reader
        .next_record()
        .expect("read BAM")
        .expect("record");
    assert_eq!(
        record.read_group(),
        &ReadGroupValue::Ambiguous(String::from("duplicate"))
    );
}

#[test]
fn duplicate_and_contradictory_reference_declarations_are_rejected() {
    for (name, header) in [
        (
            "duplicate",
            "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:1000\n@SQ\tSN:chr1\tLN:1000\n",
        ),
        (
            "contradictory",
            "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:1000\n@SQ\tSN:chr1\tLN:999\n",
        ),
    ] {
        let workspace = TestWorkspace::create();
        let path = workspace.path(&format!("{name}.bam"));
        write_bam(&path, header, &standard_references(), &[]).expect("write BAM");

        let error = BamReader::open(&path, FieldPlan::counters(), ReaderOptions::default())
            .err()
            .expect("duplicate @SQ must fail");
        assert_eq!(error.category(), ErrorCategory::InputFormat);
    }
}

#[test]
fn invalid_reference_length_is_rejected() {
    let workspace = TestWorkspace::create();
    let path = workspace.path("negative-length.bam");
    let header = "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:-1\n";
    write_bam(&path, header, &standard_references(), &[]).expect("write BAM");

    let error = BamReader::open(&path, FieldPlan::counters(), ReaderOptions::default())
        .err()
        .expect("negative length must fail");
    assert_eq!(error.category(), ErrorCategory::InputFormat);
}

#[test]
fn actual_sorted_coordinates_are_accepted_without_a_coordinate_header_claim() {
    for (name, header, expected_sort_order) in [
        (
            "absent",
            "@SQ\tSN:chr1\tLN:1000\n",
            SortOrder::Absent,
        ),
        (
            "unknown",
            "@HD\tVN:1.6\tSO:unknown\n@SQ\tSN:chr1\tLN:1000\n",
            SortOrder::Unknown,
        ),
    ] {
        let workspace = TestWorkspace::create();
        let path = workspace.path(&format!("{name}.bam"));
        let records = [
            mapped_record("first", 0, 10, 4),
            mapped_record("second", 0, 20, 4),
        ];
        write_bam(&path, header, &standard_references(), &records).expect("write BAM");

        let mut reader = BamReader::open(&path, FieldPlan::counters(), ReaderOptions::default())
            .expect("open sorted BAM");
        assert_eq!(reader.header().sort_order(), &expected_sort_order);
        assert_eq!(drain(&mut reader).expect("stream sorted BAM"), 2);
    }
}

#[test]
fn coordinate_header_claim_does_not_hide_a_regression() {
    let error = stream_count(
        &fixture_path("coordinate_regression.bam"),
        FieldPlan::counters(),
    )
    .expect_err("regression must fail");
    assert_eq!(error.category(), ErrorCategory::InputUnsorted);
    let rendered = error.render_human(false);
    assert!(rendered.contains("previous_position"));
    assert!(rendered.contains("current_position"));
    assert!(!rendered.contains("earlier"));
    assert!(error.render_human(true).contains("earlier"));
}

#[test]
fn mapped_record_after_no_coordinate_tail_is_rejected() {
    let workspace = TestWorkspace::create();
    let path = workspace.path("mapped-after-tail.bam");
    let records = [
        mapped_record("anchor", 0, 10, 4),
        RecordSpec::unmapped("tail", "NNNN"),
        mapped_record("late", 0, 20, 4),
    ];
    write_bam(&path, standard_header(), &standard_references(), &records).expect("write BAM");

    let error = stream_count(&path, FieldPlan::counters()).expect_err("tail violation must fail");
    assert_eq!(error.category(), ErrorCategory::InputUnsorted);
}

#[test]
fn reference_bound_violation_is_rejected() {
    let workspace = TestWorkspace::create();
    let path = workspace.path("past-reference.bam");
    let record = mapped_record("past-end", 0, 998, 4);
    write_bam(
        &path,
        standard_header(),
        &standard_references(),
        &[record],
    )
    .expect("write BAM");

    let error = stream_count(&path, FieldPlan::coverage()).expect_err("past-end record must fail");
    assert_eq!(error.category(), ErrorCategory::InputCorrupt);
    assert!(error.render_human(false).contains("reference_length"));
}

#[test]
fn reserved_flag_bits_are_rejected_as_unsupported() {
    let workspace = TestWorkspace::create();
    let path = workspace.path("reserved-flags.bam");
    let mut record = mapped_record("reserved", 0, 10, 4);
    record.flags = 0x1000;
    write_bam(
        &path,
        standard_header(),
        &standard_references(),
        &[record],
    )
    .expect("write BAM");

    let error = stream_count(&path, FieldPlan::counters()).expect_err("reserved flag must fail");
    assert_eq!(error.category(), ErrorCategory::UnsupportedRecord);
}

#[test]
fn unrequested_optional_fields_are_not_exposed() {
    let mut reader = open_fixture("tags_and_read_groups.bam", FieldPlan::counters());
    let record = reader
        .next_record()
        .expect("read record")
        .expect("record");

    assert_eq!(record.edit_distance(), &FieldValue::NotRequested);
    assert_eq!(record.mismatch_descriptor(), &FieldValue::NotRequested);
    assert_eq!(record.read_group(), &ReadGroupValue::NotRequested);
    assert!(record.raw_cigar().is_none());
}

#[test]
fn header_identity_and_field_plan_are_deterministic() {
    let first = open_fixture("basic.bam", combined_plan());
    let second = open_fixture("basic.bam", combined_plan());
    assert_eq!(first.header().identity(), second.header().identity());
    assert_eq!(
        first.field_plan().to_json().to_compact_string(),
        second.field_plan().to_json().to_compact_string()
    );
    assert_eq!(first.header().identity().sha256().len(), 64);
}

fn combined_plan() -> FieldPlan {
    FieldPlan::counters()
        .union(&FieldPlan::coverage())
        .with_optional_tags()
}

fn open_fixture(name: &str, plan: FieldPlan) -> BamReader {
    BamReader::open(fixture_path(name), plan, ReaderOptions::default())
        .unwrap_or_else(|error| panic!("open fixture {name}: {error}"))
}

fn stream_count(path: &Path, plan: FieldPlan) -> Result<u64, aligngauge_core::AlignGaugeError> {
    let mut reader = BamReader::open(path, plan, ReaderOptions::default())?;
    drain(&mut reader)
}

fn drain(reader: &mut BamReader) -> Result<u64, aligngauge_core::AlignGaugeError> {
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

fn standard_references() -> Vec<ReferenceSpec> {
    vec![ReferenceSpec {
        name: String::from("chr1"),
        length: 1000,
    }]
}

fn standard_header() -> &'static str {
    "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:1000\n"
}

fn mapped_record(name: &str, reference_id: i32, position: i32, length: u32) -> RecordSpec {
    RecordSpec::mapped(
        name,
        reference_id,
        position,
        vec![CigarOp::new(length, 'M')],
        "A".repeat(usize::try_from(length).expect("test length fits usize")),
    )
}
