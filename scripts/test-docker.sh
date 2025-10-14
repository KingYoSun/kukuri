#!/bin/bash
# Docker環境でのテスト実行スクリプト

set -euo pipefail

PROJECT_NAME="kukuri_tests"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
COMPOSE_FILE="${REPO_ROOT}/docker-compose.test.yml"
ENV_FILE="${REPO_ROOT}/kukuri-tauri/tests/.env.p2p"
RESULTS_DIR="${REPO_ROOT}/test-results"
BOOTSTRAP_DEFAULT_PEER="03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8@127.0.0.1:11233"
BOOTSTRAP_CONTAINER="kukuri-p2p-bootstrap"

usage() {
  cat <<'EOF'
Usage: ./test-docker.sh [command] [options]

Commands:
  all          Run all tests (default)
  rust         Run Rust tests only
  ts           Run TypeScript tests only
  lint         Run lint/format checks only
  build        Build the Docker image only
  clean        Clean containers and images
  cache-clean  Clean including cache volumes
  p2p          Run P2P integration tests inside Docker

Options for p2p:
  --tests <name>          Cargo test filter (default: modules::p2p::tests::iroh_integration_tests::)
  --bootstrap <peers>     KUKURI_BOOTSTRAP_PEERS (comma separated node@host:port)
  --no-build              Skip docker compose build
  --keep-env              Keep generated .env.p2p after execution
  --rust-log <value>      RUST_LOG for P2P (default: debug)
  --rust-backtrace <val>  RUST_BACKTRACE for P2P (default: full)
  -h, --help              Show this help
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
  DOCKER_BUILDKIT=1 compose_run '' build test-runner
  echo '[OK] Docker image built successfully'
}

run_all_tests() {
  [[ $NO_BUILD -eq 1 ]] || build_image
  echo 'Running all tests in Docker...'
  compose_run '' run --rm test-runner
  echo '[OK] All tests passed'
}

run_rust_tests() {
  [[ $NO_BUILD -eq 1 ]] || build_image
  echo 'Running Rust tests in Docker...'
  compose_run '' run --rm rust-test bash -lc "cargo test --workspace --all-features -- --nocapture"
  echo '[OK] Rust tests passed'
}

run_ts_tests() {
  [[ $NO_BUILD -eq 1 ]] || build_image
  echo 'Running TypeScript tests in Docker...'
  compose_run '' run --rm ts-test
  echo '[OK] TypeScript tests passed'
}

run_lint_check() {
  [[ $NO_BUILD -eq 1 ]] || build_image
  echo 'Running lint and format checks in Docker...'
  compose_run '' run --rm lint-check
  echo '[OK] Lint and format checks passed'
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
  }
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

  local cargo_cmd
  local cargo_args
  case "$TESTS" in
    all)
      cargo_args=(test --workspace --all-features -- --nocapture)
      ;;
    workspace)
      cargo_args=(test --all-features -- --nocapture)
      ;;
    *)
      cargo_args=(test --package kukuri-tauri --lib "${TESTS}" -- --nocapture --test-threads=1)
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

TESTS="modules::p2p::tests::iroh_integration_tests::"
BOOTSTRAP_PEERS=""
NO_BUILD=0
KEEP_ENV=0
RUST_LOG="debug"
RUST_BACKTRACE="full"

case "$COMMAND" in
  -h|--help)
    usage
    exit 0
    ;;
  all|rust|ts|lint|build|clean|cache-clean)
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
