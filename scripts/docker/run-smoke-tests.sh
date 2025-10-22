#!/bin/bash
set -euo pipefail

DEFAULT_BOOTSTRAP="03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8@127.0.0.1:11233"
export ENABLE_P2P_INTEGRATION="${ENABLE_P2P_INTEGRATION:-1}"
export KUKURI_FORCE_LOCALHOST_ADDRS="${KUKURI_FORCE_LOCALHOST_ADDRS:-0}"
export KUKURI_BOOTSTRAP_PEERS="${KUKURI_BOOTSTRAP_PEERS:-$DEFAULT_BOOTSTRAP}"

P2P_MAINLINE_TEST="${P2P_MAINLINE_TEST_TARGET:-p2p_mainline_smoke}"
P2P_GOSSIP_TEST="${P2P_GOSSIP_TEST_TARGET:-p2p_gossip_smoke}"

run_p2p_suite() {
  local prefix="$1"
  local target="$2"
  local description="$3"

  local -a cargo_cmd=(cargo test --package kukuri-tauri --test "$target" -- --nocapture --test-threads=1)

  echo "=== Running ${description} (integration target: ${target}) ==="

  "${cargo_cmd[@]}" &
  local pid=$!
  local cmd_str
  printf -v cmd_str '%q ' "${cargo_cmd[@]}"
  eval "${prefix}_PID=${pid}"
  eval "${prefix}_CMD=\"${cmd_str% }\""
}

WAIT_SECONDS="${BOOTSTRAP_WAIT_SECONDS:-10}"
echo "=== Waiting ${WAIT_SECONDS}s for bootstrap startup ==="
sleep "${WAIT_SECONDS}"

echo "=== Running Rust P2P smoke tests (parallel mainline & gossip) ==="
cd /app/kukuri-tauri/src-tauri
set +e
run_p2p_suite "P2P_GOSSIP" "$P2P_GOSSIP_TEST" "Rust P2P gossip smoke tests"
run_p2p_suite "P2P_MAINLINE" "$P2P_MAINLINE_TEST" "Rust P2P mainline smoke tests"

wait ${P2P_GOSSIP_PID}
GOSSIP_STATUS=$?
wait ${P2P_MAINLINE_PID}
MAINLINE_STATUS=$?
set -e

if [ ${GOSSIP_STATUS} -ne 0 ] || [ ${MAINLINE_STATUS} -ne 0 ]; then
  echo "Rust P2P smoke tests failed (gossip=${GOSSIP_STATUS}, mainline=${MAINLINE_STATUS})" >&2
  if [ ${GOSSIP_STATUS} -ne 0 ]; then
    echo "  • Gossip command: ${P2P_GOSSIP_CMD}" >&2
  fi
  if [ ${MAINLINE_STATUS} -ne 0 ]; then
    echo "  • Mainline command: ${P2P_MAINLINE_CMD}" >&2
  fi
  exit 1
fi

echo "=== Rust P2P smoke tests completed ==="

echo "=== Running TypeScript integration smoke tests ==="
cd /app/kukuri-tauri
pnpm test:integration

echo "=== Smoke tests passed! ==="
