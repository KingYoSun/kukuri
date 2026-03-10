#!/bin/bash
# Entry point for TypeScript test containers

set -euo pipefail

REPO_ROOT="/app"
APP_DIR="${REPO_ROOT}/kukuri-tauri"

cd "$APP_DIR"

if [[ ! -d node_modules ]]; then
  echo "[ts-test] node_modules not found. Installing dependencies with pnpm..."
  pnpm install --frozen-lockfile --ignore-workspace
fi

if [[ $# -gt 0 ]]; then
  echo "[ts-test] Running command: $*"
  exec "$@"
else
  echo "[ts-test] No command supplied. Running default 'pnpm test'."
  exec pnpm test
fi
