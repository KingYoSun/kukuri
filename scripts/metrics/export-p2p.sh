#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
DEFAULT_OUTPUT="${REPO_ROOT}/docs/01_project/activeContext/artefacts/metrics/$(date -u +"%Y%m%dT%H%M%SZ")-p2p-metrics.json"

usage() {
  cat <<'USAGE'
Usage: scripts/metrics/export-p2p.sh [--output <path>] [--pretty]

Runs the kukuri-tauri p2p_metrics_export binary and writes the snapshot to the given path.
Defaults to docs/01_project/activeContext/artefacts/metrics/<timestamp>-p2p-metrics.json.
USAGE
}

OUTPUT_PATH="$DEFAULT_OUTPUT"
PRETTY=""

while (($#)); do
  case "$1" in
    -o|--output)
      OUTPUT_PATH="$2"
      shift 2
      ;;
    --pretty)
      PRETTY="--pretty"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

pushd "$REPO_ROOT" >/dev/null

cargo run \
  --manifest-path kukuri-tauri/src-tauri/Cargo.toml \
  --bin p2p_metrics_export -- \
  --output "$OUTPUT_PATH" \
  $PRETTY

popd >/dev/null
