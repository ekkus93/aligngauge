#!/usr/bin/env bash
set -euo pipefail

script_directory=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repository_root=$(cd -- "$script_directory/../.." && pwd)
# shellcheck disable=SC1091
source "$repository_root/tools/reference/samtools/image.lock"

source_base='https://ftp-trace.ncbi.nlm.nih.gov/ReferenceSamples/giab/data/AshkenazimTrio/HG002_NA24385_son/Element_AVITI_20231018'
source_name='HG002_GRCh38-GIABv3_Element-StdInsert_2X150_81x_20231018.bam'
source_bam="$source_base/$source_name"
source_bai="$source_bam.bai"
source_bam_md5='f5360b7adbc798c90a78f290de928eca'
source_bai_md5='1d7fd88891eee203c02fb852cac95301'
region='chr20:10000000-11000000'
seed='42'
fraction='0.37037037037037'
output_directory=${1:-"$repository_root/testdata/local/hg002-grch38-giabv3-chr20-10-11mb-30x"}

for executable in curl docker md5sum sha256sum; do
  command -v "$executable" >/dev/null || {
    echo "required executable not found: $executable" >&2
    exit 2
  }
done

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

curl --fail --location --retry 3 --output "$staging/source.bam.bai" "$source_bai"
printf '%s  %s\n' "$source_bai_md5" "$staging/source.bam.bai" | md5sum --check -

docker pull "$SAMTOOLS_IMAGE" >/dev/null

docker run --rm --network bridge --read-only --cap-drop ALL \
  --security-opt no-new-privileges --pids-limit 512 --memory 4g --cpus 4 \
  --tmpfs /tmp:rw,noexec,nosuid,size=256m \
  --volume "$staging:/out:rw" \
  "$SAMTOOLS_IMAGE" \
  samtools view \
    -X \
    -b \
    --subsample-seed "$seed" \
    --subsample "$fraction" \
    --threads 3 \
    --output /out/subset.bam \
    "$source_bam" \
    /out/source.bam.bai \
    "$region"

docker run --rm --network none --read-only --cap-drop ALL \
  --security-opt no-new-privileges --pids-limit 256 --memory 2g --cpus 2 \
  --tmpfs /tmp:rw,noexec,nosuid,size=64m \
  --volume "$staging:/out:rw" \
  "$SAMTOOLS_IMAGE" samtools index -@ 1 /out/subset.bam /out/subset.bam.bai

docker run --rm --network none --read-only --cap-drop ALL \
  --security-opt no-new-privileges --pids-limit 128 --memory 1g --cpus 1 \
  --tmpfs /tmp:rw,noexec,nosuid,size=32m \
  --volume "$staging:/out:ro" \
  "$SAMTOOLS_IMAGE" samtools quickcheck -v /out/subset.bam

subset_sha256=$(sha256sum "$staging/subset.bam" | cut -d' ' -f1)
index_sha256=$(sha256sum "$staging/subset.bam.bai" | cut -d' ' -f1)

cat >"$staging/prepared.manifest" <<EOF
schema=aligngauge-hg002-preparation-v1
source_bam=$source_bam
source_bam_md5=$source_bam_md5
source_bai=$source_bai
source_bai_md5=$source_bai_md5
reference_build=GRCh38-GIABv3
region=$region
subsample_seed=$seed
subsample_fraction=$fraction
samtools_image=$SAMTOOLS_IMAGE
subset_sha256=$subset_sha256
index_sha256=$index_sha256
EOF

rm -- "$staging/source.bam.bai"
printf 'complete\n' >"$staging/_SUCCESS"
mv -- "$staging" "$output_directory"
trap - EXIT
printf 'prepared %s\n' "$output_directory"
