"""Observable contracts for the read-only audit command (fixture Git history)."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import sys
import unittest

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
SPEC = importlib.util.spec_from_file_location("audit_check", HERE / "check.py")
audit = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(audit)


class AuditContract(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.git("init", "-q")
        self.git("config", "user.name", "Fixture")
        self.git("config", "user.email", "fixture@example.invalid")
        self.write("sample.rs", "field\n" * 1000)
        self.write("xtask/src/oversized.rs", "// fixture policy\n")
        self.ratchet([{"path": "sample.rs", "lines": 1100}])
        self.base = self.commit()
        self.baseline = json.loads((ROOT / "xtask/refactoring-audit-baseline.json").read_text(encoding="utf-8"))
        b = self.baseline
        b.update(baseline_commit=self.base, audit_start_commit=self.base, audit_date="2026-09-08")
        b["code_tree_oids"] = {"xtask": self.git("rev-parse", f"{self.base}:xtask").strip()}
        scope = b["scope"]
        scope["source_policy_blob"] = self.git("rev-parse", f"{self.base}:xtask/src/oversized.rs").strip()
        paths = self.git("ls-tree", "-r", "--name-only", self.base).splitlines()
        scope["tracked_candidate_count"] = len(paths)
        scope["tracked_candidate_paths_sha256"] = audit.digest(paths)
        for group in scope["groups"]:
            group.update(selector_regex="^sample\\.rs$", before_count=1, member_count=1,
                         member_paths_sha256=audit.digest(["sample.rs"]), added_paths=[], removed_paths=[])
        b["oversized"].update(count=1, ratchet_blob=self.git("rev-parse", f"{self.base}:xtask/oversized-baseline.json").strip(),
                              files=[dict(path="sample.rs", before_lines=1000, observed_lines=1000, ratchet_lines=1100)])
        b["structural_metrics"] = [dict(id="fixture_fields", before=1000, after=1000,
            before_paths=["sample.rs"], after_paths=["sample.rs"], measurement_regex="^field$",
            issues=[872], meaning="fixture measurement", measurement_scope=dict(kind="whole_file", before_symbol=None, after_symbol=None))]
        self.write("xtask/refactoring-audit-baseline.json", json.dumps(b))
        self.head = self.commit()

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root).decode("utf-8")

    def write(self, path, value):
        dest = self.root / path
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(value, encoding="utf-8", newline="\n")

    def ratchet(self, files):
        self.write("xtask/oversized-baseline.json", json.dumps(dict(files=files)))

    def commit(self):
        self.git("add", ".")
        self.git("commit", "-qm", "fixture")
        return self.git("rev-parse", "HEAD").strip()

    def check(self, **options):
        return audit.check(self.root, now="2026-09-09T00:00:00Z", **options)

    def test_equal_and_dirty_state_are_read_only_and_deterministic(self):
        self.write("sample.rs", "dirty\n")
        self.write("untracked.rs", "untracked\n")
        before = self.git("status", "--porcelain=v1")
        first = self.check()
        self.assertFalse(first["audit_required"])
        self.assertEqual(first, self.check())
        self.assertEqual(first["baseline_commit"], self.base)
        self.assertEqual(first["current_commit"], self.head)
        self.assertEqual(before, self.git("status", "--porcelain=v1"))
        self.assertEqual((self.root / "sample.rs").read_text(), "dirty\n")

    def test_new_path_and_increased_cap_trigger_any(self):
        self.ratchet([dict(path="sample.rs", lines=1101), dict(path="new.rs", lines=1000)])
        self.write("new.rs", "new\n" * 1000)
        self.commit()
        result = self.check()
        self.assertTrue(result["audit_required"])
        self.assertEqual([s["value"] for s in result["signals"]], [1, 1])
        self.assertEqual(set(result["candidate_paths"]), {"sample.rs", "new.rs"})

    def test_removal_and_shrink_do_not_trigger(self):
        self.ratchet([dict(path="sample.rs", lines=1000)])
        self.commit()
        self.assertFalse(self.check()["audit_required"])
        self.ratchet([])
        self.commit()
        self.assertFalse(self.check()["audit_required"])

    def test_force_requires_reason_and_keeps_it(self):
        with self.assertRaises(audit.AuditError):
            self.check(force=True, reason="  ")
        result = self.check(force=True, reason="release\n$(touch injected)")
        self.assertTrue(result["audit_required"])
        self.assertIn("$(touch injected)", result["markdown"])
        self.assertFalse((self.root / "injected").exists())

    def test_baseline_corruption_semantics_and_unknown_schema(self):
        for mutate in [lambda b: b.update(schema_version=99),
                       lambda b: b.update(baseline_commit="0" * 40),
                       lambda b: b["scope"].update(tracked_candidate_count=999),
                       lambda b: b["structural_metrics"][0].update(after=999),
                       lambda b: b["scope"]["groups"][0].update(id="INV-2"),
                       lambda b: b["oversized"]["files"][0].update(ratchet_lines=1001)]:
            with self.subTest(mutate=mutate):
                bad = copy.deepcopy(self.baseline)
                mutate(bad)
                self.write("xtask/refactoring-audit-baseline.json", json.dumps(bad))
                self.commit()
                with self.assertRaises(audit.AuditError):
                    self.check(force=True, reason="force cannot bypass corruption")

    def test_invalid_ratchet_and_nonancestor_fail(self):
        self.ratchet([dict(path="sample.rs", lines=1000)] * 2)
        self.commit()
        with self.assertRaises(audit.AuditError):
            self.check()
        with self.assertRaises(audit.AuditError):
            self.check(current=self.base)

    def test_each_signal_independently_triggers_and_paths_remain_data(self):
        path = "space 日本語" + (" " if os.name == "nt" else "\n") + "$(touch nope).rs"
        self.write(path, "new\n" * 1000)
        self.ratchet([dict(path="sample.rs", lines=1100), dict(path=path, lines=1000)])
        self.commit()
        report = self.check()
        self.assertEqual([s["value"] for s in report["signals"]], [1, 0])
        self.assertEqual(report["diagnostics"]["max_path_change_commits"], 1)
        self.assertIn("$(touch nope)", report["markdown"])
        self.assertTrue(report["audit_required"])
        self.ratchet([dict(path="sample.rs", lines=1101)])
        self.commit()
        self.assertEqual([s["value"] for s in self.check()["signals"]], [0, 1])

    def test_nonancestor_history_is_rejected(self):
        self.git("checkout", "--orphan", "disconnected")
        self.commit()
        with self.assertRaises(audit.AuditError):
            self.check()

    def test_reviewed_baseline_update_is_accepted_without_automatic_advancement(self):
        self.ratchet([dict(path="sample.rs", lines=1101)])
        current = self.commit()
        self.assertTrue(self.check()["audit_required"])
        b = copy.deepcopy(self.baseline)
        b["baseline_commit"] = current
        b["code_tree_oids"]["xtask"] = self.git("rev-parse", f"{current}:xtask").strip()
        paths = self.git("ls-tree", "-r", "--name-only", current).splitlines()
        b["scope"].update(tracked_candidate_count=len(paths), tracked_candidate_paths_sha256=audit.digest(paths))
        b["oversized"]["ratchet_blob"] = self.git("rev-parse", f"{current}:xtask/oversized-baseline.json").strip()
        b["oversized"]["files"][0]["ratchet_lines"] = 1101
        self.write("xtask/refactoring-audit-baseline.json", json.dumps(b))
        self.commit()
        before = (self.root / audit.BASELINE).read_bytes()
        result = self.check()
        self.assertFalse(result["audit_required"])
        self.assertEqual(result["baseline_commit"], current)
        self.assertEqual((self.root / audit.BASELINE).read_bytes(), before)

    def test_missing_git_repo_and_duplicate_json_keys_fail(self):
        with tempfile.TemporaryDirectory() as empty:
            with self.assertRaises(audit.AuditError):
                audit.check(Path(empty))
        with self.assertRaises(audit.AuditError):
            audit.json_value('{"a":1,"a":2}')

    def test_threshold_boundaries_and_invalid_negative(self):
        self.assertFalse(audit.evaluate(1, 2))
        self.assertFalse(audit.evaluate(2, 2))
        self.assertTrue(audit.evaluate(3, 2))
        with self.assertRaises(audit.AuditError):
            audit.evaluate(-1, 0)

    def test_bad_time_and_management_marker_reason(self):
        with self.assertRaises(audit.AuditError):
            audit.check(self.root, now="2026-09-09T00:00:00")
        with self.assertRaises(audit.AuditError):
            audit.check(self.root, now="2026-09-07T00:00:00Z")
        result = self.check(force=True, reason="<!-- kukuri-refactoring-audit:v1 -->")
        self.assertNotIn("<!-- kukuri-refactoring-audit:v1 -->", result["markdown"])

    def test_real_evaluator_output_drives_upsert_without_network(self):
        # Exercise the actual Python -> JSON -> Node contract, without a token.
        program = """
          import { upsert } from './scripts/refactoring-audit/upsert.mjs';
          let input = '';
          for await (const chunk of process.stdin) input += chunk;
          const writes = [];
          const api = { list: async () => [], create: async body => {
            writes.push(body); return { number: 1 }; }, update: async () => { throw Error('unexpected update'); } };
          const outcome = await upsert(JSON.parse(input), api);
          process.stdout.write(JSON.stringify({outcome, writes}));
        """
        for force in [False, True]:
            report = self.check(force=force, reason="release\n$(touch nope)" if force else "")
            result = subprocess.run(["node", "--input-type=module", "-e", program], cwd=ROOT,
                                    input=json.dumps(report), capture_output=True, text=True, encoding="utf-8", check=True)
            payload = json.loads(result.stdout)
            self.assertEqual(len(payload["writes"]), int(force))
            self.assertEqual(payload["outcome"]["action"], "created" if force else "none")

    def test_cli_exit_and_json(self):
        for args, code in [([], 0), (["--force-audit", "--reason", "release"], 0),
                           (["--force-audit"], 1), (["--current", "0" * 40], 1)]:
            entry = [os.environ["KUKURI_AUDIT_XTASK"], "refactoring-audit-check"] if os.environ.get("KUKURI_AUDIT_XTASK") else [sys.executable, str(HERE / "check.py")]
            result = subprocess.run([*entry, "--repo", str(self.root),
                                     "--now", "2026-09-09T00:00:00Z", "--format", "json", *args], capture_output=True, text=True, encoding="utf-8")
            self.assertEqual(result.returncode, code, result.stderr)
            if code == 0:
                self.assertIsInstance(json.loads(result.stdout)["audit_required"], bool)
            else:
                self.assertEqual(result.stdout, "")


if __name__ == "__main__":
    unittest.main()
