#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 REPOSITORY_RELATIVE_BAM REPOSITORY_RELATIVE_BED OUTPUT_DIRECTORY" >&2
  exit 2
}

[[ $# -eq 3 ]] || usage
input_relative=$1
target_relative=$2
output_directory=$3

script_directory=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repository_root=$(cd -- "$script_directory/../../.." && pwd)
# shellcheck disable=SC1091
source "$script_directory/image.lock"
container_user="$(id -u):$(id -g)"

for relative in "$input_relative" "$target_relative"; do
  [[ "$relative" != /* && "$relative" != *".."* ]] || {
    echo "input paths must be repository-relative without '..': $relative" >&2
    exit 3
  }
done
input_path="$repository_root/$input_relative"
target_path="$repository_root/$target_relative"
[[ -f "$input_path" ]] || { echo "alignment does not exist: $input_path" >&2; exit 3; }
[[ -f "$target_path" ]] || { echo "target BED does not exist: $target_path" >&2; exit 3; }
[[ ! -e "$output_directory" ]] || {
  echo "output destination already exists: $output_directory" >&2
  exit 4
}

output_parent=$(dirname -- "$output_directory")
output_name=$(basename -- "$output_directory")
mkdir -p -- "$output_parent"
staging=$(mktemp -d "$output_parent/.${output_name}.staging.XXXXXX")
cleanup() { rm -rf -- "$staging"; }
trap cleanup EXIT

container_input="/work/$input_relative"
container_targets="/work/$target_relative"
samtools_arguments=(
  depth
  -a
  -q 0
  -Q 0
  -G UNMAP,SECONDARY,QCFAIL,DUP,SUPPLEMENTARY
  -b "$container_targets"
  "$container_input"
)
printf '%q ' samtools "${samtools_arguments[@]}" >"$staging/invocation.txt"
printf '\n' >>"$staging/invocation.txt"
printf '%s\n' "$SAMTOOLS_IMAGE" >"$staging/image.txt"

docker pull "$SAMTOOLS_IMAGE" >/dev/null
docker run --rm --network none --read-only --cap-drop ALL \
  --user "$container_user" \
  --security-opt no-new-privileges --pids-limit 128 --memory 1g --cpus 1 \
  --tmpfs /tmp:rw,noexec,nosuid,size=32m \
  "$SAMTOOLS_IMAGE" samtools --version >"$staging/version.txt"

set +e
/usr/bin/time -f '%e' -o "$staging/wall_seconds.txt" \
  docker run --rm --network none --read-only --cap-drop ALL \
    --user "$container_user" \
    --security-opt no-new-privileges --pids-limit 256 --memory 2g --cpus 2 \
    --tmpfs /tmp:rw,noexec,nosuid,size=64m \
    --volume "$repository_root:/work:ro" \
    "$SAMTOOLS_IMAGE" samtools "${samtools_arguments[@]}" \
    >"$staging/stdout.txt" 2>"$staging/stderr.txt"
status=$?
set -e
printf '%s\n' "$status" >"$staging/exit_status.txt"
[[ $status -eq 0 ]] || { cat "$staging/stderr.txt" >&2; exit "$status"; }
[[ -s "$staging/version.txt" && -s "$staging/wall_seconds.txt" ]] || {
  echo "target depth reference capture is incomplete" >&2
  exit 5
}
printf 'complete\n' >"$staging/_SUCCESS"
mv -- "$staging" "$output_directory"
trap - EXIT
