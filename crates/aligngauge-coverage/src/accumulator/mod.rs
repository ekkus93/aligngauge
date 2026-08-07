//! One exact parameterized chunked-delta accumulator for every chunk size.

use std::collections::BTreeMap;

use aligngauge_core::AlignGaugeError;
use aligngauge_hts::{ValidatedHeader, ValidatedRecord};

use crate::cigar::{for_each_coverage_block, record_is_accepted};
use crate::plan::CoverageMemoryPlan;
use crate::util::{chunk_size_u64, coverage_overflow, internal_error, resource_error};

struct ReferenceReduction {
    name: String,
    length: u64,
    accepted_aligned_bases: u64,
    covered_reference_bases: u64,
    uncovered_reference_bases: u64,
    depth_sum: u64,
    finalized: bool,
}

impl ReferenceReduction {
    fn from_header(reference: &aligngauge_hts::ReferenceSequence) -> Self {
        Self {
            name: reference.name().to_owned(),
            length: reference.length(),
            accepted_aligned_bases: 0,
            covered_reference_bases: 0,
            uncovered_reference_bases: 0,
            depth_sum: 0,
            finalized: false,
        }
    }
}

/// Stateful exact coverage collector driven by an already validated record stream.
///
/// This is the collector boundary used by v0.1 release orchestration to feed counters and
/// coverage from the same `BamReader` traversal. It does not open or seek the input itself.
pub struct CoverageCollector {
    thresholds: Vec<u32>,
    plan: CoverageMemoryPlan,
    references: Vec<ReferenceReduction>,
    next_reference_index: usize,
    current_reference_index: Option<usize>,
    chunk_start: u64,
    chunk_end: u64,
    current_depth: u64,
    delta: Vec<i128>,
    active_delta_positions: usize,
    pending_events: BTreeMap<u64, i128>,
    depth_histogram: BTreeMap<u64, u64>,
    total_accepted_aligned_bases: u64,
}

impl CoverageCollector {
    /// Initialize one exact collector from a validated header and precomputed memory plan.
    ///
    /// # Errors
    /// Returns a typed resource or arithmetic failure if the planned delta allocation cannot be
    /// represented safely.
    pub fn new(
        header: &ValidatedHeader,
        thresholds: Vec<u32>,
        plan: CoverageMemoryPlan,
    ) -> Result<Self, AlignGaugeError> {
        let delta_len = plan
            .chunk_size_bases
            .checked_add(1)
            .ok_or_else(|| resource_error("coverage delta length overflowed"))?;
        let mut depth_histogram = BTreeMap::new();
        depth_histogram.insert(0, 0);
        Ok(Self {
            thresholds,
            plan,
            references: header
                .references()
                .iter()
                .map(ReferenceReduction::from_header)
                .collect(),
            next_reference_index: 0,
            current_reference_index: None,
            chunk_start: 0,
            chunk_end: 0,
            current_depth: 0,
            delta: vec![0_i128; delta_len],
            active_delta_positions: 0,
            pending_events: BTreeMap::new(),
            depth_histogram,
            total_accepted_aligned_bases: 0,
        })
    }

    /// Observe one record from the same validated stream used by the other v0.1 collectors.
    ///
    /// # Errors
    /// Returns a typed fatal error for impossible validated state, checked arithmetic failure,
    /// out-of-bounds CIGAR coverage, or bounded-memory exhaustion.
    pub fn observe(&mut self, record: &ValidatedRecord<'_>) -> Result<(), AlignGaugeError> {
        if !record_is_accepted(record.flags()) {
            return Ok(());
        }
        let coordinate = record.coordinate().ok_or_else(|| {
            internal_error("accepted mapped coverage record has no validated coordinate")
                .with_detail("record_index", record.index())
        })?;
        let reference_index = usize::try_from(coordinate.reference_id).map_err(|source| {
            internal_error("validated coverage reference ID does not fit usize")
                .with_detail("record_index", record.index())
                .with_source(source)
        })?;
        let reference_length = self
            .references
            .get(reference_index)
            .ok_or_else(|| {
                internal_error("validated coverage reference ID is absent from header reductions")
                    .with_detail("record_index", record.index())
            })?
            .length;
        let start = u64::try_from(coordinate.position).map_err(|source| {
            internal_error("validated coverage coordinate does not fit u64")
                .with_detail("record_index", record.index())
                .with_source(source)
        })?;
        self.prepare_reference(reference_index, start)?;

        let raw_cigar = record.raw_cigar().ok_or_else(|| {
            internal_error("coverage field plan did not expose validated raw CIGAR")
                .with_detail("record_index", record.index())
        })?;
        let mut accepted_bases = 0_u64;
        for_each_coverage_block(raw_cigar, start, reference_length, |block| {
            accepted_bases = accepted_bases
                .checked_add(block.end - block.start)
                .ok_or_else(|| coverage_overflow("record accepted aligned bases"))?;
            Ok(())
        })?;

        self.total_accepted_aligned_bases = self
            .total_accepted_aligned_bases
            .checked_add(accepted_bases)
            .ok_or_else(|| coverage_overflow("total accepted aligned bases"))?;
        let reference = self
            .references
            .get_mut(reference_index)
            .ok_or_else(|| internal_error("coverage reference reduction disappeared"))?;
        reference.accepted_aligned_bases = reference
            .accepted_aligned_bases
            .checked_add(accepted_bases)
            .ok_or_else(|| coverage_overflow("per-reference accepted aligned bases"))?;

        for_each_coverage_block(raw_cigar, start, reference_length, |block| {
            self.add_event(block.start, 1)?;
            self.add_event(block.end, -1)
        })?;
        Ok(())
    }

    fn prepare_reference(
        &mut self,
        reference_index: usize,
        record_start: u64,
    ) -> Result<(), AlignGaugeError> {
        match self.current_reference_index {
            Some(current) if reference_index < current => {
                return Err(internal_error(
                    "coverage reference order regressed after reader validation",
                ));
            }
            Some(current) if reference_index > current => {
                self.finish_current_reference()?;
                while self.next_reference_index < reference_index {
                    self.finalize_unvisited_reference(self.next_reference_index)?;
                }
                self.start_reference(reference_index)?;
            }
            None => {
                while self.next_reference_index < reference_index {
                    self.finalize_unvisited_reference(self.next_reference_index)?;
                }
                self.start_reference(reference_index)?;
            }
            Some(_) => {}
        }
        self.flush_until(record_start)
    }

    fn start_reference(&mut self, reference_index: usize) -> Result<(), AlignGaugeError> {
        if reference_index != self.next_reference_index {
            return Err(internal_error(
                "coverage reference-finalization cursor is inconsistent",
            ));
        }
        if self.active_delta_positions != 0
            || !self.pending_events.is_empty()
            || self.current_depth != 0
        {
            return Err(internal_error(
                "coverage state leaked across reference transition",
            ));
        }
        let length = self
            .references
            .get(reference_index)
            .ok_or_else(|| internal_error("coverage reference start index is invalid"))?
            .length;
        self.current_reference_index = Some(reference_index);
        self.chunk_start = 0;
        self.chunk_end = length.min(chunk_size_u64(&self.plan)?);
        Ok(())
    }
}

mod events;
mod reduce;
