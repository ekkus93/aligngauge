use std::collections::BTreeSet;
use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use aligngauge_cli::{analyze_bam, analyze_release_with_reference};
use aligngauge_core::{
    AlignGaugeError, ConfigEnvironment, ConfigOverrides, ErrorCategory, LogFormat,
    ProcessEnvironment, ResolvedConfig, diagnostic_log_format_hint, resolve_config,
};
use aligngauge_formats::{OutputFile, OutputPayload, PublisherOptions, publish_output_directory};
use aligngauge_hts::{FieldPlan, ReaderOptions};
use aligngauge_metrics::{CompatibilityFormat, CounterCollector};

#[derive(Debug)]
enum CliAction {
    Help(String),
    Release {
        config_path: Option<PathBuf>,
        reference: Option<PathBuf>,
        overrides: ConfigOverrides,
        diagnostic_hint: LogFormat,
    },
    Legacy {
        input: PathBuf,
        output_format: OutputFormat,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputFormat {
    Human,
    Json,
    SamtoolsFlagstat,
    SamtoolsIdxstats,
}

fn main() {
    let arguments: Vec<OsString> = env::args_os().collect();
    let program = arguments
        .first()
        .cloned()
        .unwrap_or_else(|| OsString::from("aligngauge"));
    match parse_arguments(&arguments, &program) {
        Ok(CliAction::Help(text)) => {
            println!("{text}");
        }
        Ok(CliAction::Release {
            config_path,
            reference,
            overrides,
            diagnostic_hint,
        }) => {
            let environment = ProcessEnvironment;
            let config = match resolve_config(config_path.as_deref(), &environment, overrides) {
                Ok(config) => config,
                Err(error) => exit_with_error(&error, diagnostic_hint),
            };
            let report = match analyze_release_with_reference(&config, reference.as_deref()) {
                Ok(report) => report,
                Err(error) => exit_with_error(&error, config.log_format),
            };
            if let Err(error) = publish_release(&config, &report) {
                exit_with_error(&error, config.log_format);
            }
            if !config.quiet {
                println!(
                    "completed aligngauge release analysis: {} -> {}",
                    config.input.display(),
                    config.outdir.display()
                );
                for warning in &report.provenance().warnings {
                    eprintln!("warning: {warning}");
                }
            }
        }
        Ok(CliAction::Legacy {
            input,
            output_format,
        }) => {
            let result = match output_format {
                OutputFormat::Human => {
                    analyze_bam(&input, FieldPlan::counters_default()).map(|report| {
                        format!(
                            "total_records={}\nmapped_records={}\nunmapped_records={}\n",
                            report.total_records, report.mapped_records, report.unmapped_records
                        )
                    })
                }
                OutputFormat::Json => analyze_bam(&input, FieldPlan::counters_default())
                    .map(|report| report.to_json_pretty()),
                OutputFormat::SamtoolsFlagstat => {
                    run_counter_compatibility(&input, CompatibilityFormat::SamtoolsFlagstat)
                }
                OutputFormat::SamtoolsIdxstats => {
                    run_counter_compatibility(&input, CompatibilityFormat::SamtoolsIdxstats)
                }
            };
            match result {
                Ok(output) => print!("{output}"),
                Err(error) => exit_with_error(&error, LogFormat::Human),
            }
        }
        Err(error) => exit_with_error(&error, diagnostic_log_format_hint(&arguments)),
    }
}

fn publish_release(
    config: &ResolvedConfig,
    report: &aligngauge_cli::ReleaseReport,
) -> Result<(), AlignGaugeError> {
    let mut files = vec![
        OutputFile::new("summary.json", report.summary_json().into_bytes())?,
        OutputFile::new("provenance.json", report.provenance_json().into_bytes())?,
    ];
    if config.compatibility_outputs.flagstat {
        files.push(OutputFile::new(
            "samtools.flagstat.txt",
            report.flagstat_text().as_bytes().to_vec(),
        )?);
    }
    if config.compatibility_outputs.idxstats {
        files.push(OutputFile::new(
            "samtools.idxstats.txt",
            report.idxstats_text().as_bytes().to_vec(),
        )?);
    }
    publish_output_directory(
        &config.outdir,
        &OutputPayload::new(files)?,
        PublisherOptions {
            preserve_failed_staging: config.preserve_failed_staging,
        },
    )
}

fn run_counter_compatibility(
    input: &std::path::Path,
    format: CompatibilityFormat,
) -> Result<String, AlignGaugeError> {
    let field_plan = CounterCollector::field_plan()?;
    let mut reader =
        aligngauge_hts::BamReader::open(input, field_plan, ReaderOptions { io_threads: 1 })?;
    let mut collector = CounterCollector::new(reader.header())?;
    while let Some(record) = reader.next_record()? {
        collector.observe(&record)?;
    }
    let report = collector.finish();
    match format {
        CompatibilityFormat::SamtoolsFlagstat => Ok(report.samtools_flagstat()),
        CompatibilityFormat::SamtoolsIdxstats => Ok(report.samtools_idxstats()),
    }
}

fn exit_with_error(error: &AlignGaugeError, log_format: LogFormat) -> ! {
    match log_format {
        LogFormat::Human => eprintln!("{}", error.render_human(false)),
        LogFormat::Json => eprintln!("{}", error.render_json(false)),
    }
    std::process::exit(i32::from(error.exit_code()));
}

fn parse_arguments(arguments: &[OsString], program: &OsStr) -> Result<CliAction, AlignGaugeError> {
    if arguments.len() < 2 {
        return Err(usage_error("missing command", program));
    }
    if is_help(&arguments[1]) {
        return Ok(CliAction::Help(usage(program)));
    }
    if arguments[1] != OsStr::new("qc") {
        return Err(usage_error("expected 'qc' subcommand", program));
    }
    parse_qc(&arguments[2..], program)
}

fn parse_qc(arguments: &[OsString], program: &OsStr) -> Result<CliAction, AlignGaugeError> {
    if arguments.iter().any(|argument| is_help(argument)) {
        return Ok(CliAction::Help(usage(program)));
    }
    let mut state = ParseState::default();
    let mut index = 0;
    while index < arguments.len() {
        let argument = &arguments[index];
        index += 1;
        match argument.to_str() {
            Some("--input") => set_path_option(
                &mut state.overrides.input,
                next_value(arguments, &mut index, "--input", program)?,
                "--input",
                program,
            )?,
            Some("--outdir") => {
                state.release_option_seen = true;
                set_path_option(
                    &mut state.overrides.outdir,
                    next_value(arguments, &mut index, "--outdir", program)?,
                    "--outdir",
                    program,
                )?;
            }
            Some("--threads") => {
                state.release_option_seen = true;
                set_once(
                    &mut state.overrides.threads,
                    parse_usize(
                        next_value(arguments, &mut index, "--threads", program)?,
                        "--threads",
                        program,
                    )?,
                    "--threads",
                    program,
                )?;
            }
            Some("--io-threads") => {
                state.release_option_seen = true;
                set_once(
                    &mut state.overrides.io_threads,
                    parse_usize(
                        next_value(arguments, &mut index, "--io-threads", program)?,
                        "--io-threads",
                        program,
                    )?,
                    "--io-threads",
                    program,
                )?;
            }
            Some("--memory-limit") => {
                state.release_option_seen = true;
                set_once(
                    &mut state.overrides.memory_limit_bytes,
                    parse_size(
                        next_value(arguments, &mut index, "--memory-limit", program)?,
                        "--memory-limit",
                        program,
                    )?,
                    "--memory-limit",
                    program,
                )?;
            }
            Some("--coverage-thresholds") => {
                state.release_option_seen = true;
                set_once(
                    &mut state.overrides.coverage_thresholds,
                    parse_thresholds(
                        next_value(arguments, &mut index, "--coverage-thresholds", program)?,
                        program,
                    )?,
                    "--coverage-thresholds",
                    program,
                )?;
            }
            Some("--config") => {
                state.release_option_seen = true;
                set_path_option(
                    &mut state.config_path,
                    next_value(arguments, &mut index, "--config", program)?,
                    "--config",
                    program,
                )?;
            }
            Some("--reference") => {
                state.release_option_seen = true;
                set_path_option(
                    &mut state.reference,
                    next_value(arguments, &mut index, "--reference", program)?,
                    "--reference",
                    program,
                )?;
            }
            Some("--log-format") => {
                state.release_option_seen = true;
                set_once(
                    &mut state.overrides.log_format,
                    parse_log_format(
                        next_value(arguments, &mut index, "--log-format", program)?,
                        program,
                    )?,
                    "--log-format",
                    program,
                )?;
            }
            Some("--quiet") => {
                state.release_option_seen = true;
                set_flag(&mut state.quiet, "--quiet", program)?;
            }
            Some("--verbose") => {
                state.release_option_seen = true;
                set_flag(&mut state.verbose, "--verbose", program)?;
            }
            Some("--preserve-failed-staging") => {
                state.release_option_seen = true;
                set_flag(
                    &mut state.preserve_failed_staging,
                    "--preserve-failed-staging",
                    program,
                )?;
            }
            Some("--format") => set_once(
                &mut state.compatibility_format,
                parse_output_format(
                    next_value(arguments, &mut index, "--format", program)?,
                    program,
                )?,
                "--format",
                program,
            )?,
            Some("--targets") => {
                return Err(unsupported_option(
                    "--targets",
                    "targeted/BED metrics are a v0.3 feature",
                    program,
                ));
            }
            Some("--profile") => {
                let value = next_value(arguments, &mut index, "--profile", program)?;
                if value == OsStr::new("targeted") {
                    return Err(unsupported_option(
                        "--profile targeted",
                        "targeted/BED metrics are a v0.3 feature",
                        program,
                    ));
                }
                return Err(usage_error(
                    format!("unsupported --profile value '{}'", value.to_string_lossy()),
                    program,
                ));
            }
            Some("--backend") => {
                return Err(unsupported_option(
                    "--backend",
                    "backend selection is not released",
                    program,
                ));
            }
            Some("--cuda-device") => {
                return Err(unsupported_option(
                    "--cuda-device",
                    "GPU selection is not released",
                    program,
                ));
            }
            Some(option) if option.starts_with('-') => {
                return Err(usage_error(format!("unknown option '{option}'"), program));
            }
            Some(value) => {
                return Err(usage_error(
                    format!("unexpected positional argument '{value}'"),
                    program,
                ));
            }
            None => {
                return Err(usage_error(
                    "arguments must be valid UTF-8 except for filesystem paths",
                    program,
                ));
            }
        }
    }
    state.finish(program)
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum FlagPresence {
    Absent,
    Present,
}

impl Default for FlagPresence {
    fn default() -> Self {
        Self::Absent
    }
}

#[derive(Debug, Default)]
struct ParseState {
    overrides: ConfigOverrides,
    config_path: Option<PathBuf>,
    reference: Option<PathBuf>,
    compatibility_format: Option<OutputFormat>,
    quiet: FlagPresence,
    verbose: FlagPresence,
    preserve_failed_staging: FlagPresence,
    release_option_seen: bool,
}

impl ParseState {
    fn finish(self, program: &OsStr) -> Result<CliAction, AlignGaugeError> {
        if self.compatibility_format.is_some()
            && (self.release_option_seen || self.overrides.outdir.is_some())
        {
            return Err(usage_error(
                "--format compatibility mode cannot be combined with release output options",
                program,
            ));
        }
        if let Some(output_format) = self.compatibility_format {
            let input = self.overrides.input.ok_or_else(|| {
                usage_error("legacy --format mode requires --input <BAM>", program)
            })?;
            return Ok(CliAction::Legacy {
                input,
                output_format,
            });
        }
        if !self.release_option_seen && self.overrides.outdir.is_none() {
            let input = self
                .overrides
                .input
                .ok_or_else(|| usage_error("legacy qc mode requires --input <BAM>", program))?;
            return Ok(CliAction::Legacy {
                input,
                output_format: OutputFormat::Human,
            });
        }

        let diagnostic_hint = self.overrides.log_format.unwrap_or(LogFormat::Human);
        Ok(CliAction::Release {
            config_path: self.config_path,
            reference: self.reference,
            overrides: ConfigOverrides {
                quiet: match self.quiet {
                    FlagPresence::Absent => None,
                    FlagPresence::Present => Some(true),
                },
                verbose: match self.verbose {
                    FlagPresence::Absent => None,
                    FlagPresence::Present => Some(true),
                },
                preserve_failed_staging: match self.preserve_failed_staging {
                    FlagPresence::Absent => None,
                    FlagPresence::Present => Some(true),
                },
                ..self.overrides
            },
            diagnostic_hint,
        })
    }
}

fn next_value(
    arguments: &[OsString],
    index: &mut usize,
    option: &'static str,
    program: &OsStr,
) -> Result<OsString, AlignGaugeError> {
    let value = arguments
        .get(*index)
        .ok_or_else(|| usage_error(format!("{option} requires a value"), program))?;
    *index = index.saturating_add(1);
    Ok(value.clone())
}

fn parse_usize(
    value: OsString,
    option: &'static str,
    program: &OsStr,
) -> Result<usize, AlignGaugeError> {
    let text = value
        .to_str()
        .ok_or_else(|| usage_error(format!("{option} must be valid UTF-8"), program))?;
    text.parse::<usize>().map_err(|source| {
        usage_error(
            format!("invalid {option} value '{text}': {source}"),
            program,
        )
    })
}

fn parse_size(
    value: OsString,
    option: &'static str,
    program: &OsStr,
) -> Result<u64, AlignGaugeError> {
    let text = value
        .to_str()
        .ok_or_else(|| usage_error(format!("{option} must be valid UTF-8"), program))?;
    let split = text
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(text.len());
    let (number, suffix) = text.split_at(split);
    let magnitude = number.parse::<u64>().map_err(|source| {
        usage_error(
            format!("invalid {option} value '{text}': {source}"),
            program,
        )
    })?;
    let multiplier = match suffix {
        "" | "B" => 1_u64,
        "KiB" => 1024,
        "MiB" => 1024_u64.pow(2),
        "GiB" => 1024_u64.pow(3),
        "TiB" => 1024_u64.pow(4),
        _ => {
            return Err(usage_error(
                format!("invalid {option} unit '{suffix}'"),
                program,
            ));
        }
    };
    magnitude.checked_mul(multiplier).ok_or_else(|| {
        usage_error(
            format!("{option} value '{text}' exceeds the supported u64 byte range"),
            program,
        )
    })
}

fn parse_thresholds(value: OsString, program: &OsStr) -> Result<Vec<u32>, AlignGaugeError> {
    let text = value
        .to_str()
        .ok_or_else(|| usage_error("--coverage-thresholds must be valid UTF-8", program))?;
    let mut thresholds = Vec::new();
    for item in text.split(',') {
        if item.is_empty() {
            return Err(usage_error(
                "--coverage-thresholds contains an empty element",
                program,
            ));
        }
        let threshold = item.parse::<u32>().map_err(|source| {
            usage_error(
                format!("invalid coverage threshold '{item}': {source}"),
                program,
            )
        })?;
        if threshold == 0 {
            return Err(usage_error(
                "coverage thresholds must be strictly positive",
                program,
            ));
        }
        thresholds.push(threshold);
    }
    Ok(thresholds)
}

fn parse_log_format(value: OsString, program: &OsStr) -> Result<LogFormat, AlignGaugeError> {
    match value.to_str() {
        Some("human") => Ok(LogFormat::Human),
        Some("json") => Ok(LogFormat::Json),
        Some(other) => Err(usage_error(
            format!("unsupported --log-format '{other}'"),
            program,
        )),
        None => Err(usage_error("--log-format must be valid UTF-8", program)),
    }
}

fn parse_output_format(value: OsString, program: &OsStr) -> Result<OutputFormat, AlignGaugeError> {
    match value.to_str() {
        Some("human") => Ok(OutputFormat::Human),
        Some("json") => Ok(OutputFormat::Json),
        Some("samtools-flagstat") => Ok(OutputFormat::SamtoolsFlagstat),
        Some("samtools-idxstats") => Ok(OutputFormat::SamtoolsIdxstats),
        Some(other) => Err(usage_error(
            format!("unsupported --format '{other}'"),
            program,
        )),
        None => Err(usage_error("--format must be valid UTF-8", program)),
    }
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
        "Usage:\n  {0} qc --input <BAM|CRAM> --outdir <DIR> [OPTIONS]\n\nRequired release values:\n  --input <PATH>                  Local BAM or CRAM input (may also come from --config)\n  --outdir <PATH>                 New output directory (may also come from --config)\n\nCRAM reference integrity:\n  --reference <FASTA>             Explicit local FASTA required for CRAM; remote lookup is disabled\n\nOptional values:\n  --threads <N>                   Collector/reduction thread limit (collector is deterministic serial)\n  --io-threads <N>                HTSlib I/O workers; 0 or 1 selects serial decoding\n  --memory-limit <SIZE>           B, KiB, MiB, GiB, or TiB (default 4GiB)\n  --coverage-thresholds <LIST>    Comma-separated positive depths (default 1,10,20,30)\n  --config <PATH>                 Strict schema_version=1 config file\n  --log-format <human|json>       Diagnostic error format\n  --quiet                         Suppress routine completion summary\n  --verbose                       Enable verbose mode in resolved provenance\n  --preserve-failed-staging       Preserve clearly incomplete staging on publication failure\n  -h, --help                      Show this help\n\nConfiguration precedence:\n  built-ins < config file < documented ALIGNGAUGE_* environment < CLI\n\nDeferred beyond v0.2:\n  --targets/--profile targeted (v0.3), --backend, --cuda-device\n\nLegacy BAM three-counter compatibility probe:\n  {0} qc --input <BAM>\n\nBAM compatibility projections retained for differential validation:\n  {0} qc --input <BAM> --format <human|json|samtools-flagstat|samtools-idxstats>",
        program.to_string_lossy()
    )
}
