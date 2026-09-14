#!/bin/bash
# Read-only index availability probes. Output is restricted to metric names/numbers.
# Embedded into monitor.sh by Terraform; can also be sourced for fixture tests.
index_health_metrics() {
  local postgres="$1" indexer="$2"
  shift 2
  local topic ids="" row="" present=0 entries=-1 logs failures=-1
  if [ "$#" -gt 0 ]; then
    for topic in "$@"; do
      # Values enter an SQL literal. Reject quotes, whitespace and shell syntax.
      [[ "$topic" =~ ^[a-zA-Z0-9:_-]{1,200}$ ]] || return 2
      ids+="${ids:+,}'$topic'"
    done
    if [ -n "$postgres" ]; then
      row="$(docker exec "$postgres" sh -c 'PGOPTIONS="-c default_transaction_read_only=on -c statement_timeout=10000" psql -X -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -v ON_ERROR_STOP=1 -c "$1"' sh \
        "SELECT (SELECT count(*) FROM cn_index.supported_topics WHERE kind='public_topic' AND id IN ($ids)), (SELECT count(*) FROM cn_index.index_entries WHERE scope_kind='public_topic' AND scope_id IN ($ids));" 2>/dev/null)" || row=""
    fi
    if [[ "$row" =~ ^([0-9]+)\|([0-9]+)$ ]]; then
      [ "${BASH_REMATCH[1]}" -eq "$#" ] && present=1
      entries="${BASH_REMATCH[2]}"
    fi
    printf 'index_expected_topics_present %s\nindex_expected_topics_entries %s\n' "$present" "$entries"
  fi
  # A rolling window also works with the deployed indexer before dedicated counters
  # exist. Missing logs are unknown (-1), never a successful zero. Never emit raw logs.
  if [ -n "$indexer" ] && logs="$(docker logs --since 10m "$indexer" 2>&1)"; then
    failures="$(printf '%s' "$logs" | grep -cF 'failed to resolve post body; not indexing the post (fail-closed)' || true)"
  fi
  printf 'body_fetch_failures_recent %s\n' "$failures"
}
