#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
source_root="$repository_root/fixtures/source"
staging_root="$(mktemp -d "${TMPDIR:-/tmp}/pevi-fixtures.XXXXXX")"
trap 'rm -rf -- "$staging_root"' EXIT

build_fixture() {
  local prefix="$1"
  local output="$2"
  local resource="$staging_root/${prefix}.res"

  "${prefix}-windres" \
    --codepage=65001 \
    --input-format=rc \
    --output-format=coff \
    "$source_root/fixture.rc" \
    "$resource"

  "${prefix}-gcc" \
    -Os \
    -s \
    -Wl,--build-id=none \
    -Wl,--no-insert-timestamp \
    "$source_root/fixture.c" \
    "$resource" \
    -o "$staging_root/$output"

  install -m 0644 "$staging_root/$output" "$repository_root/fixtures/$output"
}

build_fixture i686-w64-mingw32 pe32_unsigned.exe
build_fixture x86_64-w64-mingw32 pe64_unsigned.exe

shasum -a 256 \
  "$repository_root/fixtures/pe32_unsigned.exe" \
  "$repository_root/fixtures/pe64_unsigned.exe"
