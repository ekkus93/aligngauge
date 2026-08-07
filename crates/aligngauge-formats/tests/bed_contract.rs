use std::path::PathBuf;

use aligngauge_core::ErrorCategory;
use aligngauge_formats::{
    SequenceContig, SequenceDictionary, TargetNormalizationConfig, normalize_targets,
    parse_bed_bytes, parse_bed_path,
};

fn dictionary() -> SequenceDictionary {
    SequenceDictionary::new(vec![
        SequenceContig {
            name: "chr1".to_owned(),
            length: 120,
        },
        SequenceContig {
            name: "chr2".to_owned(),
            length: 80,
        },
    ])
    .expect("test dictionary is valid")
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendor_style_capture_panel.bed")
}

#[test]
fn committed_vendor_style_fixture_preserves_identity_and_normalizes_exactly() {
    let path = fixture_path();
    let parsed = parse_bed_path(&path, &dictionary()).expect("fixture should parse");

    assert_eq!(
        parsed.identity.path.as_deref(),
        Some(path.to_string_lossy().as_ref())
    );
    assert_eq!(parsed.identity.size_bytes, 275);
    assert_eq!(
        parsed.identity.sha256,
        "ebab4ad34dc4b17fbc418a3d3274003aa2d1927dae47f18f4cef6761c54b9094"
    );
    assert_eq!(parsed.identity.source_interval_count, 4);
    assert_eq!(
        parsed
            .intervals
            .iter()
            .map(|interval| interval.line_number)
            .collect::<Vec<_>>(),
        [4, 5, 6, 7]
    );
    assert_eq!(parsed.intervals[0].name.as_deref(), Some("GENE1_exon1"));
    assert_eq!(parsed.intervals[3].name.as_deref(), Some("GENE3_exon1"));
    assert_eq!(parsed.stats.track_lines_skipped, 1);
    assert_eq!(parsed.stats.browser_lines_skipped, 1);
    assert_eq!(parsed.stats.comment_lines_skipped, 1);

    let targets = normalize_targets(parsed, TargetNormalizationConfig { flank_bases: 5 })
        .expect("fixture normalization should succeed");
    assert_eq!(targets.merged_intervals.len(), 3);
    assert_eq!(targets.merged_intervals[0].contig, "chr1");
    assert_eq!(
        (
            targets.merged_intervals[0].start,
            targets.merged_intervals[0].end
        ),
        (5, 40)
    );
    assert_eq!(targets.merged_intervals[0].source_interval_indices, [0, 1]);
    assert_eq!(
        (
            targets.merged_intervals[1].start,
            targets.merged_intervals[1].end
        ),
        (85, 105)
    );
    assert_eq!(targets.merged_intervals[2].contig, "chr2");
    assert_eq!(
        (
            targets.merged_intervals[2].start,
            targets.merged_intervals[2].end
        ),
        (0, 17)
    );
    assert_eq!(targets.normalization.overlap_merges, 1);
    assert_eq!(targets.normalization.left_flank_clips, 0);
    assert_eq!(targets.normalization.aggregate_territory_bases, 72);

    let actions = targets.provenance_actions();
    assert!(actions.iter().any(|value| value == "targets:flank_bases=5"));
    assert!(
        actions
            .iter()
            .any(|value| value == "targets:aggregate_territory_bases=72")
    );
    assert!(actions.iter().any(|value| {
        value == "targets:sha256=ebab4ad34dc4b17fbc418a3d3274003aa2d1927dae47f18f4cef6761c54b9094"
    }));
}

#[test]
fn mixed_ascii_horizontal_whitespace_is_accepted_without_collapsing_empty_tab_fields() {
    let parsed = parse_bed_bytes(
        b"chr1  0\t10  A\t0 +\nchr1\t20  30\tB 0\t-\n",
        &dictionary(),
    )
    .expect("mixed ASCII horizontal whitespace should parse");
    assert_eq!(parsed.intervals.len(), 2);
    assert_eq!(parsed.intervals[0].name.as_deref(), Some("A"));
    assert_eq!(parsed.intervals[1].name.as_deref(), Some("B"));

    let error = parse_bed_bytes(b"chr1\t0\t10\t\t0\t+\n", &dictionary())
        .expect_err("an empty tab-delimited BED field must fail");
    assert_eq!(error.category(), ErrorCategory::TargetFormat);
}

#[test]
fn inconsistent_bed_width_is_fatal() {
    let error = parse_bed_bytes(b"chr1\t0\t10\nchr1\t20\t30\tname\n", &dictionary())
        .expect_err("mixed BED3 and BED4 records must fail");
    assert_eq!(error.category(), ErrorCategory::TargetFormat);
}

#[test]
fn deterministic_normalization_is_independent_of_source_order() {
    let first = parse_bed_bytes(
        b"chr2\t10\t20\nchr1\t30\t40\nchr1\t10\t25\nchr1\t20\t35\n",
        &dictionary(),
    )
    .expect("first permutation should parse");
    let second = parse_bed_bytes(
        b"chr1\t20\t35\nchr1\t10\t25\nchr2\t10\t20\nchr1\t30\t40\n",
        &dictionary(),
    )
    .expect("second permutation should parse");

    let first = normalize_targets(first, TargetNormalizationConfig::default())
        .expect("first permutation should normalize");
    let second = normalize_targets(second, TargetNormalizationConfig::default())
        .expect("second permutation should normalize");

    let geometry = |targets: &aligngauge_formats::TargetSet| {
        targets
            .merged_intervals
            .iter()
            .map(|interval| (interval.contig.clone(), interval.start, interval.end))
            .collect::<Vec<_>>()
    };
    assert_eq!(geometry(&first), geometry(&second));
    assert_eq!(
        first.normalization.aggregate_territory_bases,
        second.normalization.aggregate_territory_bases
    );
}

#[test]
fn missing_target_path_and_duplicate_dictionary_are_typed_failures() {
    let missing = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/definitely-missing-target-file.bed");
    let error = parse_bed_path(&missing, &dictionary()).expect_err("missing target must fail");
    assert_eq!(error.category(), ErrorCategory::InputNotFound);

    let error = SequenceDictionary::new(vec![
        SequenceContig {
            name: "chr1".to_owned(),
            length: 10,
        },
        SequenceContig {
            name: "chr1".to_owned(),
            length: 20,
        },
    ])
    .expect_err("duplicate dictionary contigs must fail");
    assert_eq!(error.category(), ErrorCategory::Configuration);
}
