#!/usr/bin/env python3
from pathlib import Path

p = Path("crates/aligngauge-metrics/src/samtools_stats.rs")
text = p.read_text()

text = text.replace("use crate::SAMTOOLS_VERSION;\n\n", "")
text = text.replace("/// Pinned MultiQC parser version.", "/// Pinned `MultiQC` parser version.")
text = text.replace(
    "    /// Observe one record from a `FieldPlan::samtools_stats()` reader.\n    pub fn observe",
    "    /// Observe one record from a `FieldPlan::samtools_stats()` reader.\n    ///\n    /// # Errors\n    ///\n    /// Returns a typed error when a required planned field is unavailable or checked arithmetic fails.\n    pub fn observe",
)
text = text.replace(
    "    /// Finalize the canonical report with the pinned Samtools 1.24 output arithmetic.\n    pub fn finish",
    "    /// Finalize the canonical report with the pinned Samtools 1.24 output arithmetic.\n    ///\n    /// # Errors\n    ///\n    /// Returns a typed error when checked arithmetic or insert-size reduction cannot be represented.\n    pub fn finish",
)
text = text.replace(
    "/// Analyze one BAM with the exact Milestone 10 Samtools stats field plan.\npub fn analyze_samtools_stats_bam",
    "/// Analyze one BAM with the exact Milestone 10 Samtools stats field plan.\n///\n/// # Errors\n///\n/// Returns a typed input, validation, compatibility, or checked-arithmetic error.\npub fn analyze_samtools_stats_bam",
)

start = text.index("impl SamtoolsStatsReport {")
end = text.index("\n}\n\nfn sn(", start) + 3
replacement = r'''impl SamtoolsStatsReport {
    /// Render only the compatibility surface frozen by ADR-0007.
    #[must_use]
    pub fn render_samtools_stats(&self) -> String {
        let mut out = String::new();
        Self::render_header(&mut out);
        self.render_summary_counts(&mut out);
        self.render_summary_bases(&mut out);
        self.render_summary_derived(&mut out);
        self.render_insert_sizes(&mut out);
        out
    }

    fn render_header(out: &mut String) {
        writeln!(
            out,
            "# This file was produced by samtools stats (1.24+htslib-1.24) and can be plotted using plot-bamstats"
        )
        .expect("String write cannot fail");
        writeln!(
            out,
            "# AlignGauge compatibility projection: {SAMTOOLS_STATS_PROFILE}; supported sections: SN,IS"
        )
        .expect("String write cannot fail");
        writeln!(out, "# Summary Numbers. Use `grep ^SN | cut -f 2-` to extract this part.")
            .expect("String write cannot fail");
    }

    fn render_summary_counts(&self, out: &mut String) {
        sn(out, "raw total sequences:", self.raw_total_sequences, Some("excluding supplementary and secondary reads"));
        sn(out, "filtered sequences:", self.filtered_sequences, None);
        sn(out, "sequences:", self.sequences, None);
        writeln!(
            out,
            "SN\tis sorted:\t{}\t# {} by coordinate",
            u8::from(self.is_sorted),
            if self.is_sorted { "sorted" } else { "not sorted" }
        )
        .expect("String write cannot fail");
        sn(out, "1st fragments:", self.first_fragments, None);
        sn(out, "last fragments:", self.last_fragments, None);
        sn(out, "reads mapped:", self.reads_mapped, None);
        sn(out, "reads mapped and paired:", self.reads_mapped_and_paired, Some("paired-end technology bit set + both mates mapped"));
        sn(out, "reads unmapped:", self.reads_unmapped, None);
        sn(out, "reads properly paired:", self.reads_properly_paired, Some("proper-pair bit set"));
        sn(out, "reads paired:", self.reads_paired, Some("paired-end technology bit set"));
        sn(out, "reads duplicated:", self.reads_duplicated, Some("PCR or optical duplicate bit set"));
        sn(out, "reads MQ0:", self.reads_mq0, Some("mapped and MQ=0"));
        sn(out, "reads QC failed:", self.reads_qc_failed, None);
        sn(out, "non-primary alignments:", self.non_primary_alignments, None);
        sn(out, "supplementary alignments:", self.supplementary_alignments, None);
    }

    fn render_summary_bases(&self, out: &mut String) {
        sn(out, "total length:", self.total_length, Some("ignores clipping"));
        sn(out, "total first fragment length:", self.total_first_fragment_length, Some("ignores clipping"));
        sn(out, "total last fragment length:", self.total_last_fragment_length, Some("ignores clipping"));
        sn(out, "bases mapped:", self.bases_mapped, Some("ignores clipping"));
        sn(out, "bases mapped (cigar):", self.bases_mapped_cigar, Some("more accurate"));
        sn(out, "bases trimmed:", self.bases_trimmed, None);
        sn(out, "bases duplicated:", self.bases_duplicated, None);
        sn(out, "mismatches:", self.mismatches, Some("from NM fields"));
        sn_text(out, "error rate:", &self.error_rate, Some("mismatches / bases mapped (cigar)"));
        sn_text(out, "average length:", &self.average_length, None);
        sn_text(out, "average first fragment length:", &self.average_first_fragment_length, None);
        sn_text(out, "average last fragment length:", &self.average_last_fragment_length, None);
        sn(out, "maximum length:", self.maximum_length, None);
        sn(out, "maximum first fragment length:", self.maximum_first_fragment_length, None);
        sn(out, "maximum last fragment length:", self.maximum_last_fragment_length, None);
        sn_text(out, "average quality:", &self.average_quality, None);
    }

    fn render_summary_derived(&self, out: &mut String) {
        sn_text(out, "insert size average:", &self.insert_size_average, None);
        sn_text(out, "insert size standard deviation:", &self.insert_size_standard_deviation, None);
        sn(out, "inward oriented pairs:", self.inward_oriented_pairs, None);
        sn(out, "outward oriented pairs:", self.outward_oriented_pairs, None);
        sn(out, "pairs with other orientation:", self.pairs_with_other_orientation, None);
        sn(out, "pairs on different chromosomes:", self.pairs_on_different_chromosomes, None);
        sn_text(out, "percentage of properly paired reads (%):", &self.percentage_properly_paired_reads, None);
    }

    fn render_insert_sizes(&self, out: &mut String) {
        writeln!(out, "# Insert sizes. Use `grep ^IS | cut -f 2-` to extract this part. The columns are: insert size, pairs total, inward oriented pairs, outward oriented pairs, other pairs").expect("String write cannot fail");
        for row in &self.insert_sizes {
            writeln!(out, "IS\t{}\t{}\t{}\t{}\t{}", row.insert_size, row.pairs_total, row.inward, row.outward, row.other).expect("String write cannot fail");
        }
    }
}
'''
text = text[:start] + replacement + text[end:]

replacements = {
    "self.mismatches as f32": "u64_to_f32(self.mismatches)",
    "self.bases_mapped_cigar as f32": "u64_to_f32(self.bases_mapped_cigar)",
    "self.quality_sum as f64": "u64_to_f64(self.quality_sum)",
    "self.total_length as f64": "u64_to_f64(self.total_length)",
    "(proper_numerator as f32 / sequences as f32) as f64": "f64::from(u64_to_f32(proper_numerator) / u64_to_f32(sequences))",
    "count as f64": "u64_to_f64(count)",
    "bulk as f64": "u64_to_f64(bulk)",
    "all_pairs as f64": "u64_to_f64(all_pairs)",
    "denominator as f64": "u64_to_f64(denominator)",
    "numerator as f32": "u64_to_f32(numerator)",
    "denominator as f32": "u64_to_f32(denominator)",
}
for old, new in replacements.items():
    text = text.replace(old, new)

needle = "max(unclipped)\n"
if text.count(needle) != 2:
    raise SystemExit(f"expected two fragment max expressions without semicolons, found {text.count(needle)}")
text = text.replace(needle, "max(unclipped);\n")

marker = "fn format_zero_decimals(numerator: u64, denominator: u64) -> String {"
helpers = r'''fn u64_to_f32(value: u64) -> f32 {
    value
        .to_string()
        .parse::<f32>()
        .expect("every u64 decimal is representable as a finite f32")
}

fn u64_to_f64(value: u64) -> f64 {
    value
        .to_string()
        .parse::<f64>()
        .expect("every u64 decimal is representable as a finite f64")
}

'''
if text.count(marker) != 1:
    raise SystemExit(f"format helper marker count={text.count(marker)}")
text = text.replace(marker, helpers + marker, 1)
p.write_text(text)
