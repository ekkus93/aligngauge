use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::ExitCode;

use aligngauge_cli::analyze_bam;
use aligngauge_core::{AlignGaugeError, Availability, BuildInfo, ErrorCategory, ToJson};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum OutputFormat {
    Legacy,
    Human,
    Json,
    SamtoolsFlagstat,
    SamtoolsIdxstats,
}

enum CliAction {
    Help(String),
    Analyze {
        input: PathBuf,
        format: OutputFormat,
    },
}

fn main() -> ExitCode {
    match parse_args() {
        Ok(CliAction::Help(help)) => {
            println!("{help}");
            ExitCode::SUCCESS
        }
        Ok(CliAction::Analyze { input, format }) => match analyze_bam(&input) {
            Ok(report) => {
                print!(
                    "{}",
                    match format {
                        OutputFormat::Legacy => {
                            let counters = report.alignment_counters();
                            format!(
                                "total\t{}\nmapped\t{}\nunmapped\t{}\n",
                                counters.total, counters.mapped, counters.unmapped
                            )
                        }
                        OutputFormat::Human => report.render_human(),
                        OutputFormat::Json => report
                            .to_summary(BuildInfo {
                                version: env!("CARGO_PKG_VERSION").to_owned(),
                                git_commit: Availability::unavailable(
                                    "build revision was not embedded",
                                ),
                            })
                            .to_json_pretty(),
                        OutputFormat::SamtoolsFlagstat => report.render_samtools_flagstat(),
                        OutputFormat::SamtoolsIdxstats => report.render_samtools_idxstats(),
                    }
                );
                ExitCode::SUCCESS
            }
            Err(error) => exit_with_error(&error),
        },
        Err(error) => exit_with_error(&error),
    }
}

fn exit_with_error(error: &AlignGaugeError) -> ExitCode {
    eprintln!("error: {}", error.render_human(false));
    ExitCode::from(error.exit_code())
}

fn parse_args() -> Result<CliAction, AlignGaugeError> {
    let mut arguments = env::args_os();
    let program = arguments
        .next()
        .unwrap_or_else(|| OsString::from("aligngauge"));
    let Some(command) = arguments.next() else {
        return Err(usage_error("missing subcommand", &program));
    };

    if is_help(&command) {
        return Ok(CliAction::Help(usage(&program)));
    }
    if command.as_os_str() != OsStr::new("qc") {
        return Err(usage_error(
            format!("unsupported subcommand '{}'", command.to_string_lossy()),
            &program,
        ));
    }

    let mut input = None;
    let mut format = OutputFormat::Legacy;
    let mut format_supplied = false;
    while let Some(argument) = arguments.next() {
        if is_help(&argument) {
            return Ok(CliAction::Help(usage(&program)));
        }
        match argument.to_str() {
            Some("--input") => {
                if input.is_some() {
                    return Err(usage_error("--input may be supplied only once", &program));
                }
                let Some(value) = arguments.next() else {
                    return Err(usage_error("--input requires a path", &program));
                };
                input = Some(PathBuf::from(value));
            }
            Some("--format") => {
                if format_supplied {
                    return Err(usage_error("--format may be supplied only once", &program));
                }
                let Some(value) = arguments.next() else {
                    return Err(usage_error("--format requires a value", &program));
                };
                format = parse_format(&value, &program)?;
                format_supplied = true;
            }
            _ => {
                return Err(usage_error(
                    format!("unsupported argument '{}'", argument.to_string_lossy()),
                    &program,
                ));
            }
        }
    }

    let input = input.ok_or_else(|| usage_error("qc requires --input <BAM>", &program))?;
    Ok(CliAction::Analyze { input, format })
}

fn parse_format(value: &OsStr, program: &OsStr) -> Result<OutputFormat, AlignGaugeError> {
    match value.to_str() {
        Some("human") => Ok(OutputFormat::Human),
        Some("json") => Ok(OutputFormat::Json),
        Some("samtools-flagstat") => Ok(OutputFormat::SamtoolsFlagstat),
        Some("samtools-idxstats") => Ok(OutputFormat::SamtoolsIdxstats),
        _ => Err(usage_error(
            format!(
                "unsupported --format '{}'; use human, json, samtools-flagstat, or samtools-idxstats",
                value.to_string_lossy()
            ),
            program,
        )),
    }
}

fn is_help(value: &OsStr) -> bool {
    value == OsStr::new("--help") || value == OsStr::new("-h")
}

fn usage_error(message: impl Into<String>, program: &OsStr) -> AlignGaugeError {
    AlignGaugeError::new(ErrorCategory::Usage, message).with_detail("usage", usage(program))
}

fn usage(program: &OsStr) -> String {
    format!(
        "Usage: {} qc --input <BAM> [--format <human|json|samtools-flagstat|samtools-idxstats>]",
        program.to_string_lossy()
    )
}
