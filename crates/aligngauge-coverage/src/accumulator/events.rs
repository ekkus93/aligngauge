//! Chunk-local delta materialization and sparse cross-chunk event handling.

use aligngauge_core::AlignGaugeError;

use super::CoverageCollector;
use crate::util::{
    apply_depth_change, chunk_size_u64, coverage_overflow, internal_error, resource_error,
    u64_from_usize,
};

impl CoverageCollector {
    pub(super) fn flush_until(&mut self, record_start: u64) -> Result<(), AlignGaugeError> {
        let reference_index = self
            .current_reference_index
            .ok_or_else(|| internal_error("coverage flush has no current reference"))?;
        let reference_length = self.references[reference_index].length;
        if record_start >= reference_length {
            return Err(internal_error(
                "validated record start lies outside the current reference",
            ));
        }

        while self.chunk_start < reference_length && record_start >= self.chunk_end {
            if self.skip_constant_chunks(record_start)? {
                continue;
            }
            self.flush_chunk()?;
        }
        Ok(())
    }

    fn skip_constant_chunks(&mut self, limit: u64) -> Result<bool, AlignGaugeError> {
        if self.active_delta_positions != 0 {
            return Ok(false);
        }
        let reference_index = self
            .current_reference_index
            .ok_or_else(|| internal_error("coverage sparse skip has no current reference"))?;
        let reference_length = self.references[reference_index].length;
        let next_event = self
            .pending_events
            .first_key_value()
            .map_or(limit, |(&position, _)| position.min(limit));
        let chunk = chunk_size_u64(&self.plan)?;
        let aligned_stop = next_event - (next_event % chunk);
        let skip_end = aligned_stop.min(reference_length);
        if skip_end <= self.chunk_start {
            return Ok(false);
        }

        self.accumulate_run(self.current_depth, skip_end - self.chunk_start)?;
        self.chunk_start = skip_end;
        self.chunk_end = reference_length.min(
            self.chunk_start
                .checked_add(chunk)
                .ok_or_else(|| coverage_overflow("chunk end"))?,
        );
        Ok(true)
    }

    fn add_event(&mut self, position: u64, change: i128) -> Result<(), AlignGaugeError> {
        if position < self.chunk_start {
            return Err(internal_error(
                "coverage event falls before the active chunk",
            ));
        }
        if position <= self.chunk_end {
            self.add_local_event(position, change)
        } else {
            self.add_pending_event(position, change)
        }
    }

    fn add_local_event(&mut self, position: u64, change: i128) -> Result<(), AlignGaugeError> {
        let offset = usize::try_from(position - self.chunk_start).map_err(|source| {
            internal_error("coverage delta offset does not fit usize").with_source(source)
        })?;
        let slot = self
            .delta
            .get_mut(offset)
            .ok_or_else(|| internal_error("coverage event falls outside allocated delta vector"))?;
        let was_zero = *slot == 0;
        *slot = slot
            .checked_add(change)
            .ok_or_else(|| coverage_overflow("coverage delta event"))?;
        let is_zero = *slot == 0;
        match (was_zero, is_zero) {
            (true, false) => {
                self.active_delta_positions = self
                    .active_delta_positions
                    .checked_add(1)
                    .ok_or_else(|| coverage_overflow("active delta positions"))?;
            }
            (false, true) => {
                self.active_delta_positions = self
                    .active_delta_positions
                    .checked_sub(1)
                    .ok_or_else(|| internal_error("active delta position count underflowed"))?;
            }
            _ => {}
        }
        Ok(())
    }

    fn add_pending_event(&mut self, position: u64, change: i128) -> Result<(), AlignGaugeError> {
        let previous = self.pending_events.get(&position).copied().unwrap_or(0);
        let combined = previous
            .checked_add(change)
            .ok_or_else(|| coverage_overflow("pending coverage event"))?;
        if combined == 0 {
            self.pending_events.remove(&position);
            return Ok(());
        }
        if previous == 0 && self.pending_events.len() >= self.plan.max_pending_event_positions {
            return Err(resource_error(
                "coverage pending-event budget was exhausted during traversal",
            )
            .with_detail(
                "maximum_pending_event_positions",
                u64_from_usize(
                    self.plan.max_pending_event_positions,
                    "pending event positions",
                )?,
            ));
        }
        self.pending_events.insert(position, combined);
        Ok(())
    }

    fn materialize_pending_events(&mut self) -> Result<(), AlignGaugeError> {
        loop {
            let Some((&position, &change)) = self.pending_events.first_key_value() else {
                break;
            };
            if position > self.chunk_end {
                break;
            }
            self.pending_events.remove(&position);
            self.add_local_event(position, change)?;
        }
        Ok(())
    }

    fn flush_chunk(&mut self) -> Result<(), AlignGaugeError> {
        let reference_index = self
            .current_reference_index
            .ok_or_else(|| internal_error("coverage chunk flush has no current reference"))?;
        let reference_length = self.references[reference_index].length;
        if self.chunk_start >= reference_length {
            return Ok(());
        }
        self.materialize_pending_events()?;
        let chunk_len_u64 = self.chunk_end - self.chunk_start;
        let chunk_len = usize::try_from(chunk_len_u64).map_err(|source| {
            internal_error("coverage chunk length does not fit usize").with_source(source)
        })?;

        if self.active_delta_positions == 0 {
            self.accumulate_run(self.current_depth, chunk_len_u64)?;
        } else {
            for offset in 0..chunk_len {
                let change = self.delta[offset];
                if change != 0 {
                    self.current_depth = apply_depth_change(self.current_depth, change)?;
                    self.delta[offset] = 0;
                    self.active_delta_positions = self
                        .active_delta_positions
                        .checked_sub(1)
                        .ok_or_else(|| internal_error("active delta count underflowed"))?;
                }
                self.accumulate_run(self.current_depth, 1)?;
            }
            let boundary_change = self.delta[chunk_len];
            if boundary_change != 0 {
                self.current_depth = apply_depth_change(self.current_depth, boundary_change)?;
                self.delta[chunk_len] = 0;
                self.active_delta_positions = self
                    .active_delta_positions
                    .checked_sub(1)
                    .ok_or_else(|| internal_error("active delta count underflowed"))?;
            }
            if self.active_delta_positions != 0 {
                return Err(internal_error(
                    "coverage delta positions remained after a complete chunk flush",
                ));
            }
        }

        self.chunk_start = self.chunk_end;
        self.chunk_end = reference_length.min(
            self.chunk_start
                .checked_add(chunk_size_u64(&self.plan)?)
                .ok_or_else(|| coverage_overflow("chunk end"))?,
        );
        Ok(())
    }

    pub(super) fn finish_current_reference(&mut self) -> Result<(), AlignGaugeError> {
        let Some(reference_index) = self.current_reference_index else {
            return Ok(());
        };
        let reference_length = self.references[reference_index].length;
        while self.chunk_start < reference_length {
            if self.active_delta_positions == 0 && self.pending_events.is_empty() {
                self.accumulate_run(self.current_depth, reference_length - self.chunk_start)?;
                self.chunk_start = reference_length;
                self.chunk_end = reference_length;
                break;
            }
            if self.skip_constant_chunks(reference_length)? {
                continue;
            }
            self.flush_chunk()?;
        }
        if self.current_depth != 0 {
            return Err(
                internal_error("coverage depth did not return to zero at reference end")
                    .with_detail(
                        "reference_index",
                        u64_from_usize(reference_index, "reference index")?,
                    )
                    .with_detail("remaining_depth", self.current_depth),
            );
        }
        if !self.pending_events.is_empty() || self.active_delta_positions != 0 {
            return Err(internal_error(
                "coverage events remained after reference finalization",
            ));
        }
        self.finalize_reference(reference_index)?;
        self.current_reference_index = None;
        self.next_reference_index = reference_index
            .checked_add(1)
            .ok_or_else(|| coverage_overflow("reference cursor"))?;
        self.chunk_start = 0;
        self.chunk_end = 0;
        Ok(())
    }
}
