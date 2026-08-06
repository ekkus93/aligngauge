//! Strict v0.1 configuration resolution.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AlignGaugeError, ErrorCategory};
use crate::json::{JsonValue, ToJson};

/// Configuration file schema version supported by this release.
pub const CONFIG_SCHEMA_VERSION: u32 = 1;

const ENV_THREADS: &str = "ALIGNGAUGE_THREADS";
const ENV_IO_THREADS: &str = "ALIGNGAUGE_IO_THREADS";
const ENV_MEMORY_LIMIT: &str = "ALIGNGAUGE_MEMORY_LIMIT";
const ENV_COVERAGE_THRESHOLDS: &str = "ALIGNGAUGE_COVERAGE_THRESHOLDS";
const ENV_LOG_FORMAT: &str = "ALIGNGAUGE_LOG_FORMAT";
const ENV_QUIET: &str = "ALIGNGAUGE_QUIET";
const ENV_VERBOSE: &str = "ALIGNGAUGE_VERBOSE";
const ENV_PRESERVE_FAILED_STAGING: &str = "ALIGNGAUGE_PRESERVE_FAILED_STAGING";

/// Output format for diagnostics.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LogFormat {
    /// Human-readable diagnostics.
    Human,
    /// Structured JSON diagnostics.
    Json,
}

impl LogFormat {
    fn parse(value: &str, source: &str) -> Result<Self, AlignGaugeError> {
        match value {
            "human" => Ok(Self::Human),
            "json" => Ok(Self::Json),
            _ => Err(configuration_error(format!(
                "{source} must be 'human' or 'json', not '{value}'"
            ))),
        }
    }

    /// Stable configuration spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Json => "json",
        }
    }
}

/// Fully resolved v0.1 configuration written to provenance.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ResolvedConfig {
    /// Configuration schema version.
    pub schema_version: u32,
    /// Local BAM input path.
    pub input: PathBuf,
    /// Final output directory.
    pub outdir: PathBuf,
    /// Collector and reduction worker count.
    pub threads: usize,
    /// `HTSlib` I/O worker count.
    pub io_threads: usize,
    /// Maximum planned memory in bytes.
    pub memory_limit_bytes: u64,
    /// Sorted unique positive coverage thresholds.
    pub coverage_thresholds: Vec<u32>,
    /// Diagnostic rendering format.
    pub log_format: LogFormat,
    /// Suppress routine progress output.
    pub quiet: bool,
    /// Enable verbose diagnostics.
    pub verbose: bool,
    /// Preserve an incomplete staging directory after failure.
    pub preserve_failed_staging: bool,
}

impl ToJson for ResolvedConfig {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (
                String::from("coverage_thresholds"),
                self.coverage_thresholds.to_json(),
            ),
            (
                String::from("input"),
                self.input.to_string_lossy().into_owned().to_json(),
            ),
            (
                String::from("io_threads"),
                JsonValue::Unsigned(u64::try_from(self.io_threads).expect("usize fits in u64")),
            ),
            (
                String::from("log_format"),
                self.log_format.as_str().to_json(),
            ),
            (
                String::from("memory_limit_bytes"),
                JsonValue::Unsigned(self.memory_limit_bytes),
            ),
            (
                String::from("outdir"),
                self.outdir.to_string_lossy().into_owned().to_json(),
            ),
            (
                String::from("preserve_failed_staging"),
                self.preserve_failed_staging.to_json(),
            ),
            (String::from("quiet"), self.quiet.to_json()),
            (
                String::from("schema_version"),
                self.schema_version.to_json(),
            ),
            (
                String::from("threads"),
                JsonValue::Unsigned(u64::try_from(self.threads).expect("usize fits in u64")),
            ),
            (String::from("verbose"), self.verbose.to_json()),
        ]))
    }
}

/// Optional values from one higher-precedence configuration source.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct ConfigOverrides {
    /// Input path override.
    pub input: Option<PathBuf>,
    /// Output directory override.
    pub outdir: Option<PathBuf>,
    /// Worker-count override.
    pub threads: Option<usize>,
    /// I/O worker-count override.
    pub io_threads: Option<usize>,
    /// Memory limit override in bytes.
    pub memory_limit_bytes: Option<u64>,
    /// Coverage-threshold override.
    pub coverage_thresholds: Option<Vec<u32>>,
    /// Log-format override.
    pub log_format: Option<LogFormat>,
    /// Quiet-mode override.
    pub quiet: Option<bool>,
    /// Verbose-mode override.
    pub verbose: Option<bool>,
    /// Failed-staging preservation override.
    pub preserve_failed_staging: Option<bool>,
}

/// Environment lookup boundary used to test precedence without mutating the process.
pub trait Environment {
    /// Return one environment variable without assuming UTF-8.
    fn get(&self, key: &str) -> Option<OsString>;
}

/// Process environment implementation.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessEnvironment;

impl Environment for ProcessEnvironment {
    fn get(&self, key: &str) -> Option<OsString> {
        std::env::var_os(key)
    }
}

/// Deterministic environment map for callers and tests.
#[derive(Debug, Clone, Default)]
pub struct MapEnvironment {
    values: BTreeMap<String, OsString>,
}

impl MapEnvironment {
    /// Construct an empty environment.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            values: BTreeMap::new(),
        }
    }

    /// Insert an environment value.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<OsString>) {
        self.values.insert(key.into(), value.into());
    }
}

impl Environment for MapEnvironment {
    fn get(&self, key: &str) -> Option<OsString> {
        self.values.get(key).cloned()
    }
}

/// Resolve built-ins, a strict configuration file, documented environment values,
/// and CLI overrides in that order.
///
/// # Errors
///
/// Returns a `configuration` error for malformed, unknown, contradictory, or
/// unresolved values.
pub fn resolve_config(
    config_path: Option<&Path>,
    environment: &impl Environment,
    cli: ConfigOverrides,
) -> Result<ResolvedConfig, AlignGaugeError> {
    let mut builder = ConfigBuilder::defaults();
    if let Some(path) = config_path {
        builder.apply(parse_config_file(path)?);
    }
    builder.apply(parse_environment(environment)?);
    builder.apply(cli);
    builder.finish()
}

/// Parse a checked byte-size value such as `4GiB`.
///
/// # Errors
///
/// Returns a `configuration` error for unsupported units, zero, or overflow.
pub fn parse_memory_limit(value: &str) -> Result<u64, AlignGaugeError> {
    let value = value.trim();
    let digit_end = value
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(value.len());
    let (digits, unit) = value.split_at(digit_end);
    if digits.is_empty() {
        return Err(configuration_error(format!(
            "memory limit '{value}' does not begin with an integer"
        )));
    }

    let amount = digits.parse::<u64>().map_err(|source| {
        configuration_error(format!("invalid memory limit '{value}'")).with_source(source)
    })?;
    if amount == 0 {
        return Err(configuration_error(
            "memory limit must be greater than zero",
        ));
    }

    let multiplier = match unit.trim().to_ascii_lowercase().as_str() {
        "" | "b" => 1,
        "kib" => 1_u64 << 10,
        "mib" => 1_u64 << 20,
        "gib" => 1_u64 << 30,
        "tib" => 1_u64 << 40,
        other => {
            return Err(configuration_error(format!(
                "unsupported memory-limit unit '{other}'; use B, KiB, MiB, GiB, or TiB"
            )));
        }
    };

    amount.checked_mul(multiplier).ok_or_else(|| {
        configuration_error(format!("memory limit '{value}' exceeds the u64 byte range"))
    })
}

/// Parse, sort, and deduplicate positive coverage thresholds.
///
/// # Errors
///
/// Returns a `configuration` error for an empty list, zero, or an invalid integer.
pub fn parse_coverage_thresholds(value: &str) -> Result<Vec<u32>, AlignGaugeError> {
    let mut thresholds = Vec::new();
    for raw in value.split(',') {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err(configuration_error(format!(
                "coverage-threshold list '{value}' contains an empty item"
            )));
        }
        let threshold = raw.parse::<u32>().map_err(|source| {
            configuration_error(format!("invalid coverage threshold '{raw}'")).with_source(source)
        })?;
        if threshold == 0 {
            return Err(configuration_error(
                "coverage thresholds must be greater than zero",
            ));
        }
        thresholds.push(threshold);
    }
    if thresholds.is_empty() {
        return Err(configuration_error(
            "at least one coverage threshold is required",
        ));
    }
    thresholds.sort_unstable();
    thresholds.dedup();
    Ok(thresholds)
}

#[derive(Debug, Clone)]
struct ConfigBuilder {
    input: Option<PathBuf>,
    outdir: Option<PathBuf>,
    threads: usize,
    io_threads: usize,
    memory_limit_bytes: u64,
    coverage_thresholds: Vec<u32>,
    log_format: LogFormat,
    quiet: bool,
    verbose: bool,
    preserve_failed_staging: bool,
}

impl ConfigBuilder {
    fn defaults() -> Self {
        Self {
            input: None,
            outdir: None,
            threads: 1,
            io_threads: 0,
            memory_limit_bytes: 4_u64 << 30,
            coverage_thresholds: vec![1, 10, 20, 30],
            log_format: LogFormat::Human,
            quiet: false,
            verbose: false,
            preserve_failed_staging: false,
        }
    }

    fn apply(&mut self, overrides: ConfigOverrides) {
        if let Some(value) = overrides.input {
            self.input = Some(value);
        }
        if let Some(value) = overrides.outdir {
            self.outdir = Some(value);
        }
        if let Some(value) = overrides.threads {
            self.threads = value;
        }
        if let Some(value) = overrides.io_threads {
            self.io_threads = value;
        }
        if let Some(value) = overrides.memory_limit_bytes {
            self.memory_limit_bytes = value;
        }
        if let Some(value) = overrides.coverage_thresholds {
            self.coverage_thresholds = value;
        }
        if let Some(value) = overrides.log_format {
            self.log_format = value;
        }
        if let Some(value) = overrides.quiet {
            self.quiet = value;
        }
        if let Some(value) = overrides.verbose {
            self.verbose = value;
        }
        if let Some(value) = overrides.preserve_failed_staging {
            self.preserve_failed_staging = value;
        }
    }

    fn finish(mut self) -> Result<ResolvedConfig, AlignGaugeError> {
        self.coverage_thresholds.sort_unstable();
        self.coverage_thresholds.dedup();
        let input = self
            .input
            .ok_or_else(|| configuration_error("input path is required"))?;
        let outdir = self
            .outdir
            .ok_or_else(|| configuration_error("output directory is required"))?;
        if self.threads == 0 {
            return Err(configuration_error("threads must be greater than zero"));
        }
        if self.memory_limit_bytes == 0 {
            return Err(configuration_error(
                "memory limit must be greater than zero",
            ));
        }
        if self.coverage_thresholds.is_empty() {
            return Err(configuration_error(
                "at least one coverage threshold is required",
            ));
        }
        if self.coverage_thresholds.contains(&0) {
            return Err(configuration_error(
                "coverage thresholds must be greater than zero",
            ));
        }
        if self.quiet && self.verbose {
            return Err(configuration_error(
                "quiet and verbose modes cannot both be enabled",
            ));
        }

        Ok(ResolvedConfig {
            schema_version: CONFIG_SCHEMA_VERSION,
            input,
            outdir,
            threads: self.threads,
            io_threads: self.io_threads,
            memory_limit_bytes: self.memory_limit_bytes,
            coverage_thresholds: self.coverage_thresholds,
            log_format: self.log_format,
            quiet: self.quiet,
            verbose: self.verbose,
            preserve_failed_staging: self.preserve_failed_staging,
        })
    }
}

fn parse_config_file(path: &Path) -> Result<ConfigOverrides, AlignGaugeError> {
    let text = fs::read_to_string(path).map_err(|source| {
        configuration_error(format!(
            "failed to read configuration file '{}'",
            path.display()
        ))
        .with_source(source)
    })?;
    let mut values = BTreeMap::new();
    let mut seen = BTreeSet::new();

    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line.split_once('=').ok_or_else(|| {
            configuration_error(format!(
                "configuration line {line_number} must use 'key = value'"
            ))
        })?;
        let key = key.trim();
        let value = value.trim();
        if !seen.insert(key.to_owned()) {
            return Err(configuration_error(format!(
                "configuration key '{key}' is duplicated"
            )));
        }
        values.insert(key.to_owned(), value.to_owned());
    }

    let schema = values.remove("schema_version").ok_or_else(|| {
        configuration_error(format!(
            "configuration file '{}' is missing schema_version",
            path.display()
        ))
    })?;
    let schema = parse_u32(&schema, "schema_version")?;
    if schema != CONFIG_SCHEMA_VERSION {
        return Err(configuration_error(format!(
            "unsupported configuration schema_version {schema}; expected {CONFIG_SCHEMA_VERSION}"
        )));
    }

    let mut overrides = ConfigOverrides::default();
    for (key, value) in values {
        match key.as_str() {
            "input" => overrides.input = Some(PathBuf::from(parse_string(&value, &key)?)),
            "outdir" => overrides.outdir = Some(PathBuf::from(parse_string(&value, &key)?)),
            "threads" => overrides.threads = Some(parse_positive_usize(&value, &key)?),
            "io_threads" => overrides.io_threads = Some(parse_usize(&value, &key)?),
            "memory_limit" => {
                overrides.memory_limit_bytes =
                    Some(parse_memory_limit(&parse_string(&value, &key)?)?);
            }
            "coverage_thresholds" => {
                overrides.coverage_thresholds =
                    Some(parse_coverage_thresholds(&parse_string(&value, &key)?)?);
            }
            "log_format" => {
                overrides.log_format = Some(LogFormat::parse(&parse_string(&value, &key)?, &key)?);
            }
            "quiet" => overrides.quiet = Some(parse_bool(&value, &key)?),
            "verbose" => overrides.verbose = Some(parse_bool(&value, &key)?),
            "preserve_failed_staging" => {
                overrides.preserve_failed_staging = Some(parse_bool(&value, &key)?);
            }
            _ => {
                return Err(configuration_error(format!(
                    "unknown configuration key '{key}'"
                )));
            }
        }
    }
    Ok(overrides)
}

fn parse_environment(environment: &impl Environment) -> Result<ConfigOverrides, AlignGaugeError> {
    let mut overrides = ConfigOverrides::default();
    if let Some(value) = environment_value(environment, ENV_THREADS)? {
        overrides.threads = Some(parse_positive_usize(&value, ENV_THREADS)?);
    }
    if let Some(value) = environment_value(environment, ENV_IO_THREADS)? {
        overrides.io_threads = Some(parse_usize(&value, ENV_IO_THREADS)?);
    }
    if let Some(value) = environment_value(environment, ENV_MEMORY_LIMIT)? {
        overrides.memory_limit_bytes = Some(parse_memory_limit(&value)?);
    }
    if let Some(value) = environment_value(environment, ENV_COVERAGE_THRESHOLDS)? {
        overrides.coverage_thresholds = Some(parse_coverage_thresholds(&value)?);
    }
    if let Some(value) = environment_value(environment, ENV_LOG_FORMAT)? {
        overrides.log_format = Some(LogFormat::parse(&value, ENV_LOG_FORMAT)?);
    }
    if let Some(value) = environment_value(environment, ENV_QUIET)? {
        overrides.quiet = Some(parse_bool(&value, ENV_QUIET)?);
    }
    if let Some(value) = environment_value(environment, ENV_VERBOSE)? {
        overrides.verbose = Some(parse_bool(&value, ENV_VERBOSE)?);
    }
    if let Some(value) = environment_value(environment, ENV_PRESERVE_FAILED_STAGING)? {
        overrides.preserve_failed_staging = Some(parse_bool(&value, ENV_PRESERVE_FAILED_STAGING)?);
    }
    Ok(overrides)
}

fn environment_value(
    environment: &impl Environment,
    key: &str,
) -> Result<Option<String>, AlignGaugeError> {
    environment
        .get(key)
        .map(|value| {
            value.into_string().map_err(|value| {
                configuration_error(format!(
                    "environment variable {key} is not valid UTF-8: {}",
                    lossy(&value)
                ))
            })
        })
        .transpose()
}

fn parse_string(value: &str, key: &str) -> Result<String, AlignGaugeError> {
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        return unescape_string(&value[1..value.len() - 1], key);
    }
    if value.contains(char::is_whitespace) {
        return Err(configuration_error(format!(
            "configuration value for '{key}' must be quoted"
        )));
    }
    Ok(value.to_owned())
}

fn unescape_string(value: &str, key: &str) -> Result<String, AlignGaugeError> {
    let mut output = String::new();
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        let escaped = characters.next().ok_or_else(|| {
            configuration_error(format!(
                "configuration string for '{key}' ends with an escape character"
            ))
        })?;
        match escaped {
            '\\' => output.push('\\'),
            '"' => output.push('"'),
            'n' => output.push('\n'),
            'r' => output.push('\r'),
            't' => output.push('\t'),
            _ => {
                return Err(configuration_error(format!(
                    "unsupported escape '\\{escaped}' in configuration key '{key}'"
                )));
            }
        }
    }
    Ok(output)
}

fn parse_bool(value: &str, source: &str) -> Result<bool, AlignGaugeError> {
    match value.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(configuration_error(format!(
            "{source} must be true or false, not '{other}'"
        ))),
    }
}

fn parse_positive_usize(value: &str, source: &str) -> Result<usize, AlignGaugeError> {
    let parsed = parse_usize(value, source)?;
    if parsed == 0 {
        return Err(configuration_error(format!(
            "{source} must be greater than zero"
        )));
    }
    Ok(parsed)
}

fn parse_usize(value: &str, source: &str) -> Result<usize, AlignGaugeError> {
    value.trim().parse::<usize>().map_err(|error| {
        configuration_error(format!("{source} must be a non-negative integer")).with_source(error)
    })
}

fn parse_u32(value: &str, source: &str) -> Result<u32, AlignGaugeError> {
    value.trim().parse::<u32>().map_err(|error| {
        configuration_error(format!("{source} must be a non-negative integer")).with_source(error)
    })
}

fn lossy(value: &OsStr) -> String {
    value.to_string_lossy().into_owned()
}

fn configuration_error(message: impl Into<String>) -> AlignGaugeError {
    AlignGaugeError::new(ErrorCategory::Configuration, message)
}

#[cfg(test)]
mod tests {
    use super::{
        ConfigOverrides, LogFormat, MapEnvironment, parse_coverage_thresholds, parse_memory_limit,
        resolve_config,
    };
    use crate::error::ErrorCategory;
    use crate::json::ToJson;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_PATH: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn memory_units_are_checked() {
        assert_eq!(parse_memory_limit("4GiB").expect("valid size"), 4_u64 << 30);
        assert_eq!(
            parse_memory_limit("512MiB").expect("valid size"),
            512_u64 << 20
        );
        assert!(parse_memory_limit("18446744073709551615TiB").is_err());
        assert!(parse_memory_limit("0B").is_err());
        assert!(parse_memory_limit("4GB").is_err());
    }

    #[test]
    fn thresholds_are_sorted_and_deduplicated() {
        assert_eq!(
            parse_coverage_thresholds("30,1,10,10,20").expect("valid thresholds"),
            vec![1, 10, 20, 30]
        );
        assert!(parse_coverage_thresholds("0,10").is_err());
        assert!(parse_coverage_thresholds("1,,10").is_err());
    }

    #[test]
    fn precedence_is_defaults_then_file_then_environment_then_cli() {
        let path = temporary_path("precedence");
        fs::write(
            &path,
            "schema_version = 1\ninput = \"file.bam\"\noutdir = \"file-out\"\nthreads = 2\nmemory_limit = \"2GiB\"\ncoverage_thresholds = \"20,1\"\n",
        )
        .expect("write config");
        let mut environment = MapEnvironment::new();
        environment.insert("ALIGNGAUGE_THREADS", "4");
        environment.insert("ALIGNGAUGE_LOG_FORMAT", "json");
        let cli = ConfigOverrides {
            input: Some(PathBuf::from("cli.bam")),
            outdir: Some(PathBuf::from("cli-out")),
            threads: Some(8),
            ..ConfigOverrides::default()
        };

        let resolved = resolve_config(Some(&path), &environment, cli).expect("resolve config");
        assert_eq!(resolved.input, PathBuf::from("cli.bam"));
        assert_eq!(resolved.outdir, PathBuf::from("cli-out"));
        assert_eq!(resolved.threads, 8);
        assert_eq!(resolved.memory_limit_bytes, 2_u64 << 30);
        assert_eq!(resolved.coverage_thresholds, vec![1, 20]);
        assert_eq!(resolved.log_format, LogFormat::Json);
        assert!(resolved.to_json_pretty().contains("\"schema_version\": 1"));
        fs::remove_file(path).expect("remove config");
    }

    #[test]
    fn unknown_keys_and_contradictory_modes_fail() {
        let path = temporary_path("unknown");
        fs::write(
            &path,
            "schema_version = 1\ninput = \"sample.bam\"\noutdir = \"out\"\nmystery = 7\n",
        )
        .expect("write config");
        let error = resolve_config(
            Some(&path),
            &MapEnvironment::new(),
            ConfigOverrides::default(),
        )
        .expect_err("unknown key must fail");
        assert_eq!(error.category(), ErrorCategory::Configuration);
        fs::remove_file(path).expect("remove config");

        let error = resolve_config(
            None,
            &MapEnvironment::new(),
            ConfigOverrides {
                input: Some(PathBuf::from("sample.bam")),
                outdir: Some(PathBuf::from("out")),
                quiet: Some(true),
                verbose: Some(true),
                ..ConfigOverrides::default()
            },
        )
        .expect_err("contradictory modes must fail");
        assert_eq!(error.category(), ErrorCategory::Configuration);
    }

    fn temporary_path(label: &str) -> PathBuf {
        let id = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "aligngauge-config-{label}-{}-{id}.conf",
            std::process::id()
        ))
    }
}
