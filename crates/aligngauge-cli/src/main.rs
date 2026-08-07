use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aligngauge_cli::{analyze_bam, analyze_release_with_reference_and_targets};
use aligngauge_core::config::{parse_coverage_thresholds, parse_memory_limit};
use aligngauge_core::{
    AlignGaugeError, AtomicPublisher, ConfigOverrides, ErrorCategory, LogFormat,
    ProcessEnvironment, ToJson, resolve_config,
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
    Legacy {
        input: PathBuf,
    },
    Compatibility {
        input: PathBuf,
        format: CompatibilityFormat,
    },
    Release {
        config_path: Option<PathBuf>,
        reference: Option<PathBuf>,
        targets: Option<PathBuf>,
        near_distance_bases: Option<u64>,
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
        Ok(CliAction::Legacy { input }) => run_legacy(&input),
        Ok(CliAction::Compatibility { input, format }) => run_compatibility(&input, format),
        Ok(CliAction::Release {
            config_path,
            reference,
            targets,
            near_distance_bases,
            overrides,
            diagnostic_hint,
        }) => {
            let config =
                match resolve_config(config_path.as_deref(), &ProcessEnvironment, overrides) {
                    Ok(config) => config,
                    Err(error) => return exit_with_error(&error, diagnostic_hint),
                };
            emit_verbose_config(&config);
            if let Err(error) = preflight_output_destination(&config.outdir) {
                return exit_with_error(&error, config.log_format);
            }
            let report = match analyze_release_with_reference_and_targets(
                &config,
                reference.as_deref(),
                targets.as_deref(),
                near_distance_bases,
            ) {
                Ok(report) => report,
                Err(error) => return exit_with_error(&error, config.log_format),
            };
            emit_warnings(&report.summary().warnings, config.log_format);
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

fn run_legacy(input: &Path) -> ExitCode {
    match analyze_bam(input) {
        Ok(report) => {
            let counters = report.alignment_counters();
            print!(
                "total\t{}\nmapped\t{}\nunmapped\t{}\n",
                counters.total, counters.mapped, counters.unmapped
            );
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
                CompatibilityFormat::SamtoolsIdxstats => match report.render_samtools_idxstats() {
                    Ok(output) => output,
                    Err(error) => return exit_with_error(&error, LogFormat::Human),
                },
            };
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => exit_with_error(&error, LogFormat::Human),
    }
}

fn emit_verbose_config(config: &aligngauge_core::ResolvedConfig) {
    if !config.verbose {
        return;
    }
    let rendered = config.to_json().to_compact_string();
    match config.log_format {
        LogFormat::Human => eprintln!("verbose: resolved_config={rendered}"),
        LogFormat::Json => eprintln!("{rendered}"),
    }
}

fn emit_warnings(warnings: &[aligngauge_core::Warning], format: LogFormat) {
    for warning in warnings {
        match format {
            LogFormat::Human => {
                eprintln!("warning[{}]: {}", warning.code, warning.message);
            }
            LogFormat::Json => eprintln!("{}", warning.to_json().to_compact_string()),
        }
    }
}

fn exit_with_error(error: &AlignGaugeError, format: LogFormat) -> ExitCode {
    match format {
        LogFormat::Human => eprintln!("error: {}", error.render_human(false)),
        LogFormat::Json => eprintln!("{}", error.render_json(false)),
    }
    ExitCode::from(error.exit_code())
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum FlagPresence {
    Absent,
    Present,
}

struct ParseState {
    overrides: ConfigOverrides,
    config_path: Option<PathBuf>,
    reference: Option<PathBuf>,
    targets: Option<PathBuf>,
    near_distance_bases: Option<u64>,
    compatibility_format: Option<CompatibilityFormat>,
    diagnostic_hint: LogFormat,
    release_option_seen: bool,
    quiet_seen: FlagPresence,
    verbose_seen: FlagPresence,
    preserve_seen: FlagPresence,
}

impl ParseState {
    fn initial() -> Self {
        Self {
            overrides: ConfigOverrides::default(),
            config_path: None,
            reference: None,
            targets: None,
            near_distance_bases: None,
            compatibility_format: None,
            diagnostic_hint: LogFormat::Human,
            release_option_seen: false,
            quiet_seen: FlagPresence::Absent,
            verbose_seen: FlagPresence::Absent,
            preserve_seen: FlagPresence::Absent,
        }
    }

    fn finish(mut self, program: &OsStr) -> Result<CliAction, AlignGaugeError> {
        if let Some(format) = self.compatibility_format {
            if self.release_option_seen {
                return Err(usage_error(
                    "--format is a compatibility probe and cannot be combined with release output options",
                    program,
                ));
            }
            let input =
                self.overrides.input.take().ok_or_else(|| {
                    usage_error("compatibility mode requires --input <BAM>", program)
                })?;
            return Ok(CliAction::Compatibility { input, format });
        }

        if !self.release_option_seen {
            let input = self
                .overrides
                .input
                .take()
                .ok_or_else(|| usage_error("qc requires --input <BAM>", program))?;
            return Ok(CliAction::Legacy { input });
        }

        if self.near_distance_bases.is_some() && self.targets.is_none() {
            return Err(usage_error(
                "--near-distance requires --targets <BED>",
                program,
            ));
        }

        Ok(CliAction::Release {
            config_path: self.config_path,
            reference: self.reference,
            targets: self.targets,
            near_distance_bases: self.near_distance_bases,
            overrides: self.overrides,
            diagnostic_hint: self.diagnostic_hint,
        })
    }
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
    parse_qc_args(arguments, &program)
}

fn parse_qc_args(
    mut arguments: impl Iterator<Item = OsString>,
    program: &OsStr,
) -> Result<CliAction, AlignGaugeError> {
    let mut state = ParseState::initial();
    while let Some(argument) = arguments.next() {
        if is_help(&argument) {
            return Ok(CliAction::Help(usage(program)));
        }
        parse_qc_option(&mut state, argument.as_os_str(), &mut arguments, program)?;
    }
    state.finish(program)
}

fn parse_qc_option(
    state: &mut ParseState,
    argument: &OsStr,
    arguments: &mut impl Iterator<Item = OsString>,
    program: &OsStr,
) -> Result<(), AlignGaugeError> {
    match argument.to_str() {
        Some("--input") => set_path_option(
            &mut state.overrides.input,
            next_value(arguments, "--input", program)?,
            "--input",
            program,
        ),
        Some(
            option @ ("--outdir"
            | "--threads"
            | "--io-threads"
            | "--memory-limit"
            | "--coverage-thresholds"
            | "--near-distance"
            | "--config"
            | "--log-format"),
        ) => parse_release_value_option(state, option, arguments, program),
        Some(option @ ("--quiet" | "--verbose" | "--preserve-failed-staging")) => {
            parse_release_flag_option(state, option, program)
        }
        Some("--format") => parse_format_option(state, arguments, program),
        Some("--reference") => {
            state.release_option_seen = true;
            set_path_option(
                &mut state.reference,
                next_value(arguments, "--reference", program)?,
                "--reference",
                program,
            )
        }
        Some("--targets") => {
            state.release_option_seen = true;
            set_path_option(
                &mut state.targets,
                next_value(arguments, "--targets", program)?,
                "--targets",
                program,
            )
        }
        Some("--profile") => Err(unsupported_option(
            argument.to_string_lossy(),
            "targeted profile selection is not released; v0.3 uses aligngauge-targeted-v0.3",
            program,
        )),
        Some("--backend" | "--cuda-device") => Err(unsupported_option(
            argument.to_string_lossy(),
            "hardware/backend selection is not released",
            program,
        )),
        _ => Err(usage_error(
            format!("unsupported argument '{}'", argument.to_string_lossy()),
            program,
        )),
    }
}

fn parse_release_value_option(
    state: &mut ParseState,
    option: &str,
    arguments: &mut impl Iterator<Item = OsString>,
    program: &OsStr,
) -> Result<(), AlignGaugeError> {
    state.release_option_seen = true;
    match option {
        "--outdir" => set_path_option(
            &mut state.overrides.outdir,
            next_value(arguments, "--outdir", program)?,
            "--outdir",
            program,
        ),
        "--threads" => {
            let value = parse_positive_usize(
                next_value(arguments, "--threads", program)?,
                "--threads",
                program,
            )?;
            set_once(&mut state.overrides.threads, value, "--threads", program)
        }
        "--io-threads" => {
            let value = parse_usize(
                next_value(arguments, "--io-threads", program)?,
                "--io-threads",
                program,
            )?;
            set_once(
                &mut state.overrides.io_threads,
                value,
                "--io-threads",
                program,
            )
        }
        "--memory-limit" => parse_memory_limit_option(state, arguments, program),
        "--coverage-thresholds" => parse_coverage_threshold_option(state, arguments, program),
        "--near-distance" => {
            let value = parse_u64(
                next_value(arguments, "--near-distance", program)?,
                "--near-distance",
                program,
            )?;
            set_once(
                &mut state.near_distance_bases,
                value,
                "--near-distance",
                program,
            )
        }
        "--config" => parse_config_option(state, arguments, program),
        "--log-format" => parse_log_format_option(state, arguments, program),
        _ => Err(parser_invariant(option)),
    }
}

fn parse_memory_limit_option(
    state: &mut ParseState,
    arguments: &mut impl Iterator<Item = OsString>,
    program: &OsStr,
) -> Result<(), AlignGaugeError> {
    let value = utf8_value(
        next_value(arguments, "--memory-limit", program)?,
        "--memory-limit",
        program,
    )?;
    set_once(
        &mut state.overrides.memory_limit_bytes,
        parse_memory_limit(&value)?,
        "--memory-limit",
        program,
    )
}

fn parse_coverage_threshold_option(
    state: &mut ParseState,
    arguments: &mut impl Iterator<Item = OsString>,
    program: &OsStr,
) -> Result<(), AlignGaugeError> {
    let value = utf8_value(
        next_value(arguments, "--coverage-thresholds", program)?,
        "--coverage-thresholds",
        program,
    )?;
    set_once(
        &mut state.overrides.coverage_thresholds,
        parse_coverage_thresholds(&value)?,
        "--coverage-thresholds",
        program,
    )
}

fn parse_config_option(
    state: &mut ParseState,
    arguments: &mut impl Iterator<Item = OsString>,
    program: &OsStr,
) -> Result<(), AlignGaugeError> {
    if state.config_path.is_some() {
        return Err(usage_error("--config may be supplied only once", program));
    }
    state.config_path = Some(PathBuf::from(next_value(arguments, "--config", program)?));
    Ok(())
}

fn parse_log_format_option(
    state: &mut ParseState,
    arguments: &mut impl Iterator<Item = OsString>,
    program: &OsStr,
) -> Result<(), AlignGaugeError> {
    if state.overrides.log_format.is_some() {
        return Err(usage_error(
            "--log-format may be supplied only once",
            program,
        ));
    }
    let value = utf8_value(
        next_value(arguments, "--log-format", program)?,
        "--log-format",
        program,
    )?;
    let format = match value.as_str() {
        "human" => LogFormat::Human,
        "json" => LogFormat::Json,
        _ => return Err(usage_error("--log-format must be human or json", program)),
    };
    state.diagnostic_hint = format;
    state.overrides.log_format = Some(format);
    Ok(())
}

fn parse_release_flag_option(
    state: &mut ParseState,
    option: &str,
    program: &OsStr,
) -> Result<(), AlignGaugeError> {
    state.release_option_seen = true;
    match option {
        "--quiet" => {
            set_flag(&mut state.quiet_seen, "--quiet", program)?;
            state.overrides.quiet = Some(true);
        }
        "--verbose" => {
            set_flag(&mut state.verbose_seen, "--verbose", program)?;
            state.overrides.verbose = Some(true);
        }
        "--preserve-failed-staging" => {
            set_flag(
                &mut state.preserve_seen,
                "--preserve-failed-staging",
                program,
            )?;
            state.overrides.preserve_failed_staging = Some(true);
        }
        _ => return Err(parser_invariant(option)),
    }
    Ok(())
}

fn parse_format_option(
    state: &mut ParseState,
    arguments: &mut impl Iterator<Item = OsString>,
    program: &OsStr,
) -> Result<(), AlignGaugeError> {
    if state.compatibility_format.is_some() {
        return Err(usage_error("--format may be supplied only once", program));
    }
    state.compatibility_format = Some(parse_compatibility_format(
        &next_value(arguments, "--format", program)?,
        program,
    )?);
    Ok(())
}

fn parser_invariant(option: &str) -> AlignGaugeError {
    AlignGaugeError::new(
        ErrorCategory::InternalInvariant,
        "CLI parser dispatched an unrecognized option",
    )
    .with_detail("option", option.to_owned())
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

fn parse_u64(
    value: OsString,
    option: &'static str,
    program: &OsStr,
) -> Result<u64, AlignGaugeError> {
    let text = utf8_value(value, option, program)?;
    text.parse::<u64>().map_err(|source| {
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

fn set_flag(
    seen: &mut FlagPresence,
    option: &'static str,
    program: &OsStr,
) -> Result<(), AlignGaugeError> {
    if matches!(*seen, FlagPresence::Present) {
        return Err(usage_error(
            format!("{option} may be supplied only once"),
            program,
        ));
    }
    *seen = FlagPresence::Present;
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
        format!("{option} is not supported by this release: {reason}"),
        program,
    )
}

fn usage_error(message: impl Into<String>, program: &OsStr) -> AlignGaugeError {
    AlignGaugeError::new(ErrorCategory::Usage, message).with_detail("usage", usage(program))
}

fn usage(program: &OsStr) -> String {
    format!(
        "Usage:\n  {0} qc --input <BAM|CRAM> --outdir <DIR> [OPTIONS]\n\nRequired release values:\n  --input <PATH>                  Local BAM or CRAM input (may also come from --config)\n  --outdir <PATH>                 New output directory (may also come from --config)\n\nCRAM reference integrity:\n  --reference <FASTA>             Explicit local FASTA required for CRAM; remote lookup is disabled\n\nTargeted v0.3 analysis:\n  --targets <BED>                 Local BED3-BED12 target definition; exact contig names required\n  --near-distance <N>             Symmetric near-target distance in bases (default 250; requires --targets)\n                                  Uses native aligngauge-targeted-v0.3 semantics; no Picard compatibility claim\n\nOptional values:\n  --threads <N>                   Collector/reduction thread limit (collector is deterministic serial)\n  --io-threads <N>                HTSlib I/O workers; 0 or 1 selects serial decoding\n  --memory-limit <SIZE>           B, KiB, MiB, GiB, or TiB (default 4GiB)\n  --coverage-thresholds <LIST>    Comma-separated positive depths (default 1,10,20,30)\n  --config <PATH>                 Strict schema_version=1 config file\n  --log-format <human|json>       Diagnostic error format\n  --quiet                         Suppress routine completion summary\n  --verbose                       Enable verbose mode in resolved provenance\n  --preserve-failed-staging       Preserve clearly incomplete staging on publication failure\n  -h, --help                      Show this help\n\nConfiguration precedence:\n  built-ins < config file < documented ALIGNGAUGE_* environment < CLI\n\nDeferred beyond v0.3:\n  --profile selection, --backend, --cuda-device\n\nLegacy BAM three-counter compatibility probe:\n  {0} qc --input <BAM>\n\nBAM compatibility projections retained for differential validation:\n  {0} qc --input <BAM> --format <human|json|samtools-flagstat|samtools-idxstats>",
        program.to_string_lossy()
    )
}
