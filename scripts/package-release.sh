#!/usr/bin/env bash
set -euo pipefail

: "${TARGET:?TARGET is required}"
: "${VERSION:?VERSION is required}"
BIN_NAME=${BIN_NAME:-nereid}
OUT_DIR=${OUT_DIR:-dist}

mkdir -p "$OUT_DIR"

BIN_PATH="target/${TARGET}/release/${BIN_NAME}"
if [[ -f "${BIN_PATH}.exe" ]]; then
  echo "Windows binary detected; use scripts/package-release.ps1 instead." >&2
  exit 1
fi

if [[ ! -f "$BIN_PATH" ]]; then
  echo "Binary not found: $BIN_PATH" >&2
  exit 1
fi

ARCHIVE_NAME="${BIN_NAME}-v${VERSION}-${TARGET}.tar.gz"

tar -C "target/${TARGET}/release" -czf "${OUT_DIR}/${ARCHIVE_NAME}" "$BIN_NAME"

if command -v sha256sum >/dev/null 2>&1; then
  sha256sum "${OUT_DIR}/${ARCHIVE_NAME}" > "${OUT_DIR}/${ARCHIVE_NAME}.sha256"
elif command -v shasum >/dev/null 2>&1; then
  shasum -a 256 "${OUT_DIR}/${ARCHIVE_NAME}" > "${OUT_DIR}/${ARCHIVE_NAME}.sha256"
fi
