//! Validated text formats and deterministic target normalization for `AlignGauge`.

pub mod bed;

pub use bed::{
    BED_NORMALIZATION_PROFILE, BedParseResult, BedParseStats, BedSourceInterval,
    MergedTargetInterval, SequenceContig, SequenceDictionary, TargetFileIdentity,
    TargetNormalizationConfig, TargetNormalizationProvenance, TargetSet, normalize_targets,
    parse_bed_bytes, parse_bed_path,
};
