#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 <prepared-full-hg002-directory> <output-directory-outside-repository>" >&2
  exit 64
}

[[ $# -eq 2 ]] || usage
prepared_directory=$1
output_directory=$2

script_directory=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repository_root=$(cd -- "$script_directory/../.." && pwd)
# shellcheck disable=SC1091
source "$repository_root/testdata/hg002/full-wgs-v0.5.env"
# shellcheck disable=SC1091
source "$repository_root/tools/reference/samtools/image.lock"
samtools_image=$SAMTOOLS_IMAGE
samtools_version=$SAMTOOLS_VERSION

for executable in awk cargo cmp cp df diff docker du findmnt free git grep jq lscpu mktemp python3 realpath sha256sum stat uname; do
  command -v "$executable" >/dev/null || {
    echo "required executable not found: $executable" >&2
    exit 69
  }
done

[[ -d "$prepared_directory" ]] || {
  echo "prepared full-HG002 directory does not exist: $prepared_directory" >&2
  exit 66
}
prepared_directory=$(realpath "$prepared_directory")
prepared_manifest="$prepared_directory/prepared.manifest"
prepared_bam="$prepared_directory/hg002-30x.bam"
prepared_bai="$prepared_directory/hg002-30x.bam.bai"
[[ -f "$prepared_directory/_SUCCESS" ]] || {
  echo "prepared full-HG002 directory is missing _SUCCESS" >&2
  exit 66
}
for required in "$prepared_manifest" "$prepared_bam" "$prepared_bai"; do
  [[ -s "$required" ]] || {
    echo "required prepared input is missing or empty: $required" >&2
    exit 66
  }
done

manifest_value() {
  local key=$1
  local count value
  count=$(awk -F= -v key="$key" '$1 == key {count += 1} END {print count + 0}' "$prepared_manifest")
  [[ "$count" == "1" ]] || {
    echo "prepared manifest must contain exactly one ${key}= entry" >&2
    exit 65
  }
  value=$(awk -v key="$key" 'index($0, key "=") == 1 {print substr($0, length(key) + 2)}' "$prepared_manifest")
  [[ -n "$value" ]] || {
    echo "prepared manifest entry ${key}= must not be empty" >&2
    exit 65
  }
  printf '%s\n' "$value"
}

[[ "$(manifest_value schema)" == "aligngauge-hg002-full-wgs-preparation-v1" ]]
[[ "$(manifest_value profile)" == "$HG002_V05_PROFILE" ]]
[[ "$(manifest_value source_bam_md5)" == "$HG002_SOURCE_BAM_MD5" ]]
[[ "$(manifest_value source_bai_md5)" == "$HG002_SOURCE_BAI_MD5" ]]
[[ "$(manifest_value reference_build)" == "$HG002_REFERENCE_BUILD" ]]
[[ "$(manifest_value target_nominal_depth)" == "$HG002_TARGET_NOMINAL_DEPTH" ]]
[[ "$(manifest_value region)" == "whole-alignment" ]]
[[ "$(manifest_value subsample_seed)" == "$HG002_SUBSAMPLE_SEED" ]]
[[ "$(manifest_value subsample_fraction)" == "$HG002_SUBSAMPLE_FRACTION" ]]
[[ "$(manifest_value samtools_version)" == "$samtools_version" ]]
[[ "$(manifest_value samtools_image)" == "$samtools_image" ]]
[[ "$(manifest_value prepared_bam)" == "hg002-30x.bam" ]]
[[ "$(manifest_value prepared_bai)" == "hg002-30x.bam.bai" ]]

expected_bam_sha=$(manifest_value prepared_bam_sha256)
expected_bai_sha=$(manifest_value prepared_bai_sha256)
printf '%s  %s\n' "$expected_bam_sha" "$prepared_bam" | sha256sum --check -
printf '%s  %s\n' "$expected_bai_sha" "$prepared_bai" | sha256sum --check -
[[ "$(stat -c '%s' "$prepared_bam")" == "$(manifest_value prepared_bam_size_bytes)" ]]
[[ "$(stat -c '%s' "$prepared_bai")" == "$(manifest_value prepared_bai_size_bytes)" ]]

cd "$repository_root"
[[ -z "$(git status --porcelain --untracked-files=all)" ]] || {
  echo "full HG002 qualification requires a clean repository" >&2
  git status --short >&2
  exit 65
}
qualification_commit=$(git rev-parse HEAD)

[[ ! -e "$output_directory" ]] || {
  echo "qualification output already exists: $output_directory" >&2
  exit 73
}
output_parent=$(dirname -- "$output_directory")
output_name=$(basename -- "$output_directory")
mkdir -p -- "$output_parent"
output_parent=$(realpath "$output_parent")
case "$output_parent/" in
  "$repository_root/"*)
    echo "full HG002 qualification output must be outside the repository: $output_parent" >&2
    exit 64
    ;;
esac
output_directory="$output_parent/$output_name"

minimum_output_free_bytes=$((16 * 1024 * 1024 * 1024))
free_kib=$(df -Pk "$output_parent" | awk 'NR==2 {print $4}')
[[ "$free_kib" =~ ^[0-9]+$ ]] || {
  echo "could not determine free output space" >&2
  exit 74
}
if (( free_kib * 1024 < minimum_output_free_bytes )); then
  echo "qualification output filesystem has less than 16 GiB free" >&2
  exit 75
fi

warmup_runs=${ALIGNGAUGE_V05_WARMUP_RUNS:-1}
measured_runs=${ALIGNGAUGE_V05_MEASURED_RUNS:-3}
[[ "$warmup_runs" =~ ^[0-9]+$ && "$measured_runs" =~ ^[0-9]+$ ]] || {
  echo "run counts must be unsigned integers" >&2
  exit 64
}
(( warmup_runs >= 1 )) || {
  echo "v0.5 qualification requires at least one warm-up run per execution mode" >&2
  exit 64
}
(( measured_runs >= 3 )) || {
  echo "v0.5 qualification requires at least three measured runs per execution mode" >&2
  exit 64
}

memory_limit=${ALIGNGAUGE_V05_MEMORY_LIMIT:-8GiB}

staging=$(mktemp -d "$output_parent/.${output_name}.staging.XXXXXX")
cleanup() {
  rm -rf -- "$staging"
}
trap cleanup EXIT
mkdir -p "$staging/environment" "$staging/runs" "$staging/differential"

cp "$prepared_manifest" "$staging/prepared.manifest"
printf '%s\n' "$qualification_commit" >"$staging/aligngauge-commit.txt"
printf 'warm-cache-after-explicit-warmup\n' >"$staging/cache-policy.txt"
printf '%s\n' "$memory_limit" >"$staging/memory-limit.txt"
printf '%s\n' "$warmup_runs" >"$staging/warmup-runs.txt"
printf '%s\n' "$measured_runs" >"$staging/measured-runs.txt"
uname -a >"$staging/environment/uname.txt"
lscpu >"$staging/environment/lscpu.txt"
free -b >"$staging/environment/memory.txt"
df -PT "$prepared_bam" "$output_parent" >"$staging/environment/filesystems.txt"
findmnt -T "$prepared_bam" >"$staging/environment/input-mount.txt"
findmnt -T "$output_parent" >"$staging/environment/output-mount.txt"

cargo build --release -p aligngauge-cli --bin aligngauge --locked
cargo build --release -p aligngauge-coverage --bin coverage_probe --locked
binary="$repository_root/target/release/aligngauge"
coverage_probe="$repository_root/target/release/coverage_probe"
[[ -x "$binary" && -x "$coverage_probe" ]]
sha256sum "$binary" "$coverage_probe" >"$staging/aligngauge-binaries.sha256"

run_release_mode() {
  local mode=$1
  local io_threads=$2
  local kind=$3
  local iteration=$4
  local root="$staging/runs/$mode/$kind-$iteration"
  mkdir -p "$root"
  python3 "$script_directory/measure-command.py" \
    --output "$root/measurement.json" -- \
    "$binary" qc \
      --input "$prepared_bam" \
      --outdir "$root/output" \
      --io-threads "$io_threads" \
      --memory-limit "$memory_limit" \
      --quiet
  [[ -f "$root/output/_SUCCESS" ]]
  if [[ "$io_threads" == "0" ]]; then
    jq -e \
      '.resolved_config.io_threads == 0 and .analysis_plan.effective_reader_io_threads == 1 and .analysis_plan.collector_threads_used == 1' \
      "$root/output/provenance.json" >/dev/null
  else
    jq -e \
      --argjson io "$io_threads" \
      '.resolved_config.io_threads == $io and .analysis_plan.effective_reader_io_threads == $io and .analysis_plan.collector_threads_used == 1' \
      "$root/output/provenance.json" >/dev/null
  fi
  sha256sum "$root/output/summary.json" >"$root/summary.sha256"
  du -sb "$root/output" | awk '{print $1}' >"$root/output-size-bytes.txt"
}

for mode_and_threads in serial:0 io2:2; do
  mode=${mode_and_threads%%:*}
  io_threads=${mode_and_threads##*:}
  for ((iteration = 1; iteration <= warmup_runs; iteration++)); do
    run_release_mode "$mode" "$io_threads" warmup "$iteration"
  done
  for ((iteration = 1; iteration <= measured_runs; iteration++)); do
    run_release_mode "$mode" "$io_threads" measured "$iteration"
  done
done

baseline="$staging/runs/serial/measured-1/output/summary.json"
for mode in serial io2; do
  for ((iteration = 1; iteration <= measured_runs; iteration++)); do
    cmp "$baseline" "$staging/runs/$mode/measured-$iteration/output/summary.json"
  done
done
printf 'byte-identical\n' >"$staging/canonical-mode-equivalence.txt"

bam_dir=$(dirname -- "$prepared_bam")
bam_name=$(basename -- "$prepared_bam")
container_user="$(id -u):$(id -g)"
docker pull "$samtools_image" >/dev/null

samtools_reference() {
  local command_name=$1
  local output=$2
  docker run --rm --network none --read-only --cap-drop ALL \
    --user "$container_user" \
    --security-opt no-new-privileges --pids-limit 256 --memory 4g --cpus 4 \
    --tmpfs /tmp:rw,noexec,nosuid,size=128m \
    --volume "$bam_dir:/input:ro" \
    "$samtools_image" samtools "$command_name" "/input/$bam_name" >"$output"
  [[ -s "$output" ]]
}

samtools_reference flagstat "$staging/differential/reference.flagstat.txt"
"$binary" qc --input "$prepared_bam" --format samtools-flagstat \
  >"$staging/differential/aligngauge.flagstat.txt"
python3 tools/reference/samtools/compare-flagstat.py \
  "$staging/differential/reference.flagstat.txt" \
  "$staging/differential/aligngauge.flagstat.txt" \
  >"$staging/differential/flagstat-comparison.txt"

samtools_reference idxstats "$staging/differential/reference.idxstats.txt"
"$binary" qc --input "$prepared_bam" --format samtools-idxstats \
  >"$staging/differential/aligngauge.idxstats.txt"
diff -u \
  "$staging/differential/reference.idxstats.txt" \
  "$staging/differential/aligngauge.idxstats.txt" \
  >"$staging/differential/idxstats.diff"

bash tools/reference/samtools/run-stats.sh \
  "$prepared_bam" "$staging/differential/samtools-stats-reference"
"$binary" qc --input "$prepared_bam" --format samtools-stats \
  >"$staging/differential/aligngauge.stats.txt"
python3 tools/reference/samtools/compare-stats.py \
  "$staging/differential/samtools-stats-reference/stdout.txt" \
  "$staging/differential/aligngauge.stats.txt" \
  "$staging/differential/samtools-stats-comparison.json"
bash tools/reference/multiqc/validate-samtools-stats.sh \
  "$staging/differential/samtools-stats-reference/stdout.txt" \
  "$staging/differential/aligngauge.stats.txt" \
  "$staging/differential/multiqc-samtools"

# Stream whole-genome depth into the reducer. The multi-billion-line depth text
# is never materialized on disk.
docker run --rm --network none --read-only --cap-drop ALL \
  --user "$container_user" \
  --security-opt no-new-privileges --pids-limit 256 --memory 4g --cpus 4 \
  --tmpfs /tmp:rw,noexec,nosuid,size=128m \
  --volume "$bam_dir:/input:ro" \
  "$samtools_image" \
  samtools depth -aa -q 0 -Q 0 -G UNMAP,SECONDARY,QCFAIL,DUP,SUPPLEMENTARY \
  "/input/$bam_name" \
  | python3 tools/reference/samtools/summarize-depth.py \
      /dev/stdin "$staging/differential/reference-coverage.json"
"$coverage_probe" "$prepared_bam" \
  >"$staging/differential/aligngauge-coverage-probe.json"
python3 tools/reference/samtools/compare-coverage.py \
  "$staging/differential/reference-coverage.json" \
  "$staging/differential/aligngauge-coverage-probe.json" \
  >"$staging/differential/coverage-comparison.txt"

bash tools/reference/picard/run-alignment-summary.sh \
  "$prepared_bam" "$staging/differential/picard-alignment-reference"
"$binary" qc --input "$prepared_bam" --format picard-alignment-summary \
  >"$staging/differential/aligngauge-picard-alignment.txt"
python3 tools/reference/picard/compare-alignment-summary.py \
  "$staging/differential/picard-alignment-reference/metrics.txt" \
  "$staging/differential/aligngauge-picard-alignment.txt" \
  "$staging/differential/picard-alignment-comparison.json"

bash tools/reference/picard/run-insert-size.sh \
  "$prepared_bam" "$staging/differential/picard-insert-reference"
"$binary" qc --input "$prepared_bam" --format picard-insert-size \
  >"$staging/differential/aligngauge-picard-insert.txt"
python3 tools/reference/picard/compare-insert-size.py \
  "$staging/differential/picard-insert-reference/metrics.txt" \
  "$staging/differential/aligngauge-picard-insert.txt" \
  "$staging/differential/picard-insert-comparison.json"
bash tools/reference/multiqc/validate-picard.sh \
  "$staging/differential/picard-insert-reference/metrics.txt" \
  "$staging/differential/aligngauge-picard-insert.txt" \
  "$staging/differential/multiqc-picard"

python3 - "$staging" "$qualification_commit" "$prepared_bam" "$prepared_bai" <<'PY'
import hashlib
import json
import statistics
import sys
from pathlib import Path

root = Path(sys.argv[1])
commit = sys.argv[2]
bam = Path(sys.argv[3])
bai = Path(sys.argv[4])
measured_runs = int((root / "measured-runs.txt").read_text().strip())


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_runs(mode: str) -> list[dict[str, object]]:
    runs = []
    for index in range(1, measured_runs + 1):
        run_root = root / "runs" / mode / f"measured-{index}"
        measurement = json.loads((run_root / "measurement.json").read_text())
        measurement["iteration"] = index
        measurement["output_size_bytes"] = int(
            (run_root / "output-size-bytes.txt").read_text().strip()
        )
        measurement["summary_sha256"] = (
            (run_root / "summary.sha256").read_text().split()[0]
        )
        runs.append(measurement)
    return runs


def stats(runs: list[dict[str, object]]) -> dict[str, object]:
    wall = [float(run["wall_seconds"]) for run in runs]
    rss = [int(run["peak_rss_kib"]) for run in runs]
    return {
        "wall_seconds": {
            "min": min(wall),
            "max": max(wall),
            "mean": statistics.fmean(wall),
            "population_stdev": statistics.pstdev(wall),
        },
        "peak_rss_kib": {
            "min": min(rss),
            "max": max(rss),
            "mean": statistics.fmean(rss),
        },
    }

serial = load_runs("serial")
io2 = load_runs("io2")
report = {
    "schema": "aligngauge-v0.5-full-hg002-qualification-v1",
    "status": "exact",
    "aligngauge_commit": commit,
    "input": {
        "bam_sha256": sha256(bam),
        "bai_sha256": sha256(bai),
        "bam_size_bytes": bam.stat().st_size,
        "bai_size_bytes": bai.stat().st_size,
    },
    "cache_policy": (root / "cache-policy.txt").read_text().strip(),
    "warmup_runs_per_mode": int((root / "warmup-runs.txt").read_text().strip()),
    "measured_runs_per_mode": measured_runs,
    "memory_limit": (root / "memory-limit.txt").read_text().strip(),
    "mode_equivalence": "byte-identical-summary-json",
    "modes": {
        "serial_io_threads_0": {"runs": serial, "statistics": stats(serial)},
        "released_io_threads_2": {"runs": io2, "statistics": stats(io2)},
    },
    "differentials": {
        "samtools_flagstat": "exact",
        "samtools_idxstats": "byte-identical",
        "samtools_stats": "exact",
        "canonical_coverage": "exact",
        "picard_alignment_summary": "exact-released-subset",
        "picard_insert_size": "exact-released-profile",
        "multiqc_samtools_stats": "exact-parsed-data",
        "multiqc_picard_insert_size": "exact-parsed-data",
    },
    "deferred": {
        "picard_wgs": True,
        "picard_hs_metrics": True,
        "indexed_partition_parallelism": True,
        "collector_parallelism": True,
    },
}
(root / "qualification.json").write_text(
    json.dumps(report, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)
PY

[[ -s "$staging/qualification.json" ]]
[[ -z "$(git status --porcelain --untracked-files=all)" ]] || {
  echo "qualification modified tracked repository state" >&2
  git status --short >&2
  exit 1
}
printf 'complete\n' >"$staging/_SUCCESS"
mv -- "$staging" "$output_directory"
trap - EXIT
printf 'full HG002 v0.5 qualification complete: %s\n' "$output_directory"
