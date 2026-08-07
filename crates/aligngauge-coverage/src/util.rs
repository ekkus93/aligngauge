//! Checked arithmetic, deterministic decimal rendering, and typed coverage errors.

use aligngauge_core::{AlignGaugeError, ErrorCategory};

use crate::plan::CoverageMemoryPlan;

pub(crate) fn apply_depth_change(depth: u64, change: i128) -> Result<u64, AlignGaugeError> {
    let next = i128::from(depth)
        .checked_add(change)
        .ok_or_else(|| coverage_overflow("coverage depth"))?;
    if next < 0 || next > i128::from(u64::MAX) {
        return Err(internal_error(
            "coverage depth event would leave the u64 range",
        ));
    }
    u64::try_from(next)
        .map_err(|source| internal_error("coverage depth conversion failed").with_source(source))
}

pub(crate) fn format_ratio_six(
    numerator: u64,
    denominator: u64,
) -> Result<String, AlignGaugeError> {
    if denominator == 0 {
        return Ok(String::from("0.000000"));
    }
    const SCALE: u128 = 1_000_000;
    let scaled = u128::from(numerator)
        .checked_mul(SCALE)
        .ok_or_else(|| coverage_overflow("decimal ratio numerator"))?;
    let rounded = scaled
        .checked_add(u128::from(denominator) / 2)
        .ok_or_else(|| coverage_overflow("decimal ratio rounding"))?
        / u128::from(denominator);
    let whole = rounded / SCALE;
    let fraction = rounded % SCALE;
    Ok(format!("{whole}.{fraction:06}"))
}

pub(crate) fn format_percentage_six(
    numerator: u64,
    denominator: u64,
) -> Result<String, AlignGaugeError> {
    if denominator == 0 {
        return Ok(String::from("0.000000"));
    }
    const SCALE: u128 = 100_000_000;
    let scaled = u128::from(numerator)
        .checked_mul(SCALE)
        .ok_or_else(|| coverage_overflow("percentage numerator"))?;
    let rounded = scaled
        .checked_add(u128::from(denominator) / 2)
        .ok_or_else(|| coverage_overflow("percentage rounding"))?
        / u128::from(denominator);
    let whole = rounded / 1_000_000;
    let fraction = rounded % 1_000_000;
    Ok(format!("{whole}.{fraction:06}"))
}

pub(crate) fn delta_bytes(
    chunk_size: usize,
    bytes_per_chunk_base: u64,
) -> Result<u64, AlignGaugeError> {
    let entries = chunk_size
        .checked_add(1)
        .ok_or_else(|| resource_error("coverage chunk entry count overflowed"))?;
    u64_from_usize(entries, "coverage delta entries")?
        .checked_mul(bytes_per_chunk_base)
        .ok_or_else(|| resource_error("coverage delta byte count overflowed"))
}

pub(crate) fn chunk_size_u64(plan: &CoverageMemoryPlan) -> Result<u64, AlignGaugeError> {
    u64_from_usize(plan.chunk_size_bases, "coverage chunk size")
}

pub(crate) fn u64_from_usize(value: usize, label: &'static str) -> Result<u64, AlignGaugeError> {
    u64::try_from(value)
        .map_err(|source| internal_error(format!("{label} does not fit u64")).with_source(source))
}

pub(crate) fn configuration_error(message: impl Into<String>) -> AlignGaugeError {
    AlignGaugeError::new(ErrorCategory::Configuration, message)
}

pub(crate) fn resource_error(message: impl Into<String>) -> AlignGaugeError {
    AlignGaugeError::new(ErrorCategory::ResourceLimit, message)
}

pub(crate) fn input_error(message: impl Into<String>) -> AlignGaugeError {
    AlignGaugeError::new(ErrorCategory::InputCorrupt, message)
}

pub(crate) fn internal_error(message: impl Into<String>) -> AlignGaugeError {
    AlignGaugeError::new(ErrorCategory::InternalInvariant, message)
}

pub(crate) fn coverage_overflow(label: &'static str) -> AlignGaugeError {
    internal_error(format!(
        "coverage arithmetic overflowed while updating {label}"
    ))
    .with_detail("operation", label)
}
