#!/usr/bin/env bash
set -euo pipefail

repository_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)
cd "$repository_root"

cargo build -p aligngauge-cli --locked
binary="$repository_root/target/debug/aligngauge"
output_root="$repository_root/target/reference/m4-counters"
rm -rf -- "$output_root"
mkdir -p -- "$output_root"

while IFS=$'\t' read -r schema id kind path sha256 index_path index_sha256 source_url source_checksum reference_build generation license validity error metrics; do
  [[ "$schema" != "schema" ]] || continue
  [[ "$kind" == "committed" && "$validity" == "valid" && "$path" == *.bam ]] || continue

  fixture_root="$output_root/$id"
  mkdir -p -- "$fixture_root"
  tools/reference/samtools/run-reference.sh flagstat "$path" "$fixture_root/reference-flagstat"
  tools/reference/samtools/run-reference.sh idxstats "$path" "$fixture_root/reference-idxstats"

  "$binary" qc --input "$path" --format samtools-flagstat >"$fixture_root/aligngauge-flagstat.txt"
  "$binary" qc --input "$path" --format samtools-idxstats >"$fixture_root/aligngauge-idxstats.txt"

  tools/reference/samtools/compare-flagstat.py \
    "$fixture_root/reference-flagstat/stdout.txt" \
    "$fixture_root/aligngauge-flagstat.txt"
  diff -u \
    "$fixture_root/reference-idxstats/stdout.txt" \
    "$fixture_root/aligngauge-idxstats.txt"
  printf 'complete\n' >"$fixture_root/_SUCCESS"
done < testdata/manifest.v1.tsv

test -f "$output_root/basic/_SUCCESS"
test -f "$output_root/flags_and_pairs/_SUCCESS"
