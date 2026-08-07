#!/usr/bin/env python3
from pathlib import Path

Path("crates/aligngauge-metrics/tests/samtools_stats.rs").write_text(r'''use std::path::{Path, PathBuf};

use aligngauge_metrics::{
    InsertSizeRow, MULTIQC_VERSION, SAMTOOLS_STATS_PROFILE, analyze_samtools_stats_bam,
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("testdata/fixtures")
        .join(name)
}

#[test]
fn basic_fixture_matches_pinned_samtools_1_24_sn_exactly() {
    let report = analyze_samtools_stats_bam(fixture("basic.bam")).expect("stats analysis");

    assert_eq!(report.raw_total_sequences, 3);
    assert_eq!(report.filtered_sequences, 0);
    assert_eq!(report.sequences, 3);
    assert!(report.is_sorted);
    assert_eq!(report.first_fragments, 3);
    assert_eq!(report.last_fragments, 0);
    assert_eq!(report.reads_mapped, 2);
    assert_eq!(report.reads_mapped_and_paired, 0);
    assert_eq!(report.reads_unmapped, 1);
    assert_eq!(report.reads_properly_paired, 0);
    assert_eq!(report.reads_paired, 0);
    assert_eq!(report.reads_duplicated, 0);
    assert_eq!(report.reads_mq0, 0);
    assert_eq!(report.reads_qc_failed, 0);
    assert_eq!(report.non_primary_alignments, 0);
    assert_eq!(report.supplementary_alignments, 0);
    assert_eq!(report.total_length, 24);
    assert_eq!(report.total_first_fragment_length, 24);
    assert_eq!(report.total_last_fragment_length, 0);
    assert_eq!(report.bases_mapped, 20);
    assert_eq!(report.bases_mapped_cigar, 20);
    assert_eq!(report.bases_trimmed, 0);
    assert_eq!(report.bases_duplicated, 0);
    assert_eq!(report.mismatches, 0);
    assert_eq!(report.error_rate, "0.000000e+00");
    assert_eq!(report.average_length, "8");
    assert_eq!(report.average_first_fragment_length, "8");
    assert_eq!(report.average_last_fragment_length, "0");
    assert_eq!(report.maximum_length, 10);
    assert_eq!(report.maximum_first_fragment_length, 10);
    assert_eq!(report.maximum_last_fragment_length, 0);
    assert_eq!(report.average_quality, "30.0");
    assert_eq!(report.insert_size_average, "0.0");
    assert_eq!(report.insert_size_standard_deviation, "0.0");
    assert_eq!(report.inward_oriented_pairs, 0);
    assert_eq!(report.outward_oriented_pairs, 0);
    assert_eq!(report.pairs_with_other_orientation, 0);
    assert_eq!(report.pairs_on_different_chromosomes, 0);
    assert_eq!(report.percentage_properly_paired_reads, "0.0");
    assert!(report.insert_sizes.is_empty());

    let text = report.render_samtools_stats();
    assert!(text.contains("This file was produced by samtools stats (1.24+htslib-1.24)"));
    assert!(text.contains(SAMTOOLS_STATS_PROFILE));
    assert!(text.contains("SN\tfiltered sequences:\t0\n"));
    assert!(!text.contains("\nCHK\t"));
    assert!(!text.contains("\nFFQ\t"));
    assert!(!text.contains("\nCOV\t"));
    assert_eq!(MULTIQC_VERSION, "1.35");
}

#[test]
fn flags_fixture_matches_pinned_samtools_1_24_sn_and_is_exactly() {
    let report = analyze_samtools_stats_bam(fixture("flags_and_pairs.bam")).expect("stats analysis");

    assert_eq!(report.raw_total_sequences, 6);
    assert_eq!(report.filtered_sequences, 0);
    assert_eq!(report.sequences, 6);
    assert!(report.is_sorted);
    assert_eq!(report.first_fragments, 5);
    assert_eq!(report.last_fragments, 1);
    assert_eq!(report.reads_mapped, 6);
    assert_eq!(report.reads_mapped_and_paired, 3);
    assert_eq!(report.reads_unmapped, 0);
    assert_eq!(report.reads_properly_paired, 2);
    assert_eq!(report.reads_paired, 4);
    assert_eq!(report.reads_duplicated, 1);
    assert_eq!(report.reads_mq0, 0);
    assert_eq!(report.reads_qc_failed, 1);
    assert_eq!(report.non_primary_alignments, 2);
    assert_eq!(report.supplementary_alignments, 1);
    assert_eq!(report.total_length, 40);
    assert_eq!(report.total_first_fragment_length, 30);
    assert_eq!(report.total_last_fragment_length, 10);
    assert_eq!(report.bases_mapped, 40);
    assert_eq!(report.bases_mapped_cigar, 45);
    assert_eq!(report.bases_trimmed, 0);
    assert_eq!(report.bases_duplicated, 5);
    assert_eq!(report.mismatches, 0);
    assert_eq!(report.error_rate, "0.000000e+00");
    assert_eq!(report.average_length, "7");
    assert_eq!(report.average_first_fragment_length, "6");
    assert_eq!(report.average_last_fragment_length, "10");
    assert_eq!(report.maximum_length, 10);
    assert_eq!(report.maximum_first_fragment_length, 10);
    assert_eq!(report.maximum_last_fragment_length, 10);
    assert_eq!(report.average_quality, "30.0");
    assert_eq!(report.insert_size_average, "70.0");
    assert_eq!(report.insert_size_standard_deviation, "0.0");
    assert_eq!(report.inward_oriented_pairs, 0);
    assert_eq!(report.outward_oriented_pairs, 0);
    assert_eq!(report.pairs_with_other_orientation, 1);
    assert_eq!(report.pairs_on_different_chromosomes, 0);
    assert_eq!(report.percentage_properly_paired_reads, "33.3");
    assert_eq!(report.insert_sizes.len(), 71);
    for (expected_size, row) in report.insert_sizes.iter().take(70).enumerate() {
        assert_eq!(row.insert_size, u32::try_from(expected_size).expect("size fits u32"));
        assert_eq!(row.pairs_total, 0);
        assert_eq!(row.inward, 0);
        assert_eq!(row.outward, 0);
        assert_eq!(row.other, 0);
    }
    assert_eq!(
        report.insert_sizes[70],
        InsertSizeRow {
            insert_size: 70,
            pairs_total: 1,
            inward: 0,
            outward: 0,
            other: 1,
        }
    );

    let rendered = report.render_samtools_stats();
    assert!(rendered.contains("IS\t70\t1\t0\t0\t1\n"));
    assert!(!rendered.contains("\nMAPQ\t"));
    assert!(!rendered.contains("\nID\t"));
    assert!(!rendered.contains("\nGCD\t"));
}
''')