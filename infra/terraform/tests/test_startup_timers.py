"""Run with Linux/WSL Python: python3 -m unittest discover -s infra/terraform/tests."""
from pathlib import Path
import re
import unittest

TEMPLATE = Path(__file__).resolve().parents[1] / 'modules/gcp-vm-compose/templates/startup.sh.tftpl'
DIRECTIVE = re.compile(r'%\{ (if (\w+)(?: != "")?|else|endif) ~\}\n?')
DURATION = re.compile(r'^(\d+)(min|s)?$')
ACTIVATION_TTL_SECS = 900
# cn-cli readiness の既定 --relation-max-age-secs と、terraform 変数の既定値。
RELATION_MAX_AGE_SECS = 7200
RELATION_ANALYZE_INTERVAL_MINUTES = 60
SYSTEMD = '/etc/systemd/system'


def render(template, **flags):
    """startup.sh.tftpl の条件分岐と timer 間隔の変数だけを展開する（他の `${...}` は unit では使わない）。"""
    template = template.replace('${relation_analyze_interval_minutes}', str(RELATION_ANALYZE_INTERVAL_MINUTES))
    output, stack, pos = [], [], 0
    for match in DIRECTIVE.finditer(template):
        if all(stack):
            output.append(template[pos:match.start()])
        pos = match.end()
        if match.group(2):
            stack.append(bool(flags[match.group(2)]))
        elif match.group(1) == 'else':
            stack[-1] = not stack[-1]
        else:
            stack.pop()
    assert not stack, 'unbalanced template directives'
    output.append(template[pos:])
    return ''.join(output)


def unit_file(script, path):
    match = re.search(rf'^cat > {re.escape(path)} <<UNIT\n(.*?)^UNIT$', script, re.M | re.S)
    if match is None:
        return None
    return dict(line.split('=', 1) for line in match.group(1).splitlines() if '=' in line)


def seconds(value):
    amount, unit = DURATION.match(value).groups()
    return int(amount) * (60 if unit == 'min' else 1)


def next_elapse(timer, boot, started, last_service_start, now=None):
    """systemd の monotonic timer の次回時刻（RandomizedDelaySec 加算前）。

    OnBootSec / OnActiveSec は基準時刻からの一回限りの予定で、評価時点（既定は timer
    起動時点）で過ぎていれば予定に入らない。OnUnitActiveSec は service の前回起動記録が
    無ければ予定に入らない。
    """
    now = started if now is None else now
    candidates = []
    if 'OnBootSec' in timer and boot + seconds(timer['OnBootSec']) > now:
        candidates.append(boot + seconds(timer['OnBootSec']))
    if 'OnActiveSec' in timer and started + seconds(timer['OnActiveSec']) > now:
        candidates.append(started + seconds(timer['OnActiveSec']))
    if 'OnUnitActiveSec' in timer and last_service_start is not None:
        candidates.append(last_service_start + seconds(timer['OnUnitActiveSec']))
    return min(candidates) if candidates else None


class StartupTimerTestCase(unittest.TestCase):
    UNIT = None

    def setUp(self):
        self.template = TEMPLATE.read_text(encoding='utf-8')
        self.flags = dict.fromkeys(re.findall(r'%\{ if (\w+)', self.template), True)
        self.script = render(self.template, **self.flags)
        self.timer = unit_file(self.script, f'{SYSTEMD}/{self.UNIT}.timer')

    def assert_timer_is_started_after_units_are_regenerated(self):
        lines = self.script.splitlines()
        disabled = lines.index(f'systemctl disable --now {self.UNIT}.timer 2>/dev/null || true')
        removed = lines.index(f'  {SYSTEMD}/{self.UNIT}.timer \\')
        written = lines.index(f'cat > {SYSTEMD}/{self.UNIT}.timer <<UNIT')
        reloaded = lines.index('systemctl daemon-reload')
        started = lines.index(f'systemctl enable --now {self.UNIT}.timer')
        self.assertLess(disabled, removed)
        self.assertLess(removed, written)
        self.assertLess(written, reloaded)
        self.assertLess(reloaded, started)

    def assert_units_absent_when(self, *disabled_flags):
        for disabled in disabled_flags:
            with self.subTest(disabled=disabled):
                script = render(self.template, **dict(self.flags, **{disabled: False}))
                self.assertIsNone(unit_file(script, f'{SYSTEMD}/{self.UNIT}.timer'))
                self.assertNotIn(f'systemctl enable --now {self.UNIT}.timer', script)
                # 前回構成の timer は常に片付ける。
                self.assertIn(f'systemctl disable --now {self.UNIT}.timer 2>/dev/null || true', script)


class ReadinessTimerTests(StartupTimerTestCase):
    UNIT = 'kukuri-readiness'

    def test_startup_rerun_after_boot_schedules_readiness_within_five_minutes(self):
        # startup はunitを削除・再生成するため service の前回起動記録は無い（#1097）。
        for started in (30, 2 * 60, 3 * 60, 6 * 3600):
            with self.subTest(started=started):
                first = next_elapse(self.timer, 0, started, None)
                self.assertIsNotNone(first, 'readiness timer has no next elapse')
                delay = seconds(self.timer['RandomizedDelaySec'])
                self.assertLessEqual(first + delay - started, 5 * 60)

    def test_readiness_repeats_before_activation_expires(self):
        # service が一度走った後は、次回が activation の有効期限内に決まり続ける。
        last = 6 * 3600
        following = next_elapse(self.timer, 0, 3 * 3600, last, now=last)
        self.assertIsNotNone(following)
        self.assertEqual(following, last + seconds(self.timer['OnUnitActiveSec']))
        delay = seconds(self.timer['RandomizedDelaySec'])
        self.assertLess(following + delay - last, ACTIVATION_TTL_SECS)

    def test_model_reproduces_missing_next_elapse_without_on_active_sec(self):
        legacy = {key: value for key, value in self.timer.items() if key != 'OnActiveSec'}
        self.assertIsNone(next_elapse(legacy, 0, 3 * 60, None))
        self.assertIsNotNone(next_elapse(legacy, 0, 30, None))

    def test_timer_is_started_after_units_are_regenerated(self):
        self.assert_timer_is_started_after_units_are_regenerated()

    def test_service_keeps_fail_closed_readiness_command(self):
        service = unit_file(self.script, '/etc/systemd/system/kukuri-readiness.service')
        self.assertEqual(service['Type'], 'oneshot')
        self.assertEqual(service['ExecStart'], '$COMPOSE_BIN run --rm cn-readiness')

    def test_readiness_units_require_indexer_stack_and_operator_config(self):
        self.assert_units_absent_when('deploy_indexer_stack', 'operator_config_enabled')


class RelationAnalyzeTimerTests(StartupTimerTestCase):
    UNIT = 'kukuri-relation-analyze'

    def test_startup_rerun_after_boot_schedules_relation_analysis(self):
        # startup は unit を削除・再生成するため service の前回起動記録は無い（#1099）。
        delay = seconds(self.timer['RandomizedDelaySec'])
        interval = seconds(self.timer['OnUnitActiveSec'])
        for started in (60, 15 * 60, 20 * 60, 6 * 3600):
            with self.subTest(started=started):
                first = next_elapse(self.timer, 0, started, None)
                self.assertIsNotNone(first, 'relation analyze timer has no next elapse')
                # 再実行の直前に成功していた解析からの最大間隔でも readiness の許容内に収まる。
                self.assertLess(interval + (first + delay - started), RELATION_MAX_AGE_SECS)

    def test_relation_analysis_repeats_at_configured_interval(self):
        last = 6 * 3600
        following = next_elapse(self.timer, 0, 3 * 3600, last, now=last)
        self.assertEqual(following, last + RELATION_ANALYZE_INTERVAL_MINUTES * 60)

    def test_model_reproduces_missing_next_elapse_without_on_active_sec(self):
        legacy = {key: value for key, value in self.timer.items() if key != 'OnActiveSec'}
        self.assertIsNone(next_elapse(legacy, 0, 20 * 60, None))
        self.assertIsNotNone(next_elapse(legacy, 0, 60, None))

    def test_timer_is_started_after_units_are_regenerated(self):
        self.assert_timer_is_started_after_units_are_regenerated()

    def test_service_keeps_oneshot_analysis_command(self):
        service = unit_file(self.script, f'{SYSTEMD}/{self.UNIT}.service')
        self.assertEqual(service['Type'], 'oneshot')
        self.assertEqual(service['ExecStart'], '$COMPOSE_BIN run --rm cn-relation-analyze')

    def test_relation_analyze_units_require_indexer_stack(self):
        self.assert_units_absent_when('deploy_indexer_stack')


if __name__ == '__main__':
    unittest.main()
