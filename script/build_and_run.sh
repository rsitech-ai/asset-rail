#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-run}"
APP_NAME="AssetRail"
PROCESS_NAME="assetrail"
BUNDLE_ID="ai.rsitech.assetrail"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_BUNDLE="$ROOT_DIR/src-tauri/target/debug/bundle/macos/$APP_NAME.app"
APP_BINARY="$APP_BUNDLE/Contents/MacOS/$PROCESS_NAME"

usage() {
  echo "usage: $0 [run|--debug|--logs|--telemetry|--verify]" >&2
}

# Reject unsupported modes before building, launching, attaching a debugger, or
# touching any existing process. The helper never terminates processes by name.
case "$MODE" in
  run|--debug|debug|--logs|logs|--telemetry|telemetry|--verify|verify)
    ;;
  *)
    usage
    exit 2
    ;;
esac

cd "$ROOT_DIR"
npm run tauri build -- --debug

# Tauri's debug binary receives a linker-level ad-hoc signature. Seal the
# assembled bundle as well so Info.plist and bundled resources are covered by
# macOS integrity verification. This remains a local ad-hoc signature; release
# signing and notarization require the owner's Apple Developer identity.
/usr/bin/codesign --force --deep --sign - "$APP_BUNDLE"
/usr/bin/codesign --verify --deep --strict "$APP_BUNDLE"

open_app() {
  /usr/bin/open "$APP_BUNDLE"
}

app_pids() {
  /bin/ps -axo pid=,command= | /usr/bin/awk -v executable="$APP_BINARY" '$2 == executable { print $1 }'
}

stop_app_instances() {
  local pid

  # Scope shutdown to processes whose executable is this exact built bundle.
  # Never terminate by a generic process name or bundle identifier.
  while IFS= read -r pid; do
    [[ -n "$pid" ]] || continue
    /bin/kill "$pid"
  done < <(app_pids)

  for _ in {1..20}; do
    [[ -z "$(app_pids)" ]] && return 0
    sleep 0.25
  done
  echo "$APP_NAME did not stop cleanly from $APP_BINARY" >&2
  return 1
}

verify_new_process() {
  local initial_pids="$1"
  local pid

  for _ in {1..20}; do
    while IFS= read -r pid; do
      [[ -n "$pid" ]] || continue
      if ! /usr/bin/grep -qxF "$pid" <<<"$initial_pids"; then
        return 0
      fi
    done < <(app_pids)
    sleep 0.25
  done
  echo "$APP_NAME did not start a new process from $APP_BINARY" >&2
  return 1
}

stop_app_instances

case "$MODE" in
  run)
    open_app
    ;;
  --debug|debug)
    lldb -- "$APP_BINARY"
    ;;
  --logs|logs)
    open_app
    /usr/bin/log stream --info --style compact --predicate "process == \"$PROCESS_NAME\""
    ;;
  --telemetry|telemetry)
    open_app
    /usr/bin/log stream --info --style compact --predicate "process == \"$PROCESS_NAME\" OR subsystem == \"$BUNDLE_ID\""
    ;;
  --verify|verify)
    initial_pids="$(app_pids)"
    open_app
    verify_new_process "$initial_pids"
    echo "$APP_NAME launched from $APP_BUNDLE"
    ;;
  *)
    # The mode was validated before the build; this branch is unreachable.
    exit 70
    ;;
esac
