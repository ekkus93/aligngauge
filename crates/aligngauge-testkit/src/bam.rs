//! Deterministic BAM and BGZF serialization used by synthetic fixtures.

use std::fs;
use std::io::Write;
use std::path::Path;

use crc32fast::Hasher;
use flate2::Compression;
use flate2::write::DeflateEncoder;
use rust_htslib::bam;

use crate::error::{Result, TestkitError};

const BAM_MAGIC: &[u8; 4] = b"BAM\x01";
const BGZF_PAYLOAD_CHUNK: usize = 32 * 1024;
const BGZF_EOF: [u8; 28] = [
    31, 139, 8, 4, 0, 0, 0, 0, 0, 255, 6, 0, 66, 67, 2, 0, 27, 0, 3, 0, 0, 0, 0, 0, 0,
    0, 0, 0,
];

/// One BAM reference declaration.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReferenceSpec {
    /// Reference name without NUL.
    pub name: String,
    /// Reference length.
    pub length: i32,
}

/// CIGAR operation.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CigarOp {
    /// Operation length.
    pub length: u32,
    /// SAM operation character.
    pub operation: char,
}

impl CigarOp {
    /// Construct a CIGAR operation.
    #[must_use]
    pub const fn new(length: u32, operation: char) -> Self {
        Self { length, operation }
    }

    fn encoded(self) -> Result<u32> {
        let code = match self.operation {
            'M' => 0,
            'I' => 1,
            'D' => 2,
            'N' => 3,
            'S' => 4,
            'H' => 5,
            'P' => 6,
            '=' => 7,
            'X' => 8,
            operation => {
                return Err(TestkitError::generation(format!(
                    "unsupported CIGAR operation {operation:?}"
                )));
            }
        };
        self.length
            .checked_shl(4)
            .and_then(|value| value.checked_add(code))
            .ok_or_else(|| TestkitError::generation("CIGAR encoding overflow"))
    }

    const fn consumes_reference(self) -> bool {
        matches!(self.operation, 'M' | 'D' | 'N' | '=' | 'X')
    }

    const fn consumes_query(self) -> bool {
        matches!(self.operation, 'M' | 'I' | 'S' | '=' | 'X')
    }
}

/// A deterministic BAM record.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RecordSpec {
    /// Query name without NUL.
    pub name: String,
    /// SAM flags.
    pub flags: u16,
    /// Zero-based reference ID, or `-1`.
    pub reference_id: i32,
    /// Zero-based position, or `-1`.
    pub position: i32,
    /// Mapping quality.
    pub mapping_quality: u8,
    /// CIGAR operations.
    pub cigar: Vec<CigarOp>,
    /// Query sequence using IUPAC symbols supported by BAM.
    pub sequence: String,
    /// Raw Phred qualities, one per query base.
    pub qualities: Vec<u8>,
    /// Mate reference ID.
    pub mate_reference_id: i32,
    /// Mate position.
    pub mate_position: i32,
    /// Template length.
    pub template_length: i32,
    /// Pre-encoded BAM auxiliary fields.
    pub auxiliary: Vec<u8>,
    /// Force long-CIGAR `CG:B,I` representation.
    pub force_long_cigar: bool,
}

impl RecordSpec {
    /// Construct a mapped record with default mate fields and qualities.
    #[must_use]
    pub fn mapped(
        name: impl Into<String>,
        reference_id: i32,
        position: i32,
        cigar: Vec<CigarOp>,
        sequence: impl Into<String>,
    ) -> Self {
        let sequence = sequence.into();
        Self {
            name: name.into(),
            flags: 0,
            reference_id,
            position,
            mapping_quality: 60,
            cigar,
            qualities: vec![30; sequence.len()],
            sequence,
            mate_reference_id: -1,
            mate_position: -1,
            template_length: 0,
            auxiliary: Vec::new(),
            force_long_cigar: false,
        }
    }

    /// Construct an unmapped record.
    #[must_use]
    pub fn unmapped(name: impl Into<String>, sequence: impl Into<String>) -> Self {
        let sequence = sequence.into();
        Self {
            name: name.into(),
            flags: 0x4,
            reference_id: -1,
            position: -1,
            mapping_quality: 0,
            cigar: Vec::new(),
            qualities: vec![30; sequence.len()],
            sequence,
            mate_reference_id: -1,
            mate_position: -1,
            template_length: 0,
            auxiliary: Vec::new(),
            force_long_cigar: false,
        }
    }
}

/// Build a `TAG:i` auxiliary field.
pub fn aux_i32(tag: [u8; 2], value: i32) -> Vec<u8> {
    let mut output = Vec::with_capacity(7);
    output.extend_from_slice(&tag);
    output.push(b'i');
    output.extend_from_slice(&value.to_le_bytes());
    output
}

/// Build a `TAG:Z` auxiliary field.
pub fn aux_string(tag: [u8; 2], value: &str) -> Vec<u8> {
    let mut output = Vec::with_capacity(value.len() + 4);
    output.extend_from_slice(&tag);
    output.push(b'Z');
    output.extend_from_slice(value.as_bytes());
    output.push(0);
    output
}

/// Serialize a complete deterministic BAM file.
pub fn serialize_bam(
    header_text: &str,
    references: &[ReferenceSpec],
    records: &[RecordSpec],
) -> Result<Vec<u8>> {
    let mut raw = Vec::new();
    raw.extend_from_slice(BAM_MAGIC);
    push_i32(
        &mut raw,
        i32::try_from(header_text.len())
            .map_err(|_| TestkitError::generation("BAM header text exceeds i32"))?,
    );
    raw.extend_from_slice(header_text.as_bytes());
    push_i32(
        &mut raw,
        i32::try_from(references.len())
            .map_err(|_| TestkitError::generation("reference count exceeds i32"))?,
    );

    for reference in references {
        if reference.name.as_bytes().contains(&0) {
            return Err(TestkitError::generation(
                "reference name contains embedded NUL",
            ));
        }
        let name_length = reference
            .name
            .len()
            .checked_add(1)
            .ok_or_else(|| TestkitError::generation("reference-name length overflow"))?;
        push_i32(
            &mut raw,
            i32::try_from(name_length)
                .map_err(|_| TestkitError::generation("reference name exceeds i32"))?,
        );
        raw.extend_from_slice(reference.name.as_bytes());
        raw.push(0);
        push_i32(&mut raw, reference.length);
    }

    for record in records {
        raw.extend_from_slice(&serialize_record(record)?);
    }

    encode_bgzf(&raw)
}

/// Serialize a BAM whose final record declares a larger block than is present.
pub fn serialize_malformed_record_length(
    header_text: &str,
    references: &[ReferenceSpec],
) -> Result<Vec<u8>> {
    let mut raw = raw_header(header_text, references)?;
    push_i32(&mut raw, 128);
    raw.extend_from_slice(&[0_u8; 17]);
    encode_bgzf(&raw)
}

/// Write a deterministic BAM file.
pub fn write_bam(
    path: &Path,
    header_text: &str,
    references: &[ReferenceSpec],
    records: &[RecordSpec],
) -> Result<()> {
    let bytes = serialize_bam(header_text, references, records)?;
    fs::write(path, bytes).map_err(|source| TestkitError::io("write BAM", path, source))
}

/// Build a BAI index for a valid coordinate-sorted BAM.
pub fn build_bai(path: &Path) -> Result<()> {
    bam::index::build(path, None, bam::index::Type::Bai, 1)
        .map_err(|error| TestkitError::htslib(format!("index {}: {error}", path.display())))
}

/// Remove bytes from the middle of a BGZF stream to create deterministic corruption.
pub fn truncate_midstream(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.len() < 96 {
        return Err(TestkitError::generation(
            "BGZF stream is too small for deterministic truncation",
        ));
    }
    let retained = bytes.len() / 2;
    Ok(bytes[..retained].to_vec())
}

fn raw_header(header_text: &str, references: &[ReferenceSpec]) -> Result<Vec<u8>> {
    let mut raw = Vec::new();
    raw.extend_from_slice(BAM_MAGIC);
    push_i32(
        &mut raw,
        i32::try_from(header_text.len())
            .map_err(|_| TestkitError::generation("BAM header text exceeds i32"))?,
    );
    raw.extend_from_slice(header_text.as_bytes());
    push_i32(
        &mut raw,
        i32::try_from(references.len())
            .map_err(|_| TestkitError::generation("reference count exceeds i32"))?,
    );
    for reference in references {
        let name_length = reference
            .name
            .len()
            .checked_add(1)
            .ok_or_else(|| TestkitError::generation("reference-name length overflow"))?;
        push_i32(
            &mut raw,
            i32::try_from(name_length)
                .map_err(|_| TestkitError::generation("reference name exceeds i32"))?,
        );
        raw.extend_from_slice(reference.name.as_bytes());
        raw.push(0);
        push_i32(&mut raw, reference.length);
    }
    Ok(raw)
}

fn serialize_record(record: &RecordSpec) -> Result<Vec<u8>> {
    if record.name.as_bytes().contains(&0) {
        return Err(TestkitError::generation(
            "record name contains embedded NUL",
        ));
    }
    if record.qualities.len() != record.sequence.len() {
        return Err(TestkitError::generation(format!(
            "record {} has {} sequence bases but {} qualities",
            record.name,
            record.sequence.len(),
            record.qualities.len()
        )));
    }

    let query_length = cigar_query_length(&record.cigar)?;
    if query_length != record.sequence.len() {
        return Err(TestkitError::generation(format!(
            "record {} CIGAR consumes {query_length} query bases but sequence has {}",
            record.name,
            record.sequence.len()
        )));
    }

    let read_name_length = record
        .name
        .len()
        .checked_add(1)
        .ok_or_else(|| TestkitError::generation("read-name length overflow"))?;
    let read_name_length_u8 = u8::try_from(read_name_length)
        .map_err(|_| TestkitError::generation("BAM read name exceeds 255 bytes"))?;
    let reference_span = cigar_reference_length(&record.cigar)?;
    let end = if record.position >= 0 {
        let span = i32::try_from(reference_span)
            .map_err(|_| TestkitError::generation("reference span exceeds i32"))?;
        record
            .position
            .checked_add(span)
            .ok_or_else(|| TestkitError::generation("record end overflow"))?
    } else {
        record.position
    };
    let bin = if record.position >= 0 && end > record.position {
        reg2bin(record.position, end)?
    } else {
        0
    };

    let actual_cigar: Vec<u32> = record
        .cigar
        .iter()
        .copied()
        .map(CigarOp::encoded)
        .collect::<Result<_>>()?;
    let use_long = record.force_long_cigar || actual_cigar.len() > usize::from(u16::MAX);
    let (serialized_cigar, cigar_count, auxiliary) = if use_long {
        let query_length_u32 = u32::try_from(record.sequence.len())
            .map_err(|_| TestkitError::generation("long-CIGAR query length exceeds u32"))?;
        let placeholder = [
            CigarOp::new(query_length_u32, 'S').encoded()?,
            CigarOp::new(reference_span, 'N').encoded()?,
        ];
        let mut auxiliary = cg_aux(&actual_cigar)?;
        auxiliary.extend_from_slice(&record.auxiliary);
        (placeholder.to_vec(), 2_u16, auxiliary)
    } else {
        let count = u16::try_from(actual_cigar.len())
            .map_err(|_| TestkitError::generation("CIGAR count exceeds u16"))?;
        (actual_cigar, count, record.auxiliary.clone())
    };

    let mut body = Vec::new();
    push_i32(&mut body, record.reference_id);
    push_i32(&mut body, record.position);
    let bin_mq_nl = (u32::from(bin) << 16)
        | (u32::from(record.mapping_quality) << 8)
        | u32::from(read_name_length_u8);
    push_u32(&mut body, bin_mq_nl);
    let flag_nc = (u32::from(record.flags) << 16) | u32::from(cigar_count);
    push_u32(&mut body, flag_nc);
    push_i32(
        &mut body,
        i32::try_from(record.sequence.len())
            .map_err(|_| TestkitError::generation("query length exceeds i32"))?,
    );
    push_i32(&mut body, record.mate_reference_id);
    push_i32(&mut body, record.mate_position);
    push_i32(&mut body, record.template_length);
    body.extend_from_slice(record.name.as_bytes());
    body.push(0);
    for operation in serialized_cigar {
        push_u32(&mut body, operation);
    }
    body.extend_from_slice(&encode_sequence(&record.sequence)?);
    body.extend_from_slice(&record.qualities);
    body.extend_from_slice(&auxiliary);

    let mut output = Vec::with_capacity(body.len() + 4);
    push_i32(
        &mut output,
        i32::try_from(body.len())
            .map_err(|_| TestkitError::generation("BAM record exceeds i32 block size"))?,
    );
    output.extend_from_slice(&body);
    Ok(output)
}

fn cigar_query_length(cigar: &[CigarOp]) -> Result<usize> {
    cigar.iter().try_fold(0_usize, |total, operation| {
        if operation.consumes_query() {
            let length = usize::try_from(operation.length)
                .map_err(|_| TestkitError::generation("CIGAR query length exceeds usize"))?;
            total
                .checked_add(length)
                .ok_or_else(|| TestkitError::generation("CIGAR query length overflow"))
        } else {
            Ok(total)
        }
    })
}

fn cigar_reference_length(cigar: &[CigarOp]) -> Result<u32> {
    cigar.iter().try_fold(0_u32, |total, operation| {
        if operation.consumes_reference() {
            total
                .checked_add(operation.length)
                .ok_or_else(|| TestkitError::generation("CIGAR reference length overflow"))
        } else {
            Ok(total)
        }
    })
}

fn encode_sequence(sequence: &str) -> Result<Vec<u8>> {
    let mut output = Vec::with_capacity(sequence.len().div_ceil(2));
    for pair in sequence.as_bytes().chunks(2) {
        let high = encode_base(pair[0])?;
        let low = pair.get(1).copied().map_or(Ok(0), encode_base)?;
        output.push((high << 4) | low);
    }
    Ok(output)
}

fn encode_base(base: u8) -> Result<u8> {
    match base.to_ascii_uppercase() {
        b'=' => Ok(0),
        b'A' => Ok(1),
        b'C' => Ok(2),
        b'M' => Ok(3),
        b'G' => Ok(4),
        b'R' => Ok(5),
        b'S' => Ok(6),
        b'V' => Ok(7),
        b'T' => Ok(8),
        b'W' => Ok(9),
        b'Y' => Ok(10),
        b'H' => Ok(11),
        b'K' => Ok(12),
        b'D' => Ok(13),
        b'B' => Ok(14),
        b'N' => Ok(15),
        other => Err(TestkitError::generation(format!(
            "unsupported BAM sequence base {:?}",
            char::from(other)
        ))),
    }
}

fn cg_aux(cigar: &[u32]) -> Result<Vec<u8>> {
    let count = i32::try_from(cigar.len())
        .map_err(|_| TestkitError::generation("CG operation count exceeds i32"))?;
    let mut output = Vec::with_capacity(8 + cigar.len() * 4);
    output.extend_from_slice(b"CG");
    output.push(b'B');
    output.push(b'I');
    output.extend_from_slice(&count.to_le_bytes());
    for operation in cigar {
        output.extend_from_slice(&operation.to_le_bytes());
    }
    Ok(output)
}

fn reg2bin(begin: i32, end_exclusive: i32) -> Result<u16> {
    let begin = u32::try_from(begin)
        .map_err(|_| TestkitError::generation("negative coordinate cannot be binned"))?;
    let end = u32::try_from(
        end_exclusive
            .checked_sub(1)
            .ok_or_else(|| TestkitError::generation("empty interval cannot be binned"))?,
    )
    .map_err(|_| TestkitError::generation("negative end cannot be binned"))?;

    let bin = if begin >> 14 == end >> 14 {
        4681 + (begin >> 14)
    } else if begin >> 17 == end >> 17 {
        585 + (begin >> 17)
    } else if begin >> 20 == end >> 20 {
        73 + (begin >> 20)
    } else if begin >> 23 == end >> 23 {
        9 + (begin >> 23)
    } else if begin >> 26 == end >> 26 {
        1 + (begin >> 26)
    } else {
        0
    };
    u16::try_from(bin).map_err(|_| TestkitError::generation("BAM bin exceeds u16"))
}

fn encode_bgzf(raw: &[u8]) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    for payload in raw.chunks(BGZF_PAYLOAD_CHUNK) {
        output.extend_from_slice(&encode_bgzf_block(payload)?);
    }
    output.extend_from_slice(&BGZF_EOF);
    Ok(output)
}

fn encode_bgzf_block(payload: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(payload)
        .map_err(|source| TestkitError::io("compress BGZF payload", "<memory>", source))?;
    let compressed = encoder
        .finish()
        .map_err(|source| TestkitError::io("finish BGZF payload", "<memory>", source))?;

    let total_size = 18_usize
        .checked_add(compressed.len())
        .and_then(|value| value.checked_add(8))
        .ok_or_else(|| TestkitError::generation("BGZF block-size overflow"))?;
    let block_size = u16::try_from(
        total_size
            .checked_sub(1)
            .ok_or_else(|| TestkitError::generation("invalid BGZF block size"))?,
    )
    .map_err(|_| TestkitError::generation("compressed BGZF block exceeds 64 KiB"))?;

    let mut output = Vec::with_capacity(total_size);
    output.extend_from_slice(&[31, 139, 8, 4]);
    output.extend_from_slice(&[0, 0, 0, 0]);
    output.extend_from_slice(&[0, 255]);
    output.extend_from_slice(&6_u16.to_le_bytes());
    output.extend_from_slice(b"BC");
    output.extend_from_slice(&2_u16.to_le_bytes());
    output.extend_from_slice(&block_size.to_le_bytes());
    output.extend_from_slice(&compressed);

    let mut hasher = Hasher::new();
    hasher.update(payload);
    output.extend_from_slice(&hasher.finalize().to_le_bytes());
    output.extend_from_slice(
        &u32::try_from(payload.len())
            .map_err(|_| TestkitError::generation("BGZF payload exceeds u32"))?
            .to_le_bytes(),
    );
    Ok(output)
}

fn push_i32(output: &mut Vec<u8>, value: i32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use rust_htslib::bam::Read;

    use super::*;

    #[test]
    fn serialized_bam_round_trips_through_htslib() {
        let references = vec![ReferenceSpec {
            name: String::from("chr1"),
            length: 1000,
        }];
        let header = "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:1000\n";
        let records = vec![RecordSpec::mapped(
            "read1",
            0,
            10,
            vec![CigarOp::new(4, 'M')],
            "ACGT",
        )];
        let bytes = serialize_bam(header, &references, &records).expect("serialize BAM");
        let path = std::env::temp_dir().join(format!(
            "aligngauge-bam-roundtrip-{}.bam",
            std::process::id()
        ));
        fs::write(&path, bytes).expect("write BAM");
        let mut reader = bam::Reader::from_path(&path).expect("open BAM");
        let observed: Vec<_> = reader.records().collect();
        assert_eq!(observed.len(), 1);
        assert!(!observed[0].as_ref().expect("record").is_unmapped());
        fs::remove_file(path).expect("remove BAM");
    }

    #[test]
    fn fixed_bgzf_output_is_deterministic() {
        let references = vec![ReferenceSpec {
            name: String::from("chr1"),
            length: 1000,
        }];
        let record = RecordSpec::mapped(
            "read1",
            0,
            10,
            vec![CigarOp::new(4, 'M')],
            "ACGT",
        );
        let first = serialize_bam("", &references, std::slice::from_ref(&record))
            .expect("serialize first");
        let second = serialize_bam("", &references, &[record]).expect("serialize second");
        assert_eq!(first, second);
        assert!(first.ends_with(&BGZF_EOF));
    }
}
