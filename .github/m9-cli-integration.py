#!/usr/bin/env python3
"""Apply guarded Milestone 9 CLI/release integration edits."""

from pathlib import Path
import re


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:140]!r}")
    file.write_text(text.replace(old, new, 1))


def regex_once(path: str, pattern: str, replacement: str) -> None:
    file = Path(path)
    text = file.read_text()
    updated, count = re.subn(pattern, lambda _: replacement, text, count=1, flags=re.S)
    if count != 1:
        raise SystemExit(f"{path}: expected one regex match, found {count}: {pattern[:100]!r}")
    file.write_text(updated)


# CLI crate consumes the permanent target-format layer directly.
replace_once(
    "crates/aligngauge-cli/Cargo.toml",
    'aligngauge-core = { path = "../aligngauge-core" }\n',
    'aligngauge-core = { path = "../aligngauge-core" }\n'
    'aligngauge-formats = { path = "../aligngauge-formats" }\n',
)

# Preserve the exact target file identity, including path, in targeted provenance.
targeted = "crates/aligngauge-coverage/src/targeted.rs"
replace_once(
    targeted,
    "pub struct TargetedCoverageReport {\n"
    "    summary: TargetedCoverageSummary,\n",
    "pub struct TargetedCoverageReport {\n"
    "    identity: TargetFileIdentity,\n"
    "    summary: TargetedCoverageSummary,\n",
)
replace_once(
    targeted,
    "        let identity = TargetFileIdentity {\n"
    "            path: None,\n"
    "            size_bytes: self.summary.target_size_bytes,\n"
    "            sha256: self.summary.target_sha256.clone(),\n"
    "            source_interval_count: self.summary.source_interval_count,\n"
    "        };\n"
    "        provenance\n"
    "            .normalization_actions\n"
    "            .extend(self.target_normalization.actions(&identity));\n",
    "        provenance.analysis_plan.insert(\n"
    "            String::from(\"target_path\"),\n"
    "            self.identity\n"
    "                .path\n"
    "                .as_ref()\n"
    "                .map_or(JsonValue::Null, |path| JsonValue::String(path.clone())),\n"
    "        );\n"
    "        provenance\n"
    "            .normalization_actions\n"
    "            .extend(self.target_normalization.actions(&self.identity));\n",
)
replace_once(
    targeted,
    "            self.selected_normalization\n"
    "                .actions(&identity)\n",
    "            self.selected_normalization\n"
    "                .actions(&self.identity)\n",
)
replace_once(
    targeted,
    "        let summary = TargetedCoverageSummary {\n"
    "            profile: TARGETED_PROFILE.to_owned(),\n",
    "        let identity = self.identity.clone();\n"
    "        let summary = TargetedCoverageSummary {\n"
    "            profile: TARGETED_PROFILE.to_owned(),\n",
)
replace_once(
    targeted,
    "            target_sha256: self.identity.sha256,\n"
    "            target_size_bytes: self.identity.size_bytes,\n"
    "            source_interval_count: self.identity.source_interval_count,\n",
    "            target_sha256: self.identity.sha256.clone(),\n"
    "            target_size_bytes: self.identity.size_bytes,\n"
    "            source_interval_count: self.identity.source_interval_count,\n",
)
replace_once(
    targeted,
    "        Ok(TargetedCoverageReport {\n"
    "            summary,\n",
    "        Ok(TargetedCoverageReport {\n"
    "            identity,\n"
    "            summary,\n",
)

# Release library: construct target sets after reader/header validation, before traversal.
lib = "crates/aligngauge-cli/src/lib.rs"
replace_once(
    lib,
    "use aligngauge_coverage::{CoverageCollector, CoverageMemoryPlan, CoverageOptions, CoverageReport};\n",
    "use aligngauge_coverage::{\n"
    "    DEFAULT_NEAR_DISTANCE_BASES, CoverageCollector, CoverageMemoryPlan, CoverageOptions,\n"
    "    CoverageReport,\n"
    "};\n"
    "use aligngauge_formats::{\n"
    "    SequenceContig, SequenceDictionary, TargetNormalizationConfig, normalize_targets,\n"
    "    parse_bed_path,\n"
    "};\n",
)
replace_once(
    lib,
    "pub fn analyze_release_with_reference(\n"
    "    config: &ResolvedConfig,\n"
    "    reference: Option<&Path>,\n"
    ") -> Result<ReleaseReport, AlignGaugeError> {\n"
    "    analyze_release_with_reference_and_hook(config, reference, &mut NoopReleaseHook)\n"
    "}\n",
    "pub fn analyze_release_with_reference(\n"
    "    config: &ResolvedConfig,\n"
    "    reference: Option<&Path>,\n"
    ") -> Result<ReleaseReport, AlignGaugeError> {\n"
    "    analyze_release_with_reference_and_targets(config, reference, None, None)\n"
    "}\n\n"
    "/// Analyze a BAM or CRAM release with an optional native v0.3 target BED.\n"
    "///\n"
    "/// `near_distance_bases` defaults to 250 when targets are supplied and is rejected when\n"
    "/// targets are absent. Target parsing occurs against the already validated alignment header\n"
    "/// before record traversal begins.\n"
    "///\n"
    "/// # Errors\n"
    "/// Returns the first configuration, target, reference, reader, collector, or provenance failure.\n"
    "pub fn analyze_release_with_reference_and_targets(\n"
    "    config: &ResolvedConfig,\n"
    "    reference: Option<&Path>,\n"
    "    targets: Option<&Path>,\n"
    "    near_distance_bases: Option<u64>,\n"
    ") -> Result<ReleaseReport, AlignGaugeError> {\n"
    "    analyze_release_with_reference_targets_and_hook(\n"
    "        config,\n"
    "        reference,\n"
    "        targets,\n"
    "        near_distance_bases,\n"
    "        &mut NoopReleaseHook,\n"
    "    )\n"
    "}\n",
)
replace_once(
    lib,
    "pub fn analyze_release_with_hook(\n"
    "    config: &ResolvedConfig,\n"
    "    hook: &mut impl ReleaseHook,\n"
    ") -> Result<ReleaseReport, AlignGaugeError> {\n"
    "    analyze_release_with_reference_and_hook(config, None, hook)\n"
    "}\n",
    "pub fn analyze_release_with_hook(\n"
    "    config: &ResolvedConfig,\n"
    "    hook: &mut impl ReleaseHook,\n"
    ") -> Result<ReleaseReport, AlignGaugeError> {\n"
    "    analyze_release_with_reference_and_hook(config, None, hook)\n"
    "}\n",
)
# Existing reference+hook wrapper delegates to generalized target-aware implementation.
regex_once(
    lib,
    r"pub fn analyze_release_with_reference_and_hook\(\n    config: &ResolvedConfig,\n    reference: Option<&Path>,\n    hook: &mut impl ReleaseHook,\n\) -> Result<ReleaseReport, AlignGaugeError> \{\n",
    "pub fn analyze_release_with_reference_and_hook(\n"
    "    config: &ResolvedConfig,\n"
    "    reference: Option<&Path>,\n"
    "    hook: &mut impl ReleaseHook,\n"
    ") -> Result<ReleaseReport, AlignGaugeError> {\n"
    "    analyze_release_with_reference_targets_and_hook(config, reference, None, None, hook)\n"
    "}\n\n"
    "/// Analyze a release with explicit target settings and deterministic fault injection.\n"
    "///\n"
    "/// # Errors\n"
    "/// Returns the first configuration, target, reference, reader, injected, collector, or\n"
    "/// provenance failure.\n"
    "pub fn analyze_release_with_reference_targets_and_hook(\n"
    "    config: &ResolvedConfig,\n"
    "    reference: Option<&Path>,\n"
    "    targets: Option<&Path>,\n"
    "    near_distance_bases: Option<u64>,\n"
    "    hook: &mut impl ReleaseHook,\n"
    ") -> Result<ReleaseReport, AlignGaugeError> {\n",
)
# Generalized body: use helper rather than ordinary collector construction.
replace_once(
    lib,
    "    let mut counter_collector = CounterCollector::new(reader.header());\n"
    "    let mut coverage_collector =\n"
    "        CoverageCollector::new(reader.header(), coverage_options.thresholds, memory_plan)?;\n",
    "    let mut counter_collector = CounterCollector::new(reader.header());\n"
    "    let mut coverage_collector = release_coverage_collector(\n"
    "        reader.header(),\n"
    "        coverage_options.thresholds,\n"
    "        memory_plan,\n"
    "        targets,\n"
    "        near_distance_bases,\n"
    "    )?;\n",
)
# Add targeted metric definitions only when targeted output exists.
replace_once(
    lib,
    "    add_coverage_metric_definitions(&mut summary.metric_definitions);\n"
    "    summary.warnings.clone_from(&warnings);\n",
    "    add_coverage_metric_definitions(&mut summary.metric_definitions);\n"
    "    if coverage.targeted().is_some() {\n"
    "        add_targeted_metric_definitions(&mut summary.metric_definitions);\n"
    "    }\n"
    "    summary.warnings.clone_from(&warnings);\n",
)
# Add target-aware collector construction helper after release coverage setup.
replace_once(
    lib,
    "fn open_release_reader(\n",
    "fn release_coverage_collector(\n"
    "    header: &aligngauge_hts::ValidatedHeader,\n"
    "    thresholds: Vec<u32>,\n"
    "    memory_plan: CoverageMemoryPlan,\n"
    "    targets: Option<&Path>,\n"
    "    near_distance_bases: Option<u64>,\n"
    ") -> Result<CoverageCollector, AlignGaugeError> {\n"
    "    let Some(target_path) = targets else {\n"
    "        if near_distance_bases.is_some() {\n"
    "            return Err(AlignGaugeError::new(\n"
    "                ErrorCategory::Configuration,\n"
    "                \"near-target distance requires a target BED\",\n"
    "            ));\n"
    "        }\n"
    "        return CoverageCollector::new(header, thresholds, memory_plan);\n"
    "    };\n"
    "    let near_distance_bases = near_distance_bases.unwrap_or(DEFAULT_NEAR_DISTANCE_BASES);\n"
    "    let dictionary = SequenceDictionary::new(\n"
    "        header\n"
    "            .references()\n"
    "            .iter()\n"
    "            .map(|reference| SequenceContig {\n"
    "                name: reference.name().to_owned(),\n"
    "                length: reference.length(),\n"
    "            })\n"
    "            .collect(),\n"
    "    )?;\n"
    "    let parsed = parse_bed_path(target_path, &dictionary)?;\n"
    "    let target_set = normalize_targets(\n"
    "        parsed.clone(),\n"
    "        TargetNormalizationConfig { flank_bases: 0 },\n"
    "    )?;\n"
    "    let selected_set = normalize_targets(\n"
    "        parsed,\n"
    "        TargetNormalizationConfig {\n"
    "            flank_bases: near_distance_bases,\n"
    "        },\n"
    "    )?;\n"
    "    CoverageCollector::new_targeted(\n"
    "        header,\n"
    "        thresholds,\n"
    "        memory_plan,\n"
    "        target_set,\n"
    "        selected_set,\n"
    "        near_distance_bases,\n"
    "    )\n"
    "}\n\n"
    "fn open_release_reader(\n",
)
# Human target section.
replace_once(
    lib,
    "        for reference in self.coverage.per_reference() {\n"
    "            writeln!(\n"
    "                output,\n"
    "                \"{}\\t{}\\t{}\\t{}\\t{}\\t{}\",\n"
    "                reference.name,\n"
    "                reference.length,\n"
    "                reference.accepted_aligned_bases,\n"
    "                reference.covered_reference_bases,\n"
    "                reference.uncovered_reference_bases,\n"
    "                reference.mean_depth,\n"
    "            )\n"
    "            .expect(\"writing to String cannot fail\");\n"
    "        }\n"
    "        output\n",
    "        for reference in self.coverage.per_reference() {\n"
    "            writeln!(\n"
    "                output,\n"
    "                \"{}\\t{}\\t{}\\t{}\\t{}\\t{}\",\n"
    "                reference.name,\n"
    "                reference.length,\n"
    "                reference.accepted_aligned_bases,\n"
    "                reference.covered_reference_bases,\n"
    "                reference.uncovered_reference_bases,\n"
    "                reference.mean_depth,\n"
    "            )\n"
    "            .expect(\"writing to String cannot fail\");\n"
    "        }\n"
    "        if let Some(targeted) = self.coverage.targeted() {\n"
    "            let targeted = targeted.summary();\n"
    "            output.push_str(\"targeted\\n\");\n"
    "            writeln!(output, \"target_territory_bases\\t{}\", targeted.target_territory_bases)\n"
    "                .expect(\"writing to String cannot fail\");\n"
    "            writeln!(output, \"on_target_bases\\t{}\", targeted.on_target_bases)\n"
    "                .expect(\"writing to String cannot fail\");\n"
    "            writeln!(output, \"near_target_bases\\t{}\", targeted.near_target_bases)\n"
    "                .expect(\"writing to String cannot fail\");\n"
    "            writeln!(output, \"off_target_bases\\t{}\", targeted.off_target_bases)\n"
    "                .expect(\"writing to String cannot fail\");\n"
    "            writeln!(output, \"dropout_target_count\\t{}\", targeted.dropout_target_count)\n"
    "                .expect(\"writing to String cannot fail\");\n"
    "            render_available_string(&mut output, \"target_mean_depth\", &targeted.target_mean_depth);\n"
    "            render_available_string(&mut output, \"target_enrichment\", &targeted.target_enrichment);\n"
    "            render_available_string(\n"
    "                &mut output,\n"
    "                \"target_uniformity_penalty_80\",\n"
    "                &targeted.target_uniformity_penalty_80,\n"
    "            );\n"
    "        }\n"
    "        output\n",
)
replace_once(
    lib,
    "/// Analyze a BAM with all Milestone 4 counters.\n",
    "fn render_available_string(\n"
    "    output: &mut String,\n"
    "    name: &str,\n"
    "    value: &Availability<String>,\n"
    ") {\n"
    "    use std::fmt::Write as _;\n"
    "    match value {\n"
    "        Availability::Available(value) => {\n"
    "            writeln!(output, \"{name}\\t{value}\")\n"
    "                .expect(\"writing to String cannot fail\");\n"
    "        }\n"
    "        Availability::Unavailable { reason } => {\n"
    "            writeln!(output, \"{name}\\tunavailable:{reason}\")\n"
    "                .expect(\"writing to String cannot fail\");\n"
    "        }\n"
    "    }\n"
    "}\n\n"
    "/// Analyze a BAM with all Milestone 4 counters.\n",
)
# Targeted metric definitions.
replace_once(
    lib,
    "fn system_info() -> SystemInfo {\n",
    "fn add_targeted_metric_definitions(definitions: &mut BTreeMap<String, MetricDefinition>) {\n"
    "    for (name, description, unit) in [\n"
    "        (\"target_territory_bases\", \"Unique zero-flank normalized target union territory\", \"bases\"),\n"
    "        (\"on_target_bases\", \"Accepted aligned reference-base observations inside target territory\", \"bases\"),\n"
    "        (\"near_target_bases\", \"Accepted aligned observations inside selected but outside target territory\", \"bases\"),\n"
    "        (\"off_target_bases\", \"Accepted aligned observations outside selected territory\", \"bases\"),\n"
    "        (\"target_enrichment\", \"Native observed target fraction divided by target genome-territory fraction\", \"ratio\"),\n"
    "        (\"target_uniformity_penalty_80\", \"Native mean target depth divided by all-target-base D20\", \"ratio\"),\n"
    "    ] {\n"
    "        definitions.insert(\n"
    "            name.to_owned(),\n"
    "            MetricDefinition {\n"
    "                description: description.to_owned(),\n"
    "                unit: unit.to_owned(),\n"
    "            },\n"
    "        );\n"
    "    }\n"
    "}\n\n"
    "fn system_info() -> SystemInfo {\n",
)

# CLI parser/main surface.
main = "crates/aligngauge-cli/src/main.rs"
replace_once(
    main,
    "use aligngauge_cli::{analyze_bam, analyze_release_with_reference};\n",
    "use aligngauge_cli::{analyze_bam, analyze_release_with_reference_and_targets};\n",
)
replace_once(
    main,
    "        reference: Option<PathBuf>,\n"
    "        overrides: ConfigOverrides,\n",
    "        reference: Option<PathBuf>,\n"
    "        targets: Option<PathBuf>,\n"
    "        near_distance_bases: Option<u64>,\n"
    "        overrides: ConfigOverrides,\n",
)
replace_once(
    main,
    "            reference,\n"
    "            overrides,\n",
    "            reference,\n"
    "            targets,\n"
    "            near_distance_bases,\n"
    "            overrides,\n",
)
replace_once(
    main,
    "            let report = match analyze_release_with_reference(&config, reference.as_deref()) {\n",
    "            let report = match analyze_release_with_reference_and_targets(\n"
    "                &config,\n"
    "                reference.as_deref(),\n"
    "                targets.as_deref(),\n"
    "                near_distance_bases,\n"
    "            ) {\n",
)
replace_once(
    main,
    "    reference: Option<PathBuf>,\n"
    "    compatibility_format: Option<CompatibilityFormat>,\n",
    "    reference: Option<PathBuf>,\n"
    "    targets: Option<PathBuf>,\n"
    "    near_distance_bases: Option<u64>,\n"
    "    compatibility_format: Option<CompatibilityFormat>,\n",
)
replace_once(
    main,
    "            reference: None,\n"
    "            compatibility_format: None,\n",
    "            reference: None,\n"
    "            targets: None,\n"
    "            near_distance_bases: None,\n"
    "            compatibility_format: None,\n",
)
replace_once(
    main,
    "        Ok(CliAction::Release {\n"
    "            config_path: self.config_path,\n"
    "            reference: self.reference,\n",
    "        if self.near_distance_bases.is_some() && self.targets.is_none() {\n"
    "            return Err(usage_error(\"--near-distance requires --targets <BED>\", program));\n"
    "        }\n\n"
    "        Ok(CliAction::Release {\n"
    "            config_path: self.config_path,\n"
    "            reference: self.reference,\n"
    "            targets: self.targets,\n"
    "            near_distance_bases: self.near_distance_bases,\n",
)
replace_once(
    main,
    "            | \"--coverage-thresholds\"\n"
    "            | \"--config\"\n",
    "            | \"--coverage-thresholds\"\n"
    "            | \"--near-distance\"\n"
    "            | \"--config\"\n",
)
replace_once(
    main,
    "        Some(\"--reference\") => {\n"
    "            state.release_option_seen = true;\n"
    "            set_path_option(\n"
    "                &mut state.reference,\n"
    "                next_value(arguments, \"--reference\", program)?,\n"
    "                \"--reference\",\n"
    "                program,\n"
    "            )\n"
    "        }\n"
    "        Some(\"--targets\" | \"--profile\") => Err(unsupported_option(\n"
    "            argument.to_string_lossy(),\n"
    "            \"targeted analysis is a v0.3 feature\",\n"
    "            program,\n"
    "        )),\n",
    "        Some(\"--reference\") => {\n"
    "            state.release_option_seen = true;\n"
    "            set_path_option(\n"
    "                &mut state.reference,\n"
    "                next_value(arguments, \"--reference\", program)?,\n"
    "                \"--reference\",\n"
    "                program,\n"
    "            )\n"
    "        }\n"
    "        Some(\"--targets\") => {\n"
    "            state.release_option_seen = true;\n"
    "            set_path_option(\n"
    "                &mut state.targets,\n"
    "                next_value(arguments, \"--targets\", program)?,\n"
    "                \"--targets\",\n"
    "                program,\n"
    "            )\n"
    "        }\n"
    "        Some(\"--profile\") => Err(unsupported_option(\n"
    "            argument.to_string_lossy(),\n"
    "            \"targeted profile selection is not released; v0.3 uses aligngauge-targeted-v0.3\",\n"
    "            program,\n"
    "        )),\n",
)
replace_once(
    main,
    "        \"--coverage-thresholds\" => parse_coverage_threshold_option(state, arguments, program),\n"
    "        \"--config\" => parse_config_option(state, arguments, program),\n",
    "        \"--coverage-thresholds\" => parse_coverage_threshold_option(state, arguments, program),\n"
    "        \"--near-distance\" => {\n"
    "            let value = parse_u64(\n"
    "                next_value(arguments, \"--near-distance\", program)?,\n"
    "                \"--near-distance\",\n"
    "                program,\n"
    "            )?;\n"
    "            set_once(\n"
    "                &mut state.near_distance_bases,\n"
    "                value,\n"
    "                \"--near-distance\",\n"
    "                program,\n"
    "            )\n"
    "        }\n"
    "        \"--config\" => parse_config_option(state, arguments, program),\n",
)
replace_once(
    main,
    "fn set_path_option(\n",
    "fn parse_u64(\n"
    "    value: OsString,\n"
    "    option: &'static str,\n"
    "    program: &OsStr,\n"
    ") -> Result<u64, AlignGaugeError> {\n"
    "    let text = utf8_value(value, option, program)?;\n"
    "    text.parse::<u64>().map_err(|source| {\n"
    "        usage_error(format!(\"{option} must be a non-negative integer\"), program)\n"
    "            .with_source(source)\n"
    "    })\n"
    "}\n\n"
    "fn set_path_option(\n",
)
# Help text.
old_usage = "Usage:\\n  {0} qc --input <BAM|CRAM> --outdir <DIR> [OPTIONS]\\n\\nRequired release values:\\n  --input <PATH>                  Local BAM or CRAM input (may also come from --config)\\n  --outdir <PATH>                 New output directory (may also come from --config)\\n\\nCRAM reference integrity:\\n  --reference <FASTA>             Explicit local FASTA required for CRAM; remote lookup is disabled\\n\\nOptional values:\\n  --threads <N>                   Collector/reduction thread limit (collector is deterministic serial)\\n  --io-threads <N>                HTSlib I/O workers; 0 or 1 selects serial decoding\\n  --memory-limit <SIZE>           B, KiB, MiB, GiB, or TiB (default 4GiB)\\n  --coverage-thresholds <LIST>    Comma-separated positive depths (default 1,10,20,30)\\n  --config <PATH>                 Strict schema_version=1 config file\\n  --log-format <human|json>       Diagnostic error format\\n  --quiet                         Suppress routine completion summary\\n  --verbose                       Enable verbose mode in resolved provenance\\n  --preserve-failed-staging       Preserve clearly incomplete staging on publication failure\\n  -h, --help                      Show this help\\n\\nConfiguration precedence:\\n  built-ins < config file < documented ALIGNGAUGE_* environment < CLI\\n\\nDeferred beyond v0.2:\\n  --targets/--profile targeted (v0.3), --backend, --cuda-device\\n"
new_usage = "Usage:\\n  {0} qc --input <BAM|CRAM> --outdir <DIR> [OPTIONS]\\n\\nRequired release values:\\n  --input <PATH>                  Local BAM or CRAM input (may also come from --config)\\n  --outdir <PATH>                 New output directory (may also come from --config)\\n\\nCRAM reference integrity:\\n  --reference <FASTA>             Explicit local FASTA required for CRAM; remote lookup is disabled\\n\\nTargeted v0.3 analysis:\\n  --targets <BED>                 Local BED3-BED12 target definition; exact contig names required\\n  --near-distance <N>             Symmetric near-target distance in bases (default 250; requires --targets)\\n                                  Uses native aligngauge-targeted-v0.3 semantics; no Picard compatibility claim\\n\\nOptional values:\\n  --threads <N>                   Collector/reduction thread limit (collector is deterministic serial)\\n  --io-threads <N>                HTSlib I/O workers; 0 or 1 selects serial decoding\\n  --memory-limit <SIZE>           B, KiB, MiB, GiB, or TiB (default 4GiB)\\n  --coverage-thresholds <LIST>    Comma-separated positive depths (default 1,10,20,30)\\n  --config <PATH>                 Strict schema_version=1 config file\\n  --log-format <human|json>       Diagnostic error format\\n  --quiet                         Suppress routine completion summary\\n  --verbose                       Enable verbose mode in resolved provenance\\n  --preserve-failed-staging       Preserve clearly incomplete staging on publication failure\\n  -h, --help                      Show this help\\n\\nConfiguration precedence:\\n  built-ins < config file < documented ALIGNGAUGE_* environment < CLI\\n\\nDeferred beyond v0.3:\\n  --profile selection, --backend, --cuda-device\\n"
replace_once(main, old_usage, new_usage)

# CLI option tests: targets now released, near-distance contract and publication oracle.
tests = "crates/aligngauge-cli/tests/release_cli_options.rs"
replace_once(
    tests,
    '        "--targets",\n        "--backend",',
    '        "--targets",\n        "--near-distance",\n        "--backend",',
)
replace_once(
    tests,
    '    assert!(stdout.contains("v0.2"));\n    assert!(stdout.contains("v0.3"));\n',
    '    assert!(stdout.contains("v0.3"));\n    assert!(stdout.contains("default 250"));\n    assert!(stdout.contains("no Picard compatibility claim"));\n',
)
replace_once(
    tests,
    "    for (option, marker) in [\n"
    "        (\"--targets\", \"v0.3\"),\n"
    "        (\"--profile\", \"v0.3\"),\n",
    "    for (option, marker) in [\n"
    "        (\"--profile\", \"not released\"),\n",
)
append = r'''

#[test]
fn targeted_release_options_publish_native_metrics_and_preserve_one_traversal() {
    let input = fixture("chunk_boundary.bam");
    let targets = repository_root()
        .join("crates/aligngauge-coverage/tests/fixtures/chunk_boundary_targets.bed");
    let outdir = temp_path("targeted-release");
    let output = Command::new(binary())
        .args(["qc", "--input"])
        .arg(&input)
        .args(["--outdir"])
        .arg(&outdir)
        .args(["--targets"])
        .arg(&targets)
        .args([
            "--near-distance",
            "5",
            "--coverage-thresholds",
            "1,2",
            "--memory-limit",
            "1GiB",
        ])
        .output()
        .expect("run targeted release");
    assert!(output.status.success(), "{}", utf8(&output.stderr));
    let stdout = utf8(&output.stdout);
    assert!(stdout.contains("targeted\n"));
    assert!(stdout.contains("on_target_bases\t16"));
    assert!(stdout.contains("near_target_bases\t12"));
    assert!(stdout.contains("off_target_bases\t0"));

    let summary = fs::read_to_string(outdir.join("summary.json")).expect("read summary");
    assert!(summary.contains("\"schema_version\": \"1.1.0\""));
    assert!(summary.contains("\"profile\": \"aligngauge-targeted-v0.3\""));
    assert!(summary.contains("\"target_territory_bases\": 22"));
    assert!(summary.contains("\"on_target_bases\": 16"));
    assert!(summary.contains("\"near_target_bases\": 12"));
    assert!(summary.contains("\"off_target_bases\": 0"));
    assert!(summary.contains("\"dropout_target_count\": 1"));

    let provenance = fs::read_to_string(outdir.join("provenance.json")).expect("read provenance");
    assert!(provenance.contains("\"alignment_traversals\": 1"));
    assert!(provenance.contains("\"near_distance_bases\": 5"));
    assert!(provenance.contains("\"targeted_profile\": \"aligngauge-targeted-v0.3\""));
    assert!(provenance.contains(&targets.to_string_lossy().replace('\\', "\\\\")));
    fs::remove_dir_all(outdir).expect("cleanup targeted output");
}

#[test]
fn near_distance_without_targets_fails_before_output_creation() {
    let input = fixture("basic.bam");
    let outdir = temp_path("near-without-targets");
    let output = Command::new(binary())
        .args(["qc", "--input"])
        .arg(&input)
        .args(["--outdir"])
        .arg(&outdir)
        .args(["--near-distance", "5"])
        .output()
        .expect("run invalid near-distance release");
    assert!(!output.status.success());
    assert!(utf8(&output.stderr).contains("requires --targets"));
    assert!(!outdir.exists());
}

#[test]
fn missing_target_bed_fails_closed_without_publishing() {
    let input = fixture("basic.bam");
    let outdir = temp_path("missing-target");
    let missing = temp_path("missing-target-bed");
    let output = Command::new(binary())
        .args(["qc", "--input"])
        .arg(&input)
        .args(["--outdir"])
        .arg(&outdir)
        .args(["--targets"])
        .arg(&missing)
        .output()
        .expect("run missing target release");
    assert!(!output.status.success());
    assert!(utf8(&output.stderr).contains("input_not_found"));
    assert!(!outdir.exists());
}
'''
file = Path(tests)
file.write_text(file.read_text() + append)

print("Milestone 9 CLI/release integration edits applied")
