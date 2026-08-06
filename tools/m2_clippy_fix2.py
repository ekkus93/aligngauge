from pathlib import Path

path = Path("crates/aligngauge-testkit/src/main.rs")
text = path.read_text(encoding="utf-8")
replacements = [
    (
        "match run(env::args_os().skip(1).collect()) {",
        "let arguments: Vec<_> = env::args_os().skip(1).collect();\n    match run(&arguments) {",
    ),
    (
        "fn run(arguments: Vec<std::ffi::OsString>) -> aligngauge_testkit::Result<()> {",
        "fn run(arguments: &[std::ffi::OsString]) -> aligngauge_testkit::Result<()> {",
    ),
    (
        "let error = run(Vec::new()).expect_err(\"missing command must fail\");",
        "let error = run(&[]).expect_err(\"missing command must fail\");",
    ),
]
for old, new in replacements:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one match for {old!r}, found {count}")
    text = text.replace(old, new)
path.write_text(text, encoding="utf-8")
Path(__file__).unlink()
