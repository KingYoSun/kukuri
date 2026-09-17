"""Run with Linux/WSL Python: python3 -m unittest discover -s infra/terraform/tests."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
REPOSITORY = ROOT.parents[1]
TEMPLATE = ROOT / 'modules/gcp-vm-compose/templates/startup.sh.tftpl'
INTERVAL_VARIABLE_FILES = (
    ROOT / 'modules/gcp-vm-compose/variables.tf',
    ROOT / 'envs/low-cost/variables.tf',
)
CN_CLI_MAIN = REPOSITORY / 'crates/cn-cli/src/main.rs'
CN_OPERATOR_CONFIG = REPOSITORY / 'crates/cn-operator/src/config.rs'
DIRECTIVE = re.compile(r'%\{ (if (\w+)(?: != "")?|else|endif) ~\}\n?')
DURATION = re.compile(r'^(\d+)(min|s)?$')
ACTIVATION_TTL_SECS = 900
# terraform 変数の既定値。
RELATION_ANALYZE_INTERVAL_MINUTES = 60
# 上限の間隔でも残す、解析の実行時間と boot にかかる時間の余裕（#1101）。
RELATION_ANALYSIS_MARGIN_SECS = 14 * 60
SYSTEMD = '/etc/systemd/system'


def render(template, interval_minutes=RELATION_ANALYZE_INTERVAL_MINUTES, **flags):
    """startup.sh.tftpl の条件分岐と timer 間隔の変数だけを展開する（他の `${...}` は unit では使わない）。"""
    template = template.replace('${relation_analyze_interval_minutes}', str(interval_minutes))
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


def relation_max_age_secs():
    """readiness service が使う cn-cli readiness の既定 --relation-max-age-secs。"""
    source = CN_CLI_MAIN.read_text(encoding='utf-8')
    match = re.search(r'default_value_t = (\d+)\)\]\s*relation_max_age_secs: i64', source)
    assert match, 'relation_max_age_secs default not found'
    return int(match.group(1))


def interval_bounds(path):
    """terraform 変数 relation_analyze_interval_minutes の validation が許す範囲。"""
    source = path.read_text(encoding='utf-8')
    block = re.search(r'variable "relation_analyze_interval_minutes" \{(.*?)\n\}', source, re.S)
    assert block, f'variable not found in {path}'
    match = re.search(r'var\.relation_analyze_interval_minutes >= (\d+) && '
                      r'var\.relation_analyze_interval_minutes <= (\d+)', block.group(1))
    assert match, f'interval bounds not validated in {path}'
    return int(match.group(1)), int(match.group(2))


def unit_file(script, path):
    match = re.search(rf'^cat > {re.escape(path)} <<UNIT\n(.*?)^UNIT$', script, re.M | re.S)
    if match is None:
        return None
    return dict(line.split('=', 1) for line in match.group(1).splitlines() if '=' in line)


def seconds(value):
    amount, unit = DURATION.match(value).groups()
    return int(amount) * (60 if unit == 'min' else 1)


def rerun_last_trigger(timer, previous_trigger):
    """startup 再実行で再生成した timer が起動時に持つ前回 trigger 時刻。

    unit を削除・再生成するため in-memory の記録は消える。Persistent=true の場合だけ
    stamp file から前回の trigger 時刻が戻る。
    """
    return previous_trigger if timer.get('Persistent') == 'true' else None


def next_elapse(timer, boot, started, last_service_start, now=None, last_trigger=None):
    """systemd の monotonic timer の次回時刻（RandomizedDelaySec 加算前）。

    OnBootSec / OnActiveSec は基準時刻からの一回限りの予定。評価時点（既定は timer 起動時点）で
    まだ来ていなければその時刻になる。過ぎていれば、trigger 記録（Persistent の stamp を含み、
    時刻は問わない）があれば予定に入らず、無ければ即時に実行される（systemd 249 で確認）。
    OnUnitActiveSec は service の前回起動記録が無ければ予定に入らない。
    """
    now = started if now is None else now
    candidates = []
    for key, base in (('OnBootSec', boot), ('OnActiveSec', started)):
        if key not in timer:
            continue
        elapse = base + seconds(timer[key])
        if elapse > now:
            candidates.append(elapse)
        elif last_trigger is None:
            candidates.append(now)
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
                last_trigger = rerun_last_trigger(self.timer, started - 1)
                first = next_elapse(self.timer, 0, started, None, last_trigger=last_trigger)
                self.assertIsNotNone(first, 'readiness timer has no next elapse')
                delay = seconds(self.timer['RandomizedDelaySec'])
                self.assertLessEqual(first + delay - started, 5 * 60)

    def test_readiness_repeats_before_activation_expires(self):
        # service が一度走った後は、次回が activation の有効期限内に決まり続ける。
        last = 6 * 3600
        following = next_elapse(self.timer, 0, 3 * 3600, last, now=last, last_trigger=last)
        self.assertIsNotNone(following)
        self.assertEqual(following, last + seconds(self.timer['OnUnitActiveSec']))
        delay = seconds(self.timer['RandomizedDelaySec'])
        self.assertLess(following + delay - last, ACTIVATION_TTL_SECS)

    def test_model_reproduces_missing_next_elapse_without_on_active_sec(self):
        # 本番（2026-09-17）の NEXT=- は、Persistent の stamp で OnBootSec が実行済み扱いになったもの。
        legacy = {key: value for key, value in self.timer.items() if key != 'OnActiveSec'}
        self.assertEqual(legacy.get('Persistent'), 'true')
        self.assertIsNone(next_elapse(legacy, 0, 3 * 60, None, last_trigger=3 * 60 - 1))
        self.assertIsNotNone(next_elapse(legacy, 0, 30, None, last_trigger=29))

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
                last_trigger = rerun_last_trigger(self.timer, started - 1)
                first = next_elapse(self.timer, 0, started, None, last_trigger=last_trigger)
                self.assertIsNotNone(first, 'relation analyze timer has no next elapse')
                # 再実行の直前に成功していた解析からの最大間隔でも readiness の許容内に収まる。
                self.assertLess(interval + (first + delay - started), relation_max_age_secs())

    def test_interval_upper_bound_keeps_relation_analysis_recent(self):
        # 上限の間隔でも、boot / startup 再実行の直後に初回の解析が終わるまでに
        # readiness の許容時間を超えない（#1101）。
        bounds = {interval_bounds(path) for path in INTERVAL_VARIABLE_FILES}
        self.assertEqual(len(bounds), 1, f'module and env bounds differ: {bounds}')
        (_, upper), = bounds
        timer = unit_file(render(self.template, interval_minutes=upper, **self.flags),
                          f'{SYSTEMD}/{self.UNIT}.timer')
        delay = seconds(timer['RandomizedDelaySec'])
        interval = seconds(timer['OnUnitActiveSec'])
        self.assertEqual(interval, upper * 60)
        # 定常状態: 前回の開始から interval + 遅延後に次の解析が始まる。
        # boot 直後: 前回の成功から最大 interval 後に停止し、boot の OnBootSec + 遅延後に始まる。
        worst_start = max(interval + delay, interval + next_elapse(timer, 0, 30, None) + delay)
        self.assertLessEqual(worst_start + RELATION_ANALYSIS_MARGIN_SECS, relation_max_age_secs())
        # 上限を 1 分超えると、同じ余裕を残せない。
        self.assertGreater(worst_start + 60 + RELATION_ANALYSIS_MARGIN_SECS, relation_max_age_secs())

    def test_operator_config_uses_same_interval_bounds(self):
        (lower, upper), = {interval_bounds(path) for path in INTERVAL_VARIABLE_FILES}
        source = CN_OPERATOR_CONFIG.read_text(encoding='utf-8')
        match = re.search(r'const RELATION_ANALYZE_INTERVAL_MINUTES_RANGE: RangeInclusive<u32> = (\d+)\.\.=(\d+);', source)
        self.assertIsNotNone(match, 'cn-operator must define the interval range')
        self.assertEqual((int(match.group(1)), int(match.group(2))), (lower, upper))

    def test_startup_rerun_after_boot_window_runs_analysis_immediately(self):
        # Persistent が無いため trigger 記録は戻らず、過ぎた OnBootSec は即時に実行される。
        self.assertNotIn('Persistent', self.timer)
        self.assertEqual(next_elapse(self.timer, 0, 20 * 60, None), 20 * 60)
        self.assertEqual(next_elapse(self.timer, 0, 60, None), 15 * 60)

    def test_relation_analysis_repeats_at_configured_interval(self):
        last = 6 * 3600
        following = next_elapse(self.timer, 0, 3 * 3600, last, now=last, last_trigger=last)
        self.assertEqual(following, last + RELATION_ANALYZE_INTERVAL_MINUTES * 60)

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
