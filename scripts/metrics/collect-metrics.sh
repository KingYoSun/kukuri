#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

usage() {
  cat <<'USAGE'
Usage: scripts/metrics/collect-metrics.sh [--output <path>]

Collects code quality metrics (TODO / any / allow(dead_code) counts) using ripgrep.
Outputs a JSON summary to stdout or writes to the provided path.
USAGE
}

OUTPUT_PATH=""

while (($#)); do
  case "$1" in
    -o|--output)
      if (($# < 2)); then
        usage
        exit 1
      fi
      OUTPUT_PATH="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if ! command -v rg >/dev/null 2>&1; then
  echo "Error: ripgrep (rg) is required but not found in PATH." >&2
  exit 127
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "Error: jq is required but not found in PATH." >&2
  exit 127
fi

collect_rg_count() {
  local pattern="$1"
  shift

  local -a options=()
  local -a paths=()
  local parse_paths=0

  for arg in "$@"; do
    if [[ "$arg" == "--" ]]; then
      parse_paths=1
      continue
    fi

    if (( parse_paths )); then
      paths+=("$arg")
    else
      options+=("$arg")
    fi
  done

  local -a cmd=(rg --json --no-heading)
  cmd+=("${options[@]}")
  cmd+=("$pattern")
  if ((${#paths[@]} > 0)); then
    cmd+=("${paths[@]}")
  fi

  local output=""
  local status=0
  if ! output="$("${cmd[@]}" 2>/dev/null)"; then
    status=$?
    if (( status > 1 )); then
      echo "ripgrep failed (exit code ${status}) for pattern '${pattern}'" >&2
      exit $status
    fi
  fi

  if [[ -z "$output" ]]; then
    echo 0
    return
  fi

  printf '%s\n' "$output" | jq -s 'map(select(.type=="match")) | length'
}

pushd "$REPO_ROOT" >/dev/null

typescript_paths=("kukuri-tauri/src")
rust_paths=("kukuri-tauri/src-tauri")

ts_todo=$(collect_rg_count 'TODO' -g '*.ts' -g '*.tsx' -- "${typescript_paths[@]}")
ts_any=$(collect_rg_count '\bany\b' --pcre2 -g '*.ts' -g '*.tsx' -- "${typescript_paths[@]}")
rust_todo=$(collect_rg_count 'TODO' -g '*.rs' -- "${rust_paths[@]}")
rust_allow_dead_code=$(collect_rg_count '#\[allow\(dead_code\)\]' -g '*.rs' -- "${rust_paths[@]}")

timestamp="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

summary=$(jq -n \
  --arg timestamp "$timestamp" \
  --argjson ts_todo "${ts_todo:-0}" \
  --argjson ts_any "${ts_any:-0}" \
  --argjson rust_todo "${rust_todo:-0}" \
  --argjson rust_allow_dead_code "${rust_allow_dead_code:-0}" \
  '{
    timestamp: $timestamp,
    typescript: {
      todo: $ts_todo,
      any: $ts_any
    },
    rust: {
      todo: $rust_todo,
      allow_dead_code: $rust_allow_dead_code
    }
  }')

if [[ -z "$OUTPUT_PATH" ]]; then
  printf '%s\n' "$summary"
else
  mkdir -p "$(dirname "$OUTPUT_PATH")"
  printf '%s\n' "$summary" > "$OUTPUT_PATH"
  echo "Metrics written to $OUTPUT_PATH"
fi

popd >/dev/null
