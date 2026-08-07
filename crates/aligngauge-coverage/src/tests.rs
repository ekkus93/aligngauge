use std::path::PathBuf;

use aligngauge_core::{ErrorCategory, ToJson};

use crate::cigar::record_is_accepted;
use crate::util::{format_percentage_six, format_ratio_six};
use crate::{CoverageMemoryPlan, CoverageOptions, analyze_bam, cigar_to_coverage_blocks};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("testdata/fixtures")
        .join(format!("{name}.bam"))
}

fn encode(length: u32, operation: u32) -> u32 {
    (length << 4) | operation
}

#[test]
fn canonical_policy_excludes_every_named_record_class() {
    assert!(record_is_accepted(0));
    for flags in [0x4, 0x100, 0x200, 0x400, 0x800] {
        assert!(!record_is_accepted(flags));
    }
}

#[test]
fn cigar_operations_emit_only_m_eq_and_x_blocks() {
    let raw = [
        encode(2, 4),
        encode(3, 0),
        encode(1, 1),
        encode(2, 7),
        encode(1, 8),
        encode(1, 2),
        encode(4, 3),
        encode(1, 6),
        encode(1, 5),
    ];
    let blocks = cigar_to_coverage_blocks(&raw, 500, 1_000).expect("convert CIGAR");
    assert_eq!(blocks.len(), 3);
    assert_eq!((blocks[0].start, blocks[0].end), (500, 503));
    assert_eq!((blocks[1].start, blocks[1].end), (503, 505));
    assert_eq!((blocks[2].start, blocks[2].end), (505, 506));
}

#[test]
fn very_long_deletion_and_skip_advance_without_coverage() {
    let raw = [
        encode(1, 0),
        encode(100_000_000, 2),
        encode(1, 7),
        encode(100_000_000, 3),
        encode(1, 8),
    ];
    let blocks = cigar_to_coverage_blocks(&raw, 10, 250_000_100).expect("convert CIGAR");
    assert_eq!(blocks.len(), 3);
    assert_eq!((blocks[0].start, blocks[0].end), (10, 11));
    assert_eq!((blocks[1].start, blocks[1].end), (100_000_011, 100_000_012));
    assert_eq!((blocks[2].start, blocks[2].end), (200_000_012, 200_000_013));
}

#[test]
fn out_of_bounds_block_is_fatal() {
    let error = cigar_to_coverage_blocks(&[encode(11, 0)], 95, 100)
        .expect_err("block must exceed reference");
    assert_eq!(error.category(), ErrorCategory::InputCorrupt);
}

#[test]
fn memory_planner_accounts_for_tracks_and_fails_low_memory() {
    let one = CoverageMemoryPlan::plan(4_u64 << 30, 1, None).expect("one-track plan");
    let three = CoverageMemoryPlan::plan(4_u64 << 30, 3, None).expect("three-track plan");
    assert_eq!(one.chunk_size_bases, 65_536);
    assert_eq!(three.chunk_size_bases, 65_536);
    assert!(three.planned_peak_bytes > one.planned_peak_bytes);
    assert!(one.planned_peak_bytes <= one.memory_limit_bytes);
    assert!(three.planned_peak_bytes <= three.memory_limit_bytes);

    let error = CoverageMemoryPlan::plan(256_u64 << 20, 1, None)
        .expect_err("low-memory plan must fail before traversal");
    assert_eq!(error.category(), ErrorCategory::ResourceLimit);
}

#[test]
fn committed_fixture_accepted_base_totals_match_policy() {
    for (name, expected) in [
        ("empty", 0),
        ("basic", 19),
        ("cigar_ops", 6),
        ("flags_and_pairs", 30),
        ("chunk_boundary", 28),
        ("multi_track", 10),
        ("tags_and_read_groups", 15),
        ("unmapped_tail", 5),
        ("integer_boundary", 1),
        ("zero_length_reference", 0),
        ("long_cigar", 66_000),
    ] {
        let report = analyze_bam(fixture(name), CoverageOptions::default())
            .unwrap_or_else(|error| panic!("{name}: {}", error.render_human(true)));
        assert_eq!(report.total_accepted_aligned_bases(), expected, "{name}");
    }
}

#[test]
fn chunk_size_does_not_change_canonical_coverage() {
    let mut baseline = None;
    for chunk_size in [1, 7, 64, 1_024, 65_536] {
        let options = CoverageOptions::default().with_chunk_size_override(chunk_size);
        let report = analyze_bam(fixture("chunk_boundary"), options).expect("coverage");
        let canonical = report.to_json_pretty();
        if let Some(expected) = &baseline {
            assert_eq!(&canonical, expected);
        } else {
            baseline = Some(canonical);
        }
    }
}

#[test]
fn empty_contigs_and_reference_transitions_reduce_exactly() {
    let basic = analyze_bam(fixture("basic"), CoverageOptions::default()).expect("coverage");
    assert_eq!(basic.per_reference().len(), 2);
    assert_eq!(basic.per_reference()[0].name, "chr1");
    assert_eq!(basic.per_reference()[0].accepted_aligned_bases, 10);
    assert_eq!(basic.per_reference()[0].covered_reference_bases, 10);
    assert_eq!(basic.per_reference()[0].mean_depth, "0.000010");
    assert_eq!(basic.per_reference()[1].name, "chr2");
    assert_eq!(basic.per_reference()[1].accepted_aligned_bases, 9);

    let empty = analyze_bam(fixture("zero_length_reference"), CoverageOptions::default())
        .expect("zero-length coverage");
    assert_eq!(empty.per_reference().len(), 1);
    assert_eq!(empty.per_reference()[0].length, 0);
    assert_eq!(empty.per_reference()[0].mean_depth, "0.000000");
    assert_eq!(empty.depth_histogram().values().sum::<u64>(), 0);
}

#[test]
fn histogram_properties_hold_on_overlapping_fixture() {
    let report =
        analyze_bam(fixture("chunk_boundary"), CoverageOptions::default()).expect("coverage");
    let territory = report.depth_histogram().values().copied().sum::<u64>();
    assert_eq!(territory, 2_000_000);
    let weighted = report
        .depth_histogram()
        .iter()
        .map(|(depth, count)| u128::from(*depth) * u128::from(*count))
        .sum::<u128>();
    assert_eq!(weighted, u128::from(report.total_accepted_aligned_bases()));
}

#[test]
fn excluded_records_do_not_change_canonical_coverage() {
    let report = analyze_bam(fixture("multi_track"), CoverageOptions::default()).expect("coverage");
    assert_eq!(report.total_accepted_aligned_bases(), 10);
    assert_eq!(report.depth_histogram().get(&1), Some(&10));
    assert_eq!(report.depth_histogram().get(&0), Some(&1_999_990));
}

#[test]
fn deterministic_cigar_fuzz_matches_per_base_oracle() {
    let mut state = 0x9e37_79b9_u64;
    for _ in 0..2_000 {
        let mut raw = Vec::new();
        let mut expected = Vec::new();
        let mut cursor = 0_usize;
        for _ in 0..32 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let operation = u32::try_from((state >> 32) % 9).expect("operation fits u32");
            let length = u32::try_from(((state >> 40) % 20) + 1).expect("length fits u32");
            raw.push(encode(length, operation));
            match operation {
                0 | 7 | 8 => {
                    let length = usize::try_from(length).expect("u32 fits usize");
                    expected.extend(cursor..cursor + length);
                    cursor += length;
                }
                2 | 3 => cursor += usize::try_from(length).expect("u32 fits usize"),
                _ => {}
            }
        }
        let blocks =
            cigar_to_coverage_blocks(&raw, 0, u64::try_from(cursor + 1).expect("cursor fits u64"))
                .expect("fuzz CIGAR must convert");
        let observed = blocks
            .iter()
            .flat_map(|block| block.start..block.end)
            .map(|position| usize::try_from(position).expect("position fits usize"))
            .collect::<Vec<_>>();
        assert_eq!(observed, expected);
    }
}

#[test]
fn ratio_rounding_is_deterministic() {
    assert_eq!(format_ratio_six(1, 3).expect("ratio"), "0.333333");
    assert_eq!(format_ratio_six(2, 3).expect("ratio"), "0.666667");
    assert_eq!(format_ratio_six(0, 0).expect("ratio"), "0.000000");
    assert_eq!(
        format_percentage_six(1, 4).expect("percentage"),
        "25.000000"
    );
}
