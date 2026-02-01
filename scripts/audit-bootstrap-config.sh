#!/usr/bin/env bash
set -euo pipefail

FILE=${1:-core/src/ffi/client.rs}

if [ ! -f "$FILE" ]; then
  echo "File not found: $FILE" >&2
  exit 1
fi

echo "==> Bootstrap peers in $FILE"
rg -n "dns4|bootstrap_peers" "$FILE" || true
