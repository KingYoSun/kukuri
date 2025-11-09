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
  build        Build the Docker image only
  clean        Clean containers and images
  cache-clean  Clean including cache volumes
  performance  Run the Rust performance harness (ignored tests) and export reports
  p2p          Run P2P integration tests inside Docker

Options for ts:
  --scenario <name>      Execute a preset scenario (e.g. trending-feed, profile-avatar-sync)
  --fixture <path>       Override VITE_TRENDING_FIXTURE_PATH for the scenario
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

build_image() {
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

run_ts_trending_feed() {
  local fixture_path="${TS_FIXTURE}"
  if [[ -z "$fixture_path" ]]; then
    fixture_path="${VITE_TRENDING_FIXTURE_PATH:-tests/fixtures/trending/default.json}"
  fi

  local scenario_dir="${RESULTS_DIR}/trending-feed"
  mkdir -p "$scenario_dir"

  local timestamp
  timestamp="$(date '+%Y%m%d-%H%M%S')"
  local vitest_targets=(
    'src/tests/unit/routes/trending.test.tsx'
    'src/tests/unit/routes/following.test.tsx'
    'src/tests/unit/hooks/useTrendingFeeds.test.tsx'
  )

  echo "Running TypeScript scenario 'trending-feed' (fixture: ${fixture_path})..."
  for target in "${vitest_targets[@]}"; do
    local slug="${target//\//_}"
    slug="${slug//./_}"
    local report_rel_path="test-results/trending-feed/${timestamp}-${slug}.json"
    local report_container_path="/app/${report_rel_path}"

    echo "  → pnpm vitest run ${target}"
    compose_run '' run --rm \
      -e "VITE_TRENDING_FIXTURE_PATH=${fixture_path}" \
      ts-test bash -lc "
        set -euo pipefail
        cd /app/kukuri-tauri
        if [ ! -f node_modules/.bin/vitest ]; then
          echo '[INFO] Installing frontend dependencies inside container (pnpm install --frozen-lockfile)...'
          pnpm install --frozen-lockfile --ignore-workspace
        fi
        pnpm vitest run '${target}' --reporter=default --reporter=json --outputFile '${report_container_path}'
      "

    if [[ -f "${REPO_ROOT}/${report_rel_path}" ]]; then
      echo "[OK] Scenario report saved to ${report_rel_path}"
    else
      echo "[WARN] Scenario report was not generated at ${report_rel_path}" >&2
    fi
  done
}

run_ts_profile_avatar_sync() {
  local timestamp
  timestamp="$(date '+%Y%m%d-%H%M%S')"
  local log_rel_path="tmp/logs/profile_avatar_sync_${timestamp}.log"
  local log_host_path="${REPO_ROOT}/${log_rel_path}"
  mkdir -p "$(dirname "$log_host_path")"

  echo "Running TypeScript scenario 'profile-avatar-sync'..."
  compose_run '' run --rm ts-test bash -lc "
    set -euo pipefail
    cd /app/kukuri-tauri
    if [ ! -f node_modules/.bin/vitest ]; then
      echo '[INFO] Installing frontend dependencies inside container (pnpm install --frozen-lockfile)...'
      pnpm install --frozen-lockfile --ignore-workspace
    fi
    pnpm vitest run \
      'src/tests/unit/components/settings/ProfileEditDialog.test.tsx' \
      'src/tests/unit/components/auth/ProfileSetup.test.tsx' \
      'src/tests/unit/hooks/useProfileAvatarSync.test.tsx' \
      | tee '/app/${log_rel_path}'
  "

  if [[ -f "$log_host_path" ]]; then
    echo "[OK] Scenario log saved to ${log_rel_path}"
  else
    echo "[WARN] Scenario log was not generated at ${log_rel_path}" >&2
  fi
}

run_ts_post_delete_cache() {
  local timestamp
  timestamp="$(date '+%Y%m%d-%H%M%S')"
  local log_rel_path="tmp/logs/post-delete-cache_docker_${timestamp}.log"
  local log_host_path="${REPO_ROOT}/${log_rel_path}"
  mkdir -p "$(dirname "$log_host_path")"

  echo "Running TypeScript scenario 'post-delete-cache'..."
  if compose_run '' run --rm ts-test pnpm vitest run \
    src/tests/unit/hooks/useDeletePost.test.ts \
    src/tests/unit/components/posts/PostCard.test.tsx >"$log_host_path" 2>&1; then
    echo "[OK] Scenario log saved to ${log_rel_path}"
  else
    echo "[ERROR] Scenario 'post-delete-cache' failed. See ${log_rel_path} for details." >&2
    return 1
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
      post-delete-cache)
        run_ts_post_delete_cache
        ;;
      *)
        echo "Unknown TypeScript scenario: $TS_SCENARIO" >&2
        exit 1
        ;;
    esac
  fi
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

case "$COMMAND" in
  -h|--help)
    usage
    exit 0
    ;;
  all|rust|lint|coverage|build|clean|cache-clean|performance)
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
