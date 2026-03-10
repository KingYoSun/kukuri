#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
DEFAULT_OUTPUT="${REPO_ROOT}/docs/01_project/activeContext/artefacts/metrics/$(date -u +"%Y%m%dT%H%M%SZ")-p2p-metrics.json"
TRENDING_OUTPUT_DIR="${REPO_ROOT}/test-results/trending-feed/metrics"

usage() {
  cat <<'USAGE'
Usage: scripts/metrics/export-p2p.sh [--job <p2p|trending>] [--output <path>] [--pretty] [--limit <n>] [--database-url <url>]

Runs the kukuri-tauri p2p_metrics_export binary and writes the snapshot to the given path.
Defaults to docs/01_project/activeContext/artefacts/metrics/<timestamp>-p2p-metrics.json for --job p2p.
When --job trending is selected, the default output is test-results/trending-feed/metrics/<timestamp>-trending-metrics.json.
USAGE
}

OUTPUT_PATH="$DEFAULT_OUTPUT"
PRETTY=""
JOB="p2p"
DATABASE_URL=""
LIMIT_VALUE=""

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
    --job)
      JOB="$2"
      shift 2
      ;;
    --database-url)
      DATABASE_URL="$2"
      shift 2
      ;;
    --limit)
      LIMIT_VALUE="$2"
      shift 2
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

if [[ "$JOB" == "trending" && "$OUTPUT_PATH" == "$DEFAULT_OUTPUT" ]]; then
  mkdir -p "$TRENDING_OUTPUT_DIR"
  OUTPUT_PATH="${TRENDING_OUTPUT_DIR}/$(date -u +"%Y%m%dT%H%M%SZ")-trending-metrics.json"
fi

pushd "$REPO_ROOT" >/dev/null

CARGO_CMD=(cargo run \
  --manifest-path kukuri-tauri/src-tauri/Cargo.toml \
  --bin p2p_metrics_export -- \
  --job "$JOB" \
  --output "$OUTPUT_PATH")

if [[ -n "$PRETTY" ]]; then
  CARGO_CMD+=("$PRETTY")
fi

if [[ -n "$DATABASE_URL" ]]; then
  CARGO_CMD+=(--database-url "$DATABASE_URL")
fi

if [[ -n "$LIMIT_VALUE" ]]; then
  CARGO_CMD+=(--limit "$LIMIT_VALUE")
fi

"${CARGO_CMD[@]}"

popd >/dev/null
