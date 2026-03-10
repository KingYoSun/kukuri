#!/bin/bash
# Helper to run a single Vitest target inside the ts-test container with pnpm/corepack fallback.

set -euo pipefail

TARGET="${1:-}"
REPORT_PATH="${2:-}"

if [[ -z "${TARGET}" || -z "${REPORT_PATH}" ]]; then
  echo "[ERROR] Usage: run-vitest-target.sh <target> <report-path>" >&2
  exit 1
fi

cd /app/kukuri-tauri

declare -a PNPM_BIN
if command -v pnpm >/dev/null 2>&1; then
  PNPM_BIN=(pnpm)
elif command -v corepack >/dev/null 2>&1; then
  PNPM_BIN=(corepack pnpm)
else
  echo "[ERROR] Neither pnpm nor corepack is available inside container." >&2
  exit 1
fi
echo "[INFO] Using ${PNPM_BIN[*]} for pnpm commands"

if [[ ! -f node_modules/.bin/vitest ]]; then
  echo "[INFO] Installing frontend dependencies inside container (${PNPM_BIN[*]} install --frozen-lockfile)..."
  "${PNPM_BIN[@]}" install --frozen-lockfile --ignore-workspace
fi

mkdir -p "$(dirname "${REPORT_PATH}")"
"${PNPM_BIN[@]}" vitest run "${TARGET}" --reporter=default --reporter=json --outputFile "${REPORT_PATH}"
