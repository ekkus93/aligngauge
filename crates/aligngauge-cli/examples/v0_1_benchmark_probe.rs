use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aligngauge_cli::analyze_release;
use aligngauge_core::{ConfigOverrides, MapEnvironment, resolve_config};
use aligngauge_coverage::{CoverageOptions, analyze_bam as analyze_coverage_bam};
use aligngauge_metrics::analyze_bam as analyze_counters_bam;
use rust_htslib::bam::{Read, Reader, Record};

const COVERAGE_THRESHOLDS: [u32; 4] = [1, 10, 20, 30];

#[derive(Clone, Copy)]
enum Mode {
    Reader,
    Counters,
    Coverage,
    Combined,
}

impl Mode {
    fn parse(value: &str) -> Result<Self, Box<dyn Error>> {
        match value {
            "reader" => Ok(Self::Reader),
            "counters" => Ok(Self::Counters),
            "coverage" => Ok(Self::Coverage),
            "combined" => Ok(Self::Combined),
            _ => Err(invalid_input(format!(
                "unsupported benchmark mode '{value}'; use reader, counters, coverage, or combined"
            ))),
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("benchmark probe error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, Box<dyn Error>> {
    let mut arguments = env::args_os();
    let program = arguments
        .next()
        .unwrap_or_else(|| OsString::from("v0_1_benchmark_probe"));
    let mode = arguments
        .next()
        .ok_or_else(|| usage_error(&program))?
        .into_string()
        .map_err(|_| invalid_input("benchmark mode must be valid UTF-8"))?;
    let input = PathBuf::from(arguments.next().ok_or_else(|| usage_error(&program))?);
    let memory_limit = parse_memory_limit(
        arguments
            .next()
            .ok_or_else(|| usage_error(&program))?
            .as_os_str(),
    )?;
    if arguments.next().is_some() {
        return Err(usage_error(&program));
    }

    match Mode::parse(&mode)? {
        Mode::Reader => benchmark_reader(&input),
        Mode::Counters => benchmark_counters(&input),
        Mode::Coverage => benchmark_coverage(&input, memory_limit),
        Mode::Combined => benchmark_combined(&input, memory_limit),
    }
}

fn parse_memory_limit(value: &std::ffi::OsStr) -> Result<u64, Box<dyn Error>> {
    let value = value
        .to_str()
        .ok_or_else(|| invalid_input("memory-limit bytes must be valid UTF-8"))?;
    let parsed = value.parse::<u64>()?;
    if parsed == 0 {
        return Err(invalid_input(
            "memory-limit bytes must be greater than zero",
        ));
    }
    Ok(parsed)
}

fn benchmark_reader(input: &Path) -> Result<String, Box<dyn Error>> {
    let mut reader = Reader::from_path(input)?;
    let mut record = Record::new();
    let mut records = 0_u64;
    while let Some(result) = reader.read(&mut record) {
        result?;
        records = records
            .checked_add(1)
            .ok_or_else(|| invalid_input("reader record count overflowed"))?;
    }
    Ok(format!("mode=reader records={records}"))
}

fn benchmark_counters(input: &Path) -> Result<String, Box<dyn Error>> {
    let report = analyze_counters_bam(input)?;
    Ok(format!(
        "mode=counters records={}",
        report.alignment_counters().total
    ))
}

fn benchmark_coverage(input: &Path, memory_limit: u64) -> Result<String, Box<dyn Error>> {
    let options = CoverageOptions::new(memory_limit, COVERAGE_THRESHOLDS.to_vec())?;
    let report = analyze_coverage_bam(input, options)?;
    Ok(format!(
        "mode=coverage accepted_bases={}",
        report.total_accepted_aligned_bases()
    ))
}

fn benchmark_combined(input: &Path, memory_limit: u64) -> Result<String, Box<dyn Error>> {
    let config = resolve_config(
        None,
        &MapEnvironment::new(),
        ConfigOverrides {
            input: Some(input.to_path_buf()),
            outdir: Some(PathBuf::from("benchmark-output-not-published")),
            memory_limit_bytes: Some(memory_limit),
            coverage_thresholds: Some(COVERAGE_THRESHOLDS.to_vec()),
            ..ConfigOverrides::default()
        },
    )?;
    let report = analyze_release(&config)?;
    Ok(format!(
        "mode=combined records={} accepted_bases={} input_traversals={}",
        report.counters().alignment_counters().total,
        report.coverage().total_accepted_aligned_bases(),
        report.input_traversals()
    ))
}

fn usage_error(program: &std::ffi::OsStr) -> Box<dyn Error> {
    invalid_input(format!(
        "usage: {} <reader|counters|coverage|combined> <BAM> <memory-limit-bytes>",
        program.to_string_lossy()
    ))
}

fn invalid_input(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(io::Error::new(io::ErrorKind::InvalidInput, message.into()))
}
