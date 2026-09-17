"""Run with Linux/WSL Python: python3 -m unittest discover -s infra/terraform/tests."""
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time
import unittest

from test_startup_timers import RELATION_ANALYZE_INTERVAL_MINUTES, SYSTEMD, TEMPLATE, render, seconds, unit_file

ROOT = Path(__file__).resolve().parents[1]
HELPER = ROOT / 'modules/gcp-vm-compose/scripts/relation-age.sh'
MONITOR = ROOT / 'modules/gcp-vm-compose/templates/monitor.sh.tftpl'
MODULE = ROOT / 'modules/gcp-vm-compose/main.tf'
UNKNOWN = 1000000000
# envs/low-cost/main.tf の alert `relation` の閾値（既定の間隔）。
THRESHOLD = RELATION_ANALYZE_INTERVAL_MINUTES * 180
BOOT = 1_789_600_000
# boot 後、startup が timer を起動するまでの時間。
TIMER_STARTED = BOOT + 90
FAR_PAST = BOOT - 10 * 3600

SCRIPT = '''
set -euo pipefail
systemctl() {
  [ "$1" = show ] && [ "$3" = -p ] && [ "$5" = --value ] || return 99
  case "$2:$4" in
    kukuri-relation-analyze.service:ExecMainExitTimestamp) printf '%s\\n' "$MOCK_EXIT" ;;
    kukuri-relation-analyze.service:ExecMainStatus) printf '%s\\n' "$MOCK_STATUS" ;;
    kukuri-relation-analyze.timer:ActiveState) printf '%s\\n' "$MOCK_TIMER_STATE" ;;
    kukuri-relation-analyze.timer:ActiveEnterTimestamp) printf '%s\\n' "$MOCK_TIMER_ENTER" ;;
    *) return 99 ;;
  esac
}
source "$1"
relation_now() { echo "$MOCK_NOW"; }
relation_uptime() { echo "$MOCK_UPTIME"; }
relation_last_success_age "$2"
'''


def systemd_time(epoch):
    """systemctl show が返す時刻の形式（VM の timezone は UTC）。"""
    return time.strftime('%a %Y-%m-%d %H:%M:%S UTC', time.gmtime(epoch))


class RelationAgeTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.state = Path(directory.name) / '.monitor-relation-analyze'

    def age(self, now, exit=None, status='0', timer='active', timer_enter=TIMER_STARTED,
            boot=BOOT, missing='n/a'):
        """monitor の 1 回の実行。exit=None は systemd に実行記録が無い状態（未実行・実行中）。"""
        env = dict(os.environ, TZ='UTC', MOCK_NOW=str(now), MOCK_UPTIME=str(now - boot),
                   MOCK_EXIT=missing if exit is None else systemd_time(exit), MOCK_STATUS=status,
                   MOCK_TIMER_STATE=timer,
                   MOCK_TIMER_ENTER='n/a' if timer_enter is None else systemd_time(timer_enter))
        output = subprocess.run(['bash', '-c', SCRIPT, 'test', str(HELPER), str(self.state)],
                                env=env, text=True, capture_output=True, check=True)
        self.assertEqual(output.stderr, '')
        return int(output.stdout)

    def assert_no_alert(self, *ages):
        for age in ages:
            self.assertLessEqual(age, THRESHOLD)

    def succeed_at(self, exit):
        self.assertEqual(self.age(exit + 5, exit=exit), 5)

    def test_boot_before_first_analysis_is_not_stale(self):
        # TR-1: 監視は boot の 5 分後から 5 分ごと、解析の初回は boot の 15 分後 + 遅延 + 実行時間。
        for state in (None, FAR_PAST):
            with self.subTest(previous_success=state):
                if state is not None:
                    self.state.write_text(f'{state} ok\n')
                ages = [self.age(BOOT + minutes * 60) for minutes in (5, 10, 15, 20)]
                self.assertEqual(ages[:3], [0, 0, 0])
                # 初回が boot の 16 分後に始まり、20 分後も実行中なら、その実行時間だけ経過する。
                self.assertEqual(ages[3], 4 * 60)
                self.assert_no_alert(*ages)

    def test_missing_timestamp_is_not_parsed_as_midnight(self):
        # 新しい systemd は記録なしを空で返す。`date -d ''` は当日 0 時になる。
        self.assertEqual(self.age(BOOT + 600, missing=''), 0)
        self.assertEqual(self.age(BOOT + 6 * 3600, missing='', timer='inactive'),
                         6 * 3600 - 960)

    def test_startup_rerun_while_analysis_runs_is_not_stale(self):
        # TR-2: startup は timer を止めて unit を再生成し、起動直後に解析を実行する（#1099）。
        rerun = BOOT + 6 * 3600
        for state in (None, rerun - 50 * 60):
            with self.subTest(previous_success=state):
                if state is not None:
                    self.state.write_text(f'{state} ok\n')
                # timer 停止中（image pull 等）: 前回の成功から経過する。
                stopped = self.age(rerun + 120, timer='inactive', timer_enter=None)
                if state is not None:
                    self.assertEqual(stopped, 52 * 60)
                # timer 起動後、初回の解析が終わるまで。
                ages = [self.age(rerun + 300 + offset, timer_enter=rerun + 300)
                        for offset in (0, 60, 300)]
                self.assertEqual(ages, [0, 0, 240])
                if state is not None:
                    self.assert_no_alert(stopped, *ages)

    def test_steady_state_running_uses_last_success(self):
        # TR-2: 定常運転中も実行中は記録が無い。前回の成功から経過する。
        self.succeed_at(BOOT + 3600)
        self.assertEqual(self.age(BOOT + 2 * 3600 + 30), 3600 + 30)

    def test_success_reports_age_since_exit(self):
        # TR-3: 従来どおり。
        exit = BOOT + 3600
        self.assertEqual(self.age(exit + 120, exit=exit), 120)
        self.assertEqual(self.state.read_text(), f'{exit} ok\n')

    def test_failure_is_unknown_until_next_success(self):
        # TR-4: 失敗は従来どおり即時に 1e9。記録が消えても（実行中・startup 再実行・boot）維持する。
        self.succeed_at(BOOT + 3600)
        failed = BOOT + 2 * 3600
        self.assertEqual(self.age(failed + 60, exit=failed, status='1'), UNKNOWN)
        self.assertEqual(self.state.read_text(), f'{BOOT + 3600} failed\n')
        rerun = failed + 1800
        reboot = failed + 3600
        for age in (self.age(failed + 3600 + 30),
                    self.age(rerun + 30, timer='inactive', timer_enter=None),
                    self.age(rerun + 120, timer_enter=rerun + 60),
                    self.age(reboot + 300, boot=reboot, timer_enter=reboot + 90)):
            self.assertEqual(age, UNKNOWN)
        self.succeed_at(reboot + 1000)

    def test_stopped_timer_alerts_after_threshold(self):
        # TR-4: timer が止まると service は unload されて記録が消える。最後の成功から経過する。
        last = BOOT + 3600
        self.succeed_at(last)
        self.assertLessEqual(self.age(last + THRESHOLD, timer='inactive', timer_enter=None), THRESHOLD)
        self.assertGreater(self.age(last + THRESHOLD + 1, timer='inactive', timer_enter=None), THRESHOLD)

    def test_never_succeeding_after_boot_alerts(self):
        # TR-5: 初回の予定（boot の 16 分後）から閾値を超えると発報する。timer が起動しない場合も同じ。
        first = BOOT + 16 * 60
        for timer, timer_enter in (('active', TIMER_STARTED), ('inactive', None)):
            with self.subTest(timer=timer):
                self.assertEqual(self.age(first + THRESHOLD, timer=timer, timer_enter=timer_enter), THRESHOLD)
                self.assertGreater(self.age(first + THRESHOLD + 1, timer=timer, timer_enter=timer_enter),
                                   THRESHOLD)

    def test_hung_first_analysis_alerts(self):
        # startup 再実行直後の解析が終わらない場合も、timer の起動から閾値を超えると発報する。
        rerun = BOOT + 6 * 3600
        self.assertGreater(self.age(rerun + 60 + THRESHOLD + 1, timer_enter=rerun), THRESHOLD)

    def test_corrupt_or_unwritable_state_is_ignored(self):
        for content in ('garbage failed-ish\n', '\n', '123abc ok\n'):
            with self.subTest(content=content):
                self.state.write_text(content)
                self.assertEqual(self.age(BOOT + 600), 0)
        self.state.unlink()
        self.state.mkdir()
        self.assertEqual(self.age(BOOT + 600), 0)
        self.state.rmdir()
        self.state = self.state / 'missing' / 'state'
        self.assertEqual(self.age(BOOT + 3700, exit=BOOT + 3600), 100)
        self.assertFalse(self.state.parent.exists())


class RelationAgeWiringTests(unittest.TestCase):
    def test_helper_constants_match_relation_timer(self):
        template = TEMPLATE.read_text(encoding='utf-8')
        flags = dict.fromkeys(re.findall(r'%\{ if (\w+)', template), True)
        timer = unit_file(render(template, **flags), f'{SYSTEMD}/kukuri-relation-analyze.timer')
        helper = HELPER.read_text(encoding='utf-8')
        constants = dict(re.findall(r'^(RELATION_\w+)=(\S+)$', helper, re.M))
        self.assertEqual(int(constants['RELATION_ANALYZE_BOOT_SECS']), seconds(timer['OnBootSec']))
        self.assertEqual(int(constants['RELATION_ANALYZE_DELAY_SECS']), seconds(timer['RandomizedDelaySec']))
        self.assertEqual(constants['RELATION_AGE_UNKNOWN'], str(UNKNOWN))

    def test_monitor_embeds_helper_for_relation_metric(self):
        monitor = MONITOR.read_text(encoding='utf-8')
        indexer_block = monitor[monitor.index('%{ if deploy_indexer_stack ~}'):monitor.index('%{ endif ~}')]
        self.assertIn('${relation_age_helpers}\n', indexer_block)
        self.assertIn('relation_age="$(relation_last_success_age "$INSTALL_DIR/.monitor-relation-analyze")"\n'
                      'write_metric relation_last_success_age_seconds "$relation_age"\n', indexer_block)
        self.assertEqual(monitor.count('ExecMainExitTimestamp'), 0)
        self.assertIn('relation_age_helpers  = file("${path.module}/scripts/relation-age.sh")',
                      MODULE.read_text(encoding='utf-8'))


if __name__ == '__main__':
    unittest.main()
