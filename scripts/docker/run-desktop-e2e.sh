#!/bin/bash
set -euo pipefail

APP_DIR="/app/kukuri-tauri"
OUTPUT_DIR="$APP_DIR/tests/e2e/output"
SCENARIO_NAME="${SCENARIO:-desktop-e2e}"
RESULT_DIR="/app/test-results/desktop-e2e"
LOG_DIR="/app/tmp/logs/desktop-e2e"
export E2E_FORBID_PENDING=0
export E2E_COMMUNITY_NODE_P2P_INVITE=0
if [[ "$SCENARIO_NAME" == "community-node-e2e" ]]; then
  RESULT_DIR="/app/test-results/community-node-e2e"
  LOG_DIR="/app/tmp/logs/community-node-e2e"
  export E2E_FORBID_PENDING=1
  export E2E_COMMUNITY_NODE_P2P_INVITE=1
fi

mkdir -p "$RESULT_DIR" "$LOG_DIR" "$OUTPUT_DIR"
rm -f "$OUTPUT_DIR"/*.png "$OUTPUT_DIR"/*.json 2>/dev/null || true

timestamp="$(date -u +"%Y%m%d-%H%M%S")"
log_file="$LOG_DIR/$timestamp.log"
snapshot_dir="$RESULT_DIR/$timestamp"
mkdir -p "$snapshot_dir"

cd "$APP_DIR"

echo "=== ${SCENARIO_NAME}: building debug bundle ==="
pnpm e2e:build

echo "=== ${SCENARIO_NAME}: running pnpm e2e:ci ==="
if [[ "$E2E_FORBID_PENDING" == "1" ]]; then
  echo "=== ${SCENARIO_NAME}: enforcing pending/skip as failures ==="
fi
E2E_COMMAND=(pnpm e2e:ci)
export E2E_SKIP_BUILD=1
if [[ -z "${TAURI_DRIVER_PORT:-}" ]]; then
  echo "Selecting available TAURI_DRIVER_PORT..."
  TAURI_DRIVER_PORT="$(node -e 'const net=require("net");
const min=Number(process.env.TAURI_PORT_MIN ?? 4700);
const max=Number(process.env.TAURI_PORT_MAX ?? 5200);
const isFree=(port)=>new Promise((resolve)=>{
  const server=net.createServer();
  server.unref();
  server.on("error",()=>resolve(false));
  server.listen(port,"127.0.0.1",()=>server.close(()=>resolve(true)));
});
(async ()=>{
  for(let port=min; port<=max; port+=1){
    if(await isFree(port) && await isFree(port+1) && await isFree(port+100)){
      console.log(port);
      return;
    }
  }
  process.exit(1);
})().catch((err)=>{console.error(err);process.exit(1);});')"
  if [[ -z "$TAURI_DRIVER_PORT" ]]; then
    echo "Failed to resolve TAURI_DRIVER_PORT" >&2
    exit 1
  fi
  export TAURI_DRIVER_PORT
  echo "Using TAURI_DRIVER_PORT=${TAURI_DRIVER_PORT}"
fi
if command -v dbus-run-session >/dev/null 2>&1; then
  echo "Detected dbus-run-session; running E2E inside a dedicated DBus session"
  E2E_COMMAND=(dbus-run-session -- pnpm e2e:ci)
fi

set +e
"${E2E_COMMAND[@]}" 2>&1 | tee "$log_file"
status=${PIPESTATUS[0]}
set -e

if compgen -G "$OUTPUT_DIR/*" > /dev/null; then
  cp -a "$OUTPUT_DIR/." "$snapshot_dir/"
fi

echo "Desktop E2E artefacts (${SCENARIO_NAME}):"
echo "  - Logs: $log_file"
echo "  - Reports: $snapshot_dir"

exit $status
