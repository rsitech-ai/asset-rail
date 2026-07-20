#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

required=(
  .github/dependabot.yml
  .gitleaks.toml
  package-lock.json
  rust-toolchain.toml
  src-tauri/Cargo.lock
  src-tauri/capabilities/default.json
  src-tauri/tauri.conf.json
)

for path in "${required[@]}"; do
  if [[ ! -s "$path" ]]; then
    echo "missing or empty security surface artifact: $path" >&2
    exit 1
  fi
done

for command in node jq rg git; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "required verification command is unavailable: $command" >&2
    exit 1
  fi
done

node --test script/security_config.node-test.mjs

jq -e '.windows == ["main"] and .permissions == []' \
  src-tauri/capabilities/default.json >/dev/null
jq -e '.app.security.capabilities == ["default"]' \
  src-tauri/tauri.conf.json >/dev/null

if rg -n 'core:default|tauri-plugin-(shell|fs|http|sql|stronghold|process|opener)' \
  package.json src-tauri/Cargo.toml src-tauri/capabilities src-tauri/tauri.conf.json; then
  echo "forbidden generic Tauri authority is present" >&2
  exit 1
fi

if rg -n 'dangerouslySetInnerHTML|\.innerHTML\s*=|document\.write\(|\beval\(|new Function\(|localStorage|sessionStorage' \
  src index.html; then
  echo "dangerous frontend sink or persistent browser storage is present" >&2
  exit 1
fi

if git ls-files | rg '(^|/)\.env($|\.)|\.(pem|key|p8|p12|mobileprovision)$'; then
  echo "tracked environment or private signing material is present" >&2
  exit 1
fi

echo "Security surface verification passed."
