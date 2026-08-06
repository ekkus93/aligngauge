use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::ExitCode;

use aligngauge_cli::{BamCounts, count_bam};

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
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(2)
        }
    }
}

fn parse_args() -> Result<CliAction, String> {
    let mut arguments = env::args_os();
    let program = arguments
        .next()
        .unwrap_or_else(|| OsString::from("aligngauge"));
    let Some(command) = arguments.next() else {
        return Err(format!("missing subcommand\n{}", usage(&program)));
    };

    if command.as_os_str() == OsStr::new("--help") || command.as_os_str() == OsStr::new("-h") {
        return Ok(CliAction::Help(usage(&program)));
    }
    if command.as_os_str() != OsStr::new("qc") {
        return Err(format!(
            "unsupported subcommand '{}'\n{}",
            command.to_string_lossy(),
            usage(&program)
        ));
    }

    let mut input = None;
    while let Some(argument) = arguments.next() {
        if argument.as_os_str() == OsStr::new("--help") || argument.as_os_str() == OsStr::new("-h")
        {
            return Ok(CliAction::Help(usage(&program)));
        }
        if argument.as_os_str() != OsStr::new("--input") {
            return Err(format!(
                "unsupported argument '{}'\n{}",
                argument.to_string_lossy(),
                usage(&program)
            ));
        }
        if input.is_some() {
            return Err(String::from("--input may be supplied only once"));
        }
        let Some(value) = arguments.next() else {
            return Err(String::from("--input requires a path"));
        };
        input = Some(PathBuf::from(value));
    }

    input
        .map(CliAction::Count)
        .ok_or_else(|| String::from("qc requires --input <BAM>"))
}

fn usage(program: &OsStr) -> String {
    format!("Usage: {} qc --input <BAM>", program.to_string_lossy())
}
