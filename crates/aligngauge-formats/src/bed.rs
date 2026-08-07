//! Fail-closed BED parsing and deterministic target normalization.

use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use aligngauge_core::{AlignGaugeError, ErrorCategory};
use sha2::{Digest, Sha256};

/// Stable target-normalization profile name.
pub const BED_NORMALIZATION_PROFILE: &str = "aligngauge-bed-v0.3";

/// One authoritative sequence dictionary entry.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SequenceContig {
    /// Exact sequence name.
    pub name: String,
    /// Declared sequence length.
    pub length: u64,
}

/// Ordered authoritative sequence dictionary used to validate target intervals.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SequenceDictionary {
    contigs: Vec<SequenceContig>,
    by_name: BTreeMap<String, usize>,
}

impl SequenceDictionary {
    /// Construct an ordered sequence dictionary.
    ///
    /// # Errors
    ///
    /// Returns `configuration` if a sequence name is empty or duplicated.
    pub fn new(contigs: Vec<SequenceContig>) -> Result<Self, AlignGaugeError> {
        let mut by_name = BTreeMap::new();
        for (index, contig) in contigs.iter().enumerate() {
            if contig.name.is_empty() {
                return Err(AlignGaugeError::new(
                    ErrorCategory::Configuration,
                    "sequence dictionary contains an empty contig name",
                ));
            }
            if by_name.insert(contig.name.clone(), index).is_some() {
                return Err(AlignGaugeError::new(
                    ErrorCategory::Configuration,
                    "sequence dictionary contains a duplicate contig name",
                )
                .with_detail("contig", contig.name.clone()));
            }
        }
        Ok(Self { contigs, by_name })
    }

    /// Ordered sequence entries.
    #[must_use]
    pub fn contigs(&self) -> &[SequenceContig] {
        &self.contigs
    }

    fn lookup(&self, name: &str) -> Option<(usize, &SequenceContig)> {
        self.by_name
            .get(name)
            .copied()
            .map(|index| (index, &self.contigs[index]))
    }
}

/// Exact identity of the original target BED bytes.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TargetFileIdentity {
    /// User-supplied path when parsing from a file.
    pub path: Option<String>,
    /// Exact original byte size.
    pub size_bytes: u64,
    /// SHA-256 of the original bytes before normalization.
    pub sha256: String,
    /// Number of accepted BED interval records.
    pub source_interval_count: u64,
}

/// Parser normalization/skip counters.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct BedParseStats {
    /// Blank lines skipped.
    pub blank_lines_skipped: u64,
    /// Comment lines skipped.
    pub comment_lines_skipped: u64,
    /// UCSC `track` lines skipped.
    pub track_lines_skipped: u64,
    /// UCSC `browser` lines skipped.
    pub browser_lines_skipped: u64,
    /// CRLF line endings normalized.
    pub crlf_lines_normalized: u64,
}

/// One validated source BED interval.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BedSourceInterval {
    /// Stable accepted-record index in source-file order.
    pub source_index: u64,
    /// One-based source line number.
    pub line_number: u64,
    /// Exact contig name.
    pub contig: String,
    /// Authoritative contig order.
    pub contig_index: usize,
    /// Authoritative contig length.
    pub contig_length: u64,
    /// Zero-based inclusive start.
    pub start: u64,
    /// Zero-based exclusive end.
    pub end: u64,
    /// Optional BED field 4.
    pub name: Option<String>,
    /// BED fields 5 through 12, preserved but uninterpreted.
    pub extra_fields: Vec<String>,
}

/// Validated BED parse result before interval normalization.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BedParseResult {
    /// Exact original file identity.
    pub identity: TargetFileIdentity,
    /// Accepted source intervals in source-file order.
    pub intervals: Vec<BedSourceInterval>,
    /// Parser normalization/skip statistics.
    pub stats: BedParseStats,
}

/// Target normalization configuration.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct TargetNormalizationConfig {
    /// Symmetric flank applied to non-empty source intervals.
    pub flank_bases: u64,
}

/// One deterministic aggregate target interval.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MergedTargetInterval {
    /// Exact contig name.
    pub contig: String,
    /// Authoritative contig order.
    pub contig_index: usize,
    /// Zero-based inclusive start after flanking.
    pub start: u64,
    /// Zero-based exclusive end after flanking.
    pub end: u64,
    /// Source interval identities contributing to this aggregate interval.
    pub source_interval_indices: Vec<u64>,
}

/// Provenance-ready deterministic normalization summary.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TargetNormalizationProvenance {
    /// Stable normalization profile.
    pub profile: String,
    /// Configured symmetric flank.
    pub flank_bases: u64,
    /// Number of accepted source intervals.
    pub source_intervals: u64,
    /// Number of non-empty source intervals contributing territory.
    pub positive_source_intervals: u64,
    /// Number of valid zero-length source intervals.
    pub empty_source_intervals: u64,
    /// Number of positions changed by deterministic sorting.
    pub reordered_positions: u64,
    /// Number of overlap merges.
    pub overlap_merges: u64,
    /// Number of directly adjacent merges.
    pub adjacency_merges: u64,
    /// Number of intervals whose requested left flank reached coordinate zero.
    pub left_flank_clips: u64,
    /// Number of intervals whose requested right flank reached the contig end.
    pub right_flank_clips: u64,
    /// Number of final aggregate intervals.
    pub merged_intervals: u64,
    /// Exact union territory represented by final aggregate intervals.
    pub aggregate_territory_bases: u64,
    /// Parser normalization/skip statistics.
    pub parse_stats: BedParseStats,
}

impl TargetNormalizationProvenance {
    /// Render compact deterministic actions suitable for the canonical provenance list.
    #[must_use]
    pub fn actions(&self, identity: &TargetFileIdentity) -> Vec<String> {
        vec![
            format!("targets:profile={}", self.profile),
            format!("targets:sha256={}", identity.sha256),
            format!("targets:size_bytes={}", identity.size_bytes),
            format!("targets:source_intervals={}", self.source_intervals),
            format!("targets:flank_bases={}", self.flank_bases),
            format!(
                "targets:positive_source_intervals={}",
                self.positive_source_intervals
            ),
            format!(
                "targets:empty_source_intervals={}",
                self.empty_source_intervals
            ),
            format!("targets:reordered_positions={}", self.reordered_positions),
            format!("targets:overlap_merges={}", self.overlap_merges),
            format!("targets:adjacency_merges={}", self.adjacency_merges),
            format!("targets:left_flank_clips={}", self.left_flank_clips),
            format!("targets:right_flank_clips={}", self.right_flank_clips),
            format!("targets:merged_intervals={}", self.merged_intervals),
            format!(
                "targets:aggregate_territory_bases={}",
                self.aggregate_territory_bases
            ),
            format!(
                "targets:blank_lines_skipped={}",
                self.parse_stats.blank_lines_skipped
            ),
            format!(
                "targets:comment_lines_skipped={}",
                self.parse_stats.comment_lines_skipped
            ),
            format!(
                "targets:track_lines_skipped={}",
                self.parse_stats.track_lines_skipped
            ),
            format!(
                "targets:browser_lines_skipped={}",
                self.parse_stats.browser_lines_skipped
            ),
            format!(
                "targets:crlf_lines_normalized={}",
                self.parse_stats.crlf_lines_normalized
            ),
        ]
    }
}

/// Fully normalized target set consumed by later targeted collectors.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TargetSet {
    /// Exact target-file identity.
    pub identity: TargetFileIdentity,
    /// Original validated intervals in source order.
    pub source_intervals: Vec<BedSourceInterval>,
    /// Deterministic aggregate target territory.
    pub merged_intervals: Vec<MergedTargetInterval>,
    /// Deterministic normalization provenance.
    pub normalization: TargetNormalizationProvenance,
}

impl TargetSet {
    /// Compact actions ready to append to canonical run provenance.
    #[must_use]
    pub fn provenance_actions(&self) -> Vec<String> {
        self.normalization.actions(&self.identity)
    }
}

/// Parse BED bytes against an authoritative sequence dictionary.
///
/// The target identity checksum covers the exact supplied bytes.
///
/// # Errors
///
/// Returns `target_format` for malformed BED data, `target_contig` for unknown
/// contigs, or `configuration` if a required invariant cannot be represented.
pub fn parse_bed_bytes(
    bytes: &[u8],
    dictionary: &SequenceDictionary,
) -> Result<BedParseResult, AlignGaugeError> {
    parse_bed(bytes, None, dictionary)
}

/// Read and parse a local BED file against an authoritative sequence dictionary.
///
/// # Errors
///
/// Returns `input_not_found` when the path does not exist, `target_format` when
/// reading or parsing fails, or `target_contig` for unknown contigs.
pub fn parse_bed_path(
    path: &Path,
    dictionary: &SequenceDictionary,
) -> Result<BedParseResult, AlignGaugeError> {
    let bytes = fs::read(path).map_err(|source| {
        let category = if source.kind() == ErrorKind::NotFound {
            ErrorCategory::InputNotFound
        } else {
            ErrorCategory::TargetFormat
        };
        AlignGaugeError::new(category, "failed to read target BED file")
            .with_sensitive_detail("target_path", path.display().to_string())
            .with_source(source)
    })?;
    parse_bed(&bytes, Some(path), dictionary)
}

/// Normalize validated targets with deterministic sorting, flanking, and merging.
///
/// # Errors
///
/// Returns `resource_limit` if aggregate target territory cannot be represented
/// as a checked `u64`, or `internal_invariant` if validated interval arithmetic
/// would otherwise overflow.
pub fn normalize_targets(
    parsed: BedParseResult,
    config: TargetNormalizationConfig,
) -> Result<TargetSet, AlignGaugeError> {
    let mut expanded = Vec::with_capacity(parsed.intervals.len());
    let mut empty_source_intervals = 0_u64;
    let mut left_flank_clips = 0_u64;
    let mut right_flank_clips = 0_u64;

    for interval in &parsed.intervals {
        if interval.start == interval.end {
            bump(&mut empty_source_intervals)?;
            continue;
        }
        let left_applied = config.flank_bases.min(interval.start);
        if left_applied < config.flank_bases {
            bump(&mut left_flank_clips)?;
        }
        let right_capacity = interval
            .contig_length
            .checked_sub(interval.end)
            .ok_or_else(|| {
                AlignGaugeError::new(
                    ErrorCategory::InternalInvariant,
                    "validated target interval exceeds its contig length",
                )
            })?;
        let right_applied = config.flank_bases.min(right_capacity);
        if right_applied < config.flank_bases {
            bump(&mut right_flank_clips)?;
        }
        let end = interval.end.checked_add(right_applied).ok_or_else(|| {
            AlignGaugeError::new(
                ErrorCategory::InternalInvariant,
                "target flank arithmetic overflowed",
            )
        })?;
        expanded.push(ExpandedInterval {
            source_index: interval.source_index,
            contig: interval.contig.clone(),
            contig_index: interval.contig_index,
            start: interval.start - left_applied,
            end,
        });
    }

    let original_order: Vec<u64> = expanded
        .iter()
        .map(|interval| interval.source_index)
        .collect();
    expanded.sort_by(|left, right| {
        left.contig_index
            .cmp(&right.contig_index)
            .then_with(|| left.start.cmp(&right.start))
            .then_with(|| left.end.cmp(&right.end))
            .then_with(|| left.source_index.cmp(&right.source_index))
    });
    let reordered_positions = count_reordered_positions(&original_order, &expanded)?;

    let (mut merged_intervals, overlap_merges, adjacency_merges) = merge_expanded(expanded)?;
    for interval in &mut merged_intervals {
        interval.source_interval_indices.sort_unstable();
    }
    let aggregate_territory_bases = aggregate_territory(&merged_intervals)?;

    let source_intervals = u64::try_from(parsed.intervals.len()).map_err(|_| {
        AlignGaugeError::new(
            ErrorCategory::ResourceLimit,
            "target interval count exceeds supported range",
        )
    })?;
    let merged_count = u64::try_from(merged_intervals.len()).map_err(|_| {
        AlignGaugeError::new(
            ErrorCategory::ResourceLimit,
            "merged target interval count exceeds supported range",
        )
    })?;
    let positive_source_intervals = source_intervals
        .checked_sub(empty_source_intervals)
        .ok_or_else(|| {
            AlignGaugeError::new(
                ErrorCategory::InternalInvariant,
                "target interval accounting underflowed",
            )
        })?;

    let normalization = TargetNormalizationProvenance {
        profile: BED_NORMALIZATION_PROFILE.to_owned(),
        flank_bases: config.flank_bases,
        source_intervals,
        positive_source_intervals,
        empty_source_intervals,
        reordered_positions,
        overlap_merges,
        adjacency_merges,
        left_flank_clips,
        right_flank_clips,
        merged_intervals: merged_count,
        aggregate_territory_bases,
        parse_stats: parsed.stats,
    };

    Ok(TargetSet {
        identity: parsed.identity,
        source_intervals: parsed.intervals,
        merged_intervals,
        normalization,
    })
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ExpandedInterval {
    source_index: u64,
    contig: String,
    contig_index: usize,
    start: u64,
    end: u64,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum SkippedLine {
    Blank,
    Comment,
    Track,
    Browser,
}

fn parse_bed(
    bytes: &[u8],
    path: Option<&Path>,
    dictionary: &SequenceDictionary,
) -> Result<BedParseResult, AlignGaugeError> {
    let text = std::str::from_utf8(bytes).map_err(|source| {
        AlignGaugeError::new(
            ErrorCategory::TargetFormat,
            "target BED file is not valid UTF-8",
        )
        .with_source(source)
    })?;
    let size_bytes = u64::try_from(bytes.len()).map_err(|_| {
        AlignGaugeError::new(
            ErrorCategory::ResourceLimit,
            "target BED file size exceeds supported range",
        )
    })?;
    let mut stats = BedParseStats::default();
    let mut intervals = Vec::new();

    for (line_index, raw_line) in text.split_inclusive('\n').enumerate() {
        let line_number = u64::try_from(line_index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| {
                AlignGaugeError::new(
                    ErrorCategory::ResourceLimit,
                    "target BED line count exceeds supported range",
                )
            })?;
        let had_crlf = raw_line.ends_with("\r\n");
        if had_crlf {
            bump(&mut stats.crlf_lines_normalized)?;
        }
        let line = raw_line
            .strip_suffix('\n')
            .unwrap_or(raw_line)
            .strip_suffix('\r')
            .unwrap_or_else(|| raw_line.strip_suffix('\n').unwrap_or(raw_line))
            .trim_end_matches([' ', '\t']);

        if let Some(kind) = skipped_line_kind(line) {
            record_skip(&mut stats, kind)?;
            continue;
        }
        let fields = split_fields(line, line_number)?;
        let interval = parse_interval(&fields, line_number, intervals.len(), dictionary)?;
        intervals.push(interval);
    }

    let source_interval_count = u64::try_from(intervals.len()).map_err(|_| {
        AlignGaugeError::new(
            ErrorCategory::ResourceLimit,
            "target interval count exceeds supported range",
        )
    })?;
    Ok(BedParseResult {
        identity: TargetFileIdentity {
            path: path.map(|value| value.display().to_string()),
            size_bytes,
            sha256: sha256_hex(bytes),
            source_interval_count,
        },
        intervals,
        stats,
    })
}

fn skipped_line_kind(line: &str) -> Option<SkippedLine> {
    let trimmed = line.trim_start_matches([' ', '\t']);
    if trimmed.is_empty() {
        return Some(SkippedLine::Blank);
    }
    if trimmed.starts_with('#') {
        return Some(SkippedLine::Comment);
    }
    match trimmed.split_ascii_whitespace().next() {
        Some("track") => Some(SkippedLine::Track),
        Some("browser") => Some(SkippedLine::Browser),
        _ => None,
    }
}

fn split_fields(line: &str, line_number: u64) -> Result<Vec<&str>, AlignGaugeError> {
    let fields: Vec<&str> = if line.contains('\t') {
        line.split('\t').map(str::trim).collect()
    } else {
        line.split_ascii_whitespace().collect()
    };
    if fields.iter().any(|field| field.is_empty()) {
        return Err(target_format_error(
            line_number,
            "BED interval contains an empty field",
        ));
    }
    if !(3..=12).contains(&fields.len()) {
        return Err(target_format_error(
            line_number,
            "BED interval must contain 3 through 12 fields",
        )
        .with_detail(
            "field_count",
            u64::try_from(fields.len()).unwrap_or(u64::MAX),
        ));
    }
    Ok(fields)
}

fn parse_interval(
    fields: &[&str],
    line_number: u64,
    accepted_count: usize,
    dictionary: &SequenceDictionary,
) -> Result<BedSourceInterval, AlignGaugeError> {
    let start = parse_coordinate(fields[1], line_number, "start")?;
    let end = parse_coordinate(fields[2], line_number, "end")?;
    if start > end {
        return Err(
            target_format_error(line_number, "BED start is greater than BED end")
                .with_detail("start", start)
                .with_detail("end", end),
        );
    }
    let (contig_index, contig) = dictionary.lookup(fields[0]).ok_or_else(|| {
        AlignGaugeError::new(
            ErrorCategory::TargetContig,
            "BED interval names a contig absent from the authoritative sequence dictionary",
        )
        .with_detail("line_number", line_number)
        .with_detail("contig", fields[0].to_owned())
    })?;
    if end > contig.length {
        return Err(target_format_error(
            line_number,
            "BED interval exceeds authoritative contig length",
        )
        .with_detail("contig", contig.name.clone())
        .with_detail("contig_length", contig.length)
        .with_detail("end", end));
    }
    let source_index = u64::try_from(accepted_count).map_err(|_| {
        AlignGaugeError::new(
            ErrorCategory::ResourceLimit,
            "target interval count exceeds supported range",
        )
    })?;
    Ok(BedSourceInterval {
        source_index,
        line_number,
        contig: contig.name.clone(),
        contig_index,
        contig_length: contig.length,
        start,
        end,
        name: fields.get(3).map(|value| (*value).to_owned()),
        extra_fields: fields
            .iter()
            .skip(4)
            .map(|value| (*value).to_owned())
            .collect(),
    })
}

fn parse_coordinate(
    token: &str,
    line_number: u64,
    coordinate: &'static str,
) -> Result<u64, AlignGaugeError> {
    token.parse::<u64>().map_err(|source| {
        target_format_error(
            line_number,
            "BED coordinate must be a non-negative integer representable as u64",
        )
        .with_detail("coordinate", coordinate)
        .with_source(source)
    })
}

fn target_format_error(line_number: u64, message: &'static str) -> AlignGaugeError {
    AlignGaugeError::new(ErrorCategory::TargetFormat, message)
        .with_detail("line_number", line_number)
}

fn record_skip(stats: &mut BedParseStats, kind: SkippedLine) -> Result<(), AlignGaugeError> {
    match kind {
        SkippedLine::Blank => bump(&mut stats.blank_lines_skipped),
        SkippedLine::Comment => bump(&mut stats.comment_lines_skipped),
        SkippedLine::Track => bump(&mut stats.track_lines_skipped),
        SkippedLine::Browser => bump(&mut stats.browser_lines_skipped),
    }
}

fn bump(value: &mut u64) -> Result<(), AlignGaugeError> {
    *value = value.checked_add(1).ok_or_else(|| {
        AlignGaugeError::new(
            ErrorCategory::ResourceLimit,
            "target normalization counter overflowed",
        )
    })?;
    Ok(())
}

fn count_reordered_positions(
    original_order: &[u64],
    expanded: &[ExpandedInterval],
) -> Result<u64, AlignGaugeError> {
    let count = original_order
        .iter()
        .zip(expanded)
        .filter(|(source_index, interval)| **source_index != interval.source_index)
        .count();
    u64::try_from(count).map_err(|_| {
        AlignGaugeError::new(
            ErrorCategory::ResourceLimit,
            "target reorder count exceeds supported range",
        )
    })
}

fn merge_expanded(
    expanded: Vec<ExpandedInterval>,
) -> Result<(Vec<MergedTargetInterval>, u64, u64), AlignGaugeError> {
    let mut merged: Vec<MergedTargetInterval> = Vec::new();
    let mut overlap_merges = 0_u64;
    let mut adjacency_merges = 0_u64;
    for interval in expanded {
        if let Some(last) = merged.last_mut()
            && last.contig_index == interval.contig_index
            && interval.start <= last.end
        {
            if interval.start == last.end {
                bump(&mut adjacency_merges)?;
            } else {
                bump(&mut overlap_merges)?;
            }
            last.end = last.end.max(interval.end);
            last.source_interval_indices.push(interval.source_index);
            continue;
        }
        merged.push(MergedTargetInterval {
            contig: interval.contig,
            contig_index: interval.contig_index,
            start: interval.start,
            end: interval.end,
            source_interval_indices: vec![interval.source_index],
        });
    }
    Ok((merged, overlap_merges, adjacency_merges))
}

fn aggregate_territory(intervals: &[MergedTargetInterval]) -> Result<u64, AlignGaugeError> {
    intervals.iter().try_fold(0_u64, |total, interval| {
        let length = interval.end.checked_sub(interval.start).ok_or_else(|| {
            AlignGaugeError::new(
                ErrorCategory::InternalInvariant,
                "normalized target interval is reversed",
            )
        })?;
        total.checked_add(length).ok_or_else(|| {
            AlignGaugeError::new(
                ErrorCategory::ResourceLimit,
                "aggregate target territory exceeds supported u64 range",
            )
        })
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dictionary() -> SequenceDictionary {
        SequenceDictionary::new(vec![
            SequenceContig {
                name: "chr1".to_owned(),
                length: 100,
            },
            SequenceContig {
                name: "chr2".to_owned(),
                length: 80,
            },
        ])
        .expect("test sequence dictionary is valid")
    }

    #[test]
    fn parses_vendor_style_bed_and_skips_directives() {
        let bytes = b"track name=Panel\r\nbrowser position chr1:1-100\r\n# comment\r\n\r\nchr1\t10\t20\tEXON_A\t0\t+\r\nchr2 5 8 EXON_B 100 -\r\n";
        let parsed = parse_bed_bytes(bytes, &dictionary()).expect("BED should parse");
        assert_eq!(parsed.intervals.len(), 2);
        assert_eq!(parsed.intervals[0].name.as_deref(), Some("EXON_A"));
        assert_eq!(parsed.intervals[0].extra_fields, ["0", "+"]);
        assert_eq!(parsed.intervals[1].name.as_deref(), Some("EXON_B"));
        assert_eq!(parsed.stats.track_lines_skipped, 1);
        assert_eq!(parsed.stats.browser_lines_skipped, 1);
        assert_eq!(parsed.stats.comment_lines_skipped, 1);
        assert_eq!(parsed.stats.blank_lines_skipped, 1);
        assert_eq!(parsed.stats.crlf_lines_normalized, 6);
    }

    #[test]
    fn accepts_bed3_through_bed12_and_preserves_optional_fields() {
        let bytes = b"chr1\t0\t1\nchr1\t1\t2\tname\t1\t+\t1\t2\t255,0,0\t1\t1\t0\n";
        let parsed = parse_bed_bytes(bytes, &dictionary()).expect("BED should parse");
        assert_eq!(parsed.intervals[0].name, None);
        assert_eq!(parsed.intervals[1].name.as_deref(), Some("name"));
        assert_eq!(parsed.intervals[1].extra_fields.len(), 8);
    }

    #[test]
    fn rejects_invalid_coordinates_and_field_counts() {
        for bytes in [
            b"chr1\tnope\t10\n".as_slice(),
            b"chr1\t-1\t10\n".as_slice(),
            b"chr1\t18446744073709551616\t20\n".as_slice(),
            b"chr1\t20\t10\n".as_slice(),
            b"chr1\t1\n".as_slice(),
            b"chr1\t1\t2\ta\tb\tc\td\te\tf\tg\th\ti\tj\n".as_slice(),
        ] {
            let error = parse_bed_bytes(bytes, &dictionary()).expect_err("BED must fail");
            assert_eq!(error.category(), ErrorCategory::TargetFormat);
        }
    }

    #[test]
    fn rejects_unknown_aliases_and_out_of_bounds_intervals() {
        let alias_error = parse_bed_bytes(b"1\t0\t10\n", &dictionary())
            .expect_err("contig aliases must not be inferred");
        assert_eq!(alias_error.category(), ErrorCategory::TargetContig);

        let bounds_error = parse_bed_bytes(b"chr1\t90\t101\n", &dictionary())
            .expect_err("out-of-bounds target must fail");
        assert_eq!(bounds_error.category(), ErrorCategory::TargetFormat);
    }

    #[test]
    fn sorts_merges_and_retains_source_mapping() {
        let parsed = parse_bed_bytes(
            b"chr2\t10\t20\tC\nchr1\t20\t30\tB\nchr1\t10\t20\tA\nchr1\t15\t25\tD\n",
            &dictionary(),
        )
        .expect("BED should parse");
        let targets = normalize_targets(parsed, TargetNormalizationConfig::default())
            .expect("normalization should succeed");
        assert_eq!(targets.merged_intervals.len(), 2);
        assert_eq!(targets.merged_intervals[0].contig, "chr1");
        assert_eq!(targets.merged_intervals[0].start, 10);
        assert_eq!(targets.merged_intervals[0].end, 30);
        assert_eq!(
            targets.merged_intervals[0].source_interval_indices,
            [1, 2, 3]
        );
        assert_eq!(targets.merged_intervals[1].contig, "chr2");
        assert_eq!(targets.merged_intervals[1].source_interval_indices, [0]);
        assert_eq!(targets.normalization.overlap_merges, 2);
        assert_eq!(targets.normalization.adjacency_merges, 0);
        assert_eq!(targets.normalization.aggregate_territory_bases, 30);
        assert!(targets.normalization.reordered_positions > 0);
    }

    #[test]
    fn directly_adjacent_intervals_merge_as_adjacency() {
        let parsed = parse_bed_bytes(b"chr1\t10\t20\tA\nchr1\t20\t30\tB\n", &dictionary())
            .expect("BED should parse");
        let targets = normalize_targets(parsed, TargetNormalizationConfig::default())
            .expect("normalization should succeed");
        assert_eq!(targets.merged_intervals.len(), 1);
        assert_eq!(targets.merged_intervals[0].start, 10);
        assert_eq!(targets.merged_intervals[0].end, 30);
        assert_eq!(targets.merged_intervals[0].source_interval_indices, [0, 1]);
        assert_eq!(targets.normalization.overlap_merges, 0);
        assert_eq!(targets.normalization.adjacency_merges, 1);
    }

    #[test]
    fn configurable_flanks_clip_only_at_authoritative_boundaries() {
        let parsed = parse_bed_bytes(b"chr1\t2\t10\nchr1\t90\t99\n", &dictionary())
            .expect("BED should parse");
        let targets = normalize_targets(parsed, TargetNormalizationConfig { flank_bases: 5 })
            .expect("normalization should succeed");
        assert_eq!(targets.merged_intervals[0].start, 0);
        assert_eq!(targets.merged_intervals[0].end, 15);
        assert_eq!(targets.merged_intervals[1].start, 85);
        assert_eq!(targets.merged_intervals[1].end, 100);
        assert_eq!(targets.normalization.left_flank_clips, 1);
        assert_eq!(targets.normalization.right_flank_clips, 1);
    }

    #[test]
    fn zero_length_intervals_remain_source_records_without_territory() {
        let parsed = parse_bed_bytes(b"chr1\t5\t5\tpoint\n", &dictionary())
            .expect("zero-length BED interval is valid");
        let targets = normalize_targets(parsed, TargetNormalizationConfig { flank_bases: 20 })
            .expect("normalization should succeed");
        assert_eq!(targets.source_intervals.len(), 1);
        assert!(targets.merged_intervals.is_empty());
        assert_eq!(targets.normalization.empty_source_intervals, 1);
        assert_eq!(targets.normalization.aggregate_territory_bases, 0);
    }

    #[test]
    fn target_identity_hashes_original_bytes_before_normalization() {
        let parsed = parse_bed_bytes(b"chr1\t0\t10\r\n", &dictionary()).expect("BED should parse");
        assert_eq!(parsed.identity.size_bytes, 11);
        assert_eq!(
            parsed.identity.sha256,
            "79c32662ee19e43b6876d2e93b8147e465a3c4938fea74f69126299e2feaed9f"
        );
        assert_eq!(parsed.stats.crlf_lines_normalized, 1);
    }

    #[test]
    fn provenance_actions_are_deterministic_and_complete() {
        let parsed =
            parse_bed_bytes(b"# x\nchr1\t0\t10\n", &dictionary()).expect("BED should parse");
        let targets = normalize_targets(parsed, TargetNormalizationConfig { flank_bases: 3 })
            .expect("normalization should succeed");
        let first = targets.provenance_actions();
        let second = targets.provenance_actions();
        assert_eq!(first, second);
        assert!(first.iter().any(|item| item == "targets:flank_bases=3"));
        assert!(
            first
                .iter()
                .any(|item| item == "targets:comment_lines_skipped=1")
        );
        assert!(first.iter().any(|item| item.starts_with("targets:sha256=")));
    }

    #[test]
    fn deterministic_mutation_fuzz_never_panics() {
        let dictionary = dictionary();
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        for length in 0..96_usize {
            for _case in 0..32 {
                let mut bytes = Vec::with_capacity(length);
                for _ in 0..length {
                    state ^= state << 7;
                    state ^= state >> 9;
                    state ^= state << 8;
                    bytes.push(state.to_le_bytes()[0]);
                }
                let _result = parse_bed_bytes(&bytes, &dictionary);
            }
        }
    }
}
