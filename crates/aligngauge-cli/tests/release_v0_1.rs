use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use aligngauge_cli::{ReleaseCheckpoint, ReleaseHook, analyze_release, analyze_release_with_hook};
use aligngauge_core::{
    AlignGaugeError, AtomicPublisher, ConfigOverrides, ErrorCategory, MapEnvironment,
    PublicationHook, PublicationStep, resolve_config,
};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture(name: &str) -> PathBuf {
    repository_root().join("testdata/fixtures").join(name)
}

fn temp_path(label: &str) -> PathBuf {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "aligngauge-m6-{label}-{}-{nanos}-{id}",
        std::process::id()
    ))
}

fn config(input: &Path, outdir: &Path) -> aligngauge_core::ResolvedConfig {
    resolve_config(
        None,
        &MapEnvironment::new(),
        ConfigOverrides {
            input: Some(input.to_path_buf()),
            outdir: Some(outdir.to_path_buf()),
            memory_limit_bytes: Some(1024 * 1024 * 1024),
            coverage_thresholds: Some(vec![1, 10, 20, 30]),
            ..ConfigOverrides::default()
        },
    )
    .expect("resolve release config")
}

fn run_release(input: &Path, outdir: &Path, extra: &[&str]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_aligngauge"));
    command
        .arg("qc")
        .arg("--input")
        .arg(input)
        .arg("--outdir")
        .arg(outdir)
        .arg("--memory-limit")
        .arg("1GiB")
        .args(extra);
    command.output().expect("run release CLI")
}

#[test]
fn valid_and_empty_release_runs_publish_complete_outputs() {
    for name in ["basic.bam", "empty.bam"] {
        let outdir = temp_path("valid");
        let output = run_release(&fixture(name), &outdir, &["--quiet"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(outdir.join("_SUCCESS").is_file());
        let summary = fs::read_to_string(outdir.join("summary.json")).expect("summary");
        let provenance = fs::read_to_string(outdir.join("provenance.json")).expect("provenance");
        assert!(summary.contains("\"threshold_percentages\""));
        assert!(summary.contains("\"per_reference\""));
        assert!(provenance.contains("\"bam_traversals\": 1"));
        fs::remove_dir_all(outdir).expect("cleanup output");
    }
}

#[test]
fn corrupt_and_unsorted_inputs_never_publish_a_destination() {
    for (name, category) in [
        ("truncated_bgzf.bam", "input_corrupt"),
        ("coordinate_regression.bam", "input_unsorted"),
    ] {
        let outdir = temp_path("invalid");
        let output = run_release(&fixture(name), &outdir, &["--quiet"]);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains(category));
        assert!(!outdir.exists());
    }
}

#[test]
fn existing_destination_is_rejected_before_analysis_and_preserved() {
    let outdir = temp_path("exists");
    fs::create_dir_all(&outdir).expect("create existing destination");
    fs::write(outdir.join("sentinel"), b"keep").expect("write sentinel");
    let output = run_release(&fixture("basic.bam"), &outdir, &["--quiet"]);
    assert_eq!(output.status.code(), Some(14));
    assert_eq!(
        fs::read(outdir.join("sentinel")).expect("sentinel"),
        b"keep"
    );
    assert!(!outdir.join("_SUCCESS").exists());
    fs::remove_dir_all(outdir).expect("cleanup existing destination");
}

#[cfg(unix)]
#[test]
fn unwritable_parent_fails_without_completed_output() {
    use std::os::unix::fs::PermissionsExt as _;

    let parent = temp_path("permission-parent");
    fs::create_dir_all(&parent).expect("create parent");
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o500)).expect("chmod parent");
    let probe = parent.join("probe");
    if fs::write(&probe, b"probe").is_ok() {
        let _ = fs::remove_file(&probe);
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700))
            .expect("restore permissions");
        fs::remove_dir_all(parent).expect("cleanup parent");
        return;
    }
    let outdir = parent.join("results");
    let output = run_release(&fixture("basic.bam"), &outdir, &["--quiet"]);
    assert_eq!(output.status.code(), Some(15));
    assert!(!outdir.exists());
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).expect("restore permissions");
    fs::remove_dir_all(parent).expect("cleanup parent");
}

struct FailReleaseAt(ReleaseCheckpoint);

impl ReleaseHook for FailReleaseAt {
    fn checkpoint(&mut self, checkpoint: ReleaseCheckpoint) -> Result<(), AlignGaugeError> {
        if checkpoint == self.0 {
            return Err(AlignGaugeError::new(
                ErrorCategory::InternalInvariant,
                "injected release pipeline failure",
            ));
        }
        Ok(())
    }
}

#[test]
fn injected_collector_failure_aborts_before_output_exists() {
    let outdir = temp_path("collector-failure");
    let config = config(&fixture("basic.bam"), &outdir);
    let error = analyze_release_with_hook(
        &config,
        &mut FailReleaseAt(ReleaseCheckpoint::BeforeCoverageCollector),
    )
    .expect_err("injected collector failure must abort");
    assert_eq!(error.category(), ErrorCategory::InternalInvariant);
    assert!(!outdir.exists());
}

#[test]
fn injected_serialization_failure_exposes_no_output_bundle_or_destination() {
    let outdir = temp_path("serialization-failure");
    let report = analyze_release(&config(&fixture("basic.bam"), &outdir)).expect("analyze release");
    let error = report
        .output_bundle_with_hook(&mut FailReleaseAt(ReleaseCheckpoint::BeforeSerialization))
        .expect_err("serialization checkpoint must abort");
    assert_eq!(error.category(), ErrorCategory::InternalInvariant);
    assert!(!outdir.exists());
}

struct FailPublicationBeforeRename;

impl PublicationHook for FailPublicationBeforeRename {
    fn checkpoint(
        &mut self,
        step: PublicationStep,
        _staging: &Path,
        _destination: &Path,
    ) -> Result<(), AlignGaugeError> {
        if step == PublicationStep::BeforeRename {
            return Err(AlignGaugeError::new(
                ErrorCategory::OutputIo,
                "injected publication failure",
            ));
        }
        Ok(())
    }
}

#[test]
fn injected_publication_failure_never_exposes_completed_destination() {
    let outdir = temp_path("publication-failure");
    let report = analyze_release(&config(&fixture("basic.bam"), &outdir)).expect("analyze release");
    let bundle = report.output_bundle();
    let publisher = AtomicPublisher::new(&outdir, false);
    let error = publisher
        .publish_with_hook(&bundle, &mut FailPublicationBeforeRename)
        .expect_err("publication must fail");
    assert_eq!(error.category(), ErrorCategory::OutputIo);
    assert!(!outdir.exists());
}

#[test]
fn canonical_results_are_deterministic_after_timing_is_excluded() {
    let outdir = temp_path("determinism");
    let config = config(&fixture("chunk_boundary.bam"), &outdir);
    let first = analyze_release(&config).expect("first release analysis");
    let second = analyze_release(&config).expect("second release analysis");
    assert_eq!(first.summary(), second.summary());
    assert_eq!(first.input_traversals(), 1);
    assert_eq!(second.input_traversals(), 1);
    let mut first_provenance = first.provenance().clone();
    let mut second_provenance = second.provenance().clone();
    first_provenance.stage_timings_ns.clear();
    second_provenance.stage_timings_ns.clear();
    assert_eq!(first_provenance, second_provenance);
}

#[test]
fn optional_samtools_files_publish_from_complete_source_metrics() {
    let outdir = temp_path("compatibility-files");
    let report = analyze_release(&config(&fixture("basic.bam"), &outdir)).expect("analyze release");
    let bundle = report
        .output_bundle_with_samtools_compatibility()
        .expect("build compatibility bundle");
    AtomicPublisher::new(&outdir, false)
        .publish(&bundle)
        .expect("publish compatibility bundle");
    assert!(outdir.join("samtools.flagstat.txt").is_file());
    assert!(outdir.join("samtools.idxstats.txt").is_file());
    assert!(outdir.join("_SUCCESS").is_file());
    fs::remove_dir_all(outdir).expect("cleanup output");
}
