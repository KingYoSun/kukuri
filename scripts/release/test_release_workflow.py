"""Check the actual workflow DAG and secret/source boundaries; no GitHub writes."""
import json
import pathlib
import shlex
import subprocess
import unittest

import yaml

ROOT = pathlib.Path(__file__).resolve().parents[2]


def workflow(name):
    # GitHub uses YAML 1.2; BaseLoader also preserves the literal `on` key.
    return yaml.load((ROOT / ".github/workflows" / name).read_text(encoding="utf-8"), Loader=yaml.BaseLoader)


class WorkflowTests(unittest.TestCase):
    def test_asset_smoke_reports_success_to_the_ci_pwsh_wrapper(self):
        result = subprocess.run([
            "pwsh", "-NoProfile", "-Command",
            "$ErrorActionPreference = 'Stop'; "
            "& ./scripts/release/test-create-preview-assets.ps1; "
            "& ./scripts/release/test-create-preview-assets-linux.ps1; "
            "if (Test-Path variable:\\LASTEXITCODE) { exit $LASTEXITCODE }",
        ], cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=120)
        self.assertIn("Linux + Windows assembly contracts passed", result.stdout)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_linux_refreshes_preinstalled_bundled_libraries_before_bundling(self):
        steps = workflow("kukuri-linux-package.yml")["jobs"]["linux-appimage"]["steps"]
        dependencies = next(step for step in steps if step.get("name") == "Linux build dependencies")
        bundle = next(step for step in steps if step.get("name") == "Build and verify AppImage and Deb")
        self.assertLess(steps.index(dependencies), steps.index(bundle))
        commands = dependencies["run"].replace("\\\n", " ").splitlines()
        installs = [shlex.split(command) for command in commands if command.strip().startswith("sudo apt-get install ")]
        # Runner images keep superseded versions whose exact source leaves the APT index (#907, #1094).
        for package in ("libgcrypt20", "libsqlite3-0"):
            self.assertTrue(any(package in command and "--no-upgrade" not in command for command in installs),
                            f"refresh the runner's preinstalled {package} before collecting exact matching source")

    def test_release_verify_installs_every_fast_cn_system_dependency(self):
        # linux-verify reruns the CN tests, including real ffmpeg video extraction (#1060).
        def packages(workflow_name, job_name):
            steps = workflow(workflow_name)["jobs"][job_name]["steps"]
            step = next(step for step in steps if step.get("name") == "Install Linux system dependencies")
            words = shlex.split(step["run"])
            return set(words[words.index("install") + 1:]) - {"-y"}

        missing = packages("kukuri-fast.yml", "linux-cn") - packages("kukuri-release.yml", "linux-verify")
        self.assertEqual(missing, set(), "release linux-verify must install the Fast CN test dependencies")

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

    def test_linux_package_cache_is_saved_only_from_main(self):
        # PR scope の cache は他の PR から読めないため、main の push で保存した cache を共有する。
        package = workflow("kukuri-linux-package.yml")
        events = package["on"]
        self.assertEqual(events["push"]["branches"], ["main"])
        self.assertEqual(events["push"]["paths"], events["pull_request"]["paths"])
        steps = package["jobs"]["linux-appimage"]["steps"]
        cache = next(step for step in steps if step.get("uses", "").startswith("Swatinem/rust-cache@"))
        self.assertEqual(cache["with"]["save-if"],
                         "${{ github.event_name == 'push' && github.ref == 'refs/heads/main' }}")

    def test_updater_endpoint_is_the_canonical_repository_stable_url(self):
        # 2026-09-16 移管後の正本。旧 owner の URL は GitHub redirect に依存するため設定へ保存しない。
        config = json.loads((ROOT / "apps/desktop/src-tauri/tauri.conf.json").read_text(encoding="utf-8"))
        updater = config["plugins"]["updater"]
        self.assertEqual(updater["endpoints"],
                         ["https://github.com/kukuri-app/kukuri/releases/latest/download/latest-preview.json"])
        self.assertTrue(updater["pubkey"].startswith("dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEI4QzVBN0U3NEIyOEQyM0YK"),
                        "the updater public key must not change with the repository move")


if __name__ == "__main__":
    unittest.main()
