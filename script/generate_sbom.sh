#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_DIR="${SBOM_OUTPUT_DIR:-$ROOT_DIR/dist/sbom}"
SYFT_BIN="${SYFT_BIN:-$(command -v syft || true)}"

if [[ -z "$SYFT_BIN" || ! -x "$SYFT_BIN" ]]; then
  echo "SBOM generation requires Syft 1.44.0; set SYFT_BIN to its verified executable" >&2
  exit 2
fi

if [[ "$($SYFT_BIN version -o json | jq -r .version)" != "1.44.0" ]]; then
  echo "SBOM generation requires the reviewed Syft version 1.44.0" >&2
  exit 2
fi

mkdir -p "$OUTPUT_DIR"
revision="$(git -C "$ROOT_DIR" rev-parse HEAD)"

(cd "$ROOT_DIR" && "$SYFT_BIN" scan "dir:." \
  --exclude './.git/**' \
  --exclude './dist/**' \
  --exclude './src-tauri/target/**' \
  --exclude './docs/open-source/sbom/**' \
  --source-name assetrail \
  --source-version "$revision" \
  -o "cyclonedx-json=$OUTPUT_DIR/assetrail-source.cdx.json" \
  -o "spdx-json=$OUTPUT_DIR/assetrail-source.spdx.json")

(cd "$ROOT_DIR" && npm sbom --sbom-format cyclonedx) > "$OUTPUT_DIR/assetrail-npm.cdx.json"

for sbom in "$OUTPUT_DIR"/*.json; do
  normalized="${sbom}.normalized"
  jq --arg root "$ROOT_DIR" \
    'walk(if type == "string" then split($root) | join("") else . end)' \
    "$sbom" > "$normalized"
  mv "$normalized" "$sbom"
done

if rg -F -e '/Users/' -e '/home/' -e '\\Users\\' "$OUTPUT_DIR"/*.json; then
  echo "generated SBOM contains a forbidden workstation home path" >&2
  exit 1
fi

shasum -a 256 "$OUTPUT_DIR"/*.json
