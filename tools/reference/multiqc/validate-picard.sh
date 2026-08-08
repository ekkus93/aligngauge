#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 <picard-insert-reference.txt> <aligngauge-insert.txt> <output-dir>" >&2
  exit 64
fi

reference_insert="$1"
actual_insert="$2"
outdir="$3"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
source "$repo_root/tools/reference/multiqc/image.lock"

[[ "$MULTIQC_VERSION" == "1.35" ]] || {
  echo "unexpected pinned MultiQC version: $MULTIQC_VERSION" >&2
  exit 1
}
[[ -f "$reference_insert" ]] || { echo "missing Picard insert-size reference: $reference_insert" >&2; exit 66; }
[[ -f "$actual_insert" ]] || { echo "missing AlignGauge insert-size metrics: $actual_insert" >&2; exit 66; }

wgs_fixture="$repo_root/tools/reference/multiqc/fixtures/picard-wgs-discovery.metrics.txt"
hs_fixture="$repo_root/tools/reference/multiqc/fixtures/picard-hs-discovery.metrics.txt"
[[ -f "$wgs_fixture" ]] || { echo "missing WGS discovery fixture: $wgs_fixture" >&2; exit 66; }
[[ -f "$hs_fixture" ]] || { echo "missing HsMetrics discovery fixture: $hs_fixture" >&2; exit 66; }

reference_abs="$(realpath "$reference_insert")"
actual_abs="$(realpath "$actual_insert")"
mkdir -p "$outdir"
outdir_abs="$(realpath "$outdir")"
work="$outdir_abs/work"
rm -rf "$work" "$outdir_abs/_SUCCESS" "$outdir_abs/report.json"
mkdir -p \
  "$work/reference-insert-input" \
  "$work/aligngauge-insert-input" \
  "$work/discovery-input"

# Use identical names so parsed reference/AlignGauge data can be byte-compared.
cp "$reference_abs" "$work/reference-insert-input/sample.picard.insert-size.txt"
cp "$actual_abs" "$work/aligngauge-insert-input/sample.picard.insert-size.txt"
cp "$wgs_fixture" "$work/discovery-input/sample.picard.wgs-metrics.txt"
cp "$hs_fixture" "$work/discovery-input/sample.picard.hs-metrics.txt"

printf '%s\n' "$MULTIQC_IMAGE" > "$outdir_abs/image.txt"
docker pull "$MULTIQC_IMAGE" >/dev/null
docker run --rm --network none "$MULTIQC_IMAGE" multiqc --version > "$outdir_abs/version.txt"
grep -Fx 'multiqc, version 1.35' "$outdir_abs/version.txt" >/dev/null

for side in reference aligngauge; do
  docker run --rm --network none \
    -v "$work:/work" \
    "$MULTIQC_IMAGE" \
    multiqc -f -m picard \
      -o "/work/${side}-insert-out" \
      "/work/${side}-insert-input" \
      > "$outdir_abs/${side}-insert.stdout.txt" \
      2> "$outdir_abs/${side}-insert.stderr.txt"

  parsed="$work/${side}-insert-out/multiqc_data/multiqc_picard_insertSize.txt"
  test -s "$parsed"
  grep -F 'PAIR_ORIENTATION' "$parsed" >/dev/null
  grep -F 'READ_PAIRS' "$parsed" >/dev/null
  grep -F 'MEAN_INSERT_SIZE' "$parsed" >/dev/null
done

cmp \
  "$work/reference-insert-out/multiqc_data/multiqc_picard_insertSize.txt" \
  "$work/aligngauge-insert-out/multiqc_data/multiqc_picard_insertSize.txt"

# These WGS/Hs files test only the pinned upstream discovery and parser contract.
# They are explicitly not AlignGauge compatibility output.
docker run --rm --network none \
  -v "$work:/work" \
  "$MULTIQC_IMAGE" \
  multiqc -f -m picard \
    -o /work/discovery-out \
    /work/discovery-input \
    > "$outdir_abs/discovery.stdout.txt" \
    2> "$outdir_abs/discovery.stderr.txt"

wgs_parsed="$work/discovery-out/multiqc_data/multiqc_picard_wgsmetrics.txt"
hs_parsed="$work/discovery-out/multiqc_data/multiqc_picard_HsMetrics.txt"
test -s "$wgs_parsed"
test -s "$hs_parsed"
grep -F 'PCT_EXC_OVERLAP' "$wgs_parsed" >/dev/null
grep -F 'PCT_30X' "$wgs_parsed" >/dev/null
grep -F 'FOLD_ENRICHMENT' "$hs_parsed" >/dev/null
grep -F 'FOLD_80_BASE_PENALTY' "$hs_parsed" >/dev/null
grep -F 'PCT_TARGET_BASES_30X' "$hs_parsed" >/dev/null

python - "$outdir_abs" <<'PY'
import hashlib
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
work = root / "work"


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()

insert = work / "aligngauge-insert-out/multiqc_data/multiqc_picard_insertSize.txt"
wgs = work / "discovery-out/multiqc_data/multiqc_picard_wgsmetrics.txt"
hs = work / "discovery-out/multiqc_data/multiqc_picard_HsMetrics.txt"
report = {
    "schema": "aligngauge-multiqc-picard-validation-v1",
    "status": "exact-supported-surfaces-plus-discovery-only-fixtures",
    "multiqc_version": "1.35",
    "supported_generated_output": {
        "picard_insert_size_3_4_0_all_reads_v1": {
            "parsed_reference_vs_aligngauge": "byte-identical",
            "parsed_sha256": digest(insert),
        }
    },
    "discovery_only": {
        "picard_wgs_metrics": {
            "status": "discovered-and-parsed",
            "parsed_sha256": digest(wgs),
            "compatibility_claim": False,
        },
        "picard_hs_metrics": {
            "status": "discovered-and-parsed",
            "parsed_sha256": digest(hs),
            "compatibility_claim": False,
        },
    },
}
(root / "report.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
PY

touch "$outdir_abs/_SUCCESS"
