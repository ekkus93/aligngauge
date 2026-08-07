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
    '''    pub fn finish(self) -> Result<PicardAlignmentSummaryReport, AlignGaugeError> {
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
