#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 <output-directory>" >&2
  exit 64
}

[[ $# -eq 1 ]] || usage
output_directory=$1
script_directory=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repository_root=$(cd -- "$script_directory/../.." && pwd)
cd "$repository_root"

for executable in cargo find git gzip install python3 rm sha256sum tar touch; do
  command -v "$executable" >/dev/null || {
    echo "required executable not found: $executable" >&2
    exit 69
  }
done
[[ ! -e "$output_directory" ]] || {
  echo "release-artifact output already exists: $output_directory" >&2
  exit 73
}
[[ -z "$(git status --porcelain --untracked-files=all)" ]] || {
  echo "release artifacts require a clean repository" >&2
  git status --short >&2
  exit 65
}

release_commit=$(git rev-parse HEAD)
source_date_epoch=$(git show -s --format=%ct HEAD)
[[ "$source_date_epoch" =~ ^[0-9]+$ ]]
artifact_name=aligngauge-v0.5.0-linux-x86_64
output_parent=$(dirname -- "$output_directory")
output_name=$(basename -- "$output_directory")
mkdir -p -- "$output_parent"
output_parent=$(realpath "$output_parent")
output_directory="$output_parent/$output_name"
staging=$(mktemp -d "$output_parent/.${output_name}.staging.XXXXXX")

# Native build products can retain their build-directory identity even when source
# paths are remapped. Use one deterministic absolute build root for every sequential
# reproducibility build instead of placing Cargo output under the random staging path.
build_root="$repository_root/target/v0.5-release-build"
case "$build_root" in
  "$repository_root/target/v0.5-release-build") ;;
  *)
    echo "refusing unexpected deterministic build root: $build_root" >&2
    exit 64
    ;;
esac
rm -rf -- "$build_root"

cleanup() {
  rm -rf -- "$staging" "$build_root"
}
trap cleanup EXIT
mkdir -p "$staging/package/$artifact_name" "$staging/generated" "$staging/final"

export SOURCE_DATE_EPOCH="$source_date_epoch"
export RUSTFLAGS="--remap-path-prefix=$repository_root=/src"
export CFLAGS="-ffile-prefix-map=$repository_root=/src -fdebug-prefix-map=$repository_root=/src"
export CXXFLAGS="$CFLAGS"
CARGO_TARGET_DIR="$build_root" \
  cargo build --release -p aligngauge-cli --bin aligngauge --locked

python3 tools/v0.5/generate-sbom.py \
  --repository-root "$repository_root" \
  --sbom "$staging/generated/sbom.cdx.json" \
  --licenses "$staging/generated/licenses.json"

install -m 0755 \
  "$build_root/release/aligngauge" \
  "$staging/package/$artifact_name/aligngauge"
install -m 0644 README.md "$staging/package/$artifact_name/README.md"
install -m 0644 LICENSE "$staging/package/$artifact_name/LICENSE"
install -m 0644 docs/SCHEMA_COMPATIBILITY.md \
  "$staging/package/$artifact_name/SCHEMA_COMPATIBILITY.md"
install -m 0644 "$staging/generated/sbom.cdx.json" \
  "$staging/package/$artifact_name/sbom.cdx.json"
install -m 0644 "$staging/generated/licenses.json" \
  "$staging/package/$artifact_name/licenses.json"

python3 - "$staging/package/$artifact_name/release-manifest.json" \
  "$release_commit" "$source_date_epoch" <<'PY'
import json
import platform
import sys
from pathlib import Path

path = Path(sys.argv[1])
report = {
    "schema": "aligngauge-release-artifact-v1",
    "product_release": "v0.5.0",
    "git_commit": sys.argv[2],
    "source_date_epoch": int(sys.argv[3]),
    "artifact_platform": "linux-x86_64",
    "build_host_platform": platform.platform(),
}
path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY

find "$staging/package/$artifact_name" -exec touch -h -d "@$source_date_epoch" {} +
tar --sort=name \
  --format=ustar \
  --mtime="@$source_date_epoch" \
  --owner=0 --group=0 --numeric-owner \
  -C "$staging/package" \
  -cf "$staging/$artifact_name.tar" "$artifact_name"
gzip -n -9 "$staging/$artifact_name.tar"
sha256sum "$staging/$artifact_name.tar.gz" \
  | sed "s#  $staging/#  #" >"$staging/SHA256SUMS"
sha256sum "$staging/package/$artifact_name/aligngauge" \
  | sed "s#  $staging/package/$artifact_name/#  #" \
  >"$staging/BINARY.SHA256"
sha256sum \
  "$staging/package/$artifact_name/aligngauge" \
  "$staging/package/$artifact_name/sbom.cdx.json" \
  "$staging/package/$artifact_name/licenses.json" \
  | sed "s#  $staging/package/$artifact_name/#  #" \
  >"$staging/CONTENTS.SHA256"

install -m 0644 "$staging/$artifact_name.tar.gz" "$staging/final/$artifact_name.tar.gz"
install -m 0644 "$staging/SHA256SUMS" "$staging/final/SHA256SUMS"
install -m 0644 "$staging/BINARY.SHA256" "$staging/final/BINARY.SHA256"
install -m 0644 "$staging/CONTENTS.SHA256" "$staging/final/CONTENTS.SHA256"
install -m 0644 "$staging/generated/sbom.cdx.json" "$staging/final/sbom.cdx.json"
install -m 0644 "$staging/generated/licenses.json" "$staging/final/licenses.json"
printf '%s\n' "$release_commit" >"$staging/final/RELEASE_COMMIT"
printf 'complete\n' >"$staging/final/_SUCCESS"
mv -- "$staging/final" "$output_directory"
trap - EXIT
rm -rf -- "$staging" "$build_root"
