//! Local checksum helpers.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::{Result, TestkitError};

/// Return the lowercase SHA-256 digest of a local file.
///
/// This function performs no network access.
///
/// # Errors
/// Returns an error when the local file cannot be opened or read.
pub fn sha256_file(path: &Path) -> Result<String> {
    let file =
        File::open(path).map_err(|source| TestkitError::io("open for hashing", path, source))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];

    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|source| TestkitError::io("read for hashing", path, source))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Verify a local file against a lowercase SHA-256 digest.
///
/// # Errors
/// Returns an error for noncanonical digest text, local I/O failure, or a
/// checksum mismatch.
pub fn verify_sha256(path: &Path, expected: &str) -> Result<()> {
    validate_sha256(expected)?;
    let actual = sha256_file(path)?;
    if actual == expected {
        Ok(())
    } else {
        Err(TestkitError::checksum(path, expected, actual))
    }
}

/// Validate the canonical lowercase SHA-256 text form.
///
/// # Errors
/// Returns an error unless the value is exactly 64 lowercase hexadecimal
/// characters.
pub fn validate_sha256(value: &str) -> Result<()> {
    let valid = value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if valid {
        Ok(())
    } else {
        Err(TestkitError::manifest(
            0,
            format!("invalid lowercase SHA-256 digest: {value}"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn hashes_local_files_deterministically() {
        let root =
            std::env::temp_dir().join(format!("aligngauge-hash-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create test directory");
        let path = root.join("payload");
        fs::write(&path, b"abc").expect("write payload");

        assert_eq!(
            sha256_file(&path).expect("hash payload"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        verify_sha256(
            &path,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        )
        .expect("verify payload");

        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn rejects_noncanonical_digest_text() {
        let error = validate_sha256("ABC").expect_err("uppercase short digest must fail");
        assert!(error.to_string().contains("invalid lowercase SHA-256"));
    }
}
