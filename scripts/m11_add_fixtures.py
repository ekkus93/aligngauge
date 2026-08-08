from pathlib import Path

path = Path("crates/aligngauge-testkit/src/corpus.rs")
text = path.read_text()
anchor = "    let tags_header = concat!(\n"
if text.count(anchor) != 1:
    raise SystemExit(f"expected one tags_header anchor, found {text.count(anchor)}")
if '"picard_alignment_edge"' in text or '"picard_insert_edge"' in text:
    raise SystemExit("M11 Picard fixtures are already present")

block = r'''    let mut picard_alignment_records = Vec::new();
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
        100, 100, 100, 100, 100, 100, 100, 100, 100, 100,
        101, 101, 101, 101, 101, 101, 101, 101, 101, 101,
        102, 102, 102, 102, 102, 102, 102, 102,
        103, 103, 103, 103, 103, 103, 103, 103,
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

'''
path.write_text(text.replace(anchor, block + anchor, 1))
