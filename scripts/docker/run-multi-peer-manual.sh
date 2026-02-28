#!/bin/bash
set -euo pipefail

APP_DIR="/app/kukuri-tauri/src-tauri"
OUTPUT_GROUP="${KUKURI_PEER_OUTPUT_GROUP:-multi-peer-manual}"
LOG_DIR="/app/tmp/logs/${OUTPUT_GROUP}"
RESULT_DIR="/app/test-results/${OUTPUT_GROUP}"
PEER_NAME="${KUKURI_PEER_NAME:-peer-client}"
LOG_PATH="${LOG_DIR}/${PEER_NAME}.log"
SUMMARY_PATH_DEFAULT="${RESULT_DIR}/${PEER_NAME}.json"

mkdir -p "$LOG_DIR" "$RESULT_DIR"

if [[ -z "${KUKURI_PEER_SUMMARY_PATH:-}" ]]; then
  export KUKURI_PEER_SUMMARY_PATH="$SUMMARY_PATH_DEFAULT"
fi
if [[ -z "${KUKURI_PEER_BOOTSTRAP_PEERS:-}" && -n "${KUKURI_BOOTSTRAP_PEERS:-}" ]]; then
  export KUKURI_PEER_BOOTSTRAP_PEERS="$KUKURI_BOOTSTRAP_PEERS"
fi
if [[ -z "${KUKURI_PEER_TOPIC:-}" ]]; then
  export KUKURI_PEER_TOPIC="kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0"
fi

cd "$APP_DIR"

echo "[multi-peer-manual] peer=${PEER_NAME} mode=${KUKURI_PEER_MODE:-listener} topic=${KUKURI_PEER_TOPIC}" | tee -a "$LOG_PATH"

set +e
cargo run --locked --bin p2p_peer_harness 2>&1 | tee -a "$LOG_PATH"
status=${PIPESTATUS[0]}
set -e

echo "[multi-peer-manual] peer=${PEER_NAME} exit_status=${status}" | tee -a "$LOG_PATH"
exit $status
