from pathlib import Path


def replace_once(path: str, old: str, new: str, label: str) -> None:
    file = Path(path)
    text = file.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    file.write_text(text.replace(old, new), encoding="utf-8")


replace_once(
    "crates/aligngauge-testkit/src/lib.rs",
    "//! Deterministic test-data and differential-validation support for AlignGauge.",
    "//! Deterministic test-data and differential-validation support for `AlignGauge`.",
    "AlignGauge doc markup",
)
replace_once(
    "crates/aligngauge-testkit/src/error.rs",
    "    /// HTSlib rejected a generated fixture or index operation.",
    "    /// `HTSlib` rejected a generated fixture or index operation.",
    "HTSlib enum doc markup",
)
replace_once(
    "crates/aligngauge-testkit/src/error.rs",
    "    /// Construct an HTSlib failure.",
    "    /// Construct an `HTSlib` failure.",
    "HTSlib constructor doc markup",
)

bam_replacements = [
    (
        "/// Build a `TAG:i` auxiliary field.\npub fn aux_i32",
        "/// Build a `TAG:i` auxiliary field.\n#[must_use]\npub fn aux_i32",
        "aux_i32 must use",
    ),
    (
        "/// Build a `TAG:Z` auxiliary field.\npub fn aux_string",
        "/// Build a `TAG:Z` auxiliary field.\n#[must_use]\npub fn aux_string",
        "aux_string must use",
    ),
    (
        "/// Serialize a complete deterministic BAM file.\npub fn serialize_bam",
        "/// Serialize a complete deterministic BAM file.\n///\n/// # Errors\n/// Returns an error when a header, reference, record, CIGAR, sequence, or BGZF\n/// value cannot be represented without overflow or violates the BAM contract.\npub fn serialize_bam",
        "serialize_bam errors doc",
    ),
    (
        "/// Serialize a BAM whose final record declares a larger block than is present.\npub fn serialize_malformed_record_length",
        "/// Serialize a BAM whose final record declares a larger block than is present.\n///\n/// # Errors\n/// Returns an error when the requested header or reference table cannot be\n/// represented as BAM/BGZF data.\npub fn serialize_malformed_record_length",
        "malformed length errors doc",
    ),
    (
        "/// Write a deterministic BAM file.\npub fn write_bam",
        "/// Write a deterministic BAM file.\n///\n/// # Errors\n/// Returns serialization failures or the underlying filesystem write error.\npub fn write_bam",
        "write_bam errors doc",
    ),
    (
        "/// Build a BAI index for a valid coordinate-sorted BAM.\npub fn build_bai",
        "/// Build a BAI index for a valid coordinate-sorted BAM.\n///\n/// # Errors\n/// Returns an error when `HTSlib` rejects the BAM or cannot write its index.\npub fn build_bai",
        "build_bai errors doc",
    ),
    (
        "/// Remove bytes from the middle of a BGZF stream to create deterministic corruption.\npub fn truncate_midstream",
        "/// Remove bytes from the middle of a BGZF stream to create deterministic corruption.\n///\n/// # Errors\n/// Returns an error when the stream is too small to truncate deterministically.\npub fn truncate_midstream",
        "truncate errors doc",
    ),
]
for old, new, label in bam_replacements:
    replace_once("crates/aligngauge-testkit/src/bam.rs", old, new, label)

replace_once(
    "crates/aligngauge-testkit/src/corpus.rs",
    "/// The function creates only local files and performs no network access.\npub fn generate_corpus",
    "/// The function creates only local files and performs no network access.\n///\n/// # Errors\n/// Returns an error on invalid fixture definitions, serialization/indexing\n/// failure, checksum failure, or any local filesystem failure.\n///\n/// The body is intentionally the canonical declarative fixture catalog. Keeping\n/// the entries together makes manifest ordering and corpus review explicit.\n#[allow(clippy::too_many_lines)]\npub fn generate_corpus",
    "generate corpus docs and scope",
)

differential_replacements = [
    (
        "/// Parse expected-result TSV.\npub fn parse_expected",
        "/// Parse expected-result TSV.\n///\n/// # Errors\n/// Returns an error for malformed rows, duplicate keys, invalid types, invalid\n/// values, or missing per-field decimal-rounding rules.\npub fn parse_expected",
        "parse_expected errors doc",
    ),
    (
        "/// Parse actual-result TSV.\npub fn parse_actual",
        "/// Parse actual-result TSV.\n///\n/// # Errors\n/// Returns an error for malformed rows, duplicate keys, invalid types, or\n/// missing values.\npub fn parse_actual",
        "parse_actual errors doc",
    ),
    (
        "/// Compare expected and actual metric vectors.\npub fn compare",
        "/// Compare expected and actual metric vectors.\n///\n/// # Errors\n/// Returns an error when a declared integer or decimal value is invalid, a\n/// decimal rule is absent, or an internal comparison invariant fails.\npub fn compare",
        "compare errors doc",
    ),
    (
        "/// Compare two TSV files and write a deterministic JSON report.\npub fn compare_files",
        "/// Compare two TSV files and write a deterministic JSON report.\n///\n/// # Errors\n/// Returns local I/O, parsing, comparison, or report-publication failures.\npub fn compare_files",
        "compare_files errors doc",
    ),
]
for old, new, label in differential_replacements:
    replace_once("crates/aligngauge-testkit/src/differential.rs", old, new, label)

hash_replacements = [
    (
        "/// This function performs no network access.\npub fn sha256_file",
        "/// This function performs no network access.\n///\n/// # Errors\n/// Returns an error when the local file cannot be opened or read.\npub fn sha256_file",
        "sha256 errors doc",
    ),
    (
        "    let mut buffer = [0_u8; 64 * 1024];",
        "    let mut buffer = vec![0_u8; 64 * 1024];",
        "heap hash buffer",
    ),
    (
        "/// Verify a local file against a lowercase SHA-256 digest.\npub fn verify_sha256",
        "/// Verify a local file against a lowercase SHA-256 digest.\n///\n/// # Errors\n/// Returns an error for noncanonical digest text, local I/O failure, or a\n/// checksum mismatch.\npub fn verify_sha256",
        "verify sha errors doc",
    ),
    (
        "/// Validate the canonical lowercase SHA-256 text form.\npub fn validate_sha256",
        "/// Validate the canonical lowercase SHA-256 text form.\n///\n/// # Errors\n/// Returns an error unless the value is exactly 64 lowercase hexadecimal\n/// characters.\npub fn validate_sha256",
        "validate sha errors doc",
    ),
]
for old, new, label in hash_replacements:
    replace_once("crates/aligngauge-testkit/src/hash.rs", old, new, label)

manifest_replacements = [
    (
        "    /// Verify all committed local files without performing network access.\n    pub fn verify_local",
        "    /// Verify all committed local files without performing network access.\n    ///\n    /// # Errors\n    /// Returns an error for an incomplete identity, missing local file, checksum\n    /// mismatch, or missing expected-metrics file.\n    pub fn verify_local",
        "entry verify errors doc",
    ),
    (
        "    /// Parse strict UTF-8 TSV text.\n    pub fn parse",
        "    /// Parse strict UTF-8 TSV text.\n    ///\n    /// # Errors\n    /// Returns an error for any schema, field-count, identity, checksum-text,\n    /// duplicate-key, or expected-validity contract violation.\n    pub fn parse",
        "manifest parse errors doc",
    ),
    (
        "    /// Load and parse a manifest from a local path.\n    pub fn load",
        "    /// Load and parse a manifest from a local path.\n    ///\n    /// # Errors\n    /// Returns local read failures or any manifest parsing error.\n    pub fn load",
        "manifest load errors doc",
    ),
    (
        "    /// Verify every committed artifact using local filesystem reads only.\n    pub fn verify_local",
        "    /// Verify every committed artifact using local filesystem reads only.\n    ///\n    /// # Errors\n    /// Returns the first committed-entry identity, local I/O, or checksum error.\n    pub fn verify_local",
        "manifest verify errors doc",
    ),
    (
        "        let path = required_path(&self.path, \"path\", &self.id)?;\n        let digest = required_text(&self.sha256, \"sha256\", &self.id)?;",
        "        let path = required_path(self.path.as_deref(), \"path\", &self.id)?;\n        let digest = required_text(self.sha256.as_deref(), \"sha256\", &self.id)?;",
        "required identity calls",
    ),
    (
        "            validate_local_identity(line_number, kind, &path, &sha256)?;",
        "            validate_local_identity(\n                line_number,\n                kind,\n                path.as_deref(),\n                sha256.as_deref(),\n            )?;",
        "local identity call",
    ),
    (
        "            validate_source_checksum(line_number, &source_checksum)?;",
        "            validate_source_checksum(line_number, source_checksum.as_deref())?;",
        "source checksum call",
    ),
    (
        "fn required_path<'a>(value: &'a Option<PathBuf>, name: &str, id: &str) -> Result<&'a Path> {\n    value.as_deref().ok_or_else(|| {",
        "fn required_path<'a>(value: Option<&'a Path>, name: &str, id: &str) -> Result<&'a Path> {\n    value.ok_or_else(|| {",
        "required path signature",
    ),
    (
        "fn required_text<'a>(value: &'a Option<String>, name: &str, id: &str) -> Result<&'a str> {\n    value.as_deref().ok_or_else(|| {",
        "fn required_text<'a>(value: Option<&'a str>, name: &str, id: &str) -> Result<&'a str> {\n    value.ok_or_else(|| {",
        "required text signature",
    ),
    (
        "    path: &Option<PathBuf>,\n    sha256: &Option<String>,",
        "    path: Option<&Path>,\n    sha256: Option<&str>,",
        "local identity signature",
    ),
    (
        "fn validate_source_checksum(line: usize, value: &Option<String>) -> Result<()> {",
        "fn validate_source_checksum(line: usize, value: Option<&str>) -> Result<()> {",
        "source checksum signature",
    ),
]
for old, new, label in manifest_replacements:
    replace_once("crates/aligngauge-testkit/src/manifest.rs", old, new, label)

Path(__file__).unlink()
