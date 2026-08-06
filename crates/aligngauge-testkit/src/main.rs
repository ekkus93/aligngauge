//! Command-line entry point for deterministic testkit operations.

use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aligngauge_testkit::{TestDataManifest, compare_files, generate_corpus};

fn main() -> ExitCode {
    match run(env::args_os().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("aligngauge-testkit: {error}");
            ExitCode::from(1)
        }
    }
}

fn run(arguments: Vec<std::ffi::OsString>) -> aligngauge_testkit::Result<()> {
    let Some(command) = arguments.first().and_then(|value| value.to_str()) else {
        return Err(aligngauge_testkit::TestkitError::generation(usage()));
    };

    match command {
        "generate-corpus" => {
            let root = parse_single_path_option(&arguments[1..], "--root")?;
            generate_corpus(&root)
        }
        "verify-manifest" => {
            let root = parse_single_path_option(&arguments[1..], "--root")?;
            let manifest_path = root.join("testdata/manifest.v1.tsv");
            TestDataManifest::load(&manifest_path)?.verify_local(&root)
        }
        "compare" => {
            let expected = parse_required_option(&arguments[1..], "--expected")?;
            let actual = parse_required_option(&arguments[1..], "--actual")?;
            let report = parse_required_option(&arguments[1..], "--report")?;
            let comparison = compare_files(
                Path::new(&expected),
                Path::new(&actual),
                Path::new(&report),
            )?;
            if comparison.is_match() {
                Ok(())
            } else {
                Err(aligngauge_testkit::TestkitError::differential(
                    "comparison contains discrepancies",
                ))
            }
        }
        _ => Err(aligngauge_testkit::TestkitError::generation(usage())),
    }
}

fn parse_single_path_option(
    arguments: &[std::ffi::OsString],
    name: &str,
) -> aligngauge_testkit::Result<PathBuf> {
    let value = parse_required_option(arguments, name)?;
    if arguments.len() != 2 {
        return Err(aligngauge_testkit::TestkitError::generation(usage()));
    }
    Ok(PathBuf::from(value))
}

fn parse_required_option(
    arguments: &[std::ffi::OsString],
    name: &str,
) -> aligngauge_testkit::Result<String> {
    let mut found = None;
    let mut index = 0;
    while index < arguments.len() {
        let key = arguments[index]
            .to_str()
            .ok_or_else(|| aligngauge_testkit::TestkitError::generation("argument is not UTF-8"))?;
        let value = arguments.get(index + 1).ok_or_else(|| {
            aligngauge_testkit::TestkitError::generation(format!("missing value for {key}"))
        })?;
        if key == name {
            if found.is_some() {
                return Err(aligngauge_testkit::TestkitError::generation(format!(
                    "duplicate option {name}"
                )));
            }
            found = Some(
                value
                    .to_str()
                    .ok_or_else(|| {
                        aligngauge_testkit::TestkitError::generation(format!(
                            "value for {name} is not UTF-8"
                        ))
                    })?
                    .to_owned(),
            );
        }
        index += 2;
    }
    found.ok_or_else(|| {
        aligngauge_testkit::TestkitError::generation(format!("required option {name} is missing"))
    })
}

fn usage() -> &'static str {
    "usage: aligngauge-testkit generate-corpus --root PATH | verify-manifest --root PATH | compare --expected PATH --actual PATH --report PATH"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missing_command() {
        let error = run(Vec::new()).expect_err("missing command must fail");
        assert!(error.to_string().contains("usage:"));
    }

    #[test]
    fn rejects_duplicate_option() {
        let arguments = vec![
            "--root".into(),
            ".".into(),
            "--root".into(),
            ".".into(),
        ];
        let error = parse_single_path_option(&arguments, "--root")
            .expect_err("duplicate option must fail");
        assert!(error.to_string().contains("duplicate option"));
    }
}
