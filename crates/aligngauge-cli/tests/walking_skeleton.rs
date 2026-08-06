use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use rust_htslib::bam::header::HeaderRecord;
use rust_htslib::bam::record::{Cigar, CigarString};
use rust_htslib::bam::{Format, Header, Record, Writer};

static NEXT_WORKSPACE_ID: AtomicU64 = AtomicU64::new(0);

struct TestWorkspace {
    root: PathBuf,
}

impl TestWorkspace {
    fn create() -> Self {
        let id = NEXT_WORKSPACE_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "aligngauge-walking-skeleton-{}-{id}",
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
fn counts_mapped_and_unmapped_records() {
    let workspace = TestWorkspace::create();
    let bam = workspace.path("mixed.bam");
    write_bam(&bam, 1, 1);

    let output = run_qc(&bam);
    assert_success(&output);
    assert_eq!(
        String::from_utf8(output.stdout).expect("UTF-8 stdout"),
        "total\t2\nmapped\t1\nunmapped\t1\n"
    );
}

#[test]
fn accepts_an_empty_valid_bam() {
    let workspace = TestWorkspace::create();
    let bam = workspace.path("empty.bam");
    write_bam(&bam, 0, 0);

    let output = run_qc(&bam);
    assert_success(&output);
    assert_eq!(
        String::from_utf8(output.stdout).expect("UTF-8 stdout"),
        "total\t0\nmapped\t0\nunmapped\t0\n"
    );
}

#[test]
fn missing_input_fails_without_counts() {
    let workspace = TestWorkspace::create();
    let output = run_qc(&workspace.path("missing.bam"));

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("[input_not_found]"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn truncated_input_fails_without_plausible_counts() {
    let workspace = TestWorkspace::create();
    let valid = workspace.path("valid.bam");
    let truncated = workspace.path("truncated.bam");
    write_bam(&valid, 4, 1);

    let bytes = fs::read(&valid).expect("read valid BAM");
    let retained = (bytes.len() / 2).max(1);
    fs::write(&truncated, &bytes[..retained]).expect("write truncated BAM");

    let output = run_qc(&truncated);
    assert!(!output.status.success());
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains("total\t"),
        "stdout unexpectedly contained counts: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("[input_corrupt]"), "stderr: {stderr}");
}

fn run_qc(path: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aligngauge"))
        .arg("qc")
        .arg("--input")
        .arg(path)
        .output()
        .expect("run aligngauge")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_bam(path: &Path, mapped_records: usize, unmapped_records: usize) {
    let mut header = Header::new();
    let mut hd = HeaderRecord::new(b"HD");
    hd.push_tag(b"VN", "1.6").push_tag(b"SO", "coordinate");
    header.push_record(&hd);
    let mut sq = HeaderRecord::new(b"SQ");
    sq.push_tag(b"SN", "chr1").push_tag(b"LN", 1_000);
    header.push_record(&sq);

    let mut writer = Writer::from_path(path, &header, Format::Bam).expect("create BAM writer");
    let cigar = CigarString(vec![Cigar::Match(4)]);
    let qualities = [30_u8; 4];

    for index in 0..mapped_records {
        let mut record = Record::new();
        let qname = format!("mapped-{index}");
        record.set(qname.as_bytes(), Some(&cigar), b"ACGT", &qualities);
        record.set_flags(0);
        record.set_tid(0);
        record.set_pos(i64::try_from(index).expect("test index fits i64") * 10);
        record.set_mapq(60);
        writer.write(&record).expect("write mapped record");
    }

    for index in 0..unmapped_records {
        let mut record = Record::new();
        let qname = format!("unmapped-{index}");
        record.set(qname.as_bytes(), None, b"TGCA", &qualities);
        record.set_unmapped();
        record.set_tid(-1);
        record.set_pos(-1);
        writer.write(&record).expect("write unmapped record");
    }
}
