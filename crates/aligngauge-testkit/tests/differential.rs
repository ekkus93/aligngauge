use std::fs;
use std::path::Path;

use aligngauge_testkit::compare_files;

#[test]
fn writes_machine_readable_discrepancy_report() {
    let root = std::env::temp_dir().join(format!("aligngauge-differential-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create test directory");

    let expected = root.join("expected.tsv");
    let actual = root.join("actual.tsv");
    let report = root.join("report.json");
    fs::write(
        &expected,
        concat!(
            "metric\ttype\texpected\trounding_decimals\tcompatibility_note\n",
            "count\tinteger\t1\t-\t-\n"
        ),
    )
    .expect("write expected");
    fs::write(&actual, "metric\ttype\tactual\ncount\tinteger\t2\n").expect("write actual");

    let comparison = compare_files(&expected, &actual, &report).expect("compare files");
    assert!(!comparison.is_match());
    let json = fs::read_to_string(&report).expect("read report");
    assert!(json.contains("\"match\":false"));
    assert!(json.ends_with('\n'));

    fs::remove_dir_all(root).expect("remove test directory");
}

#[test]
fn committed_expected_files_use_strict_headers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repository root");
    for name in ["basic.tsv", "cigar_ops.tsv", "flags_and_pairs.tsv"] {
        let text = fs::read_to_string(root.join("testdata/expected").join(name))
            .expect("read expected file");
        assert!(
            text.starts_with("metric\ttype\texpected\trounding_decimals\tcompatibility_note\n")
        );
    }
}
