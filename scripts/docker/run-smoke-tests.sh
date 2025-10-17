#!/bin/bash
set -euo pipefail

DEFAULT_BOOTSTRAP="03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8@127.0.0.1:11233"
export ENABLE_P2P_INTEGRATION="${ENABLE_P2P_INTEGRATION:-1}"
export KUKURI_FORCE_LOCALHOST_ADDRS="${KUKURI_FORCE_LOCALHOST_ADDRS:-0}"
export KUKURI_BOOTSTRAP_PEERS="${KUKURI_BOOTSTRAP_PEERS:-$DEFAULT_BOOTSTRAP}"

WAIT_SECONDS="${BOOTSTRAP_WAIT_SECONDS:-10}"
echo "=== Waiting ${WAIT_SECONDS}s for bootstrap startup ==="
sleep "${WAIT_SECONDS}"

echo "=== Running Rust P2P smoke tests (parallel mainline & gossip) ==="
cd /app/kukuri-tauri/src-tauri
set +e
cargo test --package kukuri-tauri --lib modules::p2p::tests::iroh_integration_tests:: -- --nocapture --test-threads=1 &
P2P_GOSSIP_PID=$!
cargo test --package kukuri-tauri --lib modules::p2p::tests::mainline_dht_tests:: -- --nocapture --test-threads=1 &
P2P_MAINLINE_PID=$!

wait ${P2P_GOSSIP_PID}
GOSSIP_STATUS=$?
wait ${P2P_MAINLINE_PID}
MAINLINE_STATUS=$?
set -e

if [ ${GOSSIP_STATUS} -ne 0 ] || [ ${MAINLINE_STATUS} -ne 0 ]; then
  echo "Rust P2P smoke tests failed (gossip=${GOSSIP_STATUS}, mainline=${MAINLINE_STATUS})" >&2
  exit 1
fi

echo "=== Rust P2P smoke tests completed ==="

echo "=== Running TypeScript integration smoke tests ==="
cd /app/kukuri-tauri
pnpm test:integration

echo "=== Smoke tests passed! ==="
