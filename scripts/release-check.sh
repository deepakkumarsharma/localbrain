#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUNDLE_DIR="$ROOT_DIR/src-tauri/target/release/bundle"

cd "$ROOT_DIR"

echo "[release-check] npm run build"
npm run build

echo "[release-check] npm run tauri -- build --bundles app"
npm run tauri -- build --bundles app

if [[ ! -d "$BUNDLE_DIR" ]]; then
  echo "[release-check] ERROR: bundle output missing at $BUNDLE_DIR"
  exit 1
fi

echo "[release-check] bundle contents"
find "$BUNDLE_DIR" -maxdepth 3 -type f | sed -n '1,200p'

if find "$BUNDLE_DIR" -type f \( -name "*.env" -o -name ".env*" \) | grep -q .; then
  echo "[release-check] ERROR: dotenv files found in bundle"
  exit 1
fi

if find "$BUNDLE_DIR" -type d -name ".localbrain" | grep -q .; then
  echo "[release-check] ERROR: .localbrain folder found in bundle"
  exit 1
fi

if find "$BUNDLE_DIR" -type f \( -name "*.sqlite" -o -name "*.db" \) | grep -q .; then
  echo "[release-check] ERROR: local database files found in bundle"
  exit 1
fi

echo "[release-check] PASS"
