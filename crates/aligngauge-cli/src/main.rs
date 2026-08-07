use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aligngauge_cli::{analyze_bam, analyze_release};
use aligngauge_core::config::{parse_coverage_thresholds, parse_memory_limit};
use aligngauge_core::{
    AlignGaugeError, AtomicPublisher, ConfigOverrides, ErrorCategory, LogFormat,
    ProcessEnvironment, resolve_config,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum CompatibilityFormat {
    Human,
    Json,
    SamtoolsFlagstat,
    SamtoolsIdxstats,
}

enum CliAction {
    Help(String),
    Compatibility {
        input: PathBuf,
        format: CompatibilityFormat,
    },
    Release {
        config_path: Option<PathBuf>,
        overrides: ConfigOverrides,
        diagnostic_hint: LogFormat,
    },
}

fn main() -> ExitCode {
    match parse_args() {
        Ok(CliAction::Help(help)) => {
            println!("{help}");
            ExitCode::SUCCESS
        }
        Ok(CliAction::Compatibility { input, format }) => run_compatibility(&input, format),
        Ok(CliAction::Release {
            config_path,
            overrides,
            diagnostic_hint,
        }) => {
            let config =
                match resolve_config(config_path.as_deref(), &ProcessEnvironment, overrides) {
                    Ok(config) => config,
                    Err(error) => return exit_with_error(&error, diagnostic_hint),
                };
            if let Err(error) = preflight_output_destination(&config.outdir) {
                return exit_with_error(&error, config.log_format);
            }
            let report = match analyze_release(&config) {
                Ok(report) => report,
                Err(error) => return exit_with_error(&error, config.log_format),
            };
            let publisher = AtomicPublisher::new(&config.outdir, config.preserve_failed_staging);
            if let Err(error) = publisher.publish(&report.output_bundle()) {
                return exit_with_error(&error, config.log_format);
            }
            if !config.quiet {
                print!("{}", report.render_human());
            }
            ExitCode::SUCCESS
        }
        Err(error) => exit_with_error(&error, LogFormat::Human),
    }
}

fn run_compatibility(input: &Path, format: CompatibilityFormat) -> ExitCode {
    match analyze_bam(input) {
        Ok(report) => {
            let output = match format {
                CompatibilityFormat::Human => report.render_human(),
                CompatibilityFormat::Json => report
                    .to_summary(aligngauge_core::BuildInfo {
                        version: env!("CARGO_PKG_VERSION").to_owned(),
                        git_commit: aligngauge_core::Availability::unavailable(
                            "build revision was not embedded",
                        ),
                    })
                    .to_json_pretty(),
                CompatibilityFormat::SamtoolsFlagstat => report.render_samtools_flagstat(),
                CompatibilityFormat::SamtoolsIdxstats => report.render_samtools_idxstats(),
            };
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => exit_with_error(&error, LogFormat::Human),
    }
}

fn exit_with_error(error: &AlignGaugeError, format: LogFormat) -> ExitCode {
    match format {
        LogFormat::Human => eprintln!("error: {}", error.render_human(false)),
        LogFormat::Json => eprintln!("{}", error.render_json(false)),
    }
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

    let mut overrides = ConfigOverrides::default();
    let mut config_path = None;
    let mut compatibility_format = None;
    let mut diagnostic_hint = LogFormat::Human;
    let mut release_option_seen = false;
    let mut quiet_seen = false;
    let mut verbose_seen = false;
    let mut preserve_seen = false;

    while let Some(argument) = arguments.next() {
        if is_help(&argument) {
            return Ok(CliAction::Help(usage(&program)));
        }
        match argument.to_str() {
            Some("--input") => {
                set_path_option(
                    &mut overrides.input,
                    next_value(&mut arguments, "--input", &program)?,
                    "--input",
                    &program,
                )?;
            }
            Some("--outdir") => {
                release_option_seen = true;
                set_path_option(
                    &mut overrides.outdir,
                    next_value(&mut arguments, "--outdir", &program)?,
                    "--outdir",
                    &program,
                )?;
            }
            Some("--threads") => {
                release_option_seen = true;
                set_once(
                    &mut overrides.threads,
                    parse_positive_usize(
                        next_value(&mut arguments, "--threads", &program)?,
                        "--threads",
                        &program,
                    )?,
                    "--threads",
                    &program,
                )?;
            }
            Some("--io-threads") => {
                release_option_seen = true;
                set_once(
                    &mut overrides.io_threads,
                    parse_usize(
                        next_value(&mut arguments, "--io-threads", &program)?,
                        "--io-threads",
                        &program,
                    )?,
                    "--io-threads",
                    &program,
                )?;
            }
            Some("--memory-limit") => {
                release_option_seen = true;
                let value = utf8_value(
                    next_value(&mut arguments, "--memory-limit", &program)?,
                    "--memory-limit",
                    &program,
                )?;
                set_once(
                    &mut overrides.memory_limit_bytes,
                    parse_memory_limit(&value)?,
                    "--memory-limit",
                    &program,
                )?;
            }
            Some("--coverage-thresholds") => {
                release_option_seen = true;
                let value = utf8_value(
                    next_value(&mut arguments, "--coverage-thresholds", &program)?,
                    "--coverage-thresholds",
                    &program,
                )?;
                set_once(
                    &mut overrides.coverage_thresholds,
                    parse_coverage_thresholds(&value)?,
                    "--coverage-thresholds",
                    &program,
                )?;
            }
            Some("--config") => {
                release_option_seen = true;
                if config_path.is_some() {
                    return Err(usage_error("--config may be supplied only once", &program));
                }
                config_path = Some(PathBuf::from(next_value(
                    &mut arguments,
                    "--config",
                    &program,
                )?));
            }
            Some("--log-format") => {
                release_option_seen = true;
                if overrides.log_format.is_some() {
                    return Err(usage_error(
                        "--log-format may be supplied only once",
                        &program,
                    ));
                }
                let value = utf8_value(
                    next_value(&mut arguments, "--log-format", &program)?,
                    "--log-format",
                    &program,
                )?;
                let format = match value.as_str() {
                    "human" => LogFormat::Human,
                    "json" => LogFormat::Json,
                    _ => {
                        return Err(usage_error("--log-format must be human or json", &program));
                    }
                };
                diagnostic_hint = format;
                overrides.log_format = Some(format);
            }
            Some("--quiet") => {
                release_option_seen = true;
                set_flag(&mut quiet_seen, "--quiet", &program)?;
                overrides.quiet = Some(true);
            }
            Some("--verbose") => {
                release_option_seen = true;
                set_flag(&mut verbose_seen, "--verbose", &program)?;
                overrides.verbose = Some(true);
            }
            Some("--preserve-failed-staging") => {
                release_option_seen = true;
                set_flag(&mut preserve_seen, "--preserve-failed-staging", &program)?;
                overrides.preserve_failed_staging = Some(true);
            }
            Some("--format") => {
                if compatibility_format.is_some() {
                    return Err(usage_error("--format may be supplied only once", &program));
                }
                compatibility_format = Some(parse_compatibility_format(
                    &next_value(&mut arguments, "--format", &program)?,
                    &program,
                )?);
            }
            Some("--reference") => {
                return Err(unsupported_option(
                    "--reference",
                    "CRAM reference resolution is a v0.2 feature",
                    &program,
                ));
            }
            Some("--targets") | Some("--profile") => {
                return Err(unsupported_option(
                    argument.to_string_lossy(),
                    "targeted analysis is a v0.3 feature",
                    &program,
                ));
            }
            Some("--backend") | Some("--cuda-device") => {
                return Err(unsupported_option(
                    argument.to_string_lossy(),
                    "hardware/backend selection is not a released v0.1 feature",
                    &program,
                ));
            }
            _ => {
                return Err(usage_error(
                    format!("unsupported argument '{}'", argument.to_string_lossy()),
                    &program,
                ));
            }
        }
    }

    if let Some(format) = compatibility_format {
        if release_option_seen || config_path.is_some() {
            return Err(usage_error(
                "--format is a compatibility probe and cannot be combined with v0.1 release options",
                &program,
            ));
        }
        let input = overrides
            .input
            .ok_or_else(|| usage_error("compatibility mode requires --input <BAM>", &program))?;
        return Ok(CliAction::Compatibility { input, format });
    }

    Ok(CliAction::Release {
        config_path,
        overrides,
        diagnostic_hint,
    })
}

fn preflight_output_destination(outdir: &Path) -> Result<(), AlignGaugeError> {
    if outdir.exists() {
        return Err(AlignGaugeError::new(
            ErrorCategory::OutputExists,
            format!("output destination '{}' already exists", outdir.display()),
        )
        .with_detail("destination", outdir.to_string_lossy().into_owned()));
    }
    let parent = outdir
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|source| {
        AlignGaugeError::new(
            ErrorCategory::OutputIo,
            format!(
                "failed to prepare output parent directory '{}' before traversal",
                parent.display()
            ),
        )
        .with_source(source)
    })
}

fn parse_compatibility_format(
    value: &OsStr,
    program: &OsStr,
) -> Result<CompatibilityFormat, AlignGaugeError> {
    match value.to_str() {
        Some("human") => Ok(CompatibilityFormat::Human),
        Some("json") => Ok(CompatibilityFormat::Json),
        Some("samtools-flagstat") => Ok(CompatibilityFormat::SamtoolsFlagstat),
        Some("samtools-idxstats") => Ok(CompatibilityFormat::SamtoolsIdxstats),
        _ => Err(usage_error(
            format!(
                "unsupported --format '{}'; use human, json, samtools-flagstat, or samtools-idxstats",
                value.to_string_lossy()
            ),
            program,
        )),
    }
}

fn next_value(
    arguments: &mut impl Iterator<Item = OsString>,
    option: &'static str,
    program: &OsStr,
) -> Result<OsString, AlignGaugeError> {
    arguments
        .next()
        .ok_or_else(|| usage_error(format!("{option} requires a value"), program))
}

fn utf8_value(
    value: OsString,
    option: &'static str,
    program: &OsStr,
) -> Result<String, AlignGaugeError> {
    value
        .into_string()
        .map_err(|_| usage_error(format!("{option} requires valid UTF-8"), program))
}

fn parse_positive_usize(
    value: OsString,
    option: &'static str,
    program: &OsStr,
) -> Result<usize, AlignGaugeError> {
    let parsed = parse_usize(value, option, program)?;
    if parsed == 0 {
        return Err(usage_error(
            format!("{option} must be greater than zero"),
            program,
        ));
    }
    Ok(parsed)
}

fn parse_usize(
    value: OsString,
    option: &'static str,
    program: &OsStr,
) -> Result<usize, AlignGaugeError> {
    let text = utf8_value(value, option, program)?;
    text.parse::<usize>().map_err(|source| {
        usage_error(format!("{option} must be a non-negative integer"), program).with_source(source)
    })
}

fn set_path_option(
    slot: &mut Option<PathBuf>,
    value: OsString,
    option: &'static str,
    program: &OsStr,
) -> Result<(), AlignGaugeError> {
    if slot.is_some() {
        return Err(usage_error(
            format!("{option} may be supplied only once"),
            program,
        ));
    }
    *slot = Some(PathBuf::from(value));
    Ok(())
}

fn set_once<T>(
    slot: &mut Option<T>,
    value: T,
    option: &'static str,
    program: &OsStr,
) -> Result<(), AlignGaugeError> {
    if slot.is_some() {
        return Err(usage_error(
            format!("{option} may be supplied only once"),
            program,
        ));
    }
    *slot = Some(value);
    Ok(())
}

fn set_flag(seen: &mut bool, option: &'static str, program: &OsStr) -> Result<(), AlignGaugeError> {
    if *seen {
        return Err(usage_error(
            format!("{option} may be supplied only once"),
            program,
        ));
    }
    *seen = true;
    Ok(())
}

fn is_help(value: &OsStr) -> bool {
    value == OsStr::new("--help") || value == OsStr::new("-h")
}

fn unsupported_option(
    option: impl std::fmt::Display,
    reason: &'static str,
    program: &OsStr,
) -> AlignGaugeError {
    usage_error(
        format!("{option} is not supported in v0.1: {reason}"),
        program,
    )
}

fn usage_error(message: impl Into<String>, program: &OsStr) -> AlignGaugeError {
    AlignGaugeError::new(ErrorCategory::Usage, message).with_detail("usage", usage(program))
}

fn usage(program: &OsStr) -> String {
    format!(
        "Usage:\n  {0} qc --input <BAM> --outdir <DIR> [OPTIONS]\n\nRequired v0.1 values:\n  --input <PATH>                  Local BAM input (may also come from --config)\n  --outdir <PATH>                 New output directory (may also come from --config)\n\nOptional v0.1 values:\n  --threads <N>                   Collector/reduction thread limit (v0.1 collector is deterministic serial)\n  --io-threads <N>                HTSlib I/O workers; 0 or 1 selects serial decoding\n  --memory-limit <SIZE>           B, KiB, MiB, GiB, or TiB (default 4GiB)\n  --coverage-thresholds <LIST>    Comma-separated positive depths (default 1,10,20,30)\n  --config <PATH>                 Strict schema_version=1 config file\n  --log-format <human|json>       Diagnostic error format\n  --quiet                         Suppress routine completion summary\n  --verbose                       Enable verbose mode in resolved provenance\n  --preserve-failed-staging       Preserve clearly incomplete staging on publication failure\n  -h, --help                      Show this help\n\nConfiguration precedence:\n  built-ins < config file < documented ALIGNGAUGE_* environment < CLI\n\nNot released in v0.1:\n  --reference (v0.2 CRAM), --targets/--profile targeted (v0.3), --backend, --cuda-device\n\nCompatibility probe retained for differential validation:\n  {0} qc --input <BAM> --format <human|json|samtools-flagstat|samtools-idxstats>",
        program.to_string_lossy()
    )
}
