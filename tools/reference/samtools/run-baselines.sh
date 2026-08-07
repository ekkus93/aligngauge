#!/usr/bin/env bash
set -euo pipefail

[[ $# -eq 2 ]] || {
  echo "usage: $0 REPOSITORY_RELATIVE_BAM OUTPUT_ROOT" >&2
  exit 2
}

script_directory=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
input_relative=$1
output_root=$2

[[ ! -e "$output_root" ]] || {
  echo "output root already exists: $output_root" >&2
  exit 3
}
mkdir -p -- "$output_root"
for command_name in flagstat idxstats depth; do
  "$script_directory/run-reference.sh" \
    "$command_name" \
    "$input_relative" \
    "$output_root/$command_name"
done
