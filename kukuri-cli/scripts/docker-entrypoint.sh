#!/bin/sh
set -eu

LOG_LEVEL="${LOG_LEVEL:-info}"

exec_cli() {
    set -- --log-level "$LOG_LEVEL" "$@"
    if [ -n "${JSON_LOGS:-}" ]; then
        set -- "$@" --json-logs
    fi
    exec kukuri-cli "$@"
}

COMMAND="${1:-bootstrap}"
if [ "$#" -gt 0 ]; then
    shift
fi

case "$COMMAND" in
    bootstrap)
        exec_cli bootstrap "$@"
        ;;
    relay)
        if [ -n "${RELAY_TOPICS:-}" ]; then
            set -- --topics "$RELAY_TOPICS" "$@"
        fi
        exec_cli relay "$@"
        ;;
    connect)
        PEER="${CONNECT_PEER:-${NODE_A_ADDR:-}}"
        if [ -z "$PEER" ]; then
            echo "Error: CONNECT_PEER or NODE_A_ADDR must be set" >&2
            exit 1
        fi
        if [ -n "${CONNECT_ARGS:-}" ]; then
            set -f
            # shellcheck disable=SC2086
            set -- ${CONNECT_ARGS} "$@"
            set +f
        fi
        exec_cli connect --peer "$PEER" "$@"
        ;;
    *)
        exec_cli "$COMMAND" "$@"
        ;;
esac
