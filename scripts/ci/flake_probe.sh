#!/usr/bin/env bash
# #1121: 同じ suite を繰り返し実行し、失敗回数を数える。
#
# usage: KUKURI_PROBE_REPEATS=10 KUKURI_PROBE_LABEL="rust (shard 1)" \
#          scripts/ci/flake_probe.sh <command> [args...]
#
# 失敗しても最後まで回し、1 回でも失敗したら終了コード 1 で終わる。各回の所要秒と結果、
# 失敗した回の番号を GitHub の step summary（無い場合は標準出力）へ書く。
set -uo pipefail

repeats="${KUKURI_PROBE_REPEATS:-10}"
label="${KUKURI_PROBE_LABEL:-probe}"

if ! [[ "$repeats" =~ ^[0-9]+$ ]] || [ "$repeats" -lt 1 ]; then
  echo "::error::KUKURI_PROBE_REPEATS must be a positive integer (got '${repeats}')"
  exit 2
fi
if [ "$#" -eq 0 ]; then
  echo "::error::no command given to flake_probe.sh"
  exit 2
fi

failures=0
failed_iterations=()
durations=()

for i in $(seq 1 "$repeats"); do
  echo "::group::${label} iteration ${i}/${repeats}: $*"
  start=$(date +%s)
  "$@"
  status=$?
  elapsed=$(( $(date +%s) - start ))
  echo "::endgroup::"
  durations+=("$elapsed")
  if [ "$status" -ne 0 ]; then
    failures=$((failures + 1))
    failed_iterations+=("$i")
    echo "::warning::${label} iteration ${i} failed with exit code ${status} after ${elapsed}s"
  else
    echo "${label} iteration ${i} passed in ${elapsed}s"
  fi
done

summary="### ${label}

- 実行: ${repeats} 回
- 失敗: ${failures} 回${failed_iterations:+（回: ${failed_iterations[*]}）}
- 各回の所要秒: ${durations[*]}
"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  printf '%s\n' "$summary" >> "$GITHUB_STEP_SUMMARY"
fi
printf '%s\n' "$summary"

[ "$failures" -eq 0 ]
