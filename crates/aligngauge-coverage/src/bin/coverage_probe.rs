//! Internal Milestone 5 coverage probe used by deterministic CI and differential validation.

use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use aligngauge_core::{AlignGaugeError, ErrorCategory, JsonValue, ToJson};
use aligngauge_coverage::{COVERAGE_STRATEGY, CoverageOptions, analyze_bam};

fn main() -> ExitCode {
    match run() {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {}", error.render_human(false));
            ExitCode::from(error.exit_code())
        }
    }
}

fn run() -> Result<String, AlignGaugeError> {
    let mut arguments = env::args_os();
    let _program = arguments.next();
    let input = arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| usage_error("coverage probe requires an input BAM"))?;

    let mut memory_limit_bytes = None;
    let mut chunk_size = None;
    let mut thresholds = None;
    while let Some(argument) = arguments.next() {
        match argument.to_str() {
            Some("--memory-limit-bytes") => {
                memory_limit_bytes = Some(parse_u64(
                    next_value(&mut arguments, "--memory-limit-bytes")?,
                    "--memory-limit-bytes",
                )?);
            }
            Some("--chunk-size") => {
                chunk_size = Some(parse_usize(
                    next_value(&mut arguments, "--chunk-size")?,
                    "--chunk-size",
                )?);
            }
            Some("--thresholds") => {
                thresholds = Some(parse_thresholds(next_value(
                    &mut arguments,
                    "--thresholds",
                )?)?);
            }
            _ => {
                return Err(usage_error(format!(
                    "unsupported coverage probe argument '{}'",
                    argument.to_string_lossy()
                )));
            }
        }
    }

    let defaults = CoverageOptions::default();
    let mut options = CoverageOptions::new(
        memory_limit_bytes.unwrap_or(defaults.memory_limit_bytes),
        thresholds.unwrap_or(defaults.thresholds),
    )?;
    if let Some(chunk_size) = chunk_size {
        options = options.with_chunk_size_override(chunk_size);
    }

    let report = analyze_bam(&input, options)?;
    Ok(JsonValue::Object(std::collections::BTreeMap::from([
        (String::from("coverage"), report.to_json()),
        (String::from("memory_plan"), report.memory_plan().to_json()?),
        (
            String::from("strategy"),
            JsonValue::String(COVERAGE_STRATEGY.to_owned()),
        ),
    ]))
    .to_json_pretty())
}

fn next_value(
    arguments: &mut impl Iterator<Item = OsString>,
    option: &'static str,
) -> Result<OsString, AlignGaugeError> {
    arguments
        .next()
        .ok_or_else(|| usage_error(format!("{option} requires a value")))
}

fn parse_u64(value: OsString, option: &'static str) -> Result<u64, AlignGaugeError> {
    let text = value
        .into_string()
        .map_err(|_| usage_error(format!("{option} must be valid UTF-8")))?;
    let parsed = text.parse::<u64>().map_err(|source| {
        usage_error(format!("{option} must be an integer")).with_source(source)
    })?;
    if parsed == 0 {
        return Err(usage_error(format!("{option} must be greater than zero")));
    }
    Ok(parsed)
}

fn parse_usize(value: OsString, option: &'static str) -> Result<usize, AlignGaugeError> {
    let parsed = parse_u64(value, option)?;
    usize::try_from(parsed)
        .map_err(|source| usage_error(format!("{option} does not fit usize")).with_source(source))
}

fn parse_thresholds(value: OsString) -> Result<Vec<u32>, AlignGaugeError> {
    let text = value
        .into_string()
        .map_err(|_| usage_error("--thresholds must be valid UTF-8"))?;
    let mut thresholds = Vec::new();
    for raw in text.split(',') {
        let threshold = raw.trim().parse::<u32>().map_err(|source| {
            usage_error(format!("invalid coverage threshold '{raw}'")).with_source(source)
        })?;
        thresholds.push(threshold);
    }
    Ok(thresholds)
}

fn usage_error(message: impl Into<String>) -> AlignGaugeError {
    AlignGaugeError::new(ErrorCategory::Usage, message).with_detail(
        "usage",
        "coverage_probe INPUT.bam [--memory-limit-bytes N] [--chunk-size N] [--thresholds 1,10,20,30]",
    )
}
