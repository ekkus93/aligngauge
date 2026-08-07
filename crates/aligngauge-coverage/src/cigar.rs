//! Canonical v0.1 record filtering and CIGAR-to-coverage event generation.

use aligngauge_core::AlignGaugeError;

use crate::util::input_error;

const FLAG_UNMAPPED: u16 = 0x4;
const FLAG_SECONDARY: u16 = 0x100;
const FLAG_QC_FAIL: u16 = 0x200;
const FLAG_DUPLICATE: u16 = 0x400;
const FLAG_SUPPLEMENTARY: u16 = 0x800;

/// One half-open covered reference block.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CoverageBlock {
    /// Zero-based inclusive start.
    pub start: u64,
    /// Zero-based exclusive end.
    pub end: u64,
}

/// Convert one validated raw BAM CIGAR into exact covered blocks.
///
/// # Errors
/// Returns `input_corrupt` on zero/unknown operations, checked coordinate overflow, or a block
/// extending beyond the declared reference.
pub fn cigar_to_coverage_blocks(
    raw_cigar: &[u32],
    start: u64,
    reference_length: u64,
) -> Result<Vec<CoverageBlock>, AlignGaugeError> {
    let mut blocks = Vec::new();
    for_each_coverage_block(raw_cigar, start, reference_length, |block| {
        blocks.push(block);
        Ok(())
    })?;
    Ok(blocks)
}

pub(crate) const fn record_is_accepted(flags: u16) -> bool {
    flags & (FLAG_UNMAPPED | FLAG_SECONDARY | FLAG_QC_FAIL | FLAG_DUPLICATE | FLAG_SUPPLEMENTARY)
        == 0
}

pub(crate) fn for_each_coverage_block(
    raw_cigar: &[u32],
    start: u64,
    reference_length: u64,
    mut observe: impl FnMut(CoverageBlock) -> Result<(), AlignGaugeError>,
) -> Result<(), AlignGaugeError> {
    let mut cursor = start;
    if cursor > reference_length {
        return Err(input_error("coverage CIGAR begins outside the declared reference"));
    }
    for encoded in raw_cigar {
        let length = u64::from(encoded >> 4);
        let operation = encoded & 0x0f;
        if length == 0 {
            return Err(input_error("coverage CIGAR contains a zero-length operation"));
        }
        match operation {
            0 | 7 | 8 => {
                let end = checked_reference_advance(cursor, length, reference_length)?;
                observe(CoverageBlock { start: cursor, end })?;
                cursor = end;
            }
            2 | 3 => {
                cursor = checked_reference_advance(cursor, length, reference_length)?;
            }
            1 | 4 | 5 | 6 => {}
            _ => {
                return Err(input_error("coverage CIGAR contains an unknown operation code")
                    .with_detail("cigar_operation_code", u64::from(operation)));
            }
        }
    }
    Ok(())
}

fn checked_reference_advance(
    cursor: u64,
    length: u64,
    reference_length: u64,
) -> Result<u64, AlignGaugeError> {
    let end = cursor
        .checked_add(length)
        .ok_or_else(|| input_error("coverage CIGAR reference coordinate overflowed"))?;
    if end > reference_length {
        return Err(input_error(
            "coverage CIGAR operation extends beyond the declared reference",
        )
        .with_detail("operation_end", end)
        .with_detail("reference_length", reference_length));
    }
    Ok(end)
}
