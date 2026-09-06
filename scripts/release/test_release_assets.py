"""Release assembly contracts: no signer, network, publish, or real user profile."""
import importlib.util
import json
import pathlib
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("release_assets", pathlib.Path(__file__).with_name("release_assets.py"))
release = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release)

SOURCE = "a" * 40
VERSION = "0.1.8"
TAG = "v0.1.8-preview.2"


class ReleaseTests(unittest.TestCase):
    def test_untrusted_or_injected_release_input_has_no_output(self):
        import subprocess
        import sys
        for event, tag, ref in (
            ("pull_request", TAG, "refs/pull/1/merge"),
            ("push", TAG, "refs/heads/main"),
            ("workflow_dispatch", TAG + "$(echo injected)", "refs/heads/main"),
        ):
            result = subprocess.run([sys.executable, str(pathlib.Path(__file__).with_name("release_assets.py")),
                "resolve-input", "--event", event, "--tag", tag, "--ref", ref, "--draft", "true"],
                text=True, capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(result.stdout, "")

    def fixture(self, root):
        for target, filename in {
            "windows-x86_64": "kukuri_0.1.8_x64-setup.exe",
            "linux-x86_64": "kukuri_0.1.8_amd64.AppImage",
            "cli-linux-x86_64": "kukuri-cli_0.1.8_x86_64-unknown-linux-gnu.tar.gz",
            "cli-linux-aarch64": "kukuri-cli_0.1.8_aarch64-unknown-linux-gnu.tar.gz",
        }.items():
            directory = root / target
            directory.mkdir()
            (directory / filename).write_bytes(b"fixture")
            files = [filename]
            updater = None if target.startswith("cli-") else filename
            key = None
            if updater:
                key = f"{target}-updater-key.pub"
                (directory / key).write_text("same-public-key")
                (directory / f"{filename}.sig").write_text("signature")
                files += [key, f"{filename}.sig"]
            if target == "linux-x86_64":
                spec = json.loads(pathlib.Path(__file__).with_name("native-runtime-sources.json").read_text())
                material = []
                for kind in ("sources", "notices"):
                    name = f"kukuri_{VERSION}_linux-native-{kind}.tar.gz"
                    (directory / name).write_bytes(b"source/notice fixture")
                    files.append(name)
                    material.append(release.file_record(directory, name))
                name = f"kukuri_{VERSION}_linux-native-compliance.json"
                (directory / name).write_text(json.dumps({
                    "source_material_complete": True, "ubuntu_source_count": 1,
                    "static_source_count": len(spec["sources"]),
                    "runtime_source": spec["runtime"]["source_commit"],
                    "runtime_normalized_sha256": spec["runtime"]["normalized_prefix_sha256"],
                    "appimage_sha256": release.file_record(directory, filename)["sha256"], "material": material,
                }))
                files.append(name)
            release.write_package(directory, target, VERSION, SOURCE, files, updater, key, "distribution")

    def test_complete_set_has_two_platforms_and_pinned_source(self):
        with tempfile.TemporaryDirectory() as work:
            root = pathlib.Path(work)
            self.fixture(root)
            plan = release.assembly_plan(root, TAG, "KingYoSun/kukuri", VERSION, SOURCE)
            self.assertEqual(set(plan["platforms"]), {"windows-x86_64", "linux-x86_64"})
            self.assertEqual(plan["source_commit"], SOURCE)
            self.assertEqual(plan["platforms"]["linux-x86_64"]["signature"], "signature")

    def test_missing_tampered_foreign_source_or_test_key_is_rejected_before_output(self):
        for defect in ("missing", "tampered", "foreign", "test-key", "key-mismatch", "duplicate", "missing-source"):
            with self.subTest(defect=defect), tempfile.TemporaryDirectory() as work:
                root = pathlib.Path(work)
                self.fixture(root)
                directory = root / "linux-x86_64"
                metadata = directory / "release-package.json"
                value = json.loads(metadata.read_text())
                if defect == "missing": metadata.unlink()
                elif defect == "tampered": (directory / value["updater_file"]).write_bytes(b"changed")
                elif defect == "foreign":
                    value["source_commit"] = "b" * 40
                    metadata.write_text(json.dumps(value))
                elif defect == "test-key":
                    value["signing_mode"] = "test"
                    metadata.write_text(json.dumps(value))
                elif defect == "key-mismatch":
                    key = directory / value["public_key_file"]
                    key.write_text("different key")
                    value["files"] = [release.file_record(directory, row["name"]) for row in value["files"]]
                    metadata.write_text(json.dumps(value))
                elif defect == "duplicate":
                    extra = root / "duplicate"
                    extra.mkdir()
                    (extra / "release-package.json").write_text(metadata.read_text())
                elif defect == "missing-source":
                    (directory / f"kukuri_{VERSION}_linux-native-sources.tar.gz").unlink()
                with self.assertRaises(ValueError):
                    release.assembly_plan(root, TAG, "KingYoSun/kukuri", VERSION, SOURCE)

    def test_invalid_tag_and_unsafe_asset_paths_are_rejected(self):
        with tempfile.TemporaryDirectory() as work:
            root = pathlib.Path(work)
            self.fixture(root)
            for tag in ("v0.1.9-preview.1", "v0.1.8", "v0.1.8-preview.1$(echo bad)"):
                with self.assertRaises(ValueError):
                    release.assembly_plan(root, tag, "KingYoSun/kukuri", VERSION, SOURCE)
            for name in ("../outside", "/absolute", "name with spaces", "file\\path"):
                with self.assertRaises(ValueError): release.file_record(root, name)


if __name__ == "__main__":
    unittest.main()
