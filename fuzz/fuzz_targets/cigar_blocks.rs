#![no_main]

use aligngauge_coverage::cigar_to_coverage_blocks;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 16 {
        return;
    }
    let start = u64::from_le_bytes(data[0..8].try_into().expect("eight-byte start"));
    let reference_length =
        u64::from_le_bytes(data[8..16].try_into().expect("eight-byte reference length"));
    let raw_cigar: Vec<u32> = data[16..]
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes(chunk.try_into().expect("four-byte CIGAR word")))
        .collect();
    let _ = cigar_to_coverage_blocks(&raw_cigar, start, reference_length);
});
