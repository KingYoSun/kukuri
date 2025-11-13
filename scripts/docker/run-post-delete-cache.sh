#!/bin/bash
set -euo pipefail

timestamp="$(date -u +"%Y%m%d-%H%M%S")"
log_dir="/app/tmp/logs"
results_dir="/app/test-results/post-delete-cache"
mkdir -p "${log_dir}" "${results_dir}"

log_path="${log_dir}/post-delete-cache_docker_${timestamp}.log"
report_rel="test-results/post-delete-cache/${timestamp}.json"
export POST_DELETE_CACHE_REPORT="/app/${report_rel}"

cd /app/kukuri-tauri

echo "=== Running post-delete-cache scenario ===" | tee "${log_path}"
set +e
pnpm vitest run --config tests/scenarios/post-delete-cache.vitest.ts 2>&1 | tee -a "${log_path}"
status=${PIPESTATUS[0]}
set -e

if [ ${status} -ne 0 ]; then
  echo "Scenario 'post-delete-cache' failed. See ${log_path} for details." >&2
  exit ${status}
fi

if [ -f "/app/${report_rel}" ]; then
  echo "Scenario report saved to /app/${report_rel}" | tee -a "${log_path}"
else
  echo "Scenario report was not generated at /app/${report_rel}" | tee -a "${log_path}"
fi

echo "Scenario log saved to ${log_path}"
