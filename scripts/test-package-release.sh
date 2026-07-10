#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
TEST_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/nereid-package-release.XXXXXX")
trap 'rm -rf "$TEST_ROOT"' EXIT

TARGET=test-target
VERSION=1.2.3
ARCHIVE_NAME="nereid-v${VERSION}-${TARGET}.tar.gz"
OUT_DIR="$TEST_ROOT/dist"

mkdir -p "$TEST_ROOT/target/${TARGET}/release"
printf '#!/bin/sh\n' > "$TEST_ROOT/target/${TARGET}/release/nereid"
cp "$REPO_ROOT/LICENSE" "$TEST_ROOT/LICENSE"

(
  cd "$TEST_ROOT"
  TARGET=$TARGET VERSION=$VERSION OUT_DIR=$OUT_DIR "$REPO_ROOT/scripts/package-release.sh"
)

CHECKSUM_PATH="$OUT_DIR/${ARCHIVE_NAME}.sha256"
RECORDED_NAME=$(awk 'NR == 1 { print $2 }' "$CHECKSUM_PATH")
if [[ "$RECORDED_NAME" != "$ARCHIVE_NAME" ]]; then
  echo "checksum records '$RECORDED_NAME', expected '$ARCHIVE_NAME'" >&2
  exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  (cd "$OUT_DIR" && sha256sum -c "${ARCHIVE_NAME}.sha256")
else
  (cd "$OUT_DIR" && shasum -a 256 -c "${ARCHIVE_NAME}.sha256")
fi
