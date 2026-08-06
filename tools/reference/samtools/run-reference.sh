#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 {flagstat|idxstats|depth} REPOSITORY_RELATIVE_BAM OUTPUT_DIRECTORY" >&2
  exit 2
}

[[ $# -eq 3 ]] || usage
command_name=$1
input_relative=$2
output_directory=$3

case "$command_name" in
  flagstat|idxstats|depth) ;;
  *) usage ;;
esac

script_directory=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repository_root=$(cd -- "$script_directory/../../.." && pwd)
# shellcheck disable=SC1091
source "$script_directory/image.lock"
container_user="$(id -u):$(id -g)"

input_path="$repository_root/$input_relative"
[[ -f "$input_path" ]] || {
  echo "input does not exist: $input_path" >&2
  exit 3
}
[[ "$input_relative" != /* && "$input_relative" != *".."* ]] || {
  echo "input must be a repository-relative path without '..'" >&2
  exit 3
}
[[ ! -e "$output_directory" ]] || {
  echo "output destination already exists: $output_directory" >&2
  exit 4
}

output_parent=$(dirname -- "$output_directory")
output_name=$(basename -- "$output_directory")
mkdir -p -- "$output_parent"
staging=$(mktemp -d "$output_parent/.${output_name}.staging.XXXXXX")
cleanup() {
  rm -rf -- "$staging"
}
trap cleanup EXIT

container_input="/work/$input_relative"
case "$command_name" in
  flagstat)
    samtools_arguments=(flagstat "$container_input")
    ;;
  idxstats)
    samtools_arguments=(idxstats "$container_input")
    ;;
  depth)
    samtools_arguments=(
      depth
      -aa
      -q 0
      -Q 0
      -G UNMAP,SECONDARY,QCFAIL,DUP,SUPPLEMENTARY
      "$container_input"
    )
    ;;
esac

printf '%q ' samtools "${samtools_arguments[@]}" >"$staging/invocation.txt"
printf '\n' >>"$staging/invocation.txt"
printf '%s\n' "$SAMTOOLS_IMAGE" >"$staging/image.txt"

docker pull "$SAMTOOLS_IMAGE" >/dev/null
docker run --rm --network none --read-only --cap-drop ALL \
  --user "$container_user" \
  --security-opt no-new-privileges --pids-limit 256 --memory 2g --cpus 2 \
  --tmpfs /tmp:rw,noexec,nosuid,size=64m \
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

[[ $status -eq 0 ]] || {
  cat "$staging/stderr.txt" >&2
  exit "$status"
}
[[ -s "$staging/stdout.txt" ]] || {
  echo "reference command produced empty stdout" >&2
  exit 5
}
[[ -s "$staging/version.txt" && -s "$staging/wall_seconds.txt" ]] || {
  echo "reference capture is incomplete" >&2
  exit 5
}

printf 'complete\n' >"$staging/_SUCCESS"
mv -- "$staging" "$output_directory"
trap - EXIT
