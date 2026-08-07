#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <input.bam> <output-dir>" >&2
  exit 64
fi

input="$1"
outdir="$2"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
source "$repo_root/tools/reference/samtools/image.lock"

[[ "$SAMTOOLS_VERSION" == "1.24" ]] || {
  echo "unexpected pinned Samtools version: $SAMTOOLS_VERSION" >&2
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
  "$outdir_abs/stdout.txt" \
  "$outdir_abs/stderr.txt" \
  "$outdir_abs/version.txt" \
  "$outdir_abs/image.txt" \
  "$outdir_abs/invocation.txt" \
  "$outdir_abs/exit_status.txt" \
  "$outdir_abs/wall_seconds.txt" \
  "$outdir_abs/_SUCCESS"

printf '%s\n' "$SAMTOOLS_IMAGE" > "$outdir_abs/image.txt"
printf 'samtools stats %q\n' "$input_abs" > "$outdir_abs/invocation.txt"

docker pull "$SAMTOOLS_IMAGE" >/dev/null

docker run --rm --network none "$SAMTOOLS_IMAGE" samtools --version \
  > "$outdir_abs/version.txt"
grep -Fx 'samtools 1.24' "$outdir_abs/version.txt" >/dev/null

start_ns="$(date +%s%N)"
set +e
docker run --rm --network none \
  -v "$input_dir:/input:ro" \
  "$SAMTOOLS_IMAGE" \
  samtools stats "/input/$input_name" \
  > "$outdir_abs/stdout.txt" \
  2> "$outdir_abs/stderr.txt"
status=$?
set -e
end_ns="$(date +%s%N)"
printf '%s\n' "$status" > "$outdir_abs/exit_status.txt"
python - "$start_ns" "$end_ns" > "$outdir_abs/wall_seconds.txt" <<'PY'
import sys
start = int(sys.argv[1])
end = int(sys.argv[2])
print(f"{(end - start) / 1_000_000_000:.6f}")
PY

if [[ $status -ne 0 ]]; then
  echo "samtools stats failed with exit status $status" >&2
  cat "$outdir_abs/stderr.txt" >&2
  exit "$status"
fi

grep -q '^SN' "$outdir_abs/stdout.txt" || {
  echo "samtools stats output is missing the SN section" >&2
  exit 1
}

touch "$outdir_abs/_SUCCESS"
