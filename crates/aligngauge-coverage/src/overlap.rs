//! Exact overlap-correction primitives for pinned Picard 3.4.0 profiles.
//!
//! Picard WGS and hybrid-selection metrics use different overlap algorithms. Keeping them as
//! separate named policies prevents a generic "clip overlaps" switch from acquiring false
//! compatibility semantics.

use std::collections::{BTreeMap, BTreeSet};

use aligngauge_core::{AlignGaugeError, ErrorCategory};

/// Pinned default Picard WGS overlap policy.
pub const PICARD_WGS_OVERLAP_PROFILE: &str = "picard-wgs-3.4.0-default-overlap-v1";
/// Pinned default Picard `HsMetrics` overlap policy.
pub const PICARD_HS_OVERLAP_PROFILE: &str = "picard-hs-3.4.0-default-overlap-v1";
/// Authoritative Milestone 13 execution mode for exact overlap correction.
pub const EXACT_OVERLAP_EXECUTION_MODE: &str = "streaming-coordinate-order-v1";
/// Indexed reference partitions are not admitted with exact overlap correction in v0.4.
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

const NAME_OVERHEAD_BYTES: u64 = 128;
const POSITION_BYTES: u64 = 32;
const LOCUS_BYTES: u64 = 48;

/// One record presented after Picard WGS record-level filtering.
#[derive(Debug, Clone, Copy)]
pub struct PicardWgsOverlapRecord<'a> {
    /// Zero-based reference index.
    pub reference_id: i32,
    /// Zero-based alignment start.
    pub start: u64,
    /// Declared reference length.
    pub reference_length: u64,
    /// Raw BAM query-name bytes.
    pub query_name: &'a [u8],
    /// Raw BAM CIGAR words (`length << 4 | operation`).
    pub raw_cigar: &'a [u32],
    /// Decoded query bases.
    pub sequence: &'a [u8],
    /// Raw Phred qualities.
    pub qualities: &'a [u8],
}

/// Per-record result from exact Picard WGS base filtering and overlap correction.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct PicardWgsOverlapSummary {
    /// High-quality observations retained after read-name de-duplication.
    pub retained_bases: u64,
    /// Observations rejected by Picard's minimum-base-quality/no-call check.
    pub baseq_excluded_bases: u64,
    /// High-quality observations rejected because the query name already contributed.
    pub overlap_excluded_bases: u64,
}

#[derive(Debug)]
struct ActiveNameState {
    positions: BTreeSet<u64>,
    last_end: u64,
    charged_bytes: u64,
}

/// Bounded streaming implementation of Picard WGS overlap semantics.
///
/// Raw query-name state is retained only while an earlier eligible observation can overlap a later
/// coordinate-sorted record. Exceeding the explicit state budget or Picard's pinned locus cap is
/// fatal; state is never truncated or converted to an approximate representation.
#[derive(Debug)]
pub struct PicardWgsOverlapCorrector {
    state_budget_bytes: u64,
    state_bytes: u64,
    active_names: BTreeMap<Vec<u8>, ActiveNameState>,
    active_loci: BTreeMap<u64, u32>,
    current_reference_id: Option<i32>,
    previous_start: Option<u64>,
    locus_accumulation_cap: u32,
}

impl PicardWgsOverlapCorrector {
    /// Create exact overlap state with an explicit hard memory reservation.
    ///
    /// # Errors
    /// Returns `resource_limit` if the reservation is zero.
    pub fn new(state_budget_bytes: u64) -> Result<Self, AlignGaugeError> {
        if state_budget_bytes == 0 {
            return Err(resource_error(
                "Picard WGS overlap state budget must be greater than zero",
            ));
        }
        Ok(Self {
            state_budget_bytes,
            state_bytes: 0,
            active_names: BTreeMap::new(),
            active_loci: BTreeMap::new(),
            current_reference_id: None,
            previous_start: None,
            locus_accumulation_cap: PICARD_WGS_LOCUS_ACCUMULATION_CAP,
        })
    }

    /// Current deterministic planning charge for active state.
    #[must_use]
    pub const fn state_bytes(&self) -> u64 {
        self.state_bytes
    }

    /// Hard caller-reserved planning budget.
    #[must_use]
    pub const fn state_budget_bytes(&self) -> u64 {
        self.state_budget_bytes
    }

    /// Observe one record and report each retained reference position.
    ///
    /// Record-level Picard filters are the caller's responsibility. This stage applies the pinned
    /// base-quality/no-call gate before raw-query-name overlap identity, matching Picard ordering.
    ///
    /// # Errors
    /// Returns a typed fatal error for malformed CIGAR/query layout, coordinate regression,
    /// arithmetic failure, state-budget exhaustion, locus-cap exhaustion, or callback failure.
    pub fn observe_record(
        &mut self,
        record: PicardWgsOverlapRecord<'_>,
        mut retain_position: impl FnMut(u64) -> Result<(), AlignGaugeError>,
    ) -> Result<PicardWgsOverlapSummary, AlignGaugeError> {
        validate_record(record)?;
        self.prepare_record(record.reference_id, record.start)?;
        let mut summary = PicardWgsOverlapSummary::default();
        let mut reference_cursor = record.start;
        let mut query_cursor = 0_usize;

        for encoded in record.raw_cigar {
            let length_u32 = encoded >> 4;
            let operation = encoded & 0x0f;
            if length_u32 == 0 {
                return Err(input_error(
                    "Picard WGS overlap CIGAR contains a zero-length operation",
                ));
            }
            let length = usize::try_from(length_u32).map_err(|source| {
                input_error("Picard WGS overlap CIGAR length does not fit usize")
                    .with_source(source)
            })?;
            match operation {
                0 | 7 | 8 => {
                    let query_end = checked_query_end(query_cursor, length, record.sequence.len())?;
                    let reference_end = checked_reference_end(
                        reference_cursor,
                        u64::from(length_u32),
                        record.reference_length,
                    )?;
                    for offset in 0..length {
                        let position = reference_cursor
                            .checked_add(u64::try_from(offset).map_err(|source| {
                                input_error("Picard WGS overlap base offset does not fit u64")
                                    .with_source(source)
                            })?)
                            .ok_or_else(|| {
                                input_error("Picard WGS overlap base coordinate overflowed")
                            })?;
                        self.observe_locus_candidate(position)?;
                        let query_index = query_cursor.checked_add(offset).ok_or_else(|| {
                            input_error("Picard WGS overlap query index overflowed")
                        })?;
                        if !base_is_eligible(
                            record.sequence[query_index],
                            record.qualities[query_index],
                        ) {
                            summary.baseq_excluded_bases = checked_increment(
                                summary.baseq_excluded_bases,
                                "Picard WGS base-quality exclusions",
                            )?;
                        } else if self.position_was_seen(record.query_name, position)? {
                            summary.overlap_excluded_bases = checked_increment(
                                summary.overlap_excluded_bases,
                                "Picard WGS overlap exclusions",
                            )?;
                        } else {
                            retain_position(position)?;
                            summary.retained_bases = checked_increment(
                                summary.retained_bases,
                                "Picard WGS retained bases",
                            )?;
                        }
                    }
                    query_cursor = query_end;
                    reference_cursor = reference_end;
                }
                1 | 4 => {
                    query_cursor = checked_query_end(query_cursor, length, record.sequence.len())?;
                }
                2 | 3 => {
                    reference_cursor = checked_reference_end(
                        reference_cursor,
                        u64::from(length_u32),
                        record.reference_length,
                    )?;
                }
                5 | 6 => {}
                _ => {
                    return Err(input_error(
                        "Picard WGS overlap CIGAR contains an unknown operation code",
                    )
                    .with_detail("cigar_operation_code", u64::from(operation)));
                }
            }
        }
        if query_cursor != record.sequence.len() {
            return Err(input_error(
                "Picard WGS overlap CIGAR query span does not match the sequence length",
            ));
        }
        Ok(summary)
    }

    fn prepare_record(&mut self, reference_id: i32, start: u64) -> Result<(), AlignGaugeError> {
        match self.current_reference_id {
            None => self.current_reference_id = Some(reference_id),
            Some(current) if reference_id < current => {
                return Err(AlignGaugeError::new(
                    ErrorCategory::InputUnsorted,
                    "Picard WGS overlap reference order regressed",
                ));
            }
            Some(current) if reference_id > current => {
                self.clear_state();
                self.current_reference_id = Some(reference_id);
            }
            Some(_) => {
                if self.previous_start.is_some_and(|previous| start < previous) {
                    return Err(AlignGaugeError::new(
                        ErrorCategory::InputUnsorted,
                        "Picard WGS overlap record position regressed",
                    ));
                }
                self.expire_before(start)?;
            }
        }
        self.previous_start = Some(start);
        Ok(())
    }

    fn clear_state(&mut self) {
        self.active_names.clear();
        self.active_loci.clear();
        self.state_bytes = 0;
        self.previous_start = None;
    }

    fn expire_before(&mut self, start: u64) -> Result<(), AlignGaugeError> {
        let released_names = self
            .active_names
            .values()
            .filter(|state| state.last_end <= start)
            .try_fold(0_u64, |sum, state| {
                sum.checked_add(state.charged_bytes).ok_or_else(|| {
                    invariant_error("Picard WGS released-name accounting overflowed")
                })
            })?;
        self.active_names.retain(|_, state| state.last_end > start);
        let expired_loci = u64::try_from(self.active_loci.range(..start).count()).map_err(|source| {
            invariant_error("Picard WGS expired-locus count does not fit u64").with_source(source)
        })?;
        let released_loci = expired_loci
            .checked_mul(LOCUS_BYTES)
            .ok_or_else(|| invariant_error("Picard WGS released-locus accounting overflowed"))?;
        self.active_loci = self.active_loci.split_off(&start);
        let released = released_names
            .checked_add(released_loci)
            .ok_or_else(|| invariant_error("Picard WGS released-state accounting overflowed"))?;
        self.state_bytes = self
            .state_bytes
            .checked_sub(released)
            .ok_or_else(|| invariant_error("Picard WGS overlap state accounting underflowed"))?;
        Ok(())
    }

    fn observe_locus_candidate(&mut self, position: u64) -> Result<(), AlignGaugeError> {
        if let Some(count) = self.active_loci.get_mut(&position) {
            if *count >= self.locus_accumulation_cap {
                return Err(
                    resource_error("Picard WGS locus accumulation cap would be exceeded")
                        .with_detail("position", position)
                        .with_detail(
                            "locus_accumulation_cap",
                            u64::from(self.locus_accumulation_cap),
                        ),
                );
            }
            *count = count
                .checked_add(1)
                .ok_or_else(|| invariant_error("Picard WGS locus observation count overflowed"))?;
            return Ok(());
        }
        self.reserve(LOCUS_BYTES)?;
        self.active_loci.insert(position, 1);
        Ok(())
    }

    fn position_was_seen(
        &mut self,
        query_name: &[u8],
        position: u64,
    ) -> Result<bool, AlignGaugeError> {
        if let Some(state) = self.active_names.get(query_name) {
            if state.positions.contains(&position) {
                return Ok(true);
            }
            self.reserve(POSITION_BYTES)?;
            let state = self.active_names.get_mut(query_name).ok_or_else(|| {
                invariant_error("Picard WGS overlap query-name state disappeared")
            })?;
            state.positions.insert(position);
            state.last_end = state.last_end.max(checked_position_end(position)?);
            state.charged_bytes = state
                .charged_bytes
                .checked_add(POSITION_BYTES)
                .ok_or_else(|| invariant_error("Picard WGS name-state charge overflowed"))?;
            return Ok(false);
        }
        let name_bytes = u64::try_from(query_name.len()).map_err(|source| {
            resource_error("Picard WGS query-name length does not fit u64").with_source(source)
        })?;
        let charge = NAME_OVERHEAD_BYTES
            .checked_add(name_bytes)
            .and_then(|value| value.checked_add(POSITION_BYTES))
            .ok_or_else(|| resource_error("Picard WGS name-state charge overflowed"))?;
        self.reserve(charge)?;
        self.active_names.insert(
            query_name.to_vec(),
            ActiveNameState {
                positions: BTreeSet::from([position]),
                last_end: checked_position_end(position)?,
                charged_bytes: charge,
            },
        );
        Ok(false)
    }

    fn reserve(&mut self, additional_bytes: u64) -> Result<(), AlignGaugeError> {
        let required = self
            .state_bytes
            .checked_add(additional_bytes)
            .ok_or_else(|| resource_error("Picard WGS overlap state accounting overflowed"))?;
        if required > self.state_budget_bytes {
            return Err(resource_error(
                "Picard WGS exact overlap state exceeds its reserved memory budget",
            )
            .with_detail("state_budget_bytes", self.state_budget_bytes)
            .with_detail("required_state_bytes", required));
        }
        self.state_bytes = required;
        Ok(())
    }
}

/// Reproduce HTSJDK 4.2.0 `SAMUtils.getNumOverlappingAlignedBasesToClip`.
///
/// The calculation is intentionally record-local: it uses the record's mate coordinate and CIGAR
/// rather than looking up a mate by query name. The equal-start tie keeps first-of-pair and makes
/// the other end eligible for clipping, matching the pinned helper.
///
/// # Errors
/// Returns `input_corrupt` for zero/unknown CIGAR operations or checked arithmetic failure.
pub fn picard_hs_trailing_read_bases_to_clip(
    flags: u16,
    alignment_start: u64,
    mate_alignment_start: Option<u64>,
    raw_cigar: &[u32],
) -> Result<u64, AlignGaugeError> {
    if flags & FLAG_PAIRED == 0 || flags & FLAG_UNMAPPED != 0 || flags & FLAG_MATE_UNMAPPED != 0 {
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
            return Err(input_error(
                "Picard Hs overlap CIGAR contains a zero-length operation",
            ));
        }
        if operation > 8 {
            return Err(
                input_error("Picard Hs overlap CIGAR contains an unknown operation code")
                    .with_detail("cigar_operation_code", u64::from(operation)),
            );
        }
        let consumes_reference = matches!(operation, 0 | 2 | 3 | 7 | 8);
        let consumes_query = matches!(operation, 0 | 1 | 4 | 7 | 8);
        let reference_span = if consumes_reference { length } else { 0 };
        let final_reference_position = if reference_span == 0 {
            reference_cursor.checked_sub(1)
        } else {
            Some(
                reference_cursor
                    .checked_add(reference_span)
                    .and_then(|value| value.checked_sub(1))
                    .ok_or_else(|| {
                        input_error("Picard Hs overlap reference coordinate overflowed")
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
                                input_error("Picard Hs partial-match clipping underflowed")
                            })?
                    };
                    clipped = clipped.checked_add(contribution).ok_or_else(|| {
                        input_error("Picard Hs overlap clipped-base count overflowed")
                    })?;
                }
                3..=6 => {}
                _ if consumes_query => {
                    clipped = clipped.checked_add(length).ok_or_else(|| {
                        input_error("Picard Hs overlap clipped-base count overflowed")
                    })?;
                }
                _ => {}
            }
        }
        if consumes_reference {
            reference_cursor = reference_cursor
                .checked_add(length)
                .ok_or_else(|| input_error("Picard Hs overlap reference coordinate overflowed"))?;
        }
    }
    Ok(clipped)
}

/// Flag-only candidate predicate for the pinned Picard WGS record path.
///
/// Supplementary alignments intentionally remain candidates; Picard's pinned secondary filter
/// removes secondary alignments but not supplementary alignments. Adapter, MAPQ, duplicate, PF,
/// and paired-read filters remain separate record-level stages.
#[must_use]
pub const fn picard_wgs_flag_candidate(flags: u16) -> bool {
    flags & (FLAG_UNMAPPED | FLAG_SECONDARY) == 0
}

fn validate_record(record: PicardWgsOverlapRecord<'_>) -> Result<(), AlignGaugeError> {
    if record.reference_id < 0 {
        return Err(input_error(
            "Picard WGS overlap record has a negative reference ID",
        ));
    }
    if record.start > record.reference_length {
        return Err(input_error(
            "Picard WGS overlap record starts outside the declared reference",
        ));
    }
    if record.sequence.len() != record.qualities.len() {
        return Err(input_error(
            "Picard WGS overlap sequence and quality lengths differ",
        ));
    }
    if record.query_name.is_empty() {
        return Err(input_error(
            "Picard WGS overlap record has an empty query name",
        ));
    }
    Ok(())
}

fn checked_query_end(
    start: usize,
    length: usize,
    query_length: usize,
) -> Result<usize, AlignGaugeError> {
    let end = start
        .checked_add(length)
        .ok_or_else(|| input_error("Picard WGS overlap query cursor overflowed"))?;
    if end > query_length {
        return Err(input_error(
            "Picard WGS overlap CIGAR consumes beyond the query sequence",
        ));
    }
    Ok(end)
}

fn checked_reference_end(
    start: u64,
    length: u64,
    reference_length: u64,
) -> Result<u64, AlignGaugeError> {
    let end = start
        .checked_add(length)
        .ok_or_else(|| input_error("Picard WGS overlap reference coordinate overflowed"))?;
    if end > reference_length {
        return Err(
            input_error("Picard WGS overlap CIGAR extends beyond the declared reference")
                .with_detail("operation_end", end)
                .with_detail("reference_length", reference_length),
        );
    }
    Ok(end)
}

fn checked_position_end(position: u64) -> Result<u64, AlignGaugeError> {
    position
        .checked_add(1)
        .ok_or_else(|| input_error("Picard WGS overlap eligible position overflowed"))
}

const fn base_is_eligible(base: u8, quality: u8) -> bool {
    quality >= PICARD_WGS_MINIMUM_BASE_QUALITY && !matches!(base, b'N' | b'n' | b'.')
}

fn checked_increment(value: u64, label: &'static str) -> Result<u64, AlignGaugeError> {
    value
        .checked_add(1)
        .ok_or_else(|| invariant_error(format!("{label} overflowed")))
}

fn input_error(message: impl Into<String>) -> AlignGaugeError {
    AlignGaugeError::new(ErrorCategory::InputCorrupt, message)
}

fn resource_error(message: impl Into<String>) -> AlignGaugeError {
    AlignGaugeError::new(ErrorCategory::ResourceLimit, message)
}

fn invariant_error(message: impl Into<String>) -> AlignGaugeError {
    AlignGaugeError::new(ErrorCategory::InternalInvariant, message)
}

#[cfg(test)]
mod tests {
    use super::{
        FLAG_MATE_UNMAPPED, FLAG_PAIRED, FLAG_READ1, PICARD_WGS_LOCUS_ACCUMULATION_CAP,
        PicardWgsOverlapCorrector, PicardWgsOverlapRecord, picard_hs_trailing_read_bases_to_clip,
        picard_wgs_flag_candidate,
    };
    use aligngauge_core::ErrorCategory;

    const fn cigar(length: u32, operation: u32) -> u32 {
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
    fn wgs_first_high_quality_observation_wins_per_name_and_locus() {
        let words = [cigar(10, 0)];
        let sequence = *b"AAAAAAAAAA";
        let qualities = [30_u8; 10];
        let mut corrector = PicardWgsOverlapCorrector::new(1 << 20).expect("budget");
        let first = corrector
            .observe_record(record(10, b"pair", &words, &sequence, &qualities), |_| {
                Ok(())
            })
            .expect("first record");
        let second = corrector
            .observe_record(record(15, b"pair", &words, &sequence, &qualities), |_| {
                Ok(())
            })
            .expect("second record");
        assert_eq!(first.retained_bases, 10);
        assert_eq!(first.overlap_excluded_bases, 0);
        assert_eq!(second.retained_bases, 5);
        assert_eq!(second.overlap_excluded_bases, 5);
    }

    #[test]
    fn wgs_base_quality_filter_precedes_overlap_identity() {
        let words = [cigar(10, 0)];
        let sequence = *b"AAAAAAAAAA";
        let low = [10_u8; 10];
        let high = [30_u8; 10];
        let mut corrector = PicardWgsOverlapCorrector::new(1 << 20).expect("budget");
        let first = corrector
            .observe_record(record(10, b"pair", &words, &sequence, &low), |_| Ok(()))
            .expect("low-quality record");
        let second = corrector
            .observe_record(record(10, b"pair", &words, &sequence, &high), |_| Ok(()))
            .expect("high-quality record");
        assert_eq!(first.baseq_excluded_bases, 10);
        assert_eq!(first.overlap_excluded_bases, 0);
        assert_eq!(second.retained_bases, 10);
        assert_eq!(second.overlap_excluded_bases, 0);
    }

    #[test]
    fn wgs_nonoverlapping_name_state_expires() {
        let words = [cigar(5, 0)];
        let sequence = *b"AAAAA";
        let qualities = [30_u8; 5];
        let mut corrector = PicardWgsOverlapCorrector::new(1 << 20).expect("budget");
        corrector
            .observe_record(record(10, b"same", &words, &sequence, &qualities), |_| {
                Ok(())
            })
            .expect("first record");
        let second = corrector
            .observe_record(record(20, b"same", &words, &sequence, &qualities), |_| {
                Ok(())
            })
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
            .observe_record(record(10, b"pair", &words, &sequence, &qualities), |_| {
                Ok(())
            })
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
            .observe_record(record(10, b"first", &words, &sequence, &qualities), |_| {
                Ok(())
            })
            .expect("first record");
        let error = corrector
            .observe_record(record(10, b"second", &words, &sequence, &qualities), |_| {
                Ok(())
            })
            .expect_err("second candidate exceeds test cap");
        assert_eq!(error.category(), ErrorCategory::ResourceLimit);
        assert_eq!(PICARD_WGS_LOCUS_ACCUMULATION_CAP, 100_000);
    }

    #[test]
    fn hs_matches_leftmost_and_equal_start_tie_breaks() {
        let words = [cigar(10, 0)];
        assert_eq!(
            picard_hs_trailing_read_bases_to_clip(FLAG_PAIRED, 100, Some(105), &words)
                .expect("leftmost"),
            5
        );
        assert_eq!(
            picard_hs_trailing_read_bases_to_clip(FLAG_PAIRED, 105, Some(100), &words)
                .expect("rightmost"),
            0
        );
        assert_eq!(
            picard_hs_trailing_read_bases_to_clip(
                FLAG_PAIRED | FLAG_READ1,
                100,
                Some(100),
                &words,
            )
            .expect("first tie"),
            0
        );
        assert_eq!(
            picard_hs_trailing_read_bases_to_clip(FLAG_PAIRED, 100, Some(100), &words)
                .expect("other tie"),
            10
        );
    }

    #[test]
    fn hs_preserves_pinned_insertion_and_extended_cigar_behavior() {
        let insertion = [cigar(5, 0), cigar(2, 1), cigar(5, 0)];
        assert_eq!(
            picard_hs_trailing_read_bases_to_clip(FLAG_PAIRED, 100, Some(103), &insertion)
                .expect("insertion"),
            9
        );
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
    fn wgs_flag_candidate_keeps_supplementary_but_rejects_secondary() {
        assert!(picard_wgs_flag_candidate(0x800));
        assert!(!picard_wgs_flag_candidate(0x100));
        assert!(!picard_wgs_flag_candidate(0x4));
    }
}
