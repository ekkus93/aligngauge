from pathlib import Path


def replace_exact(path: str, old: str, new: str, count: int = 1) -> None:
    p = Path(path)
    text = p.read_text()
    found = text.count(old)
    if found != count:
        raise SystemExit(f"{path}: expected {count} match(es), found {found}: {old[:100]!r}")
    p.write_text(text.replace(old, new, count))

replace_exact(
    "crates/aligngauge-hts/src/plan.rs",
    "/// Build the Picard 3.4.0 default ALL_READS insert-size plan.",
    "/// Build the Picard 3.4.0 default `ALL_READS` insert-size plan.",
)

path = "crates/aligngauge-metrics/src/picard.rs"
replace_exact(
    path,
    '''    pub fn finish(self) -> PicardAlignmentSummaryReport {
        let first = self.first.finish(PicardAlignmentCategory::FirstOfPair);
        let second = self.second.finish(PicardAlignmentCategory::SecondOfPair);
        let mut pair = self.pair.finish(PicardAlignmentCategory::Pair);
        pair.bad_cycles = first.bad_cycles.saturating_add(second.bad_cycles);
        let unpaired = self.unpaired.finish(PicardAlignmentCategory::Unpaired);

        let mut rows = Vec::with_capacity(4);
        if first.total_reads > 0 {
            rows.push(first);
            rows.push(second);
            rows.push(pair);
        }
        if unpaired.total_reads > 0 || rows.is_empty() {
            rows.push(unpaired);
        }
        PicardAlignmentSummaryReport { rows }
    }
''',
    '''    /// Finalize category ordering and Picard's paired `BAD_CYCLES` override.
    ///
    /// # Errors
    /// Returns a typed error if combining the paired bad-cycle counts overflows.
    pub fn finish(self) -> Result<PicardAlignmentSummaryReport, AlignGaugeError> {
        let first = self.first.finish(PicardAlignmentCategory::FirstOfPair);
        let second = self.second.finish(PicardAlignmentCategory::SecondOfPair);
        let mut pair = self.pair.finish(PicardAlignmentCategory::Pair);
        pair.bad_cycles = checked_add(
            first.bad_cycles,
            second.bad_cycles,
            "alignment.pair_bad_cycles",
        )?;
        let unpaired = self.unpaired.finish(PicardAlignmentCategory::Unpaired);

        let mut rows = Vec::with_capacity(4);
        if first.total_reads > 0 {
            rows.push(first);
            rows.push(second);
            rows.push(pair);
        }
        if unpaired.total_reads > 0 || rows.is_empty() {
            rows.push(unpaired);
        }
        Ok(PicardAlignmentSummaryReport { rows })
    }
''',
)
replace_exact(
    path,
    "        seen = seen.saturating_add(*value);",
    '''        seen = seen
            .checked_add(*value)
            .expect("cumulative histogram count cannot exceed the checked source count");''',
    count=2,
)
replace_exact(path, "    Ok(collector.finish())\n", "    collector.finish()\n")

# Clippy documentation normalization without suppressions.
for old, new in [
    ("/// Exact default ALL_READS insert-size compatibility profile.", "/// Exact default `ALL_READS` insert-size compatibility profile."),
    ("/// Picard alignment-summary category emitted by the ALL_READS collector.", "/// Picard alignment-summary category emitted by the `ALL_READS` collector."),
    ("    /// Finalize category ordering and Picard's paired BAD_CYCLES override.\n    #[must_use]\n", ""),
    ("/// One Picard InsertSizeMetrics ALL_READS row plus its trimmed orientation histogram.", "/// One Picard `InsertSizeMetrics` `ALL_READS` row plus its trimmed orientation histogram."),
    ("    /// Render the default ALL_READS InsertSizeMetrics table and trimmed histogram surface.", "    /// Render the default `ALL_READS` `InsertSizeMetrics` table and trimmed histogram surface."),
    ("/// Checked single-pass collector for Picard 3.4.0 default ALL_READS insert-size metrics.", "/// Checked single-pass collector for Picard 3.4.0 default `ALL_READS` insert-size metrics."),
    ("/// Analyze one BAM with the exact default ALL_READS Picard insert-size field plan.", "/// Analyze one BAM with the exact default `ALL_READS` Picard insert-size field plan."),
]:
    replace_exact(path, old, new)

# Express Java truncating casts through checked decimal conversion so Rust has no unchecked cast.
replace_exact(
    path,
    "    let trim_width = width_float.trunc() as u32;\n",
    '''    let trim_width = truncating_f64_to_u32(width_float).ok_or_else(|| {
        overflow("insert.histogram_width")
    })?;
''',
)
replace_exact(path, "        if low != high {\n", "        if low.to_bits() != high.to_bits() {\n")
replace_exact(
    path,
    "            if window.iter().filter(|base| **base == b'N').count() <= MAX_ADAPTER_ERRORS {\n",
    '''            let ambiguous = window.iter().fold(0_usize, |count, base| {
                count + usize::from(*base == b'N')
            });
            if ambiguous <= MAX_ADAPTER_ERRORS {
''',
)
replace_exact(
    path,
    "    (low.unwrap_or(0.0) + high.unwrap_or(0.0)) / 2.0\n",
    "    f64::midpoint(low.unwrap_or(0.0), high.unwrap_or(0.0))\n",
    count=2,
)
replace_exact(path, "    if count % 2 == 0 {\n", "    if count.is_multiple_of(2) {\n")
replace_exact(
    path,
    '''    } else {
        value.trunc() as i32
    }
}
''',
    '''    } else {
        value.trunc().to_string().parse::<i32>().unwrap_or_else(|_| {
            if value.is_sign_negative() { i32::MIN } else { i32::MAX }
        })
    }
}

fn truncating_f64_to_u32(value: f64) -> Option<u32> {
    if !value.is_finite() || value < 0.0 || value > f64::from(u32::MAX) {
        return None;
    }
    value.trunc().to_string().parse::<u32>().ok()
}
''',
)
