"""Run with Linux/WSL Python: python3 -m unittest discover -s infra/terraform/tests."""
from pathlib import Path
import re
import unittest

TEMPLATE = Path(__file__).resolve().parents[1] / 'modules/gcp-vm-compose/templates/startup.sh.tftpl'
DIRECTIVE = re.compile(r'%\{ (if (\w+)(?: != "")?|else|endif) ~\}\n?')
DURATION = re.compile(r'^(\d+)(min|s)?$')
ACTIVATION_TTL_SECS = 900


def render(template, **flags):
    """startup.sh.tftpl の条件分岐だけを展開する（`${...}` は readiness unit では使わない）。"""
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


class ReadinessTimerTests(unittest.TestCase):
    def setUp(self):
        self.template = TEMPLATE.read_text(encoding='utf-8')
        self.script = render(self.template, **dict.fromkeys(
            re.findall(r'%\{ if (\w+)', self.template), True))
        self.timer = unit_file(self.script, '/etc/systemd/system/kukuri-readiness.timer')

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
        lines = self.script.splitlines()
        removed = lines.index('  /etc/systemd/system/kukuri-readiness.timer \\')
        written = lines.index('cat > /etc/systemd/system/kukuri-readiness.timer <<UNIT')
        reloaded = lines.index('systemctl daemon-reload')
        started = lines.index('systemctl enable --now kukuri-readiness.timer')
        self.assertLess(lines.index('systemctl disable --now kukuri-readiness.timer 2>/dev/null || true'), removed)
        self.assertLess(removed, written)
        self.assertLess(written, reloaded)
        self.assertLess(reloaded, started)

    def test_service_keeps_fail_closed_readiness_command(self):
        service = unit_file(self.script, '/etc/systemd/system/kukuri-readiness.service')
        self.assertEqual(service['Type'], 'oneshot')
        self.assertEqual(service['ExecStart'], '$COMPOSE_BIN run --rm cn-readiness')

    def test_readiness_units_require_indexer_stack_and_operator_config(self):
        flags = dict.fromkeys(re.findall(r'%\{ if (\w+)', self.template), True)
        for disabled in ('deploy_indexer_stack', 'operator_config_enabled'):
            with self.subTest(disabled=disabled):
                script = render(self.template, **dict(flags, **{disabled: False}))
                self.assertIsNone(unit_file(script, '/etc/systemd/system/kukuri-readiness.timer'))
                self.assertNotIn('systemctl enable --now kukuri-readiness.timer', script)
                # 前回構成の timer は常に片付ける。
                self.assertIn('systemctl disable --now kukuri-readiness.timer 2>/dev/null || true', script)


if __name__ == '__main__':
    unittest.main()
