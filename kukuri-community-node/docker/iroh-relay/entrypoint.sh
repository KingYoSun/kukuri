#!/bin/sh
set -eu

if [ "${IROH_RELAY_DEV:-1}" = "1" ]; then
  exec /usr/local/bin/iroh-relay --dev
fi

if [ -n "${IROH_RELAY_CONFIG_PATH:-}" ]; then
  exec /usr/local/bin/iroh-relay --config-path "${IROH_RELAY_CONFIG_PATH}"
fi

exec /usr/local/bin/iroh-relay --dev
