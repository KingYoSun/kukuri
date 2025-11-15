#!/bin/bash
set -euo pipefail

APP_DIR="/app/kukuri-tauri"
OUTPUT_DIR="$APP_DIR/tests/e2e/output"
RESULT_DIR="/app/test-results/desktop-e2e"
LOG_DIR="/app/tmp/logs/desktop-e2e"

mkdir -p "$RESULT_DIR" "$LOG_DIR" "$OUTPUT_DIR"
rm -f "$OUTPUT_DIR"/*.png "$OUTPUT_DIR"/*.json 2>/dev/null || true

timestamp="$(date -u +"%Y%m%d-%H%M%S")"
log_file="$LOG_DIR/$timestamp.log"
snapshot_dir="$RESULT_DIR/$timestamp"
mkdir -p "$snapshot_dir"

cd "$APP_DIR"

echo "=== desktop-e2e: building debug bundle ==="
pnpm e2e:build

echo "=== desktop-e2e: running pnpm e2e:ci ==="
set +e
pnpm e2e:ci 2>&1 | tee "$log_file"
status=${PIPESTATUS[0]}
set -e

if compgen -G "$OUTPUT_DIR/*" > /dev/null; then
  cp -a "$OUTPUT_DIR/." "$snapshot_dir/"
fi

echo "Desktop E2E artefacts:"
echo "  - Logs: $log_file"
echo "  - Reports: $snapshot_dir"

exit $status
