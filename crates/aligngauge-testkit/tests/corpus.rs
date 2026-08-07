use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use aligngauge_testkit::{ArtifactKind, ExpectedValidity, TestDataManifest, generate_corpus};
use rust_htslib::bam::{self, Read};

#[test]
fn committed_manifest_verifies_without_network_access() {
    let root = repository_root();
    let manifest =
        TestDataManifest::load(&root.join("testdata/manifest.v1.tsv")).expect("load manifest");
    manifest.verify_local(&root).expect("verify local corpus");

    let external = manifest.external_entries();
    assert_eq!(external.len(), 1);
    assert_eq!(external[0].kind, ArtifactKind::External);
    assert!(external[0].path.is_none());
    assert!(external[0].sha256.is_none());
}

#[test]
fn corpus_regeneration_is_byte_identical() {
    let first = temporary_root("first");
    let second = temporary_root("second");
    generate_corpus(&first).expect("generate first corpus");
    generate_corpus(&second).expect("generate second corpus");

    let first_files = collect_files(&first).expect("collect first files");
    let second_files = collect_files(&second).expect("collect second files");
    assert_eq!(first_files, second_files);

    fs::remove_dir_all(first).expect("remove first corpus");
    fs::remove_dir_all(second).expect("remove second corpus");
}

#[test]
fn every_valid_fixture_streams_and_long_cigar_is_restored() {
    let root = repository_root();
    let manifest =
        TestDataManifest::load(&root.join("testdata/manifest.v1.tsv")).expect("load manifest");

    for entry in &manifest.entries {
        if entry.kind != ArtifactKind::Committed
            || entry.expected_validity != ExpectedValidity::Valid
        {
            continue;
        }
        let Some(relative_path) = &entry.path else {
            panic!("valid committed fixture must have path");
        };
        let path = root.join(relative_path);
        let mut reader = bam::Reader::from_path(&path)
            .unwrap_or_else(|error| panic!("open {}: {error}", path.display()));
        for record in reader.records() {
            record.unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        }
    }

    let long_path = root.join("testdata/fixtures/long_cigar.bam");
    let mut long_reader = bam::Reader::from_path(&long_path).expect("open long-CIGAR fixture");
    let record = long_reader
        .records()
        .next()
        .expect("long-CIGAR record")
        .expect("read long-CIGAR record");
    assert_eq!(record.cigar().len(), 66_000);
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate is two levels below repository")
        .to_path_buf()
}

fn temporary_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "aligngauge-corpus-{label}-{}-{nonce}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create temporary root");
    root
}

fn collect_files(root: &Path) -> std::io::Result<BTreeMap<PathBuf, Vec<u8>>> {
    let mut output = BTreeMap::new();
    collect_directory(root, root, &mut output)?;
    Ok(output)
}

fn collect_directory(
    root: &Path,
    directory: &Path,
    output: &mut BTreeMap<PathBuf, Vec<u8>>,
) -> std::io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(directory)?.collect::<std::io::Result<_>>()?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_directory(root, &path, output)?;
        } else {
            let relative = path
                .strip_prefix(root)
                .expect("collected path is below root")
                .to_path_buf();
            output.insert(relative, fs::read(path)?);
        }
    }
    Ok(())
}
