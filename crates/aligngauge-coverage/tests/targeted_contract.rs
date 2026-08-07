use std::path::PathBuf;

use aligngauge_core::{Availability, TargetedCoverageSummary};
use aligngauge_coverage::{CoverageOptions, analyze_bam_with_targets};

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn alignment_fixture() -> PathBuf {
    repository_root().join("testdata/fixtures/chunk_boundary.bam")
}

fn target_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/chunk_boundary_targets.bed")
}

fn assert_aggregate_oracle(targeted: &TargetedCoverageSummary, total_aligned_bases: u64) {
    assert_eq!(targeted.profile, "aligngauge-targeted-v0.3");
    assert_eq!(targeted.coverage_profile, "aligngauge-v0.1");
    assert!(targeted.duplicate_adjusted);
    assert_eq!(targeted.source_interval_count, 4);
    assert_eq!(targeted.near_distance_bases, 5);
    assert_eq!(targeted.genome_territory_bases, 2_000_000);
    assert_eq!(targeted.target_territory_bases, 22);
    assert_eq!(targeted.near_target_territory_bases, 16);
    assert_eq!(targeted.on_target_bases, 16);
    assert_eq!(targeted.near_target_bases, 12);
    assert_eq!(targeted.off_target_bases, 0);
    assert_eq!(
        targeted.on_target_bases + targeted.near_target_bases + targeted.off_target_bases,
        total_aligned_bases
    );
    assert_eq!(
        targeted.target_depth_histogram,
        [("0".to_owned(), 10), ("1".to_owned(), 8), ("2".to_owned(), 4)]
            .into_iter()
            .collect()
    );
    assert_eq!(
        targeted.target_mean_depth,
        Availability::Available(String::from("0.727273"))
    );
    assert_eq!((targeted.target_covered_bases, targeted.target_uncovered_bases), (12, 10));
    assert_eq!(targeted.threshold_bases.get("1"), Some(&12));
    assert_eq!(targeted.threshold_bases.get("2"), Some(&4));
    assert_eq!(
        targeted.threshold_percentages.get("1"),
        Some(&Availability::Available(String::from("54.545455")))
    );
    assert_eq!(
        targeted.threshold_percentages.get("2"),
        Some(&Availability::Available(String::from("18.181818")))
    );
    assert_eq!(targeted.dropout_target_count, 1);
    assert_eq!(
        targeted.target_enrichment,
        Availability::Available(String::from("51948.051948"))
    );
    assert_eq!(targeted.target_depth_20th_percentile, Availability::Available(0));
    assert_eq!(
        targeted.target_uniformity_penalty_80,
        Availability::unavailable("target_depth_20th_percentile_is_zero")
    );
}

fn assert_per_target_oracle(targeted: &TargetedCoverageSummary) {
    assert_eq!(targeted.per_target.len(), 4);
    let a = &targeted.per_target[0];
    assert_eq!((a.source_index, a.line_number, a.start, a.end), (0, 1, 65_534, 65_542));
    assert_eq!(a.name.as_deref(), Some("targetA"));
    assert_eq!(a.depth_sum, 12);
    assert_eq!(a.mean_depth, Availability::Available(String::from("1.500000")));
    assert_eq!((a.covered_bases, a.uncovered_bases), (8, 0));

    let b = &targeted.per_target[1];
    assert_eq!(b.name.as_deref(), Some("targetB"));
    assert_eq!(b.depth_sum, 4);
    assert_eq!(b.mean_depth, Availability::Available(String::from("1.000000")));
    assert_eq!((b.covered_bases, b.uncovered_bases), (4, 0));

    let c = &targeted.per_target[2];
    assert_eq!(c.name.as_deref(), Some("targetC"));
    assert_eq!(c.depth_sum, 2);
    assert_eq!(c.mean_depth, Availability::Available(String::from("0.166667")));
    assert_eq!((c.covered_bases, c.uncovered_bases), (2, 10));
    assert_eq!(c.threshold_bases.get("1"), Some(&2));
    assert_eq!(
        c.threshold_percentages.get("1"),
        Some(&Availability::Available(String::from("16.666667")))
    );
    assert_eq!(c.zero_coverage_runs.len(), 1);
    assert_eq!((c.zero_coverage_runs[0].start, c.zero_coverage_runs[0].end), (65_550, 65_560));
    assert_eq!(c.longest_zero_coverage_run_bases, 10);

    let empty = &targeted.per_target[3];
    assert_eq!(empty.name.as_deref(), Some("targetEmpty"));
    assert_eq!((empty.length, empty.depth_sum), (0, 0));
    assert_eq!(empty.mean_depth, Availability::unavailable("zero_length_target"));
    assert_eq!(
        empty.threshold_percentages.get("1"),
        Some(&Availability::unavailable("zero_length_target"))
    );
    assert!(empty.zero_coverage_runs.is_empty());
}

#[test]
fn exact_target_partition_and_per_source_metrics_match_oracle() {
    let report = analyze_bam_with_targets(
        alignment_fixture(),
        target_fixture(),
        5,
        CoverageOptions::new(4_u64 << 30, vec![1, 2]).expect("coverage options"),
    )
    .expect("targeted coverage");
    let total_aligned_bases = report.total_accepted_aligned_bases();
    let targeted = report.targeted().expect("targeted report").summary();

    assert_eq!(total_aligned_bases, 28);
    assert_aggregate_oracle(targeted, total_aligned_bases);
    assert_per_target_oracle(targeted);
}

#[test]
fn targeted_reduction_is_independent_of_chunk_size() {
    let mut baseline = None;
    for chunk_size in [1, 7, 1_024, 65_536] {
        let options = CoverageOptions::new(4_u64 << 30, vec![1, 2])
            .expect("coverage options")
            .with_chunk_size_override(chunk_size);
        let report = analyze_bam_with_targets(alignment_fixture(), target_fixture(), 5, options)
            .expect("targeted coverage");
        let targeted = report.targeted().expect("targeted report").summary().clone();
        if let Some(expected) = &baseline {
            assert_eq!(&targeted, expected, "chunk size {chunk_size}");
        } else {
            baseline = Some(targeted);
        }
    }
}
