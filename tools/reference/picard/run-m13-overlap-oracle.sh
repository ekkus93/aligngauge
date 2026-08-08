#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <input.bam> <output-dir>" >&2
  exit 64
fi

input="$1"
outdir="$2"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
source "$repo_root/tools/reference/picard/image.lock"

[[ "$version" == "3.4.0" ]] || {
  echo "unexpected pinned Picard version: $version" >&2
  exit 1
}
[[ "$image" == "broadinstitute/picard@sha256:f11df229b5b49ea28a872c04fc5d33e76bd14890754079390f36897c16194b28" ]] || {
  echo "unexpected pinned Picard image: $image" >&2
  exit 1
}
[[ "$jar" == "/usr/picard/picard.jar" ]] || {
  echo "unexpected pinned Picard jar: $jar" >&2
  exit 1
}
[[ -f "$input" ]] || {
  echo "input BAM does not exist: $input" >&2
  exit 66
}
[[ -f "$repo_root/tools/reference/picard/M13OverlapOracle.java" ]] || {
  echo "M13 Picard oracle source is missing" >&2
  exit 66
}

input_abs="$(realpath "$input")"
input_dir="$(dirname "$input_abs")"
input_name="$(basename "$input_abs")"
mkdir -p "$outdir"
outdir_abs="$(realpath "$outdir")"
oracle_dir="$repo_root/tools/reference/picard"

rm -f \
  "$outdir_abs/oracle.tsv" \
  "$outdir_abs/stdout.tmp" \
  "$outdir_abs/stderr.txt" \
  "$outdir_abs/image.txt" \
  "$outdir_abs/invocation.txt" \
  "$outdir_abs/exit_status.txt" \
  "$outdir_abs/_SUCCESS"

printf '%s\n' "$image" > "$outdir_abs/image.txt"
printf 'Picard %s / HTSJDK 4.2.0 M13OverlapOracle input=%q\n' "$version" "$input_abs" > "$outdir_abs/invocation.txt"

docker pull "$image" >/dev/null

set +e
docker run --rm --network none \
  --entrypoint "$java" \
  -v "$input_dir:/input:ro" \
  -v "$oracle_dir:/oracle:ro" \
  "$image" \
  --class-path "$jar" \
  /oracle/M13OverlapOracle.java \
  "/input/$input_name" \
  > "$outdir_abs/stdout.tmp" \
  2> "$outdir_abs/stderr.txt"
status=$?
set -e
printf '%s\n' "$status" > "$outdir_abs/exit_status.txt"

if [[ $status -ne 0 ]]; then
  echo "pinned Picard/HTSJDK M13 overlap oracle failed with exit status $status" >&2
  cat "$outdir_abs/stderr.txt" >&2
  exit "$status"
fi

expected_keys=(
  wgs_retained_bases
  wgs_baseq_excluded_bases
  wgs_overlap_excluded_bases
  hs_overlap_clipped_read_bases
)
for key in "${expected_keys[@]}"; do
  [[ "$(grep -c "^${key}"$'\t' "$outdir_abs/stdout.tmp")" -eq 1 ]] || {
    echo "oracle output is missing or duplicates key: $key" >&2
    exit 1
  }
done
[[ "$(wc -l < "$outdir_abs/stdout.tmp")" -eq 4 ]] || {
  echo "oracle output contains unexpected lines" >&2
  cat "$outdir_abs/stdout.tmp" >&2
  exit 1
}

mv "$outdir_abs/stdout.tmp" "$outdir_abs/oracle.tsv"
touch "$outdir_abs/_SUCCESS"
