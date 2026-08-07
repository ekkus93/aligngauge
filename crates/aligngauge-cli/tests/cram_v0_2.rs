use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use aligngauge_cli::{analyze_release, analyze_release_with_reference};
use aligngauge_core::{ConfigOverrides, ErrorCategory, MapEnvironment, ToJson, resolve_config};
use aligngauge_hts::{AlignmentFormat, detect_alignment_format};
use aligngauge_testkit::bam::{CigarOp, RecordSpec, ReferenceSpec, serialize_bam};
use rust_htslib::bam::{self, Format, Read};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);
const REFERENCE_MD5: &str = "dee764dc7924160b161de01b8c77167c";

struct FixturePair {
    root: PathBuf,
    bam: PathBuf,
    cram: PathBuf,
    reference: PathBuf,
    wrong_reference: PathBuf,
}

impl Drop for FixturePair {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn temp_path(label: &str) -> PathBuf {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "aligngauge-m7-{label}-{}-{nanos}-{id}",
        std::process::id()
    ))
}

fn make_pair() -> FixturePair {
    let root = temp_path("pair");
    fs::create_dir_all(&root).expect("create fixture root");
    let reference = root.join("reference.fa");
    let wrong_reference = root.join("wrong.fa");
    let bam_path = root.join("equivalent.bam");
    let cram_path = root.join("equivalent.cram");
    let sequence = "ACGT".repeat(250);
    fs::write(&reference, format!(">chr1\n{sequence}\n")).expect("write reference");
    fs::write(&wrong_reference, format!(">chr1\n{}\n", "A".repeat(1000)))
        .expect("write wrong reference");
    rust_htslib::faidx::build(reference.clone()).expect("index reference");
    rust_htslib::faidx::build(wrong_reference.clone()).expect("index wrong reference");

    let header = format!("@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:1000\tM5:{REFERENCE_MD5}\n");
    let references = [ReferenceSpec {
        name: String::from("chr1"),
        length: 1000,
    }];
    let records = [
        RecordSpec::mapped(
            "mapped-a",
            0,
            100,
            vec![CigarOp::new(10, 'M')],
            "ACGTACGTAC",
        ),
        RecordSpec::mapped(
            "mapped-b",
            0,
            120,
            vec![
                CigarOp::new(4, 'M'),
                CigarOp::new(2, 'D'),
                CigarOp::new(4, 'M'),
            ],
            "ACGTACGT",
        ),
        RecordSpec::unmapped("unmapped", "NNNN"),
    ];
    fs::write(
        &bam_path,
        serialize_bam(&header, &references, &records).expect("serialize BAM"),
    )
    .expect("write BAM");

    let mut reader = bam::Reader::from_path(&bam_path).expect("open generated BAM");
    let output_header = bam::Header::from_template(reader.header());
    let mut writer =
        bam::Writer::from_path(&cram_path, &output_header, Format::Cram).expect("open CRAM writer");
    writer
        .set_reference(&reference)
        .expect("set CRAM writer reference");
    for record in reader.records() {
        writer
            .write(&record.expect("read BAM record"))
            .expect("write CRAM record");
    }
    drop(writer);

    FixturePair {
        root,
        bam: bam_path,
        cram: cram_path,
        reference,
        wrong_reference,
    }
}

fn config(input: &Path) -> aligngauge_core::ResolvedConfig {
    resolve_config(
        None,
        &MapEnvironment::new(),
        ConfigOverrides {
            input: Some(input.to_path_buf()),
            outdir: Some(temp_path("unused-output")),
            memory_limit_bytes: Some(1024 * 1024 * 1024),
            coverage_thresholds: Some(vec![1, 2, 10]),
            ..ConfigOverrides::default()
        },
    )
    .expect("resolve config")
}

#[test]
fn equivalent_bam_and_cram_have_identical_canonical_results() {
    let fixtures = make_pair();
    assert_eq!(
        detect_alignment_format(&fixtures.bam).expect("detect BAM"),
        AlignmentFormat::Bam
    );
    assert_eq!(
        detect_alignment_format(&fixtures.cram).expect("detect CRAM"),
        AlignmentFormat::Cram
    );

    let bam = analyze_release(&config(&fixtures.bam)).expect("analyze BAM");
    let cram = analyze_release_with_reference(&config(&fixtures.cram), Some(&fixtures.reference))
        .expect("analyze CRAM");

    assert_eq!(bam.counters(), cram.counters());
    assert_eq!(bam.coverage(), cram.coverage());
    assert_eq!(bam.summary(), cram.summary());
    assert_eq!(
        bam.provenance().header_identity,
        cram.provenance().header_identity
    );
    assert_eq!(
        bam.provenance().backend_versions,
        cram.provenance().backend_versions
    );
    assert_eq!(
        bam.provenance().resource_limits,
        cram.provenance().resource_limits
    );
    assert_eq!(
        bam.provenance().normalization_actions,
        cram.provenance().normalization_actions
    );
    assert_eq!(bam.provenance().warnings, cram.provenance().warnings);
    assert_eq!(bam.provenance().system, cram.provenance().system);

    let mut bam_plan = bam.provenance().analysis_plan.clone();
    let mut cram_plan = cram.provenance().analysis_plan.clone();
    for key in [
        "input_format",
        "bam_traversals",
        "cram_traversals",
        "local_reference",
    ] {
        bam_plan.remove(key);
        cram_plan.remove(key);
    }
    assert_eq!(bam_plan, cram_plan);

    let cram_provenance = cram.provenance().to_json_pretty();
    assert!(cram_provenance.contains("\"input_format\": \"cram\""));
    assert!(cram_provenance.contains("\"htslib_network_transport_enabled\": false"));
    assert!(cram_provenance.contains("\"sha256\""));
    assert!(cram_provenance.contains(REFERENCE_MD5));
}

#[test]
fn cram_requires_an_explicit_reference_and_rejects_missing_or_mismatched_sequence() {
    let fixtures = make_pair();
    let missing = analyze_release_with_reference(&config(&fixtures.cram), None)
        .expect_err("missing reference must fail");
    assert_eq!(missing.category(), ErrorCategory::ReferenceRequired);

    let missing_contig = fixtures.root.join("missing-contig.fa");
    fs::write(&missing_contig, format!(">chr2\n{}\n", "ACGT".repeat(250)))
        .expect("write missing-contig reference");
    let missing_sequence =
        analyze_release_with_reference(&config(&fixtures.cram), Some(&missing_contig))
            .expect_err("missing required contig must fail");
    assert_eq!(
        missing_sequence.category(),
        ErrorCategory::ReferenceRequired
    );
    let missing_report = missing_sequence.render_human(false);
    assert!(missing_report.contains("chr1"));
    assert!(missing_report.contains(REFERENCE_MD5));

    let mismatch =
        analyze_release_with_reference(&config(&fixtures.cram), Some(&fixtures.wrong_reference))
            .expect_err("wrong reference must fail");
    assert_eq!(mismatch.category(), ErrorCategory::ReferenceMismatch);
}

#[test]
fn truncated_cram_is_typed_corrupt() {
    let fixtures = make_pair();
    let bytes = fs::read(&fixtures.cram).expect("read CRAM");
    let truncated = fixtures.root.join("truncated.cram");
    fs::write(&truncated, &bytes[..bytes.len() / 2]).expect("write truncated CRAM");
    let error = analyze_release_with_reference(&config(&truncated), Some(&fixtures.reference))
        .expect_err("truncated CRAM must fail");
    assert_eq!(error.category(), ErrorCategory::InputCorrupt);
}

#[test]
fn malformed_cram_version_is_typed_corrupt() {
    let fixtures = make_pair();
    let mut bytes = fs::read(&fixtures.cram).expect("read CRAM");
    assert!(bytes.len() > 5, "generated CRAM must contain a version field");
    assert_eq!(&bytes[..4], b"CRAM");
    bytes[4] = 0xff;
    let corrupted = fixtures.root.join("corrupted-version.cram");
    fs::write(&corrupted, bytes).expect("write corrupted CRAM");
    let error = analyze_release_with_reference(&config(&corrupted), Some(&fixtures.reference))
        .expect_err("malformed CRAM version must fail");
    assert_eq!(error.category(), ErrorCategory::InputCorrupt);
}

#[test]
fn hostile_reference_environment_cannot_replace_explicit_local_policy() {
    let fixtures = make_pair();
    let outdir = fixtures.root.join("cli-output");
    let output = Command::new(env!("CARGO_BIN_EXE_aligngauge"))
        .arg("qc")
        .arg("--input")
        .arg(&fixtures.cram)
        .arg("--reference")
        .arg(&fixtures.reference)
        .arg("--outdir")
        .arg(&outdir)
        .arg("--memory-limit")
        .arg("1GiB")
        .arg("--quiet")
        .env("REF_PATH", "http://127.0.0.1:9/%s")
        .env("REF_CACHE", fixtures.root.join("hostile-cache"))
        .env("HTS_PATH", fixtures.root.join("hostile-plugins"))
        .output()
        .expect("run CRAM CLI");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(outdir.join("_SUCCESS").is_file());
}
