#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC_DIR="$ROOT_DIR/src-tauri/target/debug"
DST_DIR="$ROOT_DIR/src-tauri/binaries"

if [[ ! -d "$SRC_DIR" ]]; then
  echo "Missing $SRC_DIR"
  echo "Build tauri once first (npm run tauri:dev), then run this again."
  exit 1
fi

mkdir -p "$DST_DIR"
cp -f "$SRC_DIR"/lib*.dylib "$DST_DIR"/
echo "Repaired local LLM runtime libs in $DST_DIR"
ls -lh "$DST_DIR"/libllama-common.0.0.9025.dylib "$DST_DIR"/libllama.0.0.9025.dylib
