#!/usr/bin/env bash
set -euo pipefail

script_directory=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repository_root=$(cd -- "$script_directory/../../.." && pwd)
output_root="$repository_root/target/reference/m5-coverage"

[[ ! -e "$output_root" ]] || {
  echo "coverage differential destination already exists: $output_root" >&2
  exit 3
}
mkdir -p -- "$output_root"

cargo build -p aligngauge-coverage --bin coverage_probe --locked

for fixture in basic cigar_ops flags_and_pairs chunk_boundary multi_track; do
  fixture_root="$output_root/$fixture"
  mkdir -p -- "$fixture_root"
  "$script_directory/run-reference.sh" \
    depth \
    "testdata/fixtures/$fixture.bam" \
    "$fixture_root/reference-depth"
  python "$script_directory/summarize-depth.py" \
    "$fixture_root/reference-depth/stdout.txt" \
    "$fixture_root/reference-summary.json"
  "$repository_root/target/debug/coverage_probe" \
    "$repository_root/testdata/fixtures/$fixture.bam" \
    > "$fixture_root/aligngauge-probe.json"
  python "$script_directory/compare-coverage.py" \
    "$fixture_root/reference-summary.json" \
    "$fixture_root/aligngauge-probe.json"
done

printf 'complete\n' > "$output_root/_SUCCESS"
