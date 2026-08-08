use std::error::Error;
use std::fs;
use std::path::PathBuf;

use aligngauge_coverage::{
    PicardWgsOverlapCorrector, PicardWgsOverlapRecord, picard_hs_trailing_read_bases_to_clip,
    picard_wgs_flag_candidate,
};
use aligngauge_testkit::bam::{CigarOp, RecordSpec, ReferenceSpec, serialize_bam};

const HEADER: &str = "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:1000\n";
const REFERENCE_LENGTH: u64 = 1_000;
const FLAG_PAIRED: u16 = 0x1;
const FLAG_MATE_UNMAPPED: u16 = 0x8;
const FLAG_READ1: u16 = 0x40;
const FLAG_READ2: u16 = 0x80;
const FLAG_SECONDARY: u16 = 0x100;
const FLAG_QC_FAIL: u16 = 0x200;
const FLAG_DUPLICATE: u16 = 0x400;
const FLAG_SUPPLEMENTARY: u16 = 0x800;

#[test]
fn write_exact_overlap_differential_fixture() -> Result<(), Box<dyn Error>> {
    let output_dir = std::env::var_os("ALIGNGAUGE_M13_OUTPUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/m13-overlap-test"));
    fs::create_dir_all(&output_dir)?;

    let records = overlap_records();
    let references = [ReferenceSpec {
        name: "chr1".to_owned(),
        length: i32::try_from(REFERENCE_LENGTH)?,
    }];
    let bam = serialize_bam(HEADER, &references, &records)?;
    fs::write(output_dir.join("overlap.bam"), bam)?;

    let counters = summarize(&records)?;
    assert_eq!(counters.wgs_retained_bases, 135);
    assert_eq!(counters.wgs_baseq_excluded_bases, 10);
    assert_eq!(counters.wgs_overlap_excluded_bases, 35);
    assert_eq!(counters.hs_overlap_clipped_read_bases, 64);
    fs::write(output_dir.join("aligngauge.tsv"), counters.render())?;
    Ok(())
}

#[derive(Debug, Default)]
struct Counters {
    wgs_retained_bases: u64,
    wgs_baseq_excluded_bases: u64,
    wgs_overlap_excluded_bases: u64,
    hs_overlap_clipped_read_bases: u64,
}

impl Counters {
    fn render(&self) -> String {
        format!(
            concat!(
                "wgs_retained_bases\t{}\n",
                "wgs_baseq_excluded_bases\t{}\n",
                "wgs_overlap_excluded_bases\t{}\n",
                "hs_overlap_clipped_read_bases\t{}\n",
            ),
            self.wgs_retained_bases,
            self.wgs_baseq_excluded_bases,
            self.wgs_overlap_excluded_bases,
            self.hs_overlap_clipped_read_bases,
        )
    }
}

fn summarize(records: &[RecordSpec]) -> Result<Counters, Box<dyn Error>> {
    let mut counters = Counters::default();
    let mut wgs = PicardWgsOverlapCorrector::new(1 << 20)?;

    for record in records {
        let raw_cigar = encode_cigar(&record.cigar)?;
        if wgs_candidate(record) {
            let result = wgs.observe_record(
                PicardWgsOverlapRecord {
                    reference_id: record.reference_id,
                    start: u64::try_from(record.position)?,
                    reference_length: REFERENCE_LENGTH,
                    query_name: record.name.as_bytes(),
                    raw_cigar: &raw_cigar,
                    sequence: record.sequence.as_bytes(),
                    qualities: &record.qualities,
                },
                |_| Ok(()),
            )?;
            counters.wgs_retained_bases = counters
                .wgs_retained_bases
                .checked_add(result.retained_bases)
                .ok_or("WGS retained counter overflow")?;
            counters.wgs_baseq_excluded_bases = counters
                .wgs_baseq_excluded_bases
                .checked_add(result.baseq_excluded_bases)
                .ok_or("WGS base-quality counter overflow")?;
            counters.wgs_overlap_excluded_bases = counters
                .wgs_overlap_excluded_bases
                .checked_add(result.overlap_excluded_bases)
                .ok_or("WGS overlap counter overflow")?;
        }

        if hs_candidate(record) {
            let mate_start = if record.mate_position < 0 {
                None
            } else {
                Some(u64::try_from(record.mate_position)?)
            };
            let clipped = picard_hs_trailing_read_bases_to_clip(
                record.flags,
                u64::try_from(record.position)?,
                mate_start,
                &raw_cigar,
            )?;
            counters.hs_overlap_clipped_read_bases = counters
                .hs_overlap_clipped_read_bases
                .checked_add(clipped)
                .ok_or("Hs overlap counter overflow")?;
        }
    }
    Ok(counters)
}

fn wgs_candidate(record: &RecordSpec) -> bool {
    picard_wgs_flag_candidate(record.flags)
        && record.flags & (FLAG_QC_FAIL | FLAG_DUPLICATE) == 0
        && record.mapping_quality >= 20
        && record.flags & FLAG_PAIRED != 0
        && record.flags & FLAG_MATE_UNMAPPED == 0
}

fn hs_candidate(record: &RecordSpec) -> bool {
    record.flags & (FLAG_SECONDARY | FLAG_QC_FAIL | FLAG_DUPLICATE) == 0
        && record.mapping_quality >= 20
}

fn encode_cigar(cigar: &[CigarOp]) -> Result<Vec<u32>, Box<dyn Error>> {
    cigar
        .iter()
        .map(|operation| {
            let code = match operation.operation {
                'M' => 0,
                'I' => 1,
                'D' => 2,
                'N' => 3,
                'S' => 4,
                'H' => 5,
                'P' => 6,
                '=' => 7,
                'X' => 8,
                other => return Err(format!("unsupported CIGAR operation: {other}")),
            };
            operation
                .length
                .checked_shl(4)
                .and_then(|value| value.checked_add(code))
                .ok_or_else(|| "CIGAR encoding overflow".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn paired_record(
    name: &str,
    position: i32,
    mate_position: i32,
    flags: u16,
    cigar: Vec<CigarOp>,
    sequence: &str,
) -> RecordSpec {
    let mut record = RecordSpec::mapped(name, 0, position, cigar, sequence);
    record.flags = FLAG_PAIRED | flags;
    record.mate_reference_id = 0;
    record.mate_position = mate_position;
    record
}

fn overlap_records() -> Vec<RecordSpec> {
    let mut records = Vec::new();

    records.push(paired_record(
        "pair-overlap",
        100,
        120,
        FLAG_READ1,
        vec![CigarOp::new(40, 'M')],
        &"A".repeat(40),
    ));
    records.push(paired_record(
        "pair-overlap",
        120,
        100,
        FLAG_READ2,
        vec![CigarOp::new(40, 'M')],
        &"C".repeat(40),
    ));

    let mut low_tail = paired_record(
        "quality-order",
        200,
        210,
        FLAG_READ1,
        vec![CigarOp::new(20, 'M')],
        &"G".repeat(20),
    );
    low_tail.qualities = [vec![30_u8; 10], vec![10_u8; 10]].concat();
    records.push(low_tail);
    records.push(paired_record(
        "quality-order",
        210,
        200,
        FLAG_READ2,
        vec![CigarOp::new(20, 'M')],
        &"T".repeat(20),
    ));

    records.push(paired_record(
        "supplementary-edge",
        300,
        305,
        FLAG_READ1,
        vec![CigarOp::new(10, 'M')],
        "AAAAAAAAAA",
    ));
    records.push(paired_record(
        "supplementary-edge",
        305,
        300,
        FLAG_READ2 | FLAG_SUPPLEMENTARY,
        vec![CigarOp::new(10, 'M')],
        "CCCCCCCCCC",
    ));
    records.push(paired_record(
        "supplementary-edge",
        307,
        300,
        FLAG_READ2 | FLAG_SECONDARY,
        vec![CigarOp::new(5, 'M')],
        "GGGGG",
    ));

    records.push(paired_record(
        "equal-start",
        400,
        400,
        FLAG_READ1,
        vec![CigarOp::new(10, 'M')],
        "AAAAAAAAAA",
    ));
    records.push(paired_record(
        "equal-start",
        400,
        400,
        FLAG_READ2,
        vec![CigarOp::new(10, 'M')],
        "CCCCCCCCCC",
    ));

    records.push(paired_record(
        "extended-cigar",
        500,
        502,
        FLAG_READ2,
        vec![CigarOp::new(5, '='), CigarOp::new(5, 'X')],
        "AAAAAAAAAA",
    ));
    records.push(paired_record(
        "insertion-cigar",
        550,
        553,
        FLAG_READ2,
        vec![
            CigarOp::new(5, 'M'),
            CigarOp::new(2, 'I'),
            CigarOp::new(5, 'M'),
        ],
        "AAAAAAAAAAAA",
    ));

    records
}
