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
[[ "$image" == *@sha256:* ]] || {
  echo "Picard image must be pinned by immutable digest: $image" >&2
  exit 1
}
[[ -f "$input" ]] || {
  echo "input BAM does not exist: $input" >&2
  exit 66
}

input_abs="$(realpath "$input")"
input_dir="$(dirname "$input_abs")"
input_name="$(basename "$input_abs")"
mkdir -p "$outdir"
outdir_abs="$(realpath "$outdir")"

rm -f \
  "$outdir_abs/metrics.txt" \
  "$outdir_abs/output_state.txt" \
  "$outdir_abs/histogram.pdf" \
  "$outdir_abs/stdout.txt" \
  "$outdir_abs/stderr.txt" \
  "$outdir_abs/image.txt" \
  "$outdir_abs/invocation.txt" \
  "$outdir_abs/exit_status.txt" \
  "$outdir_abs/wall_seconds.txt" \
  "$outdir_abs/_SUCCESS"

printf '%s\n' "$image" > "$outdir_abs/image.txt"
printf 'Picard %s CollectInsertSizeMetrics INPUT=%q OUTPUT=metrics.txt HISTOGRAM_FILE=histogram.pdf\n' "$version" "$input_abs" > "$outdir_abs/invocation.txt"

docker pull "$image" >/dev/null

start_ns="$(date +%s%N)"
set +e
docker run --rm --network none \
  --entrypoint "$java" \
  -v "$input_dir:/input:ro" \
  -v "$outdir_abs:/output" \
  "$image" \
  -jar "$jar" CollectInsertSizeMetrics \
  INPUT="/input/$input_name" \
  OUTPUT=/output/metrics.txt \
  HISTOGRAM_FILE=/output/histogram.pdf \
  > "$outdir_abs/stdout.txt" \
  2> "$outdir_abs/stderr.txt"
status=$?
set -e
end_ns="$(date +%s%N)"
printf '%s\n' "$status" > "$outdir_abs/exit_status.txt"
python3 - "$start_ns" "$end_ns" > "$outdir_abs/wall_seconds.txt" <<'PY'
import sys
start = int(sys.argv[1])
end = int(sys.argv[2])
print(f"{(end - start) / 1_000_000_000:.6f}")
PY

if [[ $status -ne 0 ]]; then
  echo "Picard CollectInsertSizeMetrics failed with exit status $status" >&2
  cat "$outdir_abs/stderr.txt" >&2
  exit "$status"
fi

if [[ -f "$outdir_abs/metrics.txt" ]]; then
  printf 'present\n' > "$outdir_abs/output_state.txt"
else
  # Picard 3.4.0 succeeds without creating OUTPUT when no insert observations exist.
  # Preserve that fact separately and create an empty capture file only so downstream
  # exact comparators have a stable pathname.
  printf 'absent\n' > "$outdir_abs/output_state.txt"
  : > "$outdir_abs/metrics.txt"
fi
if [[ -s "$outdir_abs/metrics.txt" ]]; then
  grep -F $'## METRICS CLASS\tpicard.analysis.InsertSizeMetrics' "$outdir_abs/metrics.txt" >/dev/null || {
    echo "non-empty Picard insert-size output is missing the expected metrics class" >&2
    exit 1
  }
fi

touch "$outdir_abs/_SUCCESS"
