//! Deterministic synthetic corpus generation.

use std::fs;
use std::path::{Path, PathBuf};

use crate::bam::{
    CigarOp, RecordSpec, ReferenceSpec, aux_i32, aux_string, build_bai, serialize_bam,
    serialize_malformed_record_length, truncate_midstream, write_bam,
};
use crate::error::{Result, TestkitError};
use crate::hash::sha256_file;
use crate::manifest::MANIFEST_SCHEMA;

const FIXTURE_COMMAND: &str =
    "cargo run -p aligngauge-testkit --locked -- generate-corpus --root .";
const HEADER: &str =
    "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:1000000\n@SQ\tSN:chr2\tLN:1000000\n";

/// Generated fixture metadata used to construct the manifest.
#[derive(Debug, Clone, Eq, PartialEq)]
struct GeneratedFixture {
    id: String,
    path: PathBuf,
    index_path: Option<PathBuf>,
    validity: &'static str,
    error_category: &'static str,
    expected_metrics: Option<PathBuf>,
}

/// Generate the complete version-1 synthetic corpus and manifest.
///
/// The function creates only local files and performs no network access.
///
/// # Errors
/// Returns an error on invalid fixture definitions, serialization/indexing
/// failure, checksum failure, or any local filesystem failure.
///
/// The body is intentionally the canonical declarative fixture catalog. Keeping
/// the entries together makes manifest ordering and corpus review explicit.
#[allow(clippy::too_many_lines)]
pub fn generate_corpus(repository_root: &Path) -> Result<()> {
    let testdata = repository_root.join("testdata");
    let fixtures = testdata.join("fixtures");
    let expected = testdata.join("expected");

    if fixtures.exists() {
        fs::remove_dir_all(&fixtures).map_err(|source| {
            TestkitError::io("remove old fixture directory", &fixtures, source)
        })?;
    }
    if expected.exists() {
        fs::remove_dir_all(&expected).map_err(|source| {
            TestkitError::io("remove old expected directory", &expected, source)
        })?;
    }
    fs::create_dir_all(&fixtures)
        .map_err(|source| TestkitError::io("create fixture directory", &fixtures, source))?;
    fs::create_dir_all(&expected)
        .map_err(|source| TestkitError::io("create expected directory", &expected, source))?;

    let references = references();
    let mut generated = Vec::new();

    generated.push(write_fixture(
        repository_root,
        "empty",
        &references,
        HEADER,
        &[],
        true,
        "valid",
        "-",
        None,
    )?);

    let basic_records = vec![
        RecordSpec::mapped(
            "mapped-1",
            0,
            100,
            vec![CigarOp::new(10, 'M')],
            "ACGTACGTAC",
        ),
        RecordSpec::mapped(
            "mapped-2",
            1,
            200,
            vec![
                CigarOp::new(4, 'M'),
                CigarOp::new(1, 'I'),
                CigarOp::new(5, 'M'),
            ],
            "AAAACGGGGG",
        ),
        RecordSpec::unmapped("unmapped-1", "NNNN"),
    ];
    let basic_expected = PathBuf::from("testdata/expected/basic.tsv");
    write_expected(
        &repository_root.join(&basic_expected),
        &[
            ("total_records", "integer", "3", "-", "-"),
            ("mapped_records", "integer", "2", "-", "-"),
            ("unmapped_records", "integer", "1", "-", "-"),
        ],
    )?;
    generated.push(write_fixture(
        repository_root,
        "basic",
        &references,
        HEADER,
        &basic_records,
        true,
        "valid",
        "-",
        Some(basic_expected),
    )?);

    let cigar_record = RecordSpec::mapped(
        "cigar-ops",
        0,
        500,
        vec![
            CigarOp::new(2, 'S'),
            CigarOp::new(3, 'M'),
            CigarOp::new(1, 'I'),
            CigarOp::new(2, '='),
            CigarOp::new(1, 'X'),
            CigarOp::new(1, 'D'),
            CigarOp::new(4, 'N'),
            CigarOp::new(1, 'P'),
            CigarOp::new(1, 'H'),
        ],
        "ACGTACGTA",
    );
    let cigar_expected = PathBuf::from("testdata/expected/cigar_ops.tsv");
    write_expected(
        &repository_root.join(&cigar_expected),
        &[
            ("total_records", "integer", "1", "-", "-"),
            ("query_bases", "integer", "9", "-", "-"),
            ("reference_span", "integer", "11", "-", "-"),
        ],
    )?;
    generated.push(write_fixture(
        repository_root,
        "cigar_ops",
        &references,
        HEADER,
        &[cigar_record],
        true,
        "valid",
        "-",
        Some(cigar_expected),
    )?);

    let mut long_record = RecordSpec::mapped(
        "long-cigar",
        0,
        1000,
        vec![CigarOp::new(1, 'M'); 66_000],
        "A".repeat(66_000),
    );
    long_record.force_long_cigar = true;
    generated.push(write_fixture(
        repository_root,
        "long_cigar",
        &references,
        HEADER,
        &[long_record],
        true,
        "valid",
        "-",
        None,
    )?);

    let flags_header = concat!(
        "@HD\tVN:1.6\tSO:coordinate\n",
        "@SQ\tSN:chr1\tLN:1000000\n",
        "@SQ\tSN:chr2\tLN:1000000\n",
        "@RG\tID:rg1\tSM:synthetic\n"
    );
    let mut first = RecordSpec::mapped("pair-a", 0, 100, vec![CigarOp::new(10, 'M')], "AAAAAAAAAA");
    first.flags = 0x1 | 0x2 | 0x40;
    first.mate_reference_id = 0;
    first.mate_position = 160;
    first.template_length = 70;
    first.auxiliary = aux_string(*b"RG", "rg1");

    let mut second =
        RecordSpec::mapped("pair-a", 0, 160, vec![CigarOp::new(10, 'M')], "CCCCCCCCCC");
    second.flags = 0x1 | 0x2 | 0x80 | 0x10 | 0x20;
    second.mate_reference_id = 0;
    second.mate_position = 100;
    second.template_length = -70;
    second.auxiliary = aux_string(*b"RG", "rg1");

    let mut secondary =
        RecordSpec::mapped("secondary", 0, 300, vec![CigarOp::new(5, 'M')], "GGGGG");
    secondary.flags = 0x100;

    let mut supplementary =
        RecordSpec::mapped("supplementary", 0, 350, vec![CigarOp::new(5, 'M')], "TTTTT");
    supplementary.flags = 0x800;

    let mut dual = RecordSpec::mapped("dual", 0, 400, vec![CigarOp::new(5, 'M')], "AAAAA");
    dual.flags = 0x100 | 0x800;

    let mut duplicate =
        RecordSpec::mapped("duplicate", 0, 450, vec![CigarOp::new(5, 'M')], "CCCCC");
    duplicate.flags = 0x400;

    let mut qc_fail = RecordSpec::mapped("qc-fail", 0, 500, vec![CigarOp::new(5, 'M')], "GGGGG");
    qc_fail.flags = 0x200;

    let mut singleton =
        RecordSpec::mapped("singleton", 0, 550, vec![CigarOp::new(5, 'M')], "TTTTT");
    singleton.flags = 0x1 | 0x8 | 0x40;
    singleton.mate_reference_id = -1;
    singleton.mate_position = -1;

    let mut discordant =
        RecordSpec::mapped("discordant", 0, 600, vec![CigarOp::new(5, 'M')], "AAAAA");
    discordant.flags = 0x1 | 0x40;
    discordant.mate_reference_id = 1;
    discordant.mate_position = 100;

    let flags_expected = PathBuf::from("testdata/expected/flags_and_pairs.tsv");
    write_expected(
        &repository_root.join(&flags_expected),
        &[
            ("total_records", "integer", "9", "-", "-"),
            (
                "secondary_records",
                "integer",
                "2",
                "-",
                "samtools-priority-profile",
            ),
            (
                "supplementary_records",
                "integer",
                "1",
                "-",
                "samtools-priority-profile",
            ),
            ("duplicate_records", "integer", "1", "-", "-"),
            ("qc_fail_records", "integer", "1", "-", "-"),
            ("paired_records", "integer", "4", "-", "-"),
        ],
    )?;
    generated.push(write_fixture(
        repository_root,
        "flags_and_pairs",
        &references,
        flags_header,
        &[
            first,
            second,
            secondary,
            supplementary,
            dual,
            duplicate,
            qc_fail,
            singleton,
            discordant,
        ],
        true,
        "valid",
        "-",
        Some(flags_expected),
    )?);

    let mut picard_alignment_records = Vec::new();
    for (index, sequence) in ["NAAA", "NCCC", "NGGG", "NTTT", "AAAA"]
        .into_iter()
        .enumerate()
    {
        let position = 100 + i32::try_from(index).expect("small fixture index") * 10;
        let mut record = RecordSpec::mapped(
            format!("picard-primary-{index}"),
            0,
            position,
            vec![CigarOp::new(4, 'M')],
            sequence,
        );
        if index == 0 {
            record.auxiliary.extend(aux_i32(*b"XN", 1));
        }
        picard_alignment_records.push(record);
    }

    let mut picard_first = RecordSpec::mapped(
        "picard-adapter-pair",
        0,
        200,
        vec![CigarOp::new(16, 'M')],
        "AATGATACGGCGACCA",
    );
    picard_first.flags = 0x1 | 0x40;
    picard_first.mapping_quality = 0;
    picard_first.mate_reference_id = 0;
    picard_first.mate_position = 300;
    picard_first.template_length = 116;
    picard_alignment_records.push(picard_first);

    let mut picard_second = RecordSpec::mapped(
        "picard-adapter-pair",
        0,
        300,
        vec![CigarOp::new(16, 'M')],
        "TGGTCGCCGTATCATT",
    );
    picard_second.flags = 0x1 | 0x80 | 0x10;
    picard_second.mapping_quality = 0;
    picard_second.mate_reference_id = 0;
    picard_second.mate_position = 200;
    picard_second.template_length = -116;
    picard_alignment_records.push(picard_second);

    for index in 0..4_i32 {
        let mut record = RecordSpec::mapped(
            format!("picard-supplementary-{index}"),
            0,
            400 + index * 10,
            vec![CigarOp::new(4, 'M')],
            "ANAA",
        );
        record.flags = 0x800;
        picard_alignment_records.push(record);
    }
    for index in 0..4_i32 {
        let mut record = RecordSpec::mapped(
            format!("picard-secondary-{index}"),
            0,
            500 + index * 10,
            vec![CigarOp::new(4, 'M')],
            "AANA",
        );
        record.flags = 0x100;
        picard_alignment_records.push(record);
    }
    generated.push(write_fixture(
        repository_root,
        "picard_alignment_edge",
        &references,
        HEADER,
        &picard_alignment_records,
        true,
        "valid",
        "-",
        None,
    )?);

    let mut picard_insert_records = Vec::new();
    let mut insert_index = 0_i32;
    let fr_sizes = [
        100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 101, 101, 101, 101, 101, 101, 101, 101,
        101, 101, 102, 102, 102, 102, 102, 102, 102, 102, 103, 103, 103, 103, 103, 103, 103, 103,
        1000,
    ];
    for size in fr_sizes {
        let position = 1_000 + insert_index * 20;
        let mut record = RecordSpec::mapped(
            format!("picard-fr-{insert_index}"),
            0,
            position,
            vec![CigarOp::new(10, 'M')],
            "AAAAAAAAAA",
        );
        record.flags = 0x1 | 0x80 | 0x10;
        record.mate_reference_id = 0;
        record.mate_position = position - 50;
        record.template_length = -size;
        picard_insert_records.push(record);
        insert_index += 1;
    }
    for size in [200, 202] {
        let position = 1_000 + insert_index * 20;
        let mut record = RecordSpec::mapped(
            format!("picard-rf-{insert_index}"),
            0,
            position,
            vec![CigarOp::new(10, 'M')],
            "CCCCCCCCCC",
        );
        record.flags = 0x1 | 0x80 | 0x10;
        record.mate_reference_id = 0;
        record.mate_position = position + 50;
        record.template_length = -size;
        picard_insert_records.push(record);
        insert_index += 1;
    }
    {
        let position = 1_000 + insert_index * 20;
        let mut record = RecordSpec::mapped(
            "picard-tandem-below-threshold",
            0,
            position,
            vec![CigarOp::new(10, 'M')],
            "GGGGGGGGGG",
        );
        record.flags = 0x1 | 0x80;
        record.mate_reference_id = 0;
        record.mate_position = position - 50;
        record.template_length = 300;
        picard_insert_records.push(record);
        insert_index += 1;
    }

    for (name, extra_flags, tlen, mate_unmapped) in [
        ("duplicate", 0x400_u16, -150_i32, false),
        ("secondary", 0x100_u16, -151_i32, false),
        ("supplementary", 0x800_u16, -152_i32, false),
        ("mate-unmapped", 0_u16, -153_i32, true),
        ("zero-tlen", 0_u16, 0_i32, false),
    ] {
        let position = 1_000 + insert_index * 20;
        let mut record = RecordSpec::mapped(
            format!("picard-excluded-{name}"),
            0,
            position,
            vec![CigarOp::new(10, 'M')],
            "TTTTTTTTTT",
        );
        record.flags = 0x1 | 0x80 | 0x10 | extra_flags;
        record.mate_reference_id = if mate_unmapped { -1 } else { 0 };
        record.mate_position = if mate_unmapped { -1 } else { position - 50 };
        if mate_unmapped {
            record.flags |= 0x8;
        }
        record.template_length = tlen;
        picard_insert_records.push(record);
        insert_index += 1;
    }
    {
        let position = 1_000 + insert_index * 20;
        let mut first_only = RecordSpec::mapped(
            "picard-excluded-first",
            0,
            position,
            vec![CigarOp::new(10, 'M')],
            "ACACACACAC",
        );
        first_only.flags = 0x1 | 0x40;
        first_only.mate_reference_id = 0;
        first_only.mate_position = position + 50;
        first_only.template_length = 154;
        picard_insert_records.push(first_only);
    }
    generated.push(write_fixture(
        repository_root,
        "picard_insert_edge",
        &references,
        HEADER,
        &picard_insert_records,
        true,
        "valid",
        "-",
        None,
    )?);

    let tags_header = concat!(
        "@HD\tVN:1.6\tSO:coordinate\n",
        "@SQ\tSN:chr1\tLN:1000000\n",
        "@SQ\tSN:chr2\tLN:1000000\n",
        "@RG\tID:known\tSM:synthetic\n",
        "@RG\tID:contradictory\tSM:first\n",
        "@RG\tID:contradictory\tSM:second\n"
    );
    let mut complete_tags =
        RecordSpec::mapped("complete-tags", 0, 700, vec![CigarOp::new(5, 'M')], "ACGTA");
    complete_tags.auxiliary.extend(aux_i32(*b"NM", 1));
    complete_tags.auxiliary.extend(aux_string(*b"MD", "2A2"));
    complete_tags.auxiliary.extend(aux_string(*b"RG", "known"));

    let missing_tags =
        RecordSpec::mapped("missing-tags", 0, 710, vec![CigarOp::new(5, 'M')], "ACGTA");
    let mut unknown_rg =
        RecordSpec::mapped("unknown-rg", 0, 720, vec![CigarOp::new(5, 'M')], "ACGTA");
    unknown_rg
        .auxiliary
        .extend(aux_string(*b"RG", "not-declared"));
    generated.push(write_fixture(
        repository_root,
        "tags_and_read_groups",
        &references,
        tags_header,
        &[complete_tags, missing_tags, unknown_rg],
        true,
        "valid",
        "-",
        None,
    )?);

    let regression = [
        RecordSpec::mapped("later", 0, 900, vec![CigarOp::new(5, 'M')], "AAAAA"),
        RecordSpec::mapped("earlier", 0, 800, vec![CigarOp::new(5, 'M')], "CCCCC"),
    ];
    generated.push(write_fixture(
        repository_root,
        "coordinate_regression",
        &references,
        HEADER,
        &regression,
        false,
        "error",
        "input_unsorted",
        None,
    )?);

    let unmapped_tail = [
        RecordSpec::mapped(
            "mapped-tail-anchor",
            0,
            950,
            vec![CigarOp::new(5, 'M')],
            "AAAAA",
        ),
        RecordSpec::unmapped("unmapped-tail", "NNNNN"),
    ];
    generated.push(write_fixture(
        repository_root,
        "unmapped_tail",
        &references,
        HEADER,
        &unmapped_tail,
        true,
        "valid",
        "-",
        None,
    )?);

    let unknown_reference = RecordSpec::mapped(
        "unknown-reference",
        7,
        10,
        vec![CigarOp::new(5, 'M')],
        "AAAAA",
    );
    generated.push(write_fixture(
        repository_root,
        "unknown_reference_id",
        &references,
        HEADER,
        &[unknown_reference],
        false,
        "error",
        "input_corrupt",
        None,
    )?);

    let boundary_references = vec![ReferenceSpec {
        name: String::from("chrBoundary"),
        length: i32::MAX,
    }];
    let boundary_header = "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chrBoundary\tLN:2147483647\n";
    let boundary = RecordSpec::mapped(
        "integer-boundary",
        0,
        536_870_900,
        vec![CigarOp::new(1, 'M')],
        "A",
    );
    generated.push(write_fixture(
        repository_root,
        "integer_boundary",
        &boundary_references,
        boundary_header,
        &[boundary],
        false,
        "valid",
        "-",
        None,
    )?);

    let zero_references = vec![ReferenceSpec {
        name: String::from("empty"),
        length: 0,
    }];
    let zero_header = "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:empty\tLN:0\n";
    generated.push(write_fixture(
        repository_root,
        "zero_length_reference",
        &zero_references,
        zero_header,
        &[],
        false,
        "valid",
        "-",
        None,
    )?);

    let chunk_boundary = [
        RecordSpec::mapped(
            "chunk-left",
            0,
            65_530,
            vec![CigarOp::new(20, 'M')],
            "A".repeat(20),
        ),
        RecordSpec::mapped(
            "chunk-right",
            0,
            65_536,
            vec![
                CigarOp::new(4, 'M'),
                CigarOp::new(20, 'N'),
                CigarOp::new(4, 'M'),
            ],
            "CCCCCCCC",
        ),
    ];
    generated.push(write_fixture(
        repository_root,
        "chunk_boundary",
        &references,
        HEADER,
        &chunk_boundary,
        true,
        "valid",
        "-",
        None,
    )?);

    let mut duplicate_track = RecordSpec::mapped(
        "duplicate-track",
        0,
        70_000,
        vec![CigarOp::new(10, 'M')],
        "AAAAAAAAAA",
    );
    duplicate_track.flags = 0x400;
    let mut qc_track = RecordSpec::mapped(
        "qc-track",
        0,
        70_005,
        vec![CigarOp::new(10, 'M')],
        "CCCCCCCCCC",
    );
    qc_track.flags = 0x200;
    let canonical_track = RecordSpec::mapped(
        "canonical-track",
        0,
        70_010,
        vec![CigarOp::new(10, 'M')],
        "GGGGGGGGGG",
    );
    generated.push(write_fixture(
        repository_root,
        "multi_track",
        &references,
        HEADER,
        &[duplicate_track, qc_track, canonical_track],
        true,
        "valid",
        "-",
        None,
    )?);

    let malformed_aux_path = fixtures.join("malformed_optional_tag.bam");
    let mut malformed_aux = RecordSpec::mapped(
        "malformed-aux",
        0,
        80_000,
        vec![CigarOp::new(5, 'M')],
        "AAAAA",
    );
    malformed_aux.auxiliary = vec![b'Z', b'Z', b'?'];
    write_bam(&malformed_aux_path, HEADER, &references, &[malformed_aux])?;
    generated.push(GeneratedFixture {
        id: String::from("malformed_optional_tag"),
        path: relative_fixture("malformed_optional_tag.bam"),
        index_path: None,
        validity: "error",
        error_category: "input_corrupt",
        expected_metrics: None,
    });

    let malformed_length_path = fixtures.join("malformed_record_length.bam");
    let malformed_length = serialize_malformed_record_length(HEADER, &references)?;
    fs::write(&malformed_length_path, malformed_length).map_err(|source| {
        TestkitError::io(
            "write malformed record-length BAM",
            &malformed_length_path,
            source,
        )
    })?;
    generated.push(GeneratedFixture {
        id: String::from("malformed_record_length"),
        path: relative_fixture("malformed_record_length.bam"),
        index_path: None,
        validity: "error",
        error_category: "input_corrupt",
        expected_metrics: None,
    });

    let truncation_source = serialize_bam(HEADER, &references, &basic_records)?;
    let truncated_path = fixtures.join("truncated_bgzf.bam");
    fs::write(&truncated_path, truncate_midstream(&truncation_source)?)
        .map_err(|source| TestkitError::io("write truncated BGZF BAM", &truncated_path, source))?;
    generated.push(GeneratedFixture {
        id: String::from("truncated_bgzf"),
        path: relative_fixture("truncated_bgzf.bam"),
        index_path: None,
        validity: "error",
        error_category: "input_corrupt",
        expected_metrics: None,
    });

    generated.sort_by(|left, right| left.id.cmp(&right.id));
    write_manifest(repository_root, &generated)?;
    Ok(())
}

fn references() -> Vec<ReferenceSpec> {
    vec![
        ReferenceSpec {
            name: String::from("chr1"),
            length: 1_000_000,
        },
        ReferenceSpec {
            name: String::from("chr2"),
            length: 1_000_000,
        },
    ]
}

#[allow(clippy::too_many_arguments)]
fn write_fixture(
    repository_root: &Path,
    id: &str,
    references: &[ReferenceSpec],
    header: &str,
    records: &[RecordSpec],
    index: bool,
    validity: &'static str,
    error_category: &'static str,
    expected_metrics: Option<PathBuf>,
) -> Result<GeneratedFixture> {
    let relative_path = relative_fixture(&format!("{id}.bam"));
    let path = repository_root.join(&relative_path);
    write_bam(&path, header, references, records)?;

    let index_path = if index {
        build_bai(&path)?;
        let generated_index = PathBuf::from(format!("{}.bai", path.display()));
        if !generated_index.exists() {
            return Err(TestkitError::generation(format!(
                "HTSlib did not create expected index {}",
                generated_index.display()
            )));
        }
        Some(PathBuf::from(format!("{}.bai", relative_path.display())))
    } else {
        None
    };

    Ok(GeneratedFixture {
        id: id.to_owned(),
        path: relative_path,
        index_path,
        validity,
        error_category,
        expected_metrics,
    })
}

fn relative_fixture(file_name: &str) -> PathBuf {
    Path::new("testdata").join("fixtures").join(file_name)
}

fn write_expected(path: &Path, rows: &[(&str, &str, &str, &str, &str)]) -> Result<()> {
    let mut output =
        String::from("metric\ttype\texpected\trounding_decimals\tcompatibility_note\n");
    for (metric, metric_type, expected, rounding, note) in rows {
        output.push_str(metric);
        output.push('\t');
        output.push_str(metric_type);
        output.push('\t');
        output.push_str(expected);
        output.push('\t');
        output.push_str(rounding);
        output.push('\t');
        output.push_str(note);
        output.push('\n');
    }
    fs::write(path, output)
        .map_err(|source| TestkitError::io("write expected metrics", path, source))
}

fn write_manifest(repository_root: &Path, fixtures: &[GeneratedFixture]) -> Result<()> {
    let mut output =
        String::from("schema\tid\tkind\tpath\tsha256\tindex_path\tindex_sha256\tsource_url\t");
    output.push_str("source_checksum\treference_build\tgeneration\tlicense\texpected_validity\t");
    output.push_str("expected_error\texpected_metrics\n");

    for fixture in fixtures {
        let path = repository_root.join(&fixture.path);
        let digest = sha256_file(&path)?;
        let (index_path, index_digest) = if let Some(relative_index) = &fixture.index_path {
            (
                relative_index.display().to_string(),
                sha256_file(&repository_root.join(relative_index))?,
            )
        } else {
            (String::from("-"), String::from("-"))
        };
        let expected_metrics = fixture
            .expected_metrics
            .as_ref()
            .map_or_else(|| String::from("-"), |path| path.display().to_string());
        let fields = [
            MANIFEST_SCHEMA.to_owned(),
            fixture.id.clone(),
            String::from("committed"),
            fixture.path.display().to_string(),
            digest,
            index_path,
            index_digest,
            String::from("generated:aligngauge-testkit-v1"),
            String::from("-"),
            String::from("synthetic-v1"),
            String::from(FIXTURE_COMMAND),
            String::from("Apache-2.0"),
            fixture.validity.to_owned(),
            fixture.error_category.to_owned(),
            expected_metrics,
        ];
        validate_tsv_fields(&fields)?;
        output.push_str(&fields.join("\t"));
        output.push('\n');
    }

    let hg002_fields = [
        MANIFEST_SCHEMA,
        "hg002-grch38-giabv3-chr20-10-11mb-30x",
        "external",
        "-",
        "-",
        "-",
        "-",
        "https://ftp-trace.ncbi.nlm.nih.gov/ReferenceSamples/giab/data/AshkenazimTrio/HG002_NA24385_son/Element_AVITI_20231018/HG002_GRCh38-GIABv3_Element-StdInsert_2X150_81x_20231018.bam",
        "md5:f5360b7adbc798c90a78f290de928eca",
        "GRCh38-GIABv3",
        "testdata/hg002/prepare.sh",
        "GIAB public reference data; preserve upstream attribution",
        "valid",
        "-",
        "-",
    ];
    output.push_str(&hg002_fields.join("\t"));
    output.push('\n');

    let manifest_path = repository_root.join("testdata/manifest.v1.tsv");
    fs::write(&manifest_path, output)
        .map_err(|source| TestkitError::io("write test-data manifest", manifest_path, source))
}

fn validate_tsv_fields(fields: &[String]) -> Result<()> {
    for field in fields {
        if field.contains(['\t', '\n', '\r']) {
            return Err(TestkitError::generation(
                "manifest field contains a TSV control character",
            ));
        }
    }
    Ok(())
}
