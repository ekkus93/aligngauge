#![no_main]

use aligngauge_formats::{SequenceContig, SequenceDictionary, parse_bed_bytes};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let dictionary = SequenceDictionary::new(vec![
        SequenceContig {
            name: "chr1".to_owned(),
            length: 248_956_422,
        },
        SequenceContig {
            name: "chr20".to_owned(),
            length: 64_444_167,
        },
    ])
    .expect("static fuzz dictionary is valid");
    let _ = parse_bed_bytes(data, &dictionary);
});
