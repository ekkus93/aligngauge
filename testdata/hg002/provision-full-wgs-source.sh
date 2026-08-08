#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
usage: testdata/hg002/provision-full-wgs-source.sh <destination-directory>

Explicitly download and verify the pinned complete HG002 WGS source BAM and BAI.
This is a maintainer preparation command. Ordinary tests and AlignGauge runtime
never invoke it implicitly.
EOF
  exit 64
}

[[ $# -eq 1 ]] || usage
requested_destination=$1

script_directory=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repository_root=$(cd -- "$script_directory/../.." && pwd)
# shellcheck disable=SC1091
source "$script_directory/full-wgs-v0.5.env"

for executable in awk curl df md5sum mkdir mv realpath stat; do
  command -v "$executable" >/dev/null || {
    echo "required executable not found: $executable" >&2
    exit 69
  }
done

parent=$(dirname -- "$requested_destination")
name=$(basename -- "$requested_destination")
mkdir -p -- "$parent"
parent=$(realpath "$parent")
destination="$parent/$name"
mkdir -p -- "$destination"
destination=$(realpath "$destination")
case "$destination/" in
  "$repository_root/"*)
    echo "full HG002 source destination must be outside the repository: $destination" >&2
    exit 64
    ;;
esac

bam_name=$(basename -- "$HG002_SOURCE_URL")
bai_name=$(basename -- "$HG002_SOURCE_BAI_URL")
bam="$destination/$bam_name"
bai="$destination/$bai_name"
bam_partial="$destination/.${bam_name}.partial"
bai_partial="$destination/.${bai_name}.partial"
manifest="$destination/source.manifest"
success_marker="$destination/_SUCCESS"
manifest_tmp="$destination/.source.manifest.tmp"
success_tmp="$destination/._SUCCESS.tmp"

# Resumability is allowed only in a dedicated directory whose contents are
# completely understood by this provisioner. Never guess around unrelated files.
shopt -s nullglob dotglob
for entry in "$destination"/*; do
  case "$entry" in
    "$bam"|"$bai"|"$bam_partial"|"$bai_partial"|"$manifest"|"$success_marker"|"$manifest_tmp"|"$success_tmp") ;;
    *)
      echo "source destination contains an unexpected entry: $entry" >&2
      exit 65
      ;;
  esac
done
shopt -u nullglob dotglob

content_length() {
  local url=$1
  local length
  length=$(curl --fail --silent --show-error --location --head "$url" \
    | awk 'BEGIN { IGNORECASE = 1 }
      /^content-length:/ {
        gsub("\\r", "", $2)
        if ($2 ~ /^[0-9]+$/) value = $2
      }
      END {
        if (value == "") exit 1
        print value
      }') || {
    echo "could not determine exact Content-Length for $url" >&2
    exit 74
  }
  [[ "$length" =~ ^[0-9]+$ && "$length" -gt 0 ]] || {
    echo "invalid Content-Length for $url: ${length:-<missing>}" >&2
    exit 74
  }
  printf '%s\n' "$length"
}

available_bytes() {
  local free_kib
  free_kib=$(df -Pk "$destination" | awk 'NR==2 {print $4}')
  [[ "$free_kib" =~ ^[0-9]+$ ]] || {
    echo "could not determine free space for $destination" >&2
    exit 74
  }
  printf '%s\n' "$((free_kib * 1024))"
}

verify_file() {
  local path=$1
  local expected_size=$2
  local expected_md5=$3
  [[ -f "$path" ]] || {
    echo "expected source file does not exist: $path" >&2
    exit 66
  }
  local actual_size
  actual_size=$(stat -c '%s' "$path")
  [[ "$actual_size" == "$expected_size" ]] || {
    echo "source size mismatch for $path: expected $expected_size, got $actual_size" >&2
    exit 65
  }
  printf '%s  %s\n' "$expected_md5" "$path" | md5sum --check -
}

download_file() {
  local label=$1
  local url=$2
  local final_path=$3
  local partial_path=$4
  local expected_size=$5
  local expected_md5=$6

  if [[ -f "$final_path" ]]; then
    [[ ! -e "$partial_path" ]] || {
      echo "$label has both a final and partial file; refusing ambiguous state" >&2
      exit 65
    }
    verify_file "$final_path" "$expected_size" "$expected_md5"
    return
  fi

  local partial_size=0
  if [[ -e "$partial_path" ]]; then
    [[ -f "$partial_path" ]] || {
      echo "$label partial path is not a regular file: $partial_path" >&2
      exit 65
    }
    partial_size=$(stat -c '%s' "$partial_path")
    (( partial_size <= expected_size )) || {
      echo "$label partial file is larger than the pinned upstream object" >&2
      exit 65
    }
  fi

  local remaining required free
  remaining=$((expected_size - partial_size))
  # Keep 2 GiB of headroom so a full filesystem does not corrupt the explicit
  # preparation state while the final range is being written.
  required=$((remaining + 2 * 1024 * 1024 * 1024))
  free=$(available_bytes)
  (( free >= required )) || {
    echo "insufficient free space to provision $label: $free bytes available, $required required for remaining bytes plus headroom" >&2
    exit 75
  }

  if (( remaining > 0 )); then
    echo "downloading $label ($remaining bytes remaining)" >&2
    curl --fail --location --retry 5 --retry-all-errors --continue-at - \
      --output "$partial_path" "$url"
  fi

  verify_file "$partial_path" "$expected_size" "$expected_md5"
  mv -- "$partial_path" "$final_path"
}

bam_size=$(content_length "$HG002_SOURCE_URL")
bai_size=$(content_length "$HG002_SOURCE_BAI_URL")

if [[ -f "$success_marker" ]]; then
  [[ -f "$manifest" ]] || {
    echo "completed source directory is missing source.manifest" >&2
    exit 65
  }
  [[ ! -e "$bam_partial" && ! -e "$bai_partial" && ! -e "$manifest_tmp" && ! -e "$success_tmp" ]] || {
    echo "completed source directory contains incomplete temporary state" >&2
    exit 65
  }
  verify_file "$bam" "$bam_size" "$HG002_SOURCE_BAM_MD5"
  verify_file "$bai" "$bai_size" "$HG002_SOURCE_BAI_MD5"
  grep -Fx 'schema=aligngauge-hg002-full-wgs-source-v1' "$manifest" >/dev/null
  grep -Fx "profile=$HG002_V05_PROFILE" "$manifest" >/dev/null
  grep -Fx "source_bam_url=$HG002_SOURCE_URL" "$manifest" >/dev/null
  grep -Fx "source_bai_url=$HG002_SOURCE_BAI_URL" "$manifest" >/dev/null
  grep -Fx "source_bam=$bam_name" "$manifest" >/dev/null
  grep -Fx "source_bai=$bai_name" "$manifest" >/dev/null
  grep -Fx "source_bam_md5=$HG002_SOURCE_BAM_MD5" "$manifest" >/dev/null
  grep -Fx "source_bai_md5=$HG002_SOURCE_BAI_MD5" "$manifest" >/dev/null
  grep -Fx "source_bam_size_bytes=$bam_size" "$manifest" >/dev/null
  grep -Fx "source_bai_size_bytes=$bai_size" "$manifest" >/dev/null
  grep -Fx "reference_build=$HG002_REFERENCE_BUILD" "$manifest" >/dev/null
  echo "pinned full HG002 source is already provisioned and verified: $destination"
  exit 0
fi

[[ ! -e "$manifest" ]] || {
  echo "incomplete source directory already contains source.manifest; remove it explicitly before resuming" >&2
  exit 65
}
[[ ! -e "$manifest_tmp" && ! -e "$success_tmp" ]] || {
  echo "temporary completion metadata already exists; refusing ambiguous state" >&2
  exit 65
}

download_file \
  "HG002 source BAI" \
  "$HG002_SOURCE_BAI_URL" \
  "$bai" \
  "$bai_partial" \
  "$bai_size" \
  "$HG002_SOURCE_BAI_MD5"
download_file \
  "HG002 source BAM" \
  "$HG002_SOURCE_URL" \
  "$bam" \
  "$bam_partial" \
  "$bam_size" \
  "$HG002_SOURCE_BAM_MD5"

cat >"$manifest_tmp" <<EOF
schema=aligngauge-hg002-full-wgs-source-v1
profile=$HG002_V05_PROFILE
source_bam_url=$HG002_SOURCE_URL
source_bai_url=$HG002_SOURCE_BAI_URL
source_bam=$bam_name
source_bai=$bai_name
source_bam_md5=$HG002_SOURCE_BAM_MD5
source_bai_md5=$HG002_SOURCE_BAI_MD5
source_bam_size_bytes=$bam_size
source_bai_size_bytes=$bai_size
reference_build=$HG002_REFERENCE_BUILD
EOF
mv -- "$manifest_tmp" "$manifest"
printf 'complete\n' >"$success_tmp"
mv -- "$success_tmp" "$success_marker"

printf 'provisioned pinned full HG002 source at %s\n' "$destination"
