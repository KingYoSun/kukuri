#!/bin/bash
# 関係解析の最終成功からの経過秒数（relation_last_success_age_seconds）を出力する。
# Terraform が monitor.sh へ埋め込む。fixture test から source してもよい。
#
# oneshot service の実行中、boot 後、startup による unit 再生成後、timer 停止後（参照を失った
# service は unload される）は、systemd に ExecMainExitTimestamp が無い。最後に観測した結果を
# state file に残し、初回実行前の期間を停止と区別する（#1102）。
# - 終了して成功: その終了からの経過（従来どおり）。
# - 終了して失敗: 以後に成功を観測するまで不明（1e9）。
# - 記録なし: 最後に観測した成功、boot 後の初回予定、有効な timer の起動のうち最も新しい時刻から
#   の経過。timer が止まっていれば最後の成功から経過する。
RELATION_ANALYZE_UNIT=kukuri-relation-analyze
# startup.sh の kukuri-relation-analyze.timer の OnBootSec と RandomizedDelaySec。
RELATION_ANALYZE_BOOT_SECS=900
RELATION_ANALYZE_DELAY_SECS=60
RELATION_AGE_UNKNOWN=1000000000

relation_now() { date +%s; }
relation_uptime() { cut -d. -f1 /proc/uptime; }

relation_show() {
  systemctl show "$RELATION_ANALYZE_UNIT.$1" -p "$2" --value 2>/dev/null || true
}

relation_epoch() {
  # 記録なしを systemd 249 は n/a、新しい版は空で返す。空は `date -d ''` が当日 0 時と解釈する。
  case "$1" in '' | n/a) return 1 ;; esac
  date -d "$1" +%s 2>/dev/null
}

relation_last_success_age() {
  local state_file="$1" now exit_epoch status last_success="" last_result="" ref timer_epoch
  now="$(relation_now)"
  if [ -r "$state_file" ]; then
    read -r last_success last_result < "$state_file" 2>/dev/null || true
  fi
  [[ "$last_success" =~ ^[0-9]+$ ]] || last_success=0
  if exit_epoch="$(relation_epoch "$(relation_show service ExecMainExitTimestamp)")"; then
    status="$(relation_show service ExecMainStatus)"
    if [ "$status" = 0 ]; then
      last_success="$exit_epoch"
      last_result=ok
    else
      last_result=failed
    fi
    # state file を書けなくても metric は送る。
    { printf '%s %s\n' "$last_success" "$last_result" > "$state_file.tmp" \
      && mv -f "$state_file.tmp" "$state_file"; } 2>/dev/null || true
    if [ "$last_result" = ok ]; then
      echo "$(( now - exit_epoch ))"
    else
      echo "$RELATION_AGE_UNKNOWN"
    fi
    return 0
  fi
  if [ "$last_result" = failed ]; then
    echo "$RELATION_AGE_UNKNOWN"
    return 0
  fi
  ref="$(( now - $(relation_uptime) + RELATION_ANALYZE_BOOT_SECS + RELATION_ANALYZE_DELAY_SECS ))"
  [ "$last_success" -gt "$ref" ] && ref="$last_success"
  if [ "$(relation_show timer ActiveState)" = active ] \
    && timer_epoch="$(relation_epoch "$(relation_show timer ActiveEnterTimestamp)")" \
    && [ "$(( timer_epoch + RELATION_ANALYZE_DELAY_SECS ))" -gt "$ref" ]; then
    ref="$(( timer_epoch + RELATION_ANALYZE_DELAY_SECS ))"
  fi
  if [ "$now" -gt "$ref" ]; then echo "$(( now - ref ))"; else echo 0; fi
}
