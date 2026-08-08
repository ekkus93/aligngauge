#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 <complete-source.bam> <complete-source.bam.bai> <output-directory>" >&2
  exit 64
}

[[ $# -eq 3 ]] || usage
source_bam=$1
source_bai=$2
output_directory=$3

script_directory=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repository_root=$(cd -- "$script_directory/../.." && pwd)
# shellcheck disable=SC1091
source "$script_directory/full-wgs-v0.5.env"
# shellcheck disable=SC1091
source "$repository_root/tools/reference/samtools/image.lock"

for executable in docker md5sum sha256sum stat df; do
  command -v "$executable" >/dev/null || {
    echo "required executable not found: $executable" >&2
    exit 69
  }
done

[[ -f "$source_bam" ]] || {
  echo "complete HG002 source BAM does not exist: $source_bam" >&2
  exit 66
}
[[ -f "$source_bai" ]] || {
  echo "complete HG002 source BAI does not exist: $source_bai" >&2
  exit 66
}
[[ ! -e "$output_directory" ]] || {
  echo "output destination already exists: $output_directory" >&2
  exit 73
}

source_bam=$(realpath "$source_bam")
source_bai=$(realpath "$source_bai")
output_parent=$(dirname -- "$output_directory")
output_name=$(basename -- "$output_directory")
mkdir -p -- "$output_parent"
output_parent=$(realpath "$output_parent")
output_directory="$output_parent/$output_name"

printf '%s  %s\n' "$HG002_SOURCE_BAM_MD5" "$source_bam" | md5sum --check -
printf '%s  %s\n' "$HG002_SOURCE_BAI_MD5" "$source_bai" | md5sum --check -

# A full 30x BAM is tens of gigabytes. Refuse an obviously undersized output
# filesystem instead of failing after hours of work. Maintainers may raise this
# floor, but may not lower it for v0.5 evidence.
minimum_free_bytes=$((64 * 1024 * 1024 * 1024))
if [[ -n "${ALIGNGAUGE_V05_MIN_FREE_BYTES:-}" ]]; then
  [[ "$ALIGNGAUGE_V05_MIN_FREE_BYTES" =~ ^[0-9]+$ ]] || {
    echo "ALIGNGAUGE_V05_MIN_FREE_BYTES must be an unsigned integer" >&2
    exit 64
  }
  if (( ALIGNGAUGE_V05_MIN_FREE_BYTES < minimum_free_bytes )); then
    echo "refusing v0.5 free-space floor below 64 GiB" >&2
    exit 64
  fi
  minimum_free_bytes=$ALIGNGAUGE_V05_MIN_FREE_BYTES
fi
free_kib=$(df -Pk "$output_parent" | awk 'NR==2 {print $4}')
[[ "$free_kib" =~ ^[0-9]+$ ]] || {
  echo "could not determine free space for $output_parent" >&2
  exit 74
}
free_bytes=$((free_kib * 1024))
if (( free_bytes < minimum_free_bytes )); then
  echo "insufficient free space for full HG002 preparation: $free_bytes bytes available, $minimum_free_bytes required" >&2
  exit 75
fi

staging=$(mktemp -d "$output_parent/.${output_name}.staging.XXXXXX")
cleanup() {
  rm -rf -- "$staging"
}
trap cleanup EXIT

container_user="$(id -u):$(id -g)"
docker pull "$SAMTOOLS_IMAGE" >/dev/null

docker run --rm --network none --read-only --cap-drop ALL \
  --user "$container_user" \
  --security-opt no-new-privileges --pids-limit 512 --memory 8g --cpus 8 \
  --tmpfs /tmp:rw,noexec,nosuid,size=512m \
  --volume "$source_bam:/input/source.bam:ro" \
  --volume "$staging:/out:rw" \
  "$SAMTOOLS_IMAGE" \
  samtools view \
    -b \
    --subsample-seed "$HG002_SUBSAMPLE_SEED" \
    --subsample "$HG002_SUBSAMPLE_FRACTION" \
    --threads 7 \
    --output /out/hg002-30x.bam \
    /input/source.bam

docker run --rm --network none --read-only --cap-drop ALL \
  --user "$container_user" \
  --security-opt no-new-privileges --pids-limit 512 --memory 4g --cpus 8 \
  --tmpfs /tmp:rw,noexec,nosuid,size=256m \
  --volume "$staging:/out:rw" \
  "$SAMTOOLS_IMAGE" \
  samtools index -@ 7 /out/hg002-30x.bam /out/hg002-30x.bam.bai

docker run --rm --network none --read-only --cap-drop ALL \
  --user "$container_user" \
  --security-opt no-new-privileges --pids-limit 128 --memory 1g --cpus 1 \
  --tmpfs /tmp:rw,noexec,nosuid,size=32m \
  --volume "$staging:/out:ro" \
  "$SAMTOOLS_IMAGE" \
  samtools quickcheck -v /out/hg002-30x.bam

prepared_bam="$staging/hg002-30x.bam"
prepared_bai="$staging/hg002-30x.bam.bai"
[[ -s "$prepared_bam" && -s "$prepared_bai" ]] || {
  echo "prepared BAM or index is missing/empty" >&2
  exit 74
}

source_bam_size_bytes=$(stat -c '%s' "$source_bam")
source_bai_size_bytes=$(stat -c '%s' "$source_bai")
prepared_bam_size_bytes=$(stat -c '%s' "$prepared_bam")
prepared_bai_size_bytes=$(stat -c '%s' "$prepared_bai")
prepared_bam_sha256=$(sha256sum "$prepared_bam" | cut -d' ' -f1)
prepared_bai_sha256=$(sha256sum "$prepared_bai" | cut -d' ' -f1)

cat >"$staging/prepared.manifest" <<EOF
schema=aligngauge-hg002-full-wgs-preparation-v1
profile=$HG002_V05_PROFILE
source_url=$HG002_SOURCE_URL
source_bai_url=$HG002_SOURCE_BAI_URL
source_bam_md5=$HG002_SOURCE_BAM_MD5
source_bai_md5=$HG002_SOURCE_BAI_MD5
source_bam_size_bytes=$source_bam_size_bytes
source_bai_size_bytes=$source_bai_size_bytes
reference_build=$HG002_REFERENCE_BUILD
source_nominal_depth=$HG002_SOURCE_NOMINAL_DEPTH
target_nominal_depth=$HG002_TARGET_NOMINAL_DEPTH
region=whole-alignment
subsample_seed=$HG002_SUBSAMPLE_SEED
subsample_fraction=$HG002_SUBSAMPLE_FRACTION
samtools_version=$SAMTOOLS_VERSION
samtools_image=$SAMTOOLS_IMAGE
prepared_bam=hg002-30x.bam
prepared_bai=hg002-30x.bam.bai
prepared_bam_size_bytes=$prepared_bam_size_bytes
prepared_bai_size_bytes=$prepared_bai_size_bytes
prepared_bam_sha256=$prepared_bam_sha256
prepared_bai_sha256=$prepared_bai_sha256
EOF

printf 'complete\n' >"$staging/_SUCCESS"
mv -- "$staging" "$output_directory"
trap - EXIT
printf 'prepared full HG002 v0.5 input at %s\n' "$output_directory"
