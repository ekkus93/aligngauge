#!/usr/bin/env python3
"""Add Milestone 9 targeted BAM/CRAM equivalence coverage."""

from pathlib import Path

path = Path("crates/aligngauge-cli/tests/cram_v0_2.rs")
text = path.read_text()

def replace_once(old: str, new: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one match, found {count}: {old[:100]!r}")
    text = text.replace(old, new, 1)

replace_once(
    "use aligngauge_cli::{analyze_release, analyze_release_with_reference};",
    "use aligngauge_cli::{\n"
    "    analyze_release, analyze_release_with_reference, analyze_release_with_reference_and_targets,\n"
    "};",
)
replace_once(
    "    wrong_reference: PathBuf,\n}",
    "    wrong_reference: PathBuf,\n    targets: PathBuf,\n}",
)
replace_once(
    "    let bam_path = root.join(\"equivalent.bam\");\n"
    "    let cram_path = root.join(\"equivalent.cram\");",
    "    let bam_path = root.join(\"equivalent.bam\");\n"
    "    let cram_path = root.join(\"equivalent.cram\");\n"
    "    let targets = root.join(\"targets.bed\");",
)
replace_once(
    "    fs::write(&wrong_reference, format!(\">chr1\\n{}\\n\", \"A\".repeat(1000)))\n"
    "        .expect(\"write wrong reference\");",
    "    fs::write(&wrong_reference, format!(\">chr1\\n{}\\n\", \"A\".repeat(1000)))\n"
    "        .expect(\"write wrong reference\");\n"
    "    fs::write(&targets, \"chr1\\t95\\t135\\tequivalent-target\\n\")\n"
    "        .expect(\"write target BED\");",
)
replace_once(
    "        wrong_reference,\n    }",
    "        wrong_reference,\n        targets,\n    }",
)
append = r'''

#[test]
fn targeted_bam_and_cram_have_identical_canonical_results() {
    let fixtures = make_pair();
    let bam = analyze_release_with_reference_and_targets(
        &config(&fixtures.bam),
        None,
        Some(&fixtures.targets),
        Some(5),
    )
    .expect("analyze targeted BAM");
    let cram = analyze_release_with_reference_and_targets(
        &config(&fixtures.cram),
        Some(&fixtures.reference),
        Some(&fixtures.targets),
        Some(5),
    )
    .expect("analyze targeted CRAM");

    assert_eq!(bam.input_traversals(), 1);
    assert_eq!(cram.input_traversals(), 1);
    assert_eq!(bam.counters(), cram.counters());
    assert_eq!(bam.coverage(), cram.coverage());
    assert_eq!(bam.summary(), cram.summary());
    let bam_targeted = bam.coverage().targeted().expect("BAM targeted summary");
    let cram_targeted = cram.coverage().targeted().expect("CRAM targeted summary");
    assert_eq!(bam_targeted.summary(), cram_targeted.summary());

    let mut bam_plan = bam.provenance().analysis_plan.clone();
    let mut cram_plan = cram.provenance().analysis_plan.clone();
    for key in [
        "input_format",
        "bam_traversals",
        "cram_traversals",
        "local_reference",
    ] {
        bam_plan.remove(key);
        cram_plan.remove(key);
    }
    assert_eq!(bam_plan, cram_plan);
    assert_eq!(
        bam.provenance().normalization_actions,
        cram.provenance().normalization_actions
    );
    assert_eq!(
        bam.provenance().analysis_plan.get("alignment_traversals"),
        Some(&aligngauge_core::JsonValue::Unsigned(1))
    );
    assert_eq!(
        bam.provenance().analysis_plan.get("target_path"),
        Some(&aligngauge_core::JsonValue::String(
            fixtures.targets.display().to_string()
        ))
    );
}
'''
path.write_text(text + append)
