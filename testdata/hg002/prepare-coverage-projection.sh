#!/usr/bin/env bash
set -euo pipefail

[[ $# -eq 2 ]] || {
  echo "usage: $0 REPOSITORY_RELATIVE_SOURCE_BAM OUTPUT_DIRECTORY" >&2
  exit 2
}
source_relative=$1
output_directory=$2
script_directory=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repository_root=$(cd -- "$script_directory/../.." && pwd)
# shellcheck disable=SC1091
source "$repository_root/tools/reference/samtools/image.lock"
container_user="$(id -u):$(id -g)"

[[ "$source_relative" != /* && "$source_relative" != *".."* ]] || {
  echo "source must be a repository-relative path without '..'" >&2
  exit 3
}
[[ -f "$repository_root/$source_relative" ]] || {
  echo "source BAM does not exist: $repository_root/$source_relative" >&2
  exit 3
}
[[ ! -e "$output_directory" ]] || {
  echo "output destination already exists: $output_directory" >&2
  exit 3
}

output_parent=$(dirname -- "$output_directory")
output_name=$(basename -- "$output_directory")
mkdir -p -- "$output_parent"
staging=$(mktemp -d "$output_parent/.${output_name}.staging.XXXXXX")
cleanup() {
  rm -rf -- "$staging"
}
trap cleanup EXIT

docker pull "$SAMTOOLS_IMAGE" >/dev/null

docker run --rm --network none --read-only --cap-drop ALL \
  --user "$container_user" \
  --security-opt no-new-privileges --pids-limit 256 --memory 2g --cpus 2 \
  --tmpfs /tmp:rw,noexec,nosuid,size=64m \
  --volume "$repository_root:/work:ro" \
  "$SAMTOOLS_IMAGE" \
  samtools view --no-PG -h "/work/$source_relative" \
  | python "$script_directory/project-coverage-sam.py" \
  | docker run --rm --interactive --network none --read-only --cap-drop ALL \
      --user "$container_user" \
      --security-opt no-new-privileges --pids-limit 256 --memory 2g --cpus 2 \
      --tmpfs /tmp:rw,noexec,nosuid,size=64m \
      --volume "$staging:/out:rw" \
      "$SAMTOOLS_IMAGE" \
      samtools view --no-PG -b -o /out/subset.bam -

docker run --rm --network none --read-only --cap-drop ALL \
  --user "$container_user" \
  --security-opt no-new-privileges --pids-limit 128 --memory 1g --cpus 1 \
  --tmpfs /tmp:rw,noexec,nosuid,size=32m \
  --volume "$staging:/out:rw" \
  "$SAMTOOLS_IMAGE" \
  samtools index -@ 1 /out/subset.bam /out/subset.bam.bai

docker run --rm --network none --read-only --cap-drop ALL \
  --user "$container_user" \
  --security-opt no-new-privileges --pids-limit 128 --memory 1g --cpus 1 \
  --tmpfs /tmp:rw,noexec,nosuid,size=32m \
  --volume "$staging:/out:ro" \
  "$SAMTOOLS_IMAGE" samtools quickcheck -v /out/subset.bam

subset_sha256=$(sha256sum "$staging/subset.bam" | cut -d' ' -f1)
index_sha256=$(sha256sum "$staging/subset.bam.bai" | cut -d' ' -f1)
cat > "$staging/prepared.manifest" <<EOF
schema=aligngauge-hg002-coverage-projection-v1
source_relative=$source_relative
reference=chr20
source_region=chr20:10000000-11000000
position_offset=9950000
projected_reference_length=1100000
mate_coordinates=normalized-to-unmapped-for-coverage-only-projection
samtools_image=$SAMTOOLS_IMAGE
subset_sha256=$subset_sha256
index_sha256=$index_sha256
EOF
printf 'complete\n' > "$staging/_SUCCESS"
mv -- "$staging" "$output_directory"
trap - EXIT
printf 'prepared coverage projection %s\n' "$output_directory"
