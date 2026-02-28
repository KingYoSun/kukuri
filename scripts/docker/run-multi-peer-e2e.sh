#!/bin/bash
set -euo pipefail

export SCENARIO="${SCENARIO:-multi-peer-e2e}"
export E2E_SPEC_PATTERN="${E2E_SPEC_PATTERN:-./tests/e2e/specs/community-node.multi-peer.spec.ts}"
export E2E_MULTI_PEER_EXPECTED_MIN="${E2E_MULTI_PEER_EXPECTED_MIN:-1}"
export E2E_MULTI_PEER_PUBLISH_PREFIX="${E2E_MULTI_PEER_PUBLISH_PREFIX:-multi-peer-publisher}"

/app/run-desktop-e2e.sh
