import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("workflow_check", Path(__file__).with_name("workflow_check.py"))
workflow = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(workflow)


class WorkflowContract(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.env = dict(RUNNER_TEMP=str(self.root), GITHUB_SHA="a" * 40,
                        GITHUB_OUTPUT=str(self.root / "output"), GITHUB_STEP_SUMMARY=str(self.root / "summary"))

    def test_arguments_preserve_manual_input_and_outputs_are_fixed(self):
        reason = "release\n$(touch injected)\ntrue\n<!-- marker -->"
        report = dict(audit_required=True, current_commit="a" * 40, markdown="summary\n")
        with patch.dict(os.environ, {**self.env, "AUDIT_FORCE": "true", "AUDIT_REASON": reason}, clear=True):
            with patch.object(workflow.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, json.dumps(report).encode())) as run:
                workflow.main()
                args = run.call_args.args[0]
                self.assertEqual(args[-1], f"--reason={reason}")
                self.assertIn("--no-default-features", args)
                self.assertNotIn("shell", run.call_args.kwargs)
        self.assertEqual((self.root / "output").read_text(), "audit_required=true\n")
        self.assertEqual((self.root / "summary").read_text(), "summary\n")
        self.assertEqual(json.loads((self.root / "refactoring-audit.json").read_text()), report)

    def test_failure_never_publishes_success_output_or_artifact(self):
        with patch.dict(os.environ, self.env, clear=True):
            with patch.object(workflow.subprocess, "run", side_effect=subprocess.CalledProcessError(1, ["cargo"])):
                with self.assertRaises(subprocess.CalledProcessError):
                    workflow.main()
        self.assertEqual(list(self.root.iterdir()), [])

    def test_invalid_force_and_mismatched_commit_never_publish(self):
        with patch.dict(os.environ, {**self.env, "AUDIT_FORCE": "true\naudit_required=true"}, clear=True):
            with patch.object(workflow.subprocess, "run") as run:
                with self.assertRaises(ValueError):
                    workflow.main()
                run.assert_not_called()
        for payload in [dict(audit_required=True, current_commit="b" * 40),
                        dict(audit_required=True, current_commit="a" * 40)]:
            with patch.dict(os.environ, self.env, clear=True):
                bad = json.dumps(payload).encode()
                with patch.object(workflow.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, bad)):
                    with self.assertRaises(ValueError):
                        workflow.main()
        self.assertEqual(list(self.root.iterdir()), [])

    def test_scheduled_false_only_publishes_false_and_summary(self):
        report = dict(audit_required=False, current_commit="a" * 40, markdown="false summary")
        with patch.dict(os.environ, self.env, clear=True):
            with patch.object(workflow.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, json.dumps(report).encode())) as run:
                workflow.main()
                self.assertNotIn("--force-audit", run.call_args.args[0])
        self.assertEqual((self.root / "output").read_text(), "audit_required=false\n")


if __name__ == "__main__":
    unittest.main()
