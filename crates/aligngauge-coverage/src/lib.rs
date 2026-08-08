//! Exact, memory-planned v0.1 coverage accumulation.

use std::path::Path;

use aligngauge_core::AlignGaugeError;
use aligngauge_formats::{
    SequenceContig, SequenceDictionary, TargetNormalizationConfig, normalize_targets,
    parse_bed_path,
};
use aligngauge_hts::{BamReader, FieldPlan, ReaderOptions};

mod accumulator;
mod cigar;
mod overlap;
mod plan;
mod report;
mod targeted;
mod util;

pub use accumulator::CoverageCollector;
pub use cigar::{CoverageBlock, cigar_to_coverage_blocks};
pub use overlap::{
    EXACT_OVERLAP_EXECUTION_MODE, INDEXED_PARTITION_EXACT_OVERLAP_SUPPORTED,
    PICARD_HS_OVERLAP_PROFILE, PICARD_WGS_LOCUS_ACCUMULATION_CAP,
    PICARD_WGS_MINIMUM_BASE_QUALITY, PICARD_WGS_OVERLAP_PROFILE, PicardWgsOverlapCorrector,
    PicardWgsOverlapRecord, PicardWgsOverlapSummary, picard_hs_trailing_read_bases_to_clip,
    picard_wgs_flag_candidate,
};
pub use plan::{CoverageMemoryPlan, CoverageOptions};
pub use report::{CoverageReport, PerReferenceCoverage};
pub use targeted::{DEFAULT_NEAR_DISTANCE_BASES, TARGETED_PROFILE, TargetedCoverageReport};

/// Stable canonical coverage profile name.
pub const COVERAGE_PROFILE: &str = "aligngauge-v0.1";
/// One exact implementation shared by every chunk size.
pub const COVERAGE_STRATEGY: &str = "parameterized-chunked-delta-v1";

/// Analyze one local BAM with the canonical v0.1 coverage profile.
///
/// The memory plan is computed before the reader is opened, so an impossible plan fails before
/// BAM traversal.
///
/// # Errors
/// Returns typed configuration, resource, reader-validation, or checked-arithmetic failures.
pub fn analyze_bam(
    path: impl AsRef<Path>,
    options: CoverageOptions,
) -> Result<CoverageReport, AlignGaugeError> {
    let plan =
        CoverageMemoryPlan::plan(options.memory_limit_bytes, 1, options.chunk_size_override)?;
    let mut reader = BamReader::open(path, FieldPlan::coverage(), ReaderOptions::default())?;
    let mut collector = CoverageCollector::new(reader.header(), options.thresholds, plan)?;
    while let Some(record) = reader.next_record()? {
        collector.observe(&record)?;
    }
    collector.finish()
}

/// Analyze one local BAM with canonical whole-genome and native targeted reductions.
///
/// This convenience entry point is primarily useful for exact coverage differential tests.
/// Production release orchestration feeds the same targeted collector from its shared reader.
///
/// # Errors
/// Returns typed target, resource, reader-validation, or checked-arithmetic failures.
pub fn analyze_bam_with_targets(
    path: impl AsRef<Path>,
    targets: impl AsRef<Path>,
    near_distance_bases: u64,
    options: CoverageOptions,
) -> Result<CoverageReport, AlignGaugeError> {
    let plan =
        CoverageMemoryPlan::plan(options.memory_limit_bytes, 1, options.chunk_size_override)?;
    let mut reader = BamReader::open(path, FieldPlan::coverage(), ReaderOptions::default())?;
    let dictionary = SequenceDictionary::new(
        reader
            .header()
            .references()
            .iter()
            .map(|reference| SequenceContig {
                name: reference.name().to_owned(),
                length: reference.length(),
            })
            .collect(),
    )?;
    let parsed = parse_bed_path(targets.as_ref(), &dictionary)?;
    let target_set =
        normalize_targets(parsed.clone(), TargetNormalizationConfig { flank_bases: 0 })?;
    let selected_set = normalize_targets(
        parsed,
        TargetNormalizationConfig {
            flank_bases: near_distance_bases,
        },
    )?;
    let mut collector = CoverageCollector::new_targeted(
        reader.header(),
        options.thresholds,
        plan,
        target_set,
        selected_set,
        near_distance_bases,
    )?;
    while let Some(record) = reader.next_record()? {
        collector.observe(&record)?;
    }
    collector.finish()
}

#[cfg(test)]
mod tests;
