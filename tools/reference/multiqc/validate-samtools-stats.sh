#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 <samtools-reference.txt> <aligngauge-stats.txt> <output-dir>" >&2
  exit 64
fi

reference="$1"
actual="$2"
outdir="$3"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
source "$repo_root/tools/reference/multiqc/image.lock"

[[ "$MULTIQC_VERSION" == "1.35" ]] || {
  echo "unexpected pinned MultiQC version: $MULTIQC_VERSION" >&2
  exit 1
}
[[ -f "$reference" ]] || { echo "missing Samtools reference text: $reference" >&2; exit 66; }
[[ -f "$actual" ]] || { echo "missing AlignGauge stats text: $actual" >&2; exit 66; }

reference_abs="$(realpath "$reference")"
actual_abs="$(realpath "$actual")"
mkdir -p "$outdir"
outdir_abs="$(realpath "$outdir")"
work="$outdir_abs/work"
rm -rf "$work" "$outdir_abs/_SUCCESS" "$outdir_abs/report.json"
mkdir -p "$work/reference-input" "$work/aligngauge-input"
cp "$reference_abs" "$work/reference-input/sample.samtools.stats"
cp "$actual_abs" "$work/aligngauge-input/sample.samtools.stats"

printf '%s\n' "$MULTIQC_IMAGE" > "$outdir_abs/image.txt"
docker pull "$MULTIQC_IMAGE" >/dev/null
docker run --rm --network none "$MULTIQC_IMAGE" multiqc --version > "$outdir_abs/version.txt"
grep -Fx 'multiqc, version 1.35' "$outdir_abs/version.txt" >/dev/null

for side in reference aligngauge; do
  docker run --rm --network none \
    -v "$work:/work" \
    "$MULTIQC_IMAGE" \
    multiqc -f -m samtools \
      -o "/work/${side}-out" \
      "/work/${side}-input" \
      > "$outdir_abs/${side}.stdout.txt" \
      2> "$outdir_abs/${side}.stderr.txt"
  test -s "$work/${side}-out/multiqc_data/multiqc_samtools_stats.txt"
  test -s "$work/${side}-out/multiqc_data/samtools_insert_size.txt"
done

cmp \
  "$work/reference-out/multiqc_data/multiqc_samtools_stats.txt" \
  "$work/aligngauge-out/multiqc_data/multiqc_samtools_stats.txt"
cmp \
  "$work/reference-out/multiqc_data/samtools_insert_size.txt" \
  "$work/aligngauge-out/multiqc_data/samtools_insert_size.txt"

python - "$outdir_abs" <<'PY'
import hashlib
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
work = root / "work"

def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()

sn = work / "aligngauge-out/multiqc_data/multiqc_samtools_stats.txt"
isize = work / "aligngauge-out/multiqc_data/samtools_insert_size.txt"
report = {
    "schema": "aligngauge-multiqc-samtools-stats-validation-v1",
    "status": "exact",
    "multiqc_version": "1.35",
    "compatibility_profile": "samtools-stats-1.24-multiqc-1.35",
    "parsed_sn_sha256": digest(sn),
    "parsed_insert_size_sha256": digest(isize),
    "comparison": {
        "multiqc_samtools_stats": "byte-identical",
        "samtools_insert_size": "byte-identical",
    },
}
(root / "report.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
PY

touch "$outdir_abs/_SUCCESS"
