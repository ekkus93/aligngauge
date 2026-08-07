use std::path::{Path, PathBuf};

use aligngauge_core::{Availability, BuildInfo, ToJson};
use aligngauge_metrics::{
    RecordClass, SAMTOOLS_FLAGSTAT_PROFILE, SAMTOOLS_IDXSTATS_PROFILE, analyze_bam,
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("testdata/fixtures")
        .join(name)
}

#[test]
fn dual_flag_uses_secondary_priority_and_full_counts_match_fixture() {
    let report = analyze_bam(fixture("flags_and_pairs.bam")).expect("analyze flags fixture");
    let counters = report.alignment_counters();

    assert_eq!(
        RecordClass::from_flags(0x100 | 0x800),
        RecordClass::Secondary
    );
    assert_eq!(counters.total, 9);
    assert_eq!(counters.qc_pass, 8);
    assert_eq!(counters.qc_fail, 1);
    assert_eq!(counters.primary, 6);
    assert_eq!(counters.secondary, 2);
    assert_eq!(counters.supplementary, 1);
    assert_eq!(counters.mapped, 9);
    assert_eq!(counters.unmapped, 0);
    assert_eq!(counters.paired, 4);
    assert_eq!(counters.proper_pair, 2);
    assert_eq!(counters.read1, 3);
    assert_eq!(counters.read2, 1);
    assert_eq!(counters.mate_mapped, 3);
    assert_eq!(counters.mate_unmapped, 1);
    assert_eq!(counters.duplicate, 1);
    assert_eq!(counters.singleton, 1);

    let pass = &report.partitions().qc_pass;
    assert_eq!(pass.both_mapped, 3);
    assert_eq!(pass.mate_different_reference, 1);
    assert_eq!(pass.mate_different_reference_mapq5, 1);
}

#[test]
fn per_reference_and_no_coordinate_counts_preserve_header_order() {
    let basic = analyze_bam(fixture("basic.bam")).expect("analyze basic");
    let references = basic.per_reference_counters();
    assert_eq!(references.len(), 2);
    assert_eq!(references[0].name, "chr1");
    assert_eq!(references[0].mapped, 1);
    assert_eq!(references[0].unmapped, Availability::Available(0));
    assert_eq!(references[1].name, "chr2");
    assert_eq!(references[1].mapped, 1);
    assert_eq!(basic.no_coordinate_unmapped(), 1);
    assert_eq!(
        basic.render_samtools_idxstats().expect("render idxstats"),
        "chr1\t1000000\t1\t0\nchr2\t1000000\t1\t0\n*\t0\t0\t1\n"
    );
}

#[test]
fn canonical_summary_is_deterministic_and_coverage_is_explicitly_unavailable() {
    let first = analyze_bam(fixture("flags_and_pairs.bam")).expect("first analysis");
    let second = analyze_bam(fixture("flags_and_pairs.bam")).expect("second analysis");
    assert_eq!(first, second);

    let summary = first.to_summary(BuildInfo {
        version: String::from("0.0.0"),
        git_commit: Availability::unavailable("test build"),
    });
    let json = summary.to_json().to_pretty_string();
    assert!(json.contains("\"alignment_counters\""));
    assert!(json.contains("\"coverage\""));
    assert!(json.contains("coverage is deferred until Milestone 5"));
    assert!(!json.contains("\"coverage\": 0"));
}

#[test]
fn compatibility_renderers_are_stable() {
    let report = analyze_bam(fixture("flags_and_pairs.bam")).expect("analyze fixture");
    let flagstat = report.render_samtools_flagstat();
    assert!(flagstat.starts_with("8 + 1 in total"));
    assert!(flagstat.contains("2 + 0 secondary\n"));
    assert!(flagstat.contains("1 + 0 supplementary\n"));
    assert!(flagstat.contains("1 + 0 singletons"));
    assert!(flagstat.contains("1 + 0 with mate mapped to a different chr (mapQ>=5)"));
    assert_eq!(SAMTOOLS_FLAGSTAT_PROFILE, "samtools-flagstat-1.24");
    assert_eq!(SAMTOOLS_IDXSTATS_PROFILE, "samtools-idxstats-1.24");
}
