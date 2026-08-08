//! Exact overlap-correction primitives for the pinned Picard 3.4.0 profiles.
//!
//! Picard's WGS and hybrid-selection collectors do not use the same overlap algorithm.
//! This module keeps those semantics separate and fail-closed rather than exposing a vague
//! boolean "clip overlaps" switch.

use std::collections::{BTreeMap, BTreeSet};

use aligngauge_core::{AlignGaugeError, ErrorCategory};

/// Pinned Picard WGS overlap policy implemented by [`PicardWgsOverlapCorrector`].
pub const PICARD_WGS_OVERLAP_PROFILE: &str = "picard-wgs-3.4.0-default-overlap-v1";
/// Pinned Picard HsMetrics overlap policy implemented by [`picard_hs_trailing_read_bases_to_clip`].
pub const PICARD_HS_OVERLAP_PROFILE: &str = "picard-hs-3.4.0-default-overlap-v1";
/// The only execution mode in which exact overlap correction is released for Milestone 13.
pub const EXACT_OVERLAP_EXECUTION_MODE: &str = "streaming-coordinate-order-v1";
/// Indexed reference-partition execution is deliberately unavailable with exact overlap correction.
pub const INDEXED_PARTITION_EXACT_OVERLAP_SUPPORTED: bool = false;
/// Picard 3.4.0 default minimum base quality for `CollectWgsMetrics`.
pub const PICARD_WGS_MINIMUM_BASE_QUALITY: u8 = 20;
/// Picard 3.4.0 default `LOCUS_ACCUMULATION_CAP`.
pub const PICARD_WGS_LOCUS_ACCUMULATION_CAP: u32 = 100_000;

const FLAG_PAIRED: u16 = 0x1;
const FLAG_UNMAPPED: u16 = 0x4;
const FLAG_MATE_UNMAPPED: u16 = 0x8;
const FLAG_READ1: u16 = 0x40;
const FLAG_SECONDARY: u16 = 0x100;

// Conservative deterministic accounting. These are planning charges, not claims about allocator
// internals. The state is rejected before a charge would cross the caller-reserved budget.
const ACTIVE_NAME_OVERHEAD_BYTES: u64 = 128;
const ACTIVE_POSITION_BYTES: u64 = 32;
const ACTIVE_LOCUS_BYTES: u64 = 48;

/// One already record-filtered WGS alignment presented to the exact overlap stage.
///
/// Record-level Picard filters (PF, secondary, adapter, MAPQ, duplicate, and pairing filters) are
/// intentionally outside this type. The overlap stage reproduces the ordering that follows those
/// filters: base-quality/no-call filtering first, then per-locus raw-query-name de-duplication.
#[derive(Debug, Clone, Copy)]
pub struct PicardWgsOverlapRecord<'a> {
    /// Zero-based reference index from the validated coordinate-sorted stream.
    pub reference_id: i32,
    /// Zero-based alignment start.
    pub start: u64,
    /// Declared reference length used for checked CIGAR traversal.
    pub reference_length: u64,
    /// Raw query-name bytes. No UTF-8 normalization or lossy conversion is allowed.
    pub query_name: &'a [u8],
    /// Raw BAM CIGAR words (`length << 4 | operation`).
    pub raw_cigar: &'a [u32],
    /// Decoded query bases corresponding to the CIGAR query span.
    pub sequence: &'a [u8],
    /// Raw Phred qualities corresponding one-for-one with `sequence`.
    pub qualities: &'a [u8],
}

/// Per-record overlap-stage result before downstream coverage reduction.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct PicardWgsOverlapSummary {
    /// High-quality observations retained after exact read-name de-duplication.
    pub retained_bases: u64,
    /// Observations excluded by Picard's minimum-base-quality/no-call gate.
    pub baseq_excluded_bases: u64,
    /// High-quality observations excluded because the same query name already contributed at the locus.
    pub overlap_excluded_bases: u64,
}

#[derive(Debug)]
struct ActiveNameState {
    reference_id: i32,
    eligible_positions: BTreeSet<u64>,
    last_eligible_end: u64,
    accounted_bytes: u64,
}

/// Bounded streaming state for Picard 3.4.0 `CollectWgsMetrics` overlap semantics.
///
/// The input record stream must be coordinate sorted. State for a raw query name is retained only
/// while an earlier high-quality observation can overlap a later record. Once the current record
/// starts at or beyond the state's final eligible position, the state is evicted. Exceeding either
/// the pinned Picard locus-observation cap or the caller-reserved state budget is fatal; the
/// implementation never truncates state or silently changes to approximate overlap handling.
#[derive(Debug)]
pub struct PicardWgsOverlapCorrector {
    state_budget_bytes: u64,
    state_bytes: u64,
    active_names: BTreeMap<Vec<u8>, ActiveNameState>,
    active_locus_observations: BTreeMap<u64, u32>,
    current_reference_id: Option<i32>,
    previous_record_start: Option<u64>,
    locus_accumulation_cap: u32,
}

impl PicardWgsOverlapCorrector {
    /// Construct exact WGS overlap state within an explicitly reserved memory budget.
    ///
    /// # Errors
    /// Returns `resource_limit` when `state_budget_bytes` is zero.
    pub fn new(state_budget_bytes: u64) -> Result<Self, AlignGaugeError> {
        if state_budget_bytes == 0 {
            return Err(overlap_resource_error(
                "Picard WGS overlap state budget must be greater than zero",
            ));
        }
        Ok(Self {
            state_budget_bytes,
            state_bytes: 0,
            active_names: BTreeMap::new(),
            active_locus_observations: BTreeMap::new(),
            current_reference_id: None,
            previous_record_start: None,
            locus_accumulation_cap: PICARD_WGS_LOCUS_ACCUMULATION_CAP,
        })
    }

    /// Current deterministic state charge in bytes.
    #[must_use]
    pub const fn state_bytes(&self) -> u64 {
        self.state_bytes
    }

    /// Caller-reserved hard state budget in bytes.
    #[must_use]
    pub const fn state_budget_bytes(&self) -> u64 {
        self.state_budget_bytes
    }

    /// Observe one record and report each retained high-quality reference position exactly once.
    ///
    /// The caller must supply records after the pinned Picard WGS record-level filters. The
    /// callback is invoked only for observations that survive base-quality/no-call filtering and
    /// raw-query-name overlap correction.
    ///
    /// # Errors
    /// Returns a typed fatal error for coordinate regression, malformed CIGAR/query layout,
    /// checked arithmetic failure, the pinned locus accumulation bound, the state memory bound,
    /// or a callback failure.
    pub fn observe_record(
        &mut self,
        record: PicardWgsOverlapRecord<'_>,
        mut retain_position: impl FnMut(u64) -> Result<(), AlignGaugeError>,
    ) -> Result<PicardWgsOverlapSummary, AlignGaugeError> {
        if record.reference_id < 0 {
            return Err(overlap_input_error(
                "Picard WGS overlap record has a negative reference ID",
            ));
        }
        if record.start > record.reference_length {
            return Err(overlap_input_error(
                "Picard WGS overlap record starts outside the declared reference",
            ));
        }
        if record.sequence.len() != record.qualities.len() {
            return Err(overlap_input_error(
                "Picard WGS overlap sequence and quality lengths differ",
            ));
        }
        if record.query_name.is_empty() {
            return Err(overlap_input_error(
                "Picard WGS overlap record has an empty query name",
            ));
        }

        self.prepare_record(record.reference_id, record.start)?;

        let mut summary = PicardWgsOverlapSummary::default();
        let mut reference_cursor = record.start;
        let mut query_cursor = 0_usize;

        for encoded in record.raw_cigar {
            let length_u32 = encoded >> 4;
            let operation = encoded & 0x0f;
            if length_u32 == 0 {
                return Err(overlap_input_error(
                    "Picard WGS overlap CIGAR contains a zero-length operation",
                ));
            }
            let length = usize::try_from(length_u32).map_err(|source| {
                overlap_input_error("Picard WGS overlap CIGAR length does not fit usize")
                    .with_source(source)
            })?;
            let reference_length = u64::from(length_u32);

            match operation {
                0 | 7 | 8 => {
                    let query_end = query_cursor.checked_add(length).ok_or_else(|| {
                        overlap_input_error("Picard WGS overlap query cursor overflowed")
                    })?;
                    if query_end > record.sequence.len() {
                        return Err(overlap_input_error(
                            "Picard WGS overlap CIGAR consumes beyond the query sequence",
                        ));
                    }
                    let reference_end = checked_reference_end(
                        reference_cursor,
                        reference_length,
                        record.reference_length,
                    )?;
                    for offset in 0..length {
                        let offset_u64 = u64::try_from(offset).map_err(|source| {
                            overlap_input_error("Picard WGS overlap base offset does not fit u64")
                                .with_source(source)
                        })?;
                        let position = reference_cursor.checked_add(offset_u64).ok_or_else(|| {
                            overlap_input_error("Picard WGS overlap base coordinate overflowed")
                        })?;
                        self.observe_locus_candidate(position)?;
                        let query_index = query_cursor.checked_add(offset).ok_or_else(|| {
                            overlap_input_error("Picard WGS overlap query index overflowed")
                        })?;
                        let quality = record.qualities[query_index];
                        let base = record.sequence[query_index];
                        if quality < PICARD_WGS_MINIMUM_BASE_QUALITY || is_no_call(base) {
                            summary.baseq_excluded_bases = checked_increment(
                                summary.baseq_excluded_bases,
                                "Picard WGS base-quality exclusions",
                            )?;
                            continue;
                        }
                        if self.position_was_seen(record.query_name, record.reference_id, position)? {
                            summary.overlap_excluded_bases = checked_increment(
                                summary.overlap_excluded_bases,
                                "Picard WGS overlap exclusions",
                            )?;
                            continue;
                        }
                        retain_position(position)?;
                        summary.retained_bases = checked_increment(
                            summary.retained_bases,
                            "Picard WGS retained bases",
                        )?;
                    }
                    query_cursor = query_end;
                    reference_cursor = reference_end;
                }
                1 | 4 => {
                    query_cursor = query_cursor.checked_add(length).ok_or_else(|| {
                        overlap_input_error("Picard WGS overlap query cursor overflowed")
                    })?;
                    if query_cursor > record.sequence.len() {
                        return Err(overlap_input_error(
                            "Picard WGS overlap CIGAR consumes beyond the query sequence",
                        ));
                    }
                }
                2 | 3 => {
                    reference_cursor = checked_reference_end(
                        reference_cursor,
                        reference_length,
                        record.reference_length,
                    )?;
                }
                5 | 6 => {}
                _ => {
                    return Err(overlap_input_error(
                        "Picard WGS overlap CIGAR contains an unknown operation code",
                    )
                    .with_detail("cigar_operation_code", u64::from(operation)));
                }
            }
        }

        if query_cursor != record.sequence.len() {
            return Err(overlap_input_error(
                "Picard WGS overlap CIGAR query span does not match the sequence length",
            ));
        }
        Ok(summary)
    }

    fn prepare_record(&mut self, reference_id: i32, start: u64) -> Result<(), AlignGaugeError> {
        match self.current_reference_id {
            None => {
                self.current_reference_id = Some(reference_id);
                self.previous_record_start = Some(start);
            }
            Some(current) if reference_id < current => {
                return Err(AlignGaugeError::new(
                    ErrorCategory::InputUnsorted,
                    "Picard WGS overlap reference order regressed",
                ));
            }
            Some(current) if reference_id > current => {
                self.clear_active_state();
                self.current_reference_id = Some(reference_id);
                self.previous_record_start = Some(start);
            }
            Some(_) => {
                if self.previous_record_start.is_some_and(|previous| start < previous) {
                    return Err(AlignGaugeError::new(
                        ErrorCategory::InputUnsorted,
                        "Picard WGS overlap record position regressed",
                    ));
                }
                self.previous_record_start = Some(start);
                self.expire_before(reference_id, start)?;
            }
        }
        Ok(())
    }

    fn clear_active_state(&mut self) {
        self.active_names.clear();
        self.active_locus_observations.clear();
        self.state_bytes = 0;
    }

    fn expire_before(&mut self, reference_id: i32, start: u64) -> Result<(), AlignGaugeError> {
        let released_names = self
            .active_names
            .values()
            .filter(|state| state.reference_id != reference_id || state.last_eligible_end <= start)
            .try_fold(0_u64, |sum, state| {
                sum.checked_add(state.accounted_bytes).ok_or_else(|| {
                    AlignGaugeError::new(
                        ErrorCategory::InternalInvariant,
                        "Picard WGS overlap released-state accounting overflowed",
                    )
                })
            })?;
        self.active_names.retain(|_, state| {
            state.reference_id == reference_id && state.last_eligible_end > start
        });

        let expired_loci = self
            .active_locus_observations
            .range(..start)
            .count();
        let expired_loci_u64 = u64::try_from(expired_loci).map_err(|source| {
            AlignGaugeError::new(
                ErrorCategory::InternalInvariant,
                "Picard WGS overlap expired-locus count does not fit u64",
            )
            .with_source(source)
        })?;
        let released_loci = expired_loci_u64.checked_mul(ACTIVE_LOCUS_BYTES).ok_or_else(|| {
            AlignGaugeError::new(
                ErrorCategory::InternalInvariant,
                "Picard WGS overlap released-locus accounting overflowed",
            )
        })?;
        self.active_locus_observations = self.active_locus_observations.split_off(&start);
        let released = released_names.checked_add(released_loci).ok_or_else(|| {
            AlignGaugeError::new(
                ErrorCategory::InternalInvariant,
                "Picard WGS overlap released-state accounting overflowed",
            )
        })?;
        self.state_bytes = self.state_bytes.checked_sub(released).ok_or_else(|| {
            AlignGaugeError::new(
                ErrorCategory::InternalInvariant,
                "Picard WGS overlap state accounting underflowed",
            )
        })?;
        Ok(())
    }

    fn observe_locus_candidate(&mut self, position: u64) -> Result<(), AlignGaugeError> {
        if let Some(count) = self.active_locus_observations.get_mut(&position) {
            if *count >= self.locus_accumulation_cap {
                return Err(overlap_resource_error(
                    "Picard WGS locus accumulation cap would be exceeded",
                )
                .with_detail("position", position)
                .with_detail("locus_accumulation_cap", u64::from(self.locus_accumulation_cap)));
            }
            *count = count.checked_add(1).ok_or_else(|| {
                AlignGaugeError::new(
                    ErrorCategory::InternalInvariant,
                    "Picard WGS locus observation count overflowed",
                )
            })?;
            return Ok(());
        }

        self.reserve(ACTIVE_LOCUS_BYTES)?;
        self.active_locus_observations.insert(position, 1);
        Ok(())
    }

    fn position_was_seen(
        &mut self,
        query_name: &[u8],
        reference_id: i32,
        position: u64,
    ) -> Result<bool, AlignGaugeError> {
        if let Some(state) = self.active_names.get_mut(query_name) {
            if state.reference_id != reference_id {
                return Err(AlignGaugeError::new(
                    ErrorCategory::InternalInvariant,
                    "Picard WGS overlap query-name state crossed references",
                ));
            }
            if state.eligible_positions.contains(&position) {
                return Ok(true);
            }
            self.reserve(ACTIVE_POSITION_BYTES)?;
            let state = self.active_names.get_mut(query_name).ok_or_else(|| {
                AlignGaugeError::new(
                    ErrorCategory::InternalInvariant,
                    "Picard WGS overlap query-name state disappeared",
                )
            })?;
            state.eligible_positions.insert(position);
            state.last_eligible_end = state.last_eligible_end.max(position.checked_add(1).ok_or_else(|| {
                overlap_input_error("Picard WGS overlap eligible position overflowed")
            })?);
            state.accounted_bytes = state
                .accounted_bytes
                .checked_add(ACTIVE_POSITION_BYTES)
                .ok_or_else(|| {
                    AlignGaugeError::new(
                        ErrorCategory::InternalInvariant,
                        "Picard WGS overlap name-state accounting overflowed",
                    )
                })?;
            return Ok(false);
        }

        let name_bytes = u64::try_from(query_name.len()).map_err(|source| {
            overlap_resource_error("Picard WGS query-name length does not fit u64")
                .with_source(source)
        })?;
        let charge = ACTIVE_NAME_OVERHEAD_BYTES
            .checked_add(name_bytes)
            .and_then(|value| value.checked_add(ACTIVE_POSITION_BYTES))
            .ok_or_else(|| overlap_resource_error("Picard WGS name-state charge overflowed"))?;
        self.reserve(charge)?;
        let end = position.checked_add(1).ok_or_else(|| {
            overlap_input_error("Picard WGS overlap eligible position overflowed")
        })?;
        self.active_names.insert(
            query_name.to_vec(),
            ActiveNameState {
                reference_id,
                eligible_positions: BTreeSet::from([position]),
                last_eligible_end: end,
                accounted_bytes: charge,
            },
        );
        Ok(false)
    }

    fn reserve(&mut self, additional_bytes: u64) -> Result<(), AlignGaugeError> {
        let required = self.state_bytes.checked_add(additional_bytes).ok_or_else(|| {
            overlap_resource_error("Picard WGS overlap state accounting overflowed")
        })?;
        if required > self.state_budget_bytes {
            return Err(overlap_resource_error(
                "Picard WGS exact overlap state exceeds its reserved memory budget",
            )
            .with_detail("state_budget_bytes", self.state_budget_bytes)
            .with_detail("required_state_bytes", required));
        }
        self.state_bytes = required;
        Ok(())
    }
}

/// Reproduce HTSJDK 4.2.0 `SAMUtils.getNumOverlappingAlignedBasesToClip` for the
/// Picard 3.4.0 default HsMetrics profile.
///
/// The calculation deliberately uses only record-local mate metadata. It does not look up a mate
/// by query name, does not require the mate to be on the same reference, and preserves HTSJDK's
/// equal-start tie break: first-of-pair is retained and the other end is eligible for clipping.
///
/// # Errors
/// Returns `input_corrupt` for zero/unknown CIGAR operations or checked arithmetic failure.
pub fn picard_hs_trailing_read_bases_to_clip(
    flags: u16,
    alignment_start: u64,
    mate_alignment_start: Option<u64>,
    raw_cigar: &[u32],
) -> Result<u64, AlignGaugeError> {
    if flags & FLAG_PAIRED == 0
        || flags & FLAG_UNMAPPED != 0
        || flags & FLAG_MATE_UNMAPPED != 0
    {
        return Ok(0);
    }
    let Some(mate_start) = mate_alignment_start else {
        return Ok(0);
    };
    if mate_start < alignment_start || (mate_start == alignment_start && flags & FLAG_READ1 != 0) {
        return Ok(0);
    }

    let mut clipped = 0_u64;
    let mut reference_cursor = alignment_start;
    for encoded in raw_cigar {
        let length = u64::from(encoded >> 4);
        let operation = encoded & 0x0f;
        if length == 0 {
            return Err(overlap_input_error(
                "Picard Hs overlap CIGAR contains a zero-length operation",
            ));
        }
        let consumes_reference = matches!(operation, 0 | 2 | 3 | 7 | 8);
        let consumes_query = matches!(operation, 0 | 1 | 4 | 7 | 8);
        if operation > 8 {
            return Err(overlap_input_error(
                "Picard Hs overlap CIGAR contains an unknown operation code",
            )
            .with_detail("cigar_operation_code", u64::from(operation)));
        }
        let reference_span = if consumes_reference { length } else { 0 };
        let final_reference_position = if reference_span == 0 {
            reference_cursor.checked_sub(1)
        } else {
            Some(
                reference_cursor
                    .checked_add(reference_span)
                    .and_then(|value| value.checked_sub(1))
                    .ok_or_else(|| {
                        overlap_input_error("Picard Hs overlap reference coordinate overflowed")
                    })?,
            )
        };

        if final_reference_position.is_some_and(|end| mate_start <= end) {
            match operation {
                0 => {
                    let contribution = if mate_start < reference_cursor {
                        length
                    } else {
                        reference_cursor
                            .checked_add(length)
                            .and_then(|end| end.checked_sub(mate_start))
                            .ok_or_else(|| {
                                overlap_input_error(
                                    "Picard Hs overlap partial-match clipping underflowed",
                                )
                            })?
                    };
                    clipped = clipped.checked_add(contribution).ok_or_else(|| {
                        overlap_input_error("Picard Hs overlap clipped-base count overflowed")
                    })?;
                }
                3 | 4 | 5 | 6 => {}
                _ => {
                    if consumes_query {
                        clipped = clipped.checked_add(length).ok_or_else(|| {
                            overlap_input_error("Picard Hs overlap clipped-base count overflowed")
                        })?;
                    }
                }
            }
        }
        if consumes_reference {
            reference_cursor = reference_cursor.checked_add(length).ok_or_else(|| {
                overlap_input_error("Picard Hs overlap reference coordinate overflowed")
            })?;
        }
    }
    Ok(clipped)
}

/// Whether the pinned Picard WGS overlap stage may receive a record after flag-only filters.
///
/// Adapter and MAPQ filtering are separate because they require fields beyond flags. Supplementary
/// records intentionally remain eligible: Picard's pinned `SecondaryAlignmentFilter` removes only
/// secondary alignments.
#[must_use]
pub const fn picard_wgs_flag_candidate(flags: u16) -> bool {
    flags & (FLAG_UNMAPPED | FLAG_SECONDARY) == 0
}

fn checked_reference_end(
    start: u64,
    length: u64,
    reference_length: u64,
) -> Result<u64, AlignGaugeError> {
    let end = start.checked_add(length).ok_or_else(|| {
        overlap_input_error("Picard WGS overlap reference coordinate overflowed")
    })?;
    if end > reference_length {
        return Err(overlap_input_error(
            "Picard WGS overlap CIGAR extends beyond the declared reference",
        )
        .with_detail("operation_end", end)
        .with_detail("reference_length", reference_length));
    }
    Ok(end)
}

const fn is_no_call(base: u8) -> bool {
    matches!(base, b'N' | b'n' | b'.')
}

fn checked_increment(value: u64, label: &'static str) -> Result<u64, AlignGaugeError> {
    value.checked_add(1).ok_or_else(|| {
        AlignGaugeError::new(
            ErrorCategory::InternalInvariant,
            format!("{label} overflowed"),
        )
    })
}

fn overlap_input_error(message: impl Into<String>) -> AlignGaugeError {
    AlignGaugeError::new(ErrorCategory::InputCorrupt, message)
}

fn overlap_resource_error(message: impl Into<String>) -> AlignGaugeError {
    AlignGaugeError::new(ErrorCategory::ResourceLimit, message)
}

#[cfg(test)]
mod tests {
    use super::{
        FLAG_MATE_UNMAPPED, FLAG_PAIRED, FLAG_READ1, PICARD_WGS_LOCUS_ACCUMULATION_CAP,
        PicardWgsOverlapCorrector, PicardWgsOverlapRecord, picard_hs_trailing_read_bases_to_clip,
        picard_wgs_flag_candidate,
    };
    use aligngauge_core::ErrorCategory;

    fn cigar(length: u32, operation: u32) -> u32 {
        (length << 4) | operation
    }

    fn record<'a>(
        start: u64,
        name: &'a [u8],
        cigar_words: &'a [u32],
        sequence: &'a [u8],
        qualities: &'a [u8],
    ) -> PicardWgsOverlapRecord<'a> {
        PicardWgsOverlapRecord {
            reference_id: 0,
            start,
            reference_length: 1_000,
            query_name: name,
            raw_cigar: cigar_words,
            sequence,
            qualities,
        }
    }

    #[test]
    fn wgs_first_high_quality_observation_wins_per_query_name_and_locus() {
        let words = [cigar(10, 0)];
        let sequence = *b"AAAAAAAAAA";
        let qualities = [30_u8; 10];
        let mut corrector = PicardWgsOverlapCorrector::new(1 << 20).expect("budget");
        let mut retained = Vec::new();
        let first = corrector
            .observe_record(
                record(10, b"pair", &words, &sequence, &qualities),
                |position| {
                    retained.push(position);
                    Ok(())
                },
            )
            .expect("first record");
        let second = corrector
            .observe_record(
                record(15, b"pair", &words, &sequence, &qualities),
                |position| {
                    retained.push(position);
                    Ok(())
                },
            )
            .expect("second record");

        assert_eq!(first.retained_bases, 10);
        assert_eq!(first.overlap_excluded_bases, 0);
        assert_eq!(second.retained_bases, 5);
        assert_eq!(second.overlap_excluded_bases, 5);
        assert_eq!(retained.len(), 15);
    }

    #[test]
    fn wgs_low_quality_first_observation_does_not_hide_good_second_observation() {
        let words = [cigar(10, 0)];
        let sequence = *b"AAAAAAAAAA";
        let low = [10_u8; 10];
        let high = [30_u8; 10];
        let mut corrector = PicardWgsOverlapCorrector::new(1 << 20).expect("budget");
        let first = corrector
            .observe_record(record(10, b"pair", &words, &sequence, &low), |_| Ok(()))
            .expect("first record");
        let second = corrector
            .observe_record(record(10, b"pair", &words, &sequence, &high), |_| Ok(()))
            .expect("second record");

        assert_eq!(first.baseq_excluded_bases, 10);
        assert_eq!(first.overlap_excluded_bases, 0);
        assert_eq!(second.retained_bases, 10);
        assert_eq!(second.overlap_excluded_bases, 0);
    }

    #[test]
    fn wgs_expired_name_state_is_released_before_nonoverlapping_record() {
        let words = [cigar(5, 0)];
        let sequence = *b"AAAAA";
        let qualities = [30_u8; 5];
        let mut corrector = PicardWgsOverlapCorrector::new(1 << 20).expect("budget");
        corrector
            .observe_record(record(10, b"same", &words, &sequence, &qualities), |_| Ok(()))
            .expect("first record");
        let second = corrector
            .observe_record(record(20, b"same", &words, &sequence, &qualities), |_| Ok(()))
            .expect("second record");
        assert_eq!(second.retained_bases, 5);
        assert_eq!(second.overlap_excluded_bases, 0);
    }

    #[test]
    fn wgs_state_budget_exhaustion_is_fatal() {
        let words = [cigar(1, 0)];
        let sequence = *b"A";
        let qualities = [30_u8; 1];
        let mut corrector = PicardWgsOverlapCorrector::new(1).expect("nonzero budget");
        let error = corrector
            .observe_record(record(10, b"pair", &words, &sequence, &qualities), |_| Ok(()))
            .expect_err("tiny budget must fail");
        assert_eq!(error.category(), ErrorCategory::ResourceLimit);
    }

    #[test]
    fn wgs_locus_cap_is_fatal_instead_of_truncating() {
        let words = [cigar(1, 0)];
        let sequence = *b"A";
        let qualities = [30_u8; 1];
        let mut corrector = PicardWgsOverlapCorrector::new(1 << 20).expect("budget");
        corrector.locus_accumulation_cap = 1;
        corrector
            .observe_record(record(10, b"first", &words, &sequence, &qualities), |_| Ok(()))
            .expect("first record");
        let error = corrector
            .observe_record(record(10, b"second", &words, &sequence, &qualities), |_| Ok(()))
            .expect_err("second candidate exceeds test cap");
        assert_eq!(error.category(), ErrorCategory::ResourceLimit);
        assert_eq!(PICARD_WGS_LOCUS_ACCUMULATION_CAP, 100_000);
    }

    #[test]
    fn hs_matches_leftmost_and_equal_start_tie_breaks() {
        let words = [cigar(10, 0)];
        let paired = FLAG_PAIRED;
        assert_eq!(
            picard_hs_trailing_read_bases_to_clip(paired, 100, Some(105), &words)
                .expect("leftmost"),
            5
        );
        assert_eq!(
            picard_hs_trailing_read_bases_to_clip(paired, 105, Some(100), &words)
                .expect("rightmost"),
            0
        );
        assert_eq!(
            picard_hs_trailing_read_bases_to_clip(
                paired | FLAG_READ1,
                100,
                Some(100),
                &words,
            )
            .expect("first tie"),
            0
        );
        assert_eq!(
            picard_hs_trailing_read_bases_to_clip(paired, 100, Some(100), &words)
                .expect("second tie"),
            10
        );
    }

    #[test]
    fn hs_preserves_htsjdk_insertion_and_extended_cigar_behavior() {
        // 5M2I5M with mate start inside the first M: partial M + the entire I + second M.
        let words = [cigar(5, 0), cigar(2, 1), cigar(5, 0)];
        assert_eq!(
            picard_hs_trailing_read_bases_to_clip(FLAG_PAIRED, 100, Some(103), &words)
                .expect("insertion"),
            9
        );

        // HTSJDK 4.2.0 treats '=' and 'X' through its non-M branch, clipping a whole
        // read-consuming element once the mate start reaches that element.
        let extended = [cigar(5, 7), cigar(5, 8)];
        assert_eq!(
            picard_hs_trailing_read_bases_to_clip(FLAG_PAIRED, 100, Some(102), &extended)
                .expect("extended cigar"),
            10
        );
    }

    #[test]
    fn hs_unpaired_or_mate_unmapped_records_do_not_clip() {
        let words = [cigar(10, 0)];
        assert_eq!(
            picard_hs_trailing_read_bases_to_clip(0, 100, Some(105), &words).expect("unpaired"),
            0
        );
        assert_eq!(
            picard_hs_trailing_read_bases_to_clip(
                FLAG_PAIRED | FLAG_MATE_UNMAPPED,
                100,
                None,
                &words,
            )
            .expect("mate unmapped"),
            0
        );
    }

    #[test]
    fn wgs_flag_candidate_keeps_supplementary_but_rejects_secondary_and_unmapped() {
        assert!(picard_wgs_flag_candidate(0x800));
        assert!(!picard_wgs_flag_candidate(0x100));
        assert!(!picard_wgs_flag_candidate(0x4));
    }
}
