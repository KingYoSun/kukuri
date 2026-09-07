"""Check the actual workflow DAG and secret/source boundaries; no GitHub writes."""
import pathlib
import shlex
import unittest

import yaml

ROOT = pathlib.Path(__file__).resolve().parents[2]


def workflow(name):
    # GitHub uses YAML 1.2; BaseLoader also preserves the literal `on` key.
    return yaml.load((ROOT / ".github/workflows" / name).read_text(encoding="utf-8"), Loader=yaml.BaseLoader)


class WorkflowTests(unittest.TestCase):
    def test_linux_refreshes_preinstalled_libgcrypt_before_bundling(self):
        steps = workflow("kukuri-linux-package.yml")["jobs"]["linux-appimage"]["steps"]
        dependencies = next(step for step in steps if step.get("name") == "Linux build dependencies")
        bundle = next(step for step in steps if step.get("name") == "Build and verify AppImage and Deb")
        self.assertLess(steps.index(dependencies), steps.index(bundle))
        commands = dependencies["run"].replace("\\\n", " ").splitlines()
        installs = [shlex.split(command) for command in commands if command.strip().startswith("sudo apt-get install ")]
        self.assertTrue(any("libgcrypt20" in command and "--no-upgrade" not in command for command in installs),
                        "refresh the runner's preinstalled libgcrypt20 before collecting exact matching source")

    def test_publish_requires_every_platform_and_validation(self):
        jobs = workflow("kukuri-release.yml")["jobs"]

        def ancestors(name):
            needs = jobs[name].get("needs", [])
            if isinstance(needs, str): needs = [needs]
            return set(needs).union(*(ancestors(parent) for parent in needs))

        self.assertTrue({"validate-release-inputs", "linux-verify", "windows-package", "linux-package",
                         "cli-package", "release-assets"}.issubset(ancestors("publish-draft")))
        for name in ancestors("publish-draft") | {"publish-draft"}:
            self.assertNotEqual(jobs[name].get("continue-on-error"), "true")
        events = workflow("kukuri-release.yml")["on"]
        self.assertEqual(set(events), {"push", "workflow_dispatch"})

    def test_user_inputs_are_not_interpolated_into_shell(self):
        jobs = workflow("kukuri-release.yml")["jobs"]
        for job in jobs.values():
            for step in job.get("steps", []):
                script = step.get("run", "")
                self.assertNotIn("${{ github.event.inputs", script)
                self.assertNotIn("${{ inputs.", script)
        initial = jobs["validate-release-inputs"]
        self.assertIn("release_source", initial["outputs"])
        for name in ("linux-verify", "windows-package", "release-assets"):
            checkout = next(s for s in jobs[name]["steps"] if s.get("uses", "").startswith("actions/checkout"))
            self.assertEqual(checkout["with"]["ref"], "${{ needs.validate-release-inputs.outputs.release_source }}")

    def test_linux_pr_does_not_receive_distribution_secrets(self):
        steps = workflow("kukuri-linux-package.yml")["jobs"]["linux-appimage"]["steps"]
        signing = next(step for step in steps if step.get("name") == "Build and verify AppImage and Deb")
        for key in ("TAURI_SIGNING_PRIVATE_KEY", "TAURI_SIGNING_PRIVATE_KEY_PASSWORD", "TAURI_UPDATER_PUBLIC_KEY"):
            expression = signing["env"][key]
            self.assertIn("inputs.signing == 'distribution'", expression)
            self.assertIn("github.event_name == 'workflow_dispatch'", expression)
            self.assertIn("startsWith(github.ref, 'refs/tags/v')", expression)
            self.assertNotIn("pull_request", expression)
        cli = workflow("kukuri-cli-package.yml")
        self.assertEqual(cli["jobs"]["cli-package"]["strategy"]["matrix"]["arch"], ["x86_64", "aarch64"])
        self.assertNotIn("secrets", cli["on"].get("workflow_call", {}))


if __name__ == "__main__":
    unittest.main()
