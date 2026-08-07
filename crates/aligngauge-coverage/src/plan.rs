//! Fail-closed coverage options and memory planning.

use std::collections::BTreeMap;

use aligngauge_core::{AlignGaugeError, JsonValue};

use crate::util::{configuration_error, delta_bytes, resource_error, u64_from_usize};

const DEFAULT_MEMORY_LIMIT_BYTES: u64 = 4_u64 << 30;
const DEFAULT_THRESHOLDS: [u32; 4] = [1, 10, 20, 30];
const MIN_CHUNK_BASES: usize = 1_024;
const MAX_CHUNK_BASES: usize = 65_536;
const DELTA_ENTRY_BYTES: u64 = 16;
const PENDING_EVENT_ENTRY_BYTES: u64 = 64;
const HISTOGRAM_ENTRY_BYTES: u64 = 64;
const READER_BUFFER_BYTES: u64 = 320_u64 << 20;
const OUTPUT_BUFFER_BYTES: u64 = 8_u64 << 20;
const REDUCTION_STATE_BYTES: u64 = 4_u64 << 20;
const SAFETY_MARGIN_BYTES: u64 = 128_u64 << 20;
const PENDING_EVENT_BUDGET_PER_TRACK: u64 = 32_u64 << 20;
const HISTOGRAM_BUDGET_PER_TRACK: u64 = 16_u64 << 20;

/// Runtime choices for one canonical coverage collection.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CoverageOptions {
    /// Hard planning limit in bytes.
    pub memory_limit_bytes: u64,
    /// Sorted unique positive cumulative depth thresholds.
    pub thresholds: Vec<u32>,
    /// Optional explicit chunk size used by differential/property tests.
    pub chunk_size_override: Option<usize>,
}

impl Default for CoverageOptions {
    fn default() -> Self {
        Self {
            memory_limit_bytes: DEFAULT_MEMORY_LIMIT_BYTES,
            thresholds: DEFAULT_THRESHOLDS.to_vec(),
            chunk_size_override: None,
        }
    }
}

impl CoverageOptions {
    /// Construct validated options.
    ///
    /// # Errors
    /// Returns `configuration` for zero memory, zero thresholds, or an empty threshold set.
    pub fn new(memory_limit_bytes: u64, mut thresholds: Vec<u32>) -> Result<Self, AlignGaugeError> {
        if memory_limit_bytes == 0 {
            return Err(configuration_error(
                "coverage memory limit must be greater than zero",
            ));
        }
        if thresholds.is_empty() {
            return Err(configuration_error(
                "at least one coverage threshold is required",
            ));
        }
        if thresholds.contains(&0) {
            return Err(configuration_error(
                "coverage thresholds must be greater than zero",
            ));
        }
        thresholds.sort_unstable();
        thresholds.dedup();
        Ok(Self {
            memory_limit_bytes,
            thresholds,
            chunk_size_override: None,
        })
    }

    /// Override the planner-selected chunk size while retaining the same algorithm.
    ///
    /// This exists for exactness/property validation. Production callers should leave the
    /// override unset so the memory planner selects the chunk size.
    #[must_use]
    pub fn with_chunk_size_override(mut self, chunk_size_bases: usize) -> Self {
        self.chunk_size_override = Some(chunk_size_bases);
        self
    }
}

/// Fail-closed memory plan for the chunked coverage accumulator.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CoverageMemoryPlan {
    /// Caller-supplied hard memory limit.
    pub memory_limit_bytes: u64,
    /// Logical coverage tracks accounted by this plan.
    pub active_tracks: usize,
    /// Selected chunk size.
    pub chunk_size_bases: usize,
    /// Delta-vector bytes reserved for all tracks.
    pub delta_bytes: u64,
    /// Pending cross-chunk event budget.
    pub pending_event_budget_bytes: u64,
    /// Maximum unique pending event positions permitted.
    pub max_pending_event_positions: usize,
    /// Histogram/reduction map budget.
    pub histogram_budget_bytes: u64,
    /// Maximum histogram bins permitted.
    pub max_histogram_bins: usize,
    /// Reader-buffer budget.
    pub reader_buffer_bytes: u64,
    /// Output-buffer budget.
    pub output_buffer_bytes: u64,
    /// Fixed reduction-state budget.
    pub reduction_state_bytes: u64,
    /// Explicit safety margin.
    pub safety_margin_bytes: u64,
    /// Sum of all planned components.
    pub planned_peak_bytes: u64,
}

#[derive(Debug, Clone, Copy)]
struct FixedPlan {
    pending_event_budget_bytes: u64,
    histogram_budget_bytes: u64,
    fixed_bytes: u64,
    bytes_per_chunk_base: u64,
}

impl CoverageMemoryPlan {
    /// Build a memory plan before BAM traversal.
    ///
    /// # Errors
    /// Returns `resource_limit` when the requested track count or memory limit cannot support the
    /// minimum exact chunked algorithm.
    pub fn plan(
        memory_limit_bytes: u64,
        active_tracks: usize,
        chunk_size_override: Option<usize>,
    ) -> Result<Self, AlignGaugeError> {
        validate_plan_inputs(memory_limit_bytes, active_tracks)?;
        let fixed = fixed_plan(active_tracks)?;
        let maximum_by_memory = maximum_chunk_by_memory(memory_limit_bytes, fixed)?;
        let chunk_size_bases = select_chunk_size(maximum_by_memory, chunk_size_override)?;
        let delta_bytes = delta_bytes(chunk_size_bases, fixed.bytes_per_chunk_base)?;
        let planned_peak_bytes = fixed
            .fixed_bytes
            .checked_add(delta_bytes)
            .ok_or_else(|| resource_error("coverage planned peak overflowed"))?;
        if planned_peak_bytes > memory_limit_bytes {
            return Err(resource_error("coverage plan exceeds the memory limit"));
        }

        Ok(Self {
            memory_limit_bytes,
            active_tracks,
            chunk_size_bases,
            delta_bytes,
            pending_event_budget_bytes: fixed.pending_event_budget_bytes,
            max_pending_event_positions: capacity(
                fixed.pending_event_budget_bytes,
                PENDING_EVENT_ENTRY_BYTES,
                "pending-event",
            )?,
            histogram_budget_bytes: fixed.histogram_budget_bytes,
            max_histogram_bins: capacity(
                fixed.histogram_budget_bytes,
                HISTOGRAM_ENTRY_BYTES,
                "histogram",
            )?,
            reader_buffer_bytes: READER_BUFFER_BYTES,
            output_buffer_bytes: OUTPUT_BUFFER_BYTES,
            reduction_state_bytes: REDUCTION_STATE_BYTES,
            safety_margin_bytes: SAFETY_MARGIN_BYTES,
            planned_peak_bytes,
        })
    }

    /// Stable deterministic JSON representation used by provenance and RSS validation.
    ///
    /// # Errors
    /// Returns `internal_invariant` if a platform-sized planned value cannot be represented as
    /// the canonical unsigned JSON integer type.
    pub fn to_json(&self) -> Result<JsonValue, AlignGaugeError> {
        Ok(JsonValue::Object(BTreeMap::from([
            (
                String::from("active_tracks"),
                JsonValue::Unsigned(u64_from_usize(self.active_tracks, "active tracks")?),
            ),
            (
                String::from("chunk_size_bases"),
                JsonValue::Unsigned(u64_from_usize(self.chunk_size_bases, "chunk size")?),
            ),
            (String::from("delta_bytes"), self.delta_bytes.into()),
            (
                String::from("histogram_budget_bytes"),
                self.histogram_budget_bytes.into(),
            ),
            (
                String::from("max_histogram_bins"),
                JsonValue::Unsigned(u64_from_usize(self.max_histogram_bins, "histogram bins")?),
            ),
            (
                String::from("max_pending_event_positions"),
                JsonValue::Unsigned(u64_from_usize(
                    self.max_pending_event_positions,
                    "pending event positions",
                )?),
            ),
            (
                String::from("memory_limit_bytes"),
                self.memory_limit_bytes.into(),
            ),
            (
                String::from("output_buffer_bytes"),
                self.output_buffer_bytes.into(),
            ),
            (
                String::from("pending_event_budget_bytes"),
                self.pending_event_budget_bytes.into(),
            ),
            (
                String::from("planned_peak_bytes"),
                self.planned_peak_bytes.into(),
            ),
            (
                String::from("reader_buffer_bytes"),
                self.reader_buffer_bytes.into(),
            ),
            (
                String::from("reduction_state_bytes"),
                self.reduction_state_bytes.into(),
            ),
            (
                String::from("safety_margin_bytes"),
                self.safety_margin_bytes.into(),
            ),
        ])))
    }
}

fn validate_plan_inputs(memory_limit_bytes: u64, active_tracks: usize) -> Result<(), AlignGaugeError> {
    if active_tracks == 0 {
        return Err(resource_error(
            "coverage requires at least one active track",
        ));
    }
    if memory_limit_bytes == 0 {
        return Err(resource_error(
            "coverage memory limit must be greater than zero",
        ));
    }
    Ok(())
}

fn fixed_plan(active_tracks: usize) -> Result<FixedPlan, AlignGaugeError> {
    let tracks = u64_from_usize(active_tracks, "active coverage tracks")?;
    let pending_event_budget_bytes = PENDING_EVENT_BUDGET_PER_TRACK
        .checked_mul(tracks)
        .ok_or_else(|| resource_error("pending-event budget overflowed"))?;
    let histogram_budget_bytes = HISTOGRAM_BUDGET_PER_TRACK
        .checked_mul(tracks)
        .ok_or_else(|| resource_error("histogram budget overflowed"))?;
    let fixed_bytes = [
        READER_BUFFER_BYTES,
        OUTPUT_BUFFER_BYTES,
        REDUCTION_STATE_BYTES,
        SAFETY_MARGIN_BYTES,
        pending_event_budget_bytes,
        histogram_budget_bytes,
    ]
    .into_iter()
    .try_fold(0_u64, |total, value| {
        total
            .checked_add(value)
            .ok_or_else(|| resource_error("coverage fixed-memory plan overflowed"))
    })?;
    let bytes_per_chunk_base = DELTA_ENTRY_BYTES
        .checked_mul(tracks)
        .ok_or_else(|| resource_error("coverage delta width overflowed"))?;
    Ok(FixedPlan {
        pending_event_budget_bytes,
        histogram_budget_bytes,
        fixed_bytes,
        bytes_per_chunk_base,
    })
}

fn maximum_chunk_by_memory(
    memory_limit_bytes: u64,
    fixed: FixedPlan,
) -> Result<usize, AlignGaugeError> {
    let minimum_delta_bytes = delta_bytes(MIN_CHUNK_BASES, fixed.bytes_per_chunk_base)?;
    let minimum_planned = fixed
        .fixed_bytes
        .checked_add(minimum_delta_bytes)
        .ok_or_else(|| resource_error("coverage minimum-memory plan overflowed"))?;
    if minimum_planned > memory_limit_bytes {
        return Err(resource_error(
            "memory limit cannot support the minimum exact coverage plan",
        )
        .with_detail("memory_limit_bytes", memory_limit_bytes)
        .with_detail("minimum_required_bytes", minimum_planned));
    }
    let available_for_delta = memory_limit_bytes - fixed.fixed_bytes;
    let maximum = available_for_delta
        .checked_div(fixed.bytes_per_chunk_base)
        .and_then(|entries| entries.checked_sub(1))
        .ok_or_else(|| resource_error("memory limit cannot hold a coverage delta vector"))?;
    usize::try_from(maximum).map_err(|source| {
        resource_error("coverage chunk capacity does not fit usize").with_source(source)
    })
}

fn select_chunk_size(
    maximum_by_memory: usize,
    chunk_size_override: Option<usize>,
) -> Result<usize, AlignGaugeError> {
    match chunk_size_override {
        Some(0) => Err(resource_error(
            "coverage chunk size must be greater than zero",
        )),
        Some(requested) if requested > maximum_by_memory => Err(resource_error(
            "requested coverage chunk size exceeds the memory plan",
        )
        .with_detail(
            "requested_chunk_size_bases",
            u64_from_usize(requested, "requested chunk size")?,
        )
        .with_detail(
            "maximum_chunk_size_bases",
            u64_from_usize(maximum_by_memory, "maximum chunk size")?,
        )),
        Some(requested) => Ok(requested),
        None => Ok(maximum_by_memory.clamp(MIN_CHUNK_BASES, MAX_CHUNK_BASES)),
    }
}

fn capacity(budget_bytes: u64, entry_bytes: u64, label: &'static str) -> Result<usize, AlignGaugeError> {
    usize::try_from(budget_bytes / entry_bytes).map_err(|source| {
        resource_error(format!("{label} capacity does not fit usize")).with_source(source)
    })
}
