#!/bin/bash
# Docker環境でのテスト実行スクリプト

set -euo pipefail

PROJECT_NAME="kukuri_tests"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
COMPOSE_FILE="${REPO_ROOT}/docker-compose.test.yml"
ENV_FILE="${REPO_ROOT}/kukuri-tauri/tests/.env.p2p"
RESULTS_DIR="${REPO_ROOT}/test-results"
COVERAGE_TMP_DIR="${RESULTS_DIR}/tarpaulin"
COVERAGE_ARTEFACT_DIR="${REPO_ROOT}/docs/01_project/activeContext/artefacts/metrics"
BOOTSTRAP_DEFAULT_PEER="03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8@127.0.0.1:11233"
BOOTSTRAP_CONTAINER="kukuri-p2p-bootstrap"
PROMETHEUS_SERVICE="prometheus-trending"
PROMETHEUS_METRICS_URL="${PROMETHEUS_METRICS_URL:-http://127.0.0.1:9898/metrics}"
COMMUNITY_NODE_BASE_URL_DEFAULT="http://127.0.0.1:18080"

P2P_MAINLINE_TEST="${P2P_MAINLINE_TEST_TARGET:-p2p_mainline_smoke}"
P2P_GOSSIP_TEST="${P2P_GOSSIP_TEST_TARGET:-p2p_gossip_smoke}"

usage() {
  cat <<'EOF'
Usage: ./test-docker.sh [command] [options]

Commands:
  all          Run all tests (default)
  rust         Run Rust tests only
  ts           Run TypeScript tests only (or specific scenarios with --scenario)
  lint         Run lint/format checks only
  coverage     Run cargo tarpaulin and export coverage artifacts
  e2e          Run desktop E2E tests (Tauri + WebDriverIO)
  e2e-community-node  Run desktop E2E tests with community node
  build        Build the Docker image only
  clean        Clean containers and images
  cache-clean  Clean including cache volumes
  performance  Run the Rust performance harness (ignored tests) and export reports
  p2p          Run P2P integration tests inside Docker

Options for ts:
  --scenario <name>      Execute a preset scenario (e.g. trending-feed, profile-avatar-sync, direct-message, user-search-pagination, topic-create, post-delete-cache, offline-sync)
  --fixture <path>       Override VITE_TRENDING_FIXTURE_PATH for the scenario
  --offline-category <name>  Offline-sync sub category (topic, post, follow, dm)
  --service-worker       Extend profile-avatar-sync scenario with Service Worker worker tests and Stage4 logs
  --no-build             Skip Docker image build (use existing image)

Options for p2p:
  --tests <name>          Cargo test filter (default: p2p_gossip_smoke)
  --bootstrap <peers>     KUKURI_BOOTSTRAP_PEERS (comma separated node@host:port)
  --no-build              Skip docker compose build
  --keep-env              Keep generated .env.p2p after execution
  --rust-log <value>      RUST_LOG for P2P (default: debug)
  --rust-backtrace <val>  RUST_BACKTRACE for P2P (default: full)
  -h, --help              Show this help
  ※ `--tests gossip` / `--tests mainline` でそれぞれ `p2p_gossip_smoke` / `p2p_mainline_smoke` を指定可能。任意のテスト名を直接渡すこともできます。
EOF
}

ensure_docker() {
  if ! command -v docker >/dev/null 2>&1; then
    echo 'docker command not found. Install Docker Desktop / Docker CLI first.' >&2
    exit 1
  fi
}

compose_run() {
  local env_file="$1"; shift
  local args=(compose '--project-name' "$PROJECT_NAME" '-f' "$COMPOSE_FILE")
  if [[ -n "$env_file" ]]; then
    args+=('--env-file' "$env_file")
  fi
  pushd "$REPO_ROOT" >/dev/null
  docker "${args[@]}" "$@"
  local code=$?
  popd >/dev/null
  return $code
}

resolve_compose_image_name() {
  local suffix="$1"
  local images
  images=$(docker compose --project-name "$PROJECT_NAME" -f "$COMPOSE_FILE" config --images 2>/dev/null || true)
  local resolved
  resolved=$(printf '%s\n' "$images" | grep -E "${suffix}$" | head -n1 || true)
  if [[ -n "$resolved" ]]; then
    printf '%s\n' "$resolved"
    return
  fi
  printf '%s-%s\n' "$PROJECT_NAME" "$suffix"
}

use_prebuilt_test_image() {
  local prebuilt_image="${KUKURI_TEST_RUNNER_IMAGE:-}"
  if [[ -z "$prebuilt_image" ]]; then
    return 1
  fi

  echo "Trying prebuilt Docker test image: ${prebuilt_image}"
  if ! docker image inspect "$prebuilt_image" >/dev/null 2>&1; then
    if ! docker pull "$prebuilt_image"; then
      echo "[WARN] Failed to pull prebuilt image (${prebuilt_image}). Falling back to local build." >&2
      return 1
    fi
  fi

  local runner_image
  local ts_image
  runner_image="$(resolve_compose_image_name test-runner)"
  ts_image="$(resolve_compose_image_name ts-test)"

  if ! docker tag "$prebuilt_image" "$runner_image"; then
    echo "[WARN] Failed to tag prebuilt image to ${runner_image}. Falling back to local build." >&2
    return 1
  fi

  if ! docker tag "$prebuilt_image" "$ts_image"; then
    echo "[WARN] Failed to tag prebuilt image to ${ts_image}. Falling back to local build." >&2
    return 1
  fi

  echo "[OK] Using prebuilt image via ${runner_image} and ${ts_image}"
  return 0
}

build_image() {
  if use_prebuilt_test_image; then
    return
  fi
  echo 'Building Docker test image (with cache optimization)...'
  DOCKER_BUILDKIT=1 compose_run '' build test-runner ts-test
  echo '[OK] Docker image built successfully'
}

prepare_coverage_dirs() {
  mkdir -p "$COVERAGE_TMP_DIR" "$COVERAGE_ARTEFACT_DIR"
  rm -f "${COVERAGE_TMP_DIR}/tarpaulin-report."* "${COVERAGE_TMP_DIR}/lcov.info"
}

save_coverage_artifacts() {
  local timestamp
  timestamp="$(date '+%Y-%m-%d-%H%M%S')"

  local json_src="${COVERAGE_TMP_DIR}/tarpaulin-report.json"
  local lcov_src="${COVERAGE_TMP_DIR}/tarpaulin-report.lcov"
  if [[ ! -f "$lcov_src" && -f "${COVERAGE_TMP_DIR}/lcov.info" ]]; then
    lcov_src="${COVERAGE_TMP_DIR}/lcov.info"
  fi
  local json_dest="${COVERAGE_ARTEFACT_DIR}/${timestamp}-tarpaulin.json"
  local lcov_dest="${COVERAGE_ARTEFACT_DIR}/${timestamp}-tarpaulin.lcov"

  if [[ -f "$json_src" ]]; then
    cp "$json_src" "$json_dest"
    echo "[OK] Coverage JSON saved to ${json_dest#$REPO_ROOT/}"
  else
    echo '[WARN] tarpaulin JSON report not found' >&2
  fi

  if [[ -f "$lcov_src" ]]; then
    cp "$lcov_src" "$lcov_dest"
    echo "[OK] Coverage LCOV saved to ${lcov_dest#$REPO_ROOT/}"
  else
    echo '[WARN] tarpaulin LCOV report not found' >&2
  fi

  if command -v jq >/dev/null 2>&1 && [[ -f "$json_src" ]]; then
    local coverage
    coverage="$(jq -r '.coverage // empty' "$json_src")"
    if [[ -n "$coverage" ]]; then
      echo "[INFO] Reported coverage: ${coverage}%"
    fi
  fi
}

run_all_tests() {
  [[ $NO_BUILD -eq 1 ]] || build_image
  echo 'Running all tests in Docker...'
  compose_run '' run --rm test-runner /app/run-tests.sh
  echo '[OK] All tests passed'
}

run_rust_tests() {
  [[ $NO_BUILD -eq 1 ]] || build_image
  echo 'Running Rust tests in Docker...'
  compose_run '' run --rm rust-test
  echo '[OK] Rust tests passed'
}

start_prometheus_trending() {
  echo "Starting ${PROMETHEUS_SERVICE} service (host network)..."
  if compose_run '' up -d "${PROMETHEUS_SERVICE}"; then
    return 0
  fi
  echo "[WARN] Failed to start ${PROMETHEUS_SERVICE}. Metrics scraping will be skipped." >&2
  return 1
}

stop_prometheus_trending() {
  echo "Stopping ${PROMETHEUS_SERVICE} service..."
  compose_run '' rm -sf "${PROMETHEUS_SERVICE}" >/dev/null 2>&1 || true
}

collect_trending_metrics_snapshot() {
  local timestamp="$1"
  local run_state="${2:-active}"
  local log_rel_path="tmp/logs/trending_metrics_job_stage4_${timestamp}.log"
  local log_host_path="${REPO_ROOT}/${log_rel_path}"
  mkdir -p "$(dirname "$log_host_path")"

  {
    echo "=== trending_metrics_job Prometheus snapshot ==="
    echo "timestamp: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    echo "endpoint: ${PROMETHEUS_METRICS_URL}"
    echo "run_state: ${run_state}"
    echo
  } >"$log_host_path"

  if command -v curl >/dev/null 2>&1; then
    if curl --silent --show-error --max-time 10 "${PROMETHEUS_METRICS_URL}" >>"$log_host_path" 2>&1; then
      echo >>"$log_host_path"
    else
      local curl_status=$?
      {
        echo
        echo "[WARN] curl failed with exit code ${curl_status}. Captured output: "
      } >>"$log_host_path"
      curl --silent "${PROMETHEUS_METRICS_URL}" >>"$log_host_path" 2>&1 || true
      echo >>"$log_host_path"
    fi
  else
    {
      echo "[WARN] curl command not found. Skipping live metrics capture."
      echo
    } >>"$log_host_path"
  fi

  {
    echo "--- ${PROMETHEUS_SERVICE} logs (tail -n 200) ---"
  } >>"$log_host_path"
  compose_run '' logs --tail 200 "${PROMETHEUS_SERVICE}" >>"$log_host_path" 2>&1 || {
    echo "[WARN] Failed to read ${PROMETHEUS_SERVICE} logs." >>"$log_host_path"
  }

  local prom_results_dir="${RESULTS_DIR}/trending-feed/prometheus"
  mkdir -p "$prom_results_dir"
  local prom_rel_path="test-results/trending-feed/prometheus/trending_metrics_job_stage4_${timestamp}.log"
  local prom_host_path="${REPO_ROOT}/${prom_rel_path}"
  cp "$log_host_path" "$prom_host_path"
  echo "[OK] Prometheus metrics log copied to ${prom_rel_path}"

  echo "[OK] Prometheus metrics log saved to ${log_rel_path}"
}

run_ts_trending_feed() {
  local fixture_path="${TS_FIXTURE}"
  if [[ -z "$fixture_path" ]]; then
    fixture_path="${VITE_TRENDING_FIXTURE_PATH:-tests/fixtures/trending/default.json}"
  fi

  local reports_dir="${RESULTS_DIR}/trending-feed/reports"
  mkdir -p "$reports_dir"

  local timestamp
  timestamp="$(date '+%Y%m%d-%H%M%S')"
  local log_dir="tmp/logs/trending-feed"
  local log_rel_path="${log_dir}/${timestamp}.log"
  local latest_rel_path="${log_dir}/latest.log"
  local log_host_path="${REPO_ROOT}/${log_rel_path}"
  local latest_host_path="${REPO_ROOT}/${latest_rel_path}"
  mkdir -p "$(dirname "$log_host_path")"
  : >"$log_host_path"
  {
    echo "=== trending-feed scenario ==="
    echo "timestamp: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
    echo "fixture: ${fixture_path}"
    echo
  } >>"$log_host_path"

  local vitest_targets=(
    'src/tests/unit/routes/trending.test.tsx'
    'src/tests/unit/routes/following.test.tsx'
    'src/tests/unit/hooks/useTrendingFeeds.test.tsx'
  )

  local prom_started=0
  if start_prometheus_trending; then
    prom_started=1
    # Give Prometheus a brief moment to boot
    sleep 2
  fi

  local vitest_status=0
  echo "Running TypeScript scenario 'trending-feed' (fixture: ${fixture_path})..."
  for target in "${vitest_targets[@]}"; do
    local slug="${target//\//_}"
    slug="${slug//./_}"
    local report_rel_path="test-results/trending-feed/reports/${timestamp}-${slug}.json"
    local report_container_path="/app/${report_rel_path}"
    {
      echo "--- Running target: ${target} ---"
      echo "report: ${report_rel_path}"
    } >>"$log_host_path"

    echo "  › pnpm vitest run ${target}"
    local command_template command
    command_template=$(cat <<'BASH'
        set -euo pipefail
        cd /app/kukuri-tauri
        declare -a PNPM_BIN
        if command -v pnpm >/dev/null 2>&1; then
          PNPM_BIN=(pnpm)
        elif command -v corepack >/dev/null 2>&1; then
          PNPM_BIN=(corepack pnpm)
        else
          echo '[ERROR] Neither pnpm nor corepack is available inside container.' >&2
          exit 1
        fi
        echo "[INFO] Using ${PNPM_BIN[*]} for pnpm commands"
        if [ ! -f node_modules/.bin/vitest ]; then
          echo "[INFO] Installing frontend dependencies inside container (${PNPM_BIN[*]} install --frozen-lockfile)..."
          "${PNPM_BIN[@]}" install --frozen-lockfile --ignore-workspace
        fi
        "${PNPM_BIN[@]}" vitest run '__TARGET__' --reporter=default --reporter=json --outputFile "__REPORT_PATH__"
BASH
    )
    command="${command_template//__TARGET__/${target}}"
    command="${command//__REPORT_PATH__/${report_container_path}}"
    if ! compose_run '' run --rm \
      -e "VITE_TRENDING_FIXTURE_PATH=${fixture_path}" \
      ts-test bash -lc "$command" | tee -a "$log_host_path"; then
      vitest_status=${PIPESTATUS[0]}
      echo "[ERROR] Vitest target ${target} failed with exit code ${vitest_status}" >&2
      break
    fi
    if [[ -f "${REPO_ROOT}/${report_rel_path}" ]]; then
      echo "[OK] Scenario report saved to ${report_rel_path}"
    else
      echo "[WARN] Scenario report was not generated at ${report_rel_path}" >&2
    fi
  done

  if [[ $prom_started -eq 1 ]]; then
    collect_trending_metrics_snapshot "${timestamp}" "active"
    stop_prometheus_trending
  else
    collect_trending_metrics_snapshot "${timestamp}" "skipped"
  fi

  if [[ -f "$log_host_path" ]]; then
    cp "$log_host_path" "$latest_host_path"
    echo "[OK] Scenario log saved to ${log_rel_path}"
    echo "[OK] Latest scenario log updated at ${latest_rel_path}"
  else
    echo "[WARN] Scenario log was not generated at ${log_rel_path}" >&2
  fi

  {
    echo
    echo "--- Exporting trending metrics snapshot (scripts/metrics/export-p2p.sh --job trending --pretty) ---"
  } >>"$log_host_path"

  local default_db="${REPO_ROOT}/kukuri-tauri/src-tauri/data/kukuri.db"
  if [[ -f "$default_db" ]]; then
    if DATABASE_URL="sqlite:${default_db}" "${SCRIPT_DIR}/metrics/export-p2p.sh" --job trending --pretty >>"$log_host_path" 2>&1; then
      echo "[OK] Trending metrics JSON saved to test-results/trending-feed/metrics" | tee -a "$log_host_path"
    else
      local export_status=$?
      echo "[WARN] Trending metrics export failed with exit code ${export_status}" | tee -a "$log_host_path" >&2
    fi
  else
    {
      echo "[WARN] Trending metrics export skipped (database not found at ${default_db})."
      echo "[WARN] Run sqlite migrations or set DATABASE_URL before executing this scenario to generate metrics JSON."
    } | tee -a "$log_host_path" >&2
  fi

  return $vitest_status
}

run_ts_profile_avatar_sync() {
  local timestamp
  timestamp="$(date '+%Y%m%d-%H%M%S')"
  local log_rel_path="tmp/logs/profile_avatar_sync_${timestamp}.log"
  if [[ $PROFILE_AVATAR_SW -eq 1 ]]; then
    log_rel_path="tmp/logs/profile_avatar_sync_stage4_${timestamp}.log"
  fi
  local log_host_path="${REPO_ROOT}/${log_rel_path}"
  mkdir -p "$(dirname "$log_host_path")"
  local tests_block=""
  local -a profile_tests=(
    "src/tests/unit/components/settings/ProfileEditDialog.test.tsx"
    "src/tests/unit/components/auth/ProfileSetup.test.tsx"
    "src/tests/unit/hooks/useProfileAvatarSync.test.tsx"
  )
  for spec in "${profile_tests[@]}"; do
    local line
    printf -v line "      '%s' \\\\\n" "$spec"
    tests_block+="$line"
  done
  if [[ $PROFILE_AVATAR_SW -eq 1 ]]; then
    local worker_line
    printf -v worker_line "      '%s' \\\\\n" 'src/tests/unit/workers/profileAvatarSyncWorker.test.ts'
    tests_block+="$worker_line"
  fi

  echo "Running TypeScript scenario 'profile-avatar-sync'..."
  local command_template command container_log_path
  container_log_path="/app/${log_rel_path}"
  command_template=$(cat <<'BASH'
        set -euo pipefail
        cd /app/kukuri-tauri
        if [ ! -f node_modules/.bin/vitest ]; then
          echo '[INFO] Installing frontend dependencies inside container (pnpm install --frozen-lockfile)...'
          pnpm install --frozen-lockfile --ignore-workspace
        fi
        pnpm vitest run \
__TEST_BLOCK__
        2>&1 | tee '__LOG_PATH__'
BASH
  )
  command="${command_template//__TEST_BLOCK__/${tests_block}}"
  command="${command//__LOG_PATH__/${container_log_path}}"
  compose_run '' run --rm ts-test bash -lc "$command"


  if [[ -f "$log_host_path" ]]; then
    echo "[OK] Scenario log saved to ${log_rel_path}"
  else
    echo "[WARN] Scenario log was not generated at ${log_rel_path}" >&2
  fi
}

run_ts_user_search_pagination() {
  local timestamp
  timestamp="$(date +%Y%m%d-%H%M%S)"
  local log_rel_path="tmp/logs/user_search_pagination_${timestamp}.log"
  local log_host_path="${REPO_ROOT}/${log_rel_path}"
  local results_dir="${RESULTS_DIR}/user-search-pagination"
  local reports_dir="${results_dir}/reports"
  local logs_dir="${results_dir}/logs"
  local search_error_dir="${results_dir}/search-error"
  local log_archive_rel_path="test-results/user-search-pagination/logs/${timestamp}.log"
  local log_archive_host_path="${REPO_ROOT}/${log_archive_rel_path}"
  mkdir -p "$(dirname "$log_host_path")" "$reports_dir" "$logs_dir" "$search_error_dir"
  : >"$log_host_path"

  echo "Running TypeScript scenario 'user-search-pagination'..."
  local vitest_targets=(
    'src/tests/unit/hooks/useUserSearchQuery.test.tsx'
    'src/tests/unit/components/search/UserSearchResults.test.tsx'
    'src/tests/unit/scenario/userSearchPaginationArtefact.test.tsx'
  )

  local vitest_status=0
  for target in "${vitest_targets[@]}"; do
    local slug="${target//\//_}"
    slug="${slug//./_}"
    local report_rel_path="test-results/user-search-pagination/reports/${timestamp}-${slug}.json"
    local command="
set -euo pipefail
cd /app/kukuri-tauri
if [ ! -f node_modules/.bin/vitest ]; then
  echo '[INFO] Installing frontend dependencies inside container (pnpm install --frozen-lockfile)...'
  pnpm install --frozen-lockfile --ignore-workspace
fi
pnpm vitest run '${target}' --testTimeout 15000 --reporter=default --reporter=json --outputFile '/app/${report_rel_path}'
"
    if ! compose_run '' run --rm \
      -e "USER_SEARCH_SCENARIO_TIMESTAMP=${timestamp}" \
      ts-test bash -lc "$command" | tee -a "$log_host_path"; then
      vitest_status=${PIPESTATUS[0]}
      echo "[ERROR] Vitest target ${target} failed with exit code ${vitest_status}" >&2
      break
    fi

    if [[ -f "${REPO_ROOT}/${report_rel_path}" ]]; then
      echo "[OK] Scenario report saved to ${report_rel_path}"
    else
      echo "[WARN] Scenario report was not generated at ${report_rel_path}" >&2
    fi
  done

  if [[ -f "$log_host_path" ]]; then
    cp "$log_host_path" "$log_archive_host_path"
    echo "[OK] Scenario log saved to ${log_rel_path}"
    echo "[OK] Archived scenario log saved to ${log_archive_rel_path}"
  else
    echo "[WARN] Scenario log was not generated at ${log_rel_path}" >&2
  fi

  if [[ $vitest_status -ne 0 ]]; then
    echo "[ERROR] Scenario 'user-search-pagination' failed. See ${log_rel_path} for details." >&2
    return $vitest_status
  fi

  return 0
}

run_ts_direct_message() {
  local timestamp
  timestamp="$(date +%Y%m%d-%H%M%S)"
  local log_rel_path="tmp/logs/vitest_direct_message_${timestamp}.log"
  local log_host_path="${REPO_ROOT}/${log_rel_path}"
  local results_dir="${RESULTS_DIR}/direct-message"
  mkdir -p "$(dirname "$log_host_path")" "$results_dir"
  : >"$log_host_path"

  echo "Running TypeScript scenario 'direct-message'..."
  local vitest_targets=(
    'src/tests/unit/components/directMessages/DirectMessageDialog.test.tsx'
    'src/tests/unit/components/directMessages/DirectMessageInbox.test.tsx'
    'src/tests/unit/components/layout/Header.test.tsx'
    'src/tests/unit/hooks/useDirectMessageBadge.test.tsx'
  )

  local vitest_status=0
  for target in "${vitest_targets[@]}"; do
    local slug="${target//\//_}"
    slug="${slug//./_}"
    local report_rel_path="test-results/direct-message/${timestamp}-${slug}.json"
    local command="
set -euo pipefail
cd /app/kukuri-tauri
if [ ! -f node_modules/.bin/vitest ]; then
  echo '[INFO] Installing frontend dependencies inside container (pnpm install --frozen-lockfile)...'
  pnpm install --frozen-lockfile --ignore-workspace
fi
pnpm vitest run '${target}' --reporter=default --reporter=json --outputFile '/app/${report_rel_path}'
"
    if ! compose_run '' run --rm ts-test bash -lc "$command" | tee -a "$log_host_path"; then
      vitest_status=${PIPESTATUS[0]}
      echo "[ERROR] Vitest target ${target} failed with exit code ${vitest_status}" >&2
      break
    fi

    if [[ -f "${REPO_ROOT}/${report_rel_path}" ]]; then
      echo "[OK] Scenario report saved to ${report_rel_path}"
    else
      echo "[WARN] Scenario report was not generated at ${report_rel_path}" >&2
    fi
  done

  if [[ $vitest_status -ne 0 ]]; then
    echo "[ERROR] Scenario 'direct-message' failed. See ${log_rel_path} for details." >&2
    return $vitest_status
  fi

  echo "[OK] Scenario log saved to ${log_rel_path}"
  return 0
}

run_ts_post_delete_cache() {
  local timestamp
  timestamp="$(date +%Y%m%d-%H%M%S)"
  local log_rel_path="tmp/logs/post_delete_cache_${timestamp}.log"
  local log_host_path="${REPO_ROOT}/${log_rel_path}"
  local results_dir="${RESULTS_DIR}/post-delete-cache"
  mkdir -p "$(dirname "$log_host_path")" "$results_dir"
  : >"$log_host_path"

  echo "Running TypeScript scenario 'post-delete-cache'..."
  local vitest_targets=(
    'src/tests/unit/hooks/useDeletePost.test.tsx'
    'src/tests/unit/components/posts/PostCard.test.tsx'
    'src/tests/unit/components/posts/PostCard.deleteOffline.test.tsx'
  )

  local vitest_status=0
  for target in "${vitest_targets[@]}"; do
    local slug="${target//\//_}"
    slug="${slug//./_}"
    local report_rel_path="test-results/post-delete-cache/${timestamp}-${slug}.json"
    local command="
set -euo pipefail
cd /app/kukuri-tauri
if [ ! -f node_modules/.bin/vitest ]; then
  echo '[INFO] Installing frontend dependencies inside container (pnpm install --frozen-lockfile)...'
  pnpm install --frozen-lockfile --ignore-workspace
fi
pnpm vitest run '${target}' --reporter=default --reporter=json --outputFile '/app/${report_rel_path}'
"
    if ! compose_run '' run --rm ts-test bash -lc "$command" | tee -a "$log_host_path"; then
      vitest_status=${PIPESTATUS[0]}
      echo "[ERROR] Vitest target ${target} failed with exit code ${vitest_status}" >&2
      break
    fi

    if [[ -f "${REPO_ROOT}/${report_rel_path}" ]]; then
      echo "[OK] Scenario report saved to ${report_rel_path}"
    else
      echo "[WARN] Scenario report was not generated at ${report_rel_path}" >&2
    fi
  done

  if [[ $vitest_status -ne 0 ]]; then
    echo "[ERROR] Scenario 'post-delete-cache' failed. See ${log_rel_path} for details." >&2
    return $vitest_status
  fi

  echo "[OK] Scenario log saved to ${log_rel_path}"
  return 0
}

run_ts_topic_create() {
  local timestamp
  timestamp="$(date +%Y%m%d-%H%M%S)"
  local log_rel_path="tmp/logs/topic_create_${timestamp}.log"
  local log_host_path="${REPO_ROOT}/${log_rel_path}"
  local results_dir="${RESULTS_DIR}/topic-create"
  mkdir -p "$(dirname "$log_host_path")" "$results_dir"
  : >"$log_host_path"

  echo "Running TypeScript scenario 'topic-create'..."
  local vitest_targets=(
    'src/tests/unit/components/topics/TopicSelector.test.tsx'
    'src/tests/unit/components/posts/PostComposer.test.tsx'
    'src/tests/unit/components/layout/Sidebar.test.tsx'
    'src/tests/unit/scenarios/topicCreateOffline.test.tsx'
  )

  local vitest_status=0
  for target in "${vitest_targets[@]}"; do
    local slug="${target//\//_}"
    slug="${slug//./_}"
    local report_rel_path="test-results/topic-create/${timestamp}-${slug}.json"
    local command="
set -euo pipefail
cd /app/kukuri-tauri
if [ ! -f node_modules/.bin/vitest ]; then
  echo '[INFO] Installing frontend dependencies inside container (pnpm install --frozen-lockfile)...'
  pnpm install --frozen-lockfile --ignore-workspace
fi
pnpm vitest run '${target}' --reporter=default --reporter=json --outputFile '/app/${report_rel_path}'
"
    if ! compose_run '' run --rm ts-test bash -lc "$command" | tee -a "$log_host_path"; then
      vitest_status=${PIPESTATUS[0]}
      echo "[ERROR] Vitest target ${target} failed with exit code ${vitest_status}" >&2
      break
    fi

    if [[ -f "${REPO_ROOT}/${report_rel_path}" ]]; then
      echo "[OK] Scenario report saved to ${report_rel_path}"
    else
      echo "[WARN] Scenario report was not generated at ${report_rel_path}" >&2
    fi
  done

  if [[ $vitest_status -ne 0 ]]; then
    echo "[ERROR] Scenario 'topic-create' failed. See ${log_rel_path} for details." >&2
    return $vitest_status
  fi

  echo "[OK] Scenario log saved to ${log_rel_path}"
  return 0
}

run_ts_offline_sync() {
  local timestamp
  timestamp="$(date '+%Y%m%d-%H%M%S')"
  local variant="${OFFLINE_SYNC_CATEGORY:-}"
  local category_suffix=""
  if [[ -n "$variant" ]]; then
    category_suffix="_${variant}"
  fi
  local log_rel_path="tmp/logs/sync_status_indicator_stage4${category_suffix}_${timestamp}.log"
  local log_host_path="${REPO_ROOT}/${log_rel_path}"
  local reports_dir="${RESULTS_DIR}/offline-sync"
  if [[ -n "$variant" ]]; then
    reports_dir="${reports_dir}/${variant}"
  fi
  mkdir -p "$(dirname "$log_host_path")" "$reports_dir"

  if [[ -n "$variant" ]]; then
    echo "Running TypeScript scenario 'offline-sync' (category: ${variant})..."
  else
    echo "Running TypeScript scenario 'offline-sync'..."
  fi

  local vitest_targets=()
  if [[ -n "$variant" ]]; then
    vitest_targets=('src/tests/unit/scenarios/offlineSyncTelemetry.test.tsx')
  else
    vitest_targets=(
      'src/tests/unit/hooks/useSyncManager.test.tsx'
      'src/tests/unit/components/SyncStatusIndicator.test.tsx'
      'src/tests/unit/components/OfflineIndicator.test.tsx'
    )
  fi
  local vitest_status=0
  for target in "${vitest_targets[@]}"; do
    local slug="${target//\//_}"
    slug="${slug//./_}"
    local report_rel_path="test-results/offline-sync"
    if [[ -n "$variant" ]]; then
      report_rel_path="${report_rel_path}/${variant}"
    fi
    report_rel_path="${report_rel_path}/${timestamp}-${slug}.json"
    local export_category=""
    if [[ -n "$variant" ]]; then
      export_category="export OFFLINE_SYNC_CATEGORY='${variant}'"
    fi
    local command="
set -euo pipefail
cd /app/kukuri-tauri
if [ ! -f node_modules/.bin/vitest ]; then
  echo '[INFO] Installing frontend dependencies inside container (pnpm install --frozen-lockfile)...'
  pnpm install --frozen-lockfile --ignore-workspace
fi
${export_category}
pnpm vitest run '${target}' --reporter=default --reporter=json --outputFile '/app/${report_rel_path}'
"
    if ! compose_run '' run --rm ts-test bash -lc "$command" | tee -a "$log_host_path"; then
      vitest_status=${PIPESTATUS[0]}
      echo "[ERROR] Vitest target ${target} failed with exit code ${vitest_status}" >&2
      break
    fi

    if [[ -f "${REPO_ROOT}/${report_rel_path}" ]]; then
      echo "[OK] Scenario report saved to ${report_rel_path}"
    else
      echo "[WARN] Scenario report was not generated at ${report_rel_path}" >&2
    fi
  done

  if [[ $vitest_status -ne 0 ]]; then
    echo "[ERROR] Scenario 'offline-sync' failed. See ${log_rel_path} for details." >&2
    return $vitest_status
  fi

  if [[ -f "$log_host_path" ]]; then
    echo "[OK] Scenario log saved to ${log_rel_path}"
  else
    echo "[WARN] Scenario log was not generated at ${log_rel_path}" >&2
  fi
}

run_ts_tests() {
  [[ $NO_BUILD -eq 1 ]] || build_image
  if [[ -z "$TS_SCENARIO" ]]; then
    echo 'Running TypeScript tests in Docker...'
    compose_run '' run --rm ts-test
    echo '[OK] TypeScript tests passed'
  else
    case "$TS_SCENARIO" in
      trending-feed)
        run_ts_trending_feed
        ;;
      profile-avatar-sync)
        run_ts_profile_avatar_sync
        ;;
      user-search-pagination)
        run_ts_user_search_pagination
        ;;
      direct-message)
        run_ts_direct_message
        ;;
      post-delete-cache)
        run_ts_post_delete_cache
        ;;
      topic-create)
        run_ts_topic_create
        ;;
      offline-sync)
        run_ts_offline_sync
        ;;
      *)
        echo "Unknown TypeScript scenario: $TS_SCENARIO" >&2
        exit 1
        ;;
    esac
  fi
}

wait_community_node() {
  local base_url="$1"
  local timeout="${2:-120}"
  local health_url="${base_url%/}/healthz"

  for ((i = 0; i < timeout; i++)); do
    if command -v curl >/dev/null 2>&1; then
      if curl --silent --show-error --max-time 5 "${health_url}" >/dev/null; then
        return 0
      fi
    elif command -v wget >/dev/null 2>&1; then
      if wget -q -T 5 -O /dev/null "${health_url}" 2>/dev/null; then
        return 0
      fi
    else
      echo "[ERROR] curl or wget is required to check community node health." >&2
      return 1
    fi
    sleep 1
  done
  return 1
}

start_community_node() {
  local base_url="$1"
  if [[ $NO_BUILD -eq 0 ]]; then
    echo 'Building community-node-user-api image...'
    compose_run '' build community-node-user-api community-node-bootstrap
  fi
  echo 'Starting community-node-user-api service...'
  if ! compose_run '' up -d community-node-user-api; then
    echo 'Failed to start community-node-user-api service.' >&2
    return 1
  fi
  if ! wait_community_node "$base_url" 120; then
    echo "community-node-user-api health check failed: ${base_url%/}/healthz" >&2
    return 1
  fi
  echo '[OK] community-node-user-api is healthy.'

  echo 'Starting community-node-bootstrap service...'
  if ! compose_run '' up -d community-node-bootstrap; then
    echo 'Failed to start community-node-bootstrap service.' >&2
    return 1
  fi
}

seed_community_node() {
  echo 'Seeding community node E2E fixtures...'
  local seed_output
  set +e
  seed_output=$(compose_run '' run --rm --entrypoint cn community-node-user-api e2e seed 2>&1)
  local status=$?
  set -e
  if [[ $status -ne 0 ]]; then
    echo "$seed_output" >&2
    echo 'Community node E2E seed failed.' >&2
    return $status
  fi

  local seed_line
  seed_line=$(printf '%s\n' "$seed_output" | awk '/^E2E_SEED_JSON=/{line=$0} END{print line}')
  if [[ -z "$seed_line" ]]; then
    echo 'Failed to capture E2E seed JSON from community node helper output.' >&2
    return 1
  fi
  local seed_json="${seed_line#E2E_SEED_JSON=}"

  local log_dir="${REPO_ROOT}/tmp/logs/community-node-e2e"
  local seed_path="${log_dir}/seed.json"
  mkdir -p "$log_dir"
  printf '%s\n' "$seed_json" > "$seed_path"
  export E2E_COMMUNITY_NODE_SEED_JSON="$seed_json"

  echo '[OK] Community node E2E seed applied.'
  issue_community_node_invite
}

issue_community_node_invite() {
  local topic_name="${E2E_COMMUNITY_NODE_TOPIC_NAME:-e2e-community-node-invite}"
  local log_dir="${REPO_ROOT}/tmp/logs/community-node-e2e"
  local log_path="${log_dir}/invite.json"

  mkdir -p "$log_dir"

  echo 'Issuing community node invite capability...'
  local invite_output
  set +e
  invite_output=$(compose_run '' run --rm --env RUST_LOG=off --entrypoint cn community-node-user-api e2e invite --topic "$topic_name" 2>&1)
  local status=$?
  set -e
  if [[ $status -ne 0 ]]; then
    echo "$invite_output" >&2
    echo 'Community node invite helper failed.' >&2
    return $status
  fi

  local invite_json
  invite_json=$(printf '%s\n' "$invite_output" | awk '/^\s*\{/{line=$0} END{print line}')
  if [[ -z "$invite_json" ]]; then
    echo 'Failed to capture invite JSON from community node helper output.' >&2
    return 1
  fi

  printf '%s\n' "$invite_json" > "$log_path"
  export E2E_COMMUNITY_NODE_INVITE_JSON="$invite_json"
  export E2E_COMMUNITY_NODE_TOPIC_NAME="$topic_name"
  echo "[OK] Community node invite issued (topic=${topic_name})."
}

cleanup_community_node() {
  echo 'Cleaning up community node E2E fixtures...'
  if ! compose_run '' run --rm --entrypoint cn community-node-user-api e2e cleanup >/dev/null 2>&1; then
    echo '[WARN] Community node E2E cleanup failed.' >&2
  else
    echo '[OK] Community node E2E cleanup completed.'
  fi
}

stop_community_node() {
  echo 'Stopping community-node services...'
  compose_run '' rm -sf community-node-user-api community-node-bootstrap community-node-postgres community-node-meilisearch >/dev/null 2>&1 || true
}

run_desktop_e2e_community_node() {
  [[ $NO_BUILD -eq 1 ]] || build_image
  echo 'Running desktop E2E tests (community node) via Docker...'

  local base_url="${COMMUNITY_NODE_BASE_URL:-$COMMUNITY_NODE_BASE_URL_DEFAULT}"
  local previous_scenario="${SCENARIO-}"
  local previous_base_url="${COMMUNITY_NODE_BASE_URL-}"
  local previous_e2e_url="${E2E_COMMUNITY_NODE_URL-}"
  local previous_invite_json="${E2E_COMMUNITY_NODE_INVITE_JSON-}"
  local previous_seed_json="${E2E_COMMUNITY_NODE_SEED_JSON-}"
  local previous_topic_name="${E2E_COMMUNITY_NODE_TOPIC_NAME-}"

  export COMMUNITY_NODE_BASE_URL="$base_url"
  export E2E_COMMUNITY_NODE_URL="$base_url"
  export SCENARIO="community-node-e2e"

  local status=0
  if ! start_community_node "$base_url"; then
    status=1
  elif ! seed_community_node; then
    status=1
  else
    set +e
    compose_run '' run --rm test-runner
    status=$?
    set -e
  fi

  cleanup_community_node
  stop_community_node

  if [[ -n "${previous_scenario-}" ]]; then
    export SCENARIO="$previous_scenario"
  else
    unset SCENARIO
  fi
  if [[ -n "${previous_base_url-}" ]]; then
    export COMMUNITY_NODE_BASE_URL="$previous_base_url"
  else
    unset COMMUNITY_NODE_BASE_URL
  fi
  if [[ -n "${previous_e2e_url-}" ]]; then
    export E2E_COMMUNITY_NODE_URL="$previous_e2e_url"
  else
    unset E2E_COMMUNITY_NODE_URL
  fi
  if [[ -n "${previous_invite_json-}" ]]; then
    export E2E_COMMUNITY_NODE_INVITE_JSON="$previous_invite_json"
  else
    unset E2E_COMMUNITY_NODE_INVITE_JSON
  fi
  if [[ -n "${previous_seed_json-}" ]]; then
    export E2E_COMMUNITY_NODE_SEED_JSON="$previous_seed_json"
  else
    unset E2E_COMMUNITY_NODE_SEED_JSON
  fi
  if [[ -n "${previous_topic_name-}" ]]; then
    export E2E_COMMUNITY_NODE_TOPIC_NAME="$previous_topic_name"
  else
    unset E2E_COMMUNITY_NODE_TOPIC_NAME
  fi

  if [[ $status -ne 0 ]]; then
    return $status
  fi

  echo '[OK] Desktop E2E scenario (community node) finished. Artefacts stored in tmp/logs/community-node-e2e/ and test-results/community-node-e2e/.'
}

run_desktop_e2e() {
  [[ $NO_BUILD -eq 1 ]] || build_image
  echo 'Running desktop E2E tests via Docker...'
  local previous="${SCENARIO-}"
  export SCENARIO="desktop-e2e"
  if ! compose_run '' run --rm test-runner; then
    local status=$?
    if [[ -n "${previous-}" ]]; then
      export SCENARIO="$previous"
    else
      unset SCENARIO
    fi
    return $status
  fi

  if [[ -n "${previous-}" ]]; then
    export SCENARIO="$previous"
  else
    unset SCENARIO
  fi

  echo '[OK] Desktop E2E scenario finished. Artefacts stored in tmp/logs/desktop-e2e/ and test-results/desktop-e2e/.'
}

run_lint_check() {
  [[ $NO_BUILD -eq 1 ]] || build_image
  echo 'Running lint and format checks in Docker...'
  compose_run '' run --rm lint-check
  echo '[OK] Lint and format checks passed'
}

run_rust_coverage() {
  [[ $NO_BUILD -eq 1 ]] || build_image
  prepare_coverage_dirs
  echo 'Running cargo tarpaulin (Rust coverage) in Docker...'
  compose_run '' run --rm rust-coverage
  save_coverage_artifacts
  echo '[OK] Rust coverage collection completed'
}

run_performance_tests() {
  [[ $NO_BUILD -eq 1 ]] || build_image
  echo 'Running Rust performance harness (ignored tests)...'
  compose_run '' run --rm --env KUKURI_PERFORMANCE_OUTPUT=/app/test-results/performance \
    rust-test cargo test --test performance -- --ignored --nocapture
  echo '[OK] Performance harness completed. Reports stored under test-results/performance'
}

cleanup() {
  echo 'Cleaning up Docker containers and images...'
  compose_run '' down --rmi local --remove-orphans
  echo '[OK] Cleanup completed'
}

cache_cleanup() {
  echo 'Performing complete cleanup including cache volumes...'
  compose_run '' down --rmi local --volumes --remove-orphans || true
  echo 'Removing cache volumes...'
  docker volume rm kukuri-cargo-registry kukuri-cargo-git kukuri-cargo-target kukuri-pnpm-store 2>/dev/null || true
  echo '[OK] Complete cleanup finished'
  echo '[INFO] Next build will take longer as caches were removed'
}

show_cache_status() {
  echo
  echo 'Cache Volume Status:'
  echo '-------------------'
  local vols=(kukuri-cargo-registry kukuri-cargo-git kukuri-cargo-target kukuri-pnpm-store)
  for vol in "${vols[@]}"; do
    if docker volume ls --quiet --filter "name=${vol}" >/dev/null 2>&1 && docker volume ls --quiet --filter "name=${vol}" | grep -q "${vol}"; then
      local size
      size=$(docker run --rm -v "${vol}:/data" alpine du -sh /data 2>/dev/null | head -n1)
      echo "  ${vol} : ${size}"
    else
      echo "  ${vol} : Not created yet"
    fi
  done
  echo
}

write_p2p_env() {
  mkdir -p "$(dirname "$ENV_FILE")"
  local bootstrap="${BOOTSTRAP_PEERS:-}"
  if [[ -z "$bootstrap" ]]; then
    bootstrap="$BOOTSTRAP_DEFAULT_PEER"
  fi
  {
    echo 'ENABLE_P2P_INTEGRATION=1'
    echo 'KUKURI_FORCE_LOCALHOST_ADDRS=0'
    echo "RUST_LOG=${RUST_LOG}"
    echo "RUST_BACKTRACE=${RUST_BACKTRACE}"
    echo "KUKURI_BOOTSTRAP_PEERS=${bootstrap}"
  } >"$ENV_FILE"
}

wait_bootstrap_healthy() {
  local timeout="${1:-60}"
  local i
  for ((i = 0; i < timeout; i++)); do
    local status
    status=$(docker inspect --format '{{.State.Health.Status}}' "$BOOTSTRAP_CONTAINER" 2>/dev/null || true)
    if [[ "$status" == "healthy" ]]; then
      return 0
    fi
    sleep 1
  done
  return 1
}

start_bootstrap() {
  echo 'Starting p2p-bootstrap container...'
  set +e
  compose_run '' up -d p2p-bootstrap
  local code=$?
  set -e
  if [[ $code -ne 0 ]]; then
    echo 'Failed to start p2p-bootstrap container.' >&2
    return $code
  fi
  if ! wait_bootstrap_healthy 60; then
    echo 'p2p-bootstrap health check failed.' >&2
    return 1
  fi
  echo '[OK] p2p-bootstrap is healthy.'
  return 0
}

stop_bootstrap() {
  set +e
  compose_run '' down --remove-orphans >/dev/null 2>&1
  set -e
}

run_p2p_tests() {
  ensure_docker
  write_p2p_env

  if [[ $NO_BUILD -eq 0 ]]; then
    echo 'Building rust-test image...'
    DOCKER_BUILDKIT=1 compose_run "$ENV_FILE" build rust-test
  fi

  local -a cargo_args
  local selected="$TESTS"

  case "$selected" in
    mainline)
      selected="$P2P_MAINLINE_TEST"
      ;;
    gossip)
      selected="$P2P_GOSSIP_TEST"
      ;;
  esac

  case "$selected" in
    all)
      cargo_args=(test --workspace --all-features -- --nocapture)
      ;;
    workspace)
      cargo_args=(test --all-features -- --nocapture)
      ;;
    modules::*|tests::*)
      cargo_args=(test --package kukuri-tauri --lib "${selected}" -- --nocapture --test-threads=1)
      ;;
    *)
      cargo_args=(test --package kukuri-tauri --test "${selected}" -- --nocapture --test-threads=1)
      ;;
  esac

  echo "Running tests (cargo ${cargo_args[*]}) inside Docker..."
  if ! start_bootstrap; then
    if [[ $KEEP_ENV -eq 0 ]]; then
      rm -f "$ENV_FILE"
    fi
    exit 1
  fi

  set +e
  compose_run "$ENV_FILE" run --rm rust-test cargo "${cargo_args[@]}"
  local code=$?
  set -e

  stop_bootstrap
  if [[ $KEEP_ENV -eq 0 ]]; then
    rm -f "$ENV_FILE"
  fi

  if [[ $code -ne 0 ]]; then
    echo "Error: docker compose exited with code $code" >&2
    exit $code
  fi

  echo '[OK] P2P integration tests completed successfully.'
}

ensure_docker

if [[ ! -f "$COMPOSE_FILE" ]]; then
  echo "docker-compose.test.yml not found: $COMPOSE_FILE" >&2
  exit 1
fi

mkdir -p "$RESULTS_DIR"

COMMAND="${1:-all}"
shift || true

TESTS="${P2P_GOSSIP_TEST}"
BOOTSTRAP_PEERS=""
NO_BUILD=0
KEEP_ENV=0
RUST_LOG="debug"
RUST_BACKTRACE="full"
TS_SCENARIO=""
TS_FIXTURE=""
PROFILE_AVATAR_SW=0
OFFLINE_SYNC_CATEGORY=""

case "$COMMAND" in
  -h|--help)
    usage
    exit 0
    ;;
  all|rust|lint|coverage|build|clean|cache-clean|performance|e2e|e2e-community-node)
    ;;
  ts)
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --scenario)
          TS_SCENARIO="$2"
          shift 2
          ;;
        --fixture)
          TS_FIXTURE="$2"
          shift 2
          ;;
        --offline-category)
          OFFLINE_SYNC_CATEGORY="$2"
          shift 2
          ;;
        --service-worker)
          PROFILE_AVATAR_SW=1
          shift
          ;;
        --no-build)
          NO_BUILD=1
          shift
          ;;
        -h|--help)
          usage
          exit 0
          ;;
        *)
          echo "Unknown option for ts command: $1" >&2
          exit 1
          ;;
esac

if [[ -n "$OFFLINE_SYNC_CATEGORY" && "$TS_SCENARIO" != "offline-sync" ]]; then
  echo "--offline-category is only supported with --scenario offline-sync" >&2
  exit 1
fi
    done
    ;;
  p2p)
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --tests)
          TESTS="$2"
          shift 2
          ;;
        --bootstrap)
          BOOTSTRAP_PEERS="$2"
          shift 2
          ;;
        --no-build)
          NO_BUILD=1
          shift
          ;;
        --keep-env)
          KEEP_ENV=1
          shift
          ;;
        --rust-log)
          RUST_LOG="$2"
          shift 2
          ;;
        --rust-backtrace)
          RUST_BACKTRACE="$2"
          shift 2
          ;;
        -h|--help)
          usage
          exit 0
          ;;
        *)
          echo "Unknown option: $1" >&2
          exit 1
          ;;
      esac
    done
    ;;
  *)
    echo "Unknown command: $COMMAND" >&2
    usage
    exit 1
    ;;
 esac

case "$COMMAND" in
  all)
    run_all_tests
    show_cache_status
    ;;
  rust)
    run_rust_tests
    show_cache_status
    ;;
  ts)
    run_ts_tests
    show_cache_status
    ;;
  lint)
    run_lint_check
    show_cache_status
    ;;
  coverage)
    run_rust_coverage
    show_cache_status
    ;;
  performance)
    run_performance_tests
    show_cache_status
    ;;
  e2e)
    run_desktop_e2e
    show_cache_status
    ;;
  e2e-community-node)
    run_desktop_e2e_community_node
    show_cache_status
    ;;
  build)
    build_image
    show_cache_status
    ;;
  clean)
    cleanup
    ;;
  cache-clean)
    cache_cleanup
    ;;
  p2p)
    run_p2p_tests
    ;;
 esac

exit 0
