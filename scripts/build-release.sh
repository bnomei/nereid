#!/usr/bin/env bash
set -euo pipefail

: "${TARGET:?TARGET is required}"

if [[ "$TARGET" == *"musl"* ]]; then
  if command -v cross >/dev/null 2>&1; then
    cross build --release --target "$TARGET"
  else
    cargo build --release --target "$TARGET"
  fi
else
  cargo build --release --target "$TARGET"
fi
