use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::ExitCode;

use aligngauge_cli::{BamCounts, count_bam};
use aligngauge_core::{AlignGaugeError, ErrorCategory};

enum CliAction {
    Help(String),
    Count(PathBuf),
}

fn main() -> ExitCode {
    match parse_args() {
        Ok(CliAction::Help(help)) => {
            println!("{help}");
            ExitCode::SUCCESS
        }
        Ok(CliAction::Count(input)) => match count_bam(&input) {
            Ok(BamCounts {
                total,
                mapped,
                unmapped,
            }) => {
                println!("total\t{total}");
                println!("mapped\t{mapped}");
                println!("unmapped\t{unmapped}");
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

    if command.as_os_str() == OsStr::new("--help") || command.as_os_str() == OsStr::new("-h") {
        return Ok(CliAction::Help(usage(&program)));
    }
    if command.as_os_str() != OsStr::new("qc") {
        return Err(usage_error(
            format!("unsupported subcommand '{}'", command.to_string_lossy()),
            &program,
        ));
    }

    let mut input = None;
    while let Some(argument) = arguments.next() {
        if argument.as_os_str() == OsStr::new("--help") || argument.as_os_str() == OsStr::new("-h")
        {
            return Ok(CliAction::Help(usage(&program)));
        }
        if argument.as_os_str() != OsStr::new("--input") {
            return Err(usage_error(
                format!("unsupported argument '{}'", argument.to_string_lossy()),
                &program,
            ));
        }
        if input.is_some() {
            return Err(usage_error("--input may be supplied only once", &program));
        }
        let Some(value) = arguments.next() else {
            return Err(usage_error("--input requires a path", &program));
        };
        input = Some(PathBuf::from(value));
    }

    input
        .map(CliAction::Count)
        .ok_or_else(|| usage_error("qc requires --input <BAM>", &program))
}

fn usage_error(message: impl Into<String>, program: &OsStr) -> AlignGaugeError {
    AlignGaugeError::new(ErrorCategory::Usage, message).with_detail("usage", usage(program))
}

fn usage(program: &OsStr) -> String {
    format!("Usage: {} qc --input <BAM>", program.to_string_lossy())
}
