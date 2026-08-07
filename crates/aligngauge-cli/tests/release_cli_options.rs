use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_aligngauge")
}

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
        "aligngauge-cli-options-{label}-{}-{nanos}-{id}",
        std::process::id()
    ))
}

fn utf8(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("CLI output is UTF-8")
}

#[test]
fn help_documents_complete_v0_1_and_deferred_surface() {
    let output = Command::new(binary())
        .arg("--help")
        .output()
        .expect("run help");
    assert!(output.status.success());
    let stdout = utf8(&output.stdout);
    for option in [
        "--input",
        "--outdir",
        "--threads",
        "--io-threads",
        "--memory-limit",
        "--coverage-thresholds",
        "--config",
        "--log-format",
        "--quiet",
        "--verbose",
        "--preserve-failed-staging",
        "--reference",
        "--targets",
        "--backend",
        "--cuda-device",
    ] {
        assert!(stdout.contains(option), "missing help option {option}");
    }
    assert!(stdout.contains("v0.2"));
    assert!(stdout.contains("v0.3"));
}

#[test]
fn release_options_are_applied_and_nonparallel_threads_warn_explicitly() {
    let input = fixture("basic.bam");
    let outdir = temp_path("all-options");
    let output = Command::new(binary())
        .args(["qc", "--input"])
        .arg(&input)
        .args(["--outdir"])
        .arg(&outdir)
        .args([
            "--threads",
            "2",
            "--io-threads",
            "2",
            "--memory-limit",
            "1GiB",
            "--coverage-thresholds",
            "1,5,10",
            "--log-format",
            "human",
            "--verbose",
            "--preserve-failed-staging",
        ])
        .output()
        .expect("run release option matrix");
    assert!(output.status.success(), "{}", utf8(&output.stderr));
    let stderr = utf8(&output.stderr);
    assert!(stderr.contains("verbose: resolved_config="));
    assert!(stderr.contains("warning[collector_threads_serial_v0_1]"));
    let provenance = fs::read_to_string(outdir.join("provenance.json")).expect("read provenance");
    let summary = fs::read_to_string(outdir.join("summary.json")).expect("read summary");
    assert!(provenance.contains("\"threads\": 2"));
    assert!(provenance.contains("\"io_threads\": 2"));
    assert!(provenance.contains("\"coverage_thresholds\""));
    assert!(provenance.contains("\"preserve_failed_staging\": true"));
    assert!(summary.contains("collector_threads_serial_v0_1"));
    fs::remove_dir_all(outdir).expect("cleanup option output");
}

#[test]
fn quiet_suppresses_completion_output_but_not_correctness_warnings() {
    let input = fixture("basic.bam");
    let outdir = temp_path("quiet-warning");
    let output = Command::new(binary())
        .args(["qc", "--input"])
        .arg(&input)
        .args(["--outdir"])
        .arg(&outdir)
        .args(["--memory-limit", "1GiB", "--threads", "2", "--quiet"])
        .output()
        .expect("run quiet release");
    assert!(output.status.success(), "{}", utf8(&output.stderr));
    assert!(output.stdout.is_empty());
    assert!(utf8(&output.stderr).contains("collector_threads_serial_v0_1"));
    fs::remove_dir_all(outdir).expect("cleanup quiet output");
}

#[test]
fn config_file_drives_a_complete_release() {
    let input = fixture("empty.bam");
    let outdir = temp_path("config-output");
    let config_path = temp_path("config-file");
    let config = format!(
        "schema_version = 1\ninput = \"{}\"\noutdir = \"{}\"\nthreads = 1\nio_threads = 0\nmemory_limit = \"1GiB\"\ncoverage_thresholds = \"1,10\"\nlog_format = \"human\"\nquiet = true\nverbose = false\npreserve_failed_staging = false\n",
        input.display(),
        outdir.display()
    );
    fs::write(&config_path, config).expect("write config file");
    let output = Command::new(binary())
        .args(["qc", "--config"])
        .arg(&config_path)
        .output()
        .expect("run configured release");
    assert!(output.status.success(), "{}", utf8(&output.stderr));
    assert!(output.stdout.is_empty());
    assert!(outdir.join("_SUCCESS").is_file());
    fs::remove_dir_all(outdir).expect("cleanup configured output");
    fs::remove_file(config_path).expect("cleanup config file");
}

#[test]
fn json_log_format_applies_to_configuration_failures() {
    let outdir = temp_path("json-error");
    let output = Command::new(binary())
        .args(["qc", "--outdir"])
        .arg(&outdir)
        .args(["--log-format", "json"])
        .output()
        .expect("run JSON diagnostic failure");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = utf8(&output.stderr);
    assert!(stderr.trim_start().starts_with('{'));
    assert!(stderr.contains("\"category\": \"configuration\""));
    assert!(!outdir.exists());
}

#[test]
fn future_release_options_are_rejected_explicitly() {
    let input = fixture("basic.bam");
    for (option, marker) in [
        ("--reference", "v0.2"),
        ("--targets", "v0.3"),
        ("--profile", "v0.3"),
        ("--backend", "not a released v0.1 feature"),
        ("--cuda-device", "not a released v0.1 feature"),
    ] {
        let output = Command::new(binary())
            .args(["qc", "--input"])
            .arg(&input)
            .arg(option)
            .output()
            .expect("run unsupported option");
        assert!(!output.status.success(), "{option} unexpectedly succeeded");
        assert!(output.stdout.is_empty());
        assert!(utf8(&output.stderr).contains(marker));
    }
}

#[test]
fn compatibility_format_cannot_silently_switch_into_release_mode() {
    let input = fixture("basic.bam");
    let outdir = temp_path("mixed-mode");
    let output = Command::new(binary())
        .args(["qc", "--input"])
        .arg(&input)
        .args(["--format", "json", "--outdir"])
        .arg(&outdir)
        .output()
        .expect("run mixed compatibility/release mode");
    assert!(!output.status.success());
    assert!(utf8(&output.stderr).contains("cannot be combined"));
    assert!(!outdir.exists());
}
