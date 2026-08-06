use std::collections::BTreeMap;
use std::path::PathBuf;

use aligngauge_core::{
    AlignmentCounters, Availability, BuildInfo, ConfigOverrides, CoveragePolicy, CoverageSummary,
    InputIdentity, JsonValue, MapEnvironment, MetricDefinition, PerReferenceCounters, Provenance,
    Summary, SystemInfo, ToJson, Warning, resolve_config,
};

#[test]
fn summary_matches_golden_contract() {
    let summary = sample_summary();
    assert_eq!(
        summary.to_json_pretty(),
        include_str!("golden/summary.json")
    );
}

#[test]
fn provenance_matches_golden_contract() {
    let provenance = sample_provenance();
    assert_eq!(
        provenance.to_json_pretty(),
        include_str!("golden/provenance.json")
    );
}

#[test]
fn unavailable_metrics_cannot_look_like_zero() {
    let unavailable = Availability::<u64>::unavailable("collector_not_enabled");
    let json = unavailable.to_json_pretty();

    assert!(json.contains("\"status\": \"unavailable\""));
    assert!(json.contains("\"reason\": \"collector_not_enabled\""));
    assert!(!json.contains("\"value\""));
    assert!(!json.contains(": 0"));
}

fn sample_summary() -> Summary {
    let application = BuildInfo {
        version: String::from("0.1.0-test"),
        git_commit: Availability::Available(String::from("0123456789abcdef")),
    };
    let definitions = BTreeMap::from([(
        String::from("alignment.total"),
        MetricDefinition {
            description: String::from("Decoded alignment records"),
            unit: String::from("records"),
        },
    )]);
    let counters = AlignmentCounters {
        total: 2,
        qc_pass: 2,
        primary: 2,
        mapped: 1,
        unmapped: 1,
        ..AlignmentCounters::default()
    };
    let per_reference = vec![PerReferenceCounters {
        name: String::from("chr1"),
        length: 1_000,
        mapped: 1,
        unmapped: Availability::unavailable("not_defined_by_canonical_profile"),
    }];
    let coverage = CoverageSummary {
        policy: CoveragePolicy {
            name: String::from("aligngauge-v0.1"),
            minimum_mapq: 0,
            include_duplicates: false,
            include_qc_fail: false,
            include_secondary: false,
            include_supplementary: false,
            mate_overlap_correction: false,
        },
        total_accepted_aligned_bases: 100,
        depth_histogram: BTreeMap::from([(String::from("0"), 900), (String::from("1"), 100)]),
        threshold_bases: BTreeMap::from([(String::from("1"), 100)]),
        covered_reference_bases: 100,
        uncovered_reference_bases: 900,
    };

    Summary::new(
        application,
        definitions,
        Availability::Available(counters),
        Availability::Available(per_reference),
        Availability::Available(coverage),
        vec![Warning {
            code: String::from("test_warning"),
            message: String::from("Synthetic fixture"),
        }],
    )
}

fn sample_provenance() -> Provenance {
    let resolved_config = resolve_config(
        None,
        &MapEnvironment::new(),
        ConfigOverrides {
            input: Some(PathBuf::from("sample.bam")),
            outdir: Some(PathBuf::from("results")),
            threads: Some(2),
            io_threads: Some(1),
            memory_limit_bytes: Some(1_u64 << 30),
            coverage_thresholds: Some(vec![1, 10, 20, 30]),
            ..ConfigOverrides::default()
        },
    )
    .expect("resolve sample configuration");

    Provenance::new(
        BuildInfo {
            version: String::from("0.1.0-test"),
            git_commit: Availability::Available(String::from("0123456789abcdef")),
        },
        resolved_config,
        InputIdentity {
            path: String::from("sample.bam"),
            size_bytes: Availability::Available(4096),
            checksum: Availability::unavailable("checksum_not_requested"),
        },
        Availability::Available(String::from("sha256:header")),
        BTreeMap::from([
            (String::from("htslib"), String::from("1.22")),
            (String::from("rust-htslib"), String::from("1.0.1")),
        ]),
        BTreeMap::from([(String::from("input_passes"), JsonValue::Unsigned(1))]),
        BTreeMap::from([(String::from("memory_limit_bytes"), 1_u64 << 30)]),
        BTreeMap::from([(String::from("total"), 123_456)]),
        vec![String::from("coverage_thresholds_sorted")],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        SystemInfo {
            os: String::from("linux"),
            architecture: String::from("x86_64"),
            logical_cpus: Availability::Available(8),
        },
    )
}
