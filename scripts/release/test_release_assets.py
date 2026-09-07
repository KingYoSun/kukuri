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
    def test_linux_appimage_without_deb_is_not_a_complete_release(self):
        with tempfile.TemporaryDirectory() as work:
            root = pathlib.Path(work)
            self.fixture(root)
            metadata = root / "linux-x86_64" / "release-package.json"
            value = json.loads(metadata.read_text())
            del value["deb_updater_file"]
            metadata.write_text(json.dumps(value))
            with self.assertRaisesRegex(ValueError, "Deb"):
                release.assembly_plan(root, TAG, "KingYoSun/kukuri", VERSION, SOURCE)

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
            deb = None
            key = None
            if updater:
                key = f"{target}-updater-key.pub"
                (directory / key).write_text("same-public-key")
                (directory / f"{filename}.sig").write_text("signature")
                files += [key, f"{filename}.sig"]
            if target == "linux-x86_64":
                deb = f"kukuri_{VERSION}_amd64.deb"
                (directory / deb).write_bytes(b"deb fixture")
                (directory / f"{deb}.sig").write_text("deb-signature")
                payload_name = f"kukuri_{VERSION}_deb-payload.json"
                (directory / payload_name).write_text(json.dumps({
                    "deb_sha256": release.file_record(directory, deb)["sha256"],
                    "source_commit": SOURCE, "package": "kukuri", "version": VERSION,
                    "architecture": "amd64", "maintainer_scripts": [],
                    "elf_paths": ["usr/bin/kukuri-desktop-tauri"],
                    "payload": [{"path": path} for path in ["usr/bin/kukuri-desktop-tauri",
                        "usr/share/doc/kukuri/copyright", "usr/share/doc/kukuri/THIRD_PARTY_NOTICES.md"]],
                }))
                files += [deb, f"{deb}.sig", payload_name]
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
                    "deb_sha256": release.file_record(directory, deb)["sha256"],
                    "deb_payload_sha256": release.file_record(directory, payload_name)["sha256"],
                    "deb_native_scope": "first-party-elf-system-shared-libraries",
                }))
                files.append(name)
            release.write_package(directory, target, VERSION, SOURCE, files, updater, key, "distribution", deb)

    def test_complete_set_has_three_platforms_and_pinned_source(self):
        with tempfile.TemporaryDirectory() as work:
            root = pathlib.Path(work)
            self.fixture(root)
            plan = release.assembly_plan(root, TAG, "KingYoSun/kukuri", VERSION, SOURCE)
            self.assertEqual(set(plan["platforms"]), {"windows-x86_64", "linux-x86_64", "linux-x86_64-deb"})
            self.assertEqual(plan["source_commit"], SOURCE)
            self.assertEqual(plan["platforms"]["linux-x86_64"]["signature"], "signature")
            self.assertEqual(plan["platforms"]["linux-x86_64-deb"]["signature"], "deb-signature")

    def test_deb_missing_tampered_mixed_or_uncovered_is_rejected(self):
        for defect in ("missing", "signature", "tampered", "arch", "version", "source", "notice", "coverage", "format"):
            with self.subTest(defect=defect), tempfile.TemporaryDirectory() as work:
                root = pathlib.Path(work)
                self.fixture(root)
                directory = root / "linux-x86_64"
                metadata = directory / "release-package.json"
                value = json.loads(metadata.read_text())
                deb = directory / value["deb_updater_file"]
                payload = directory / f"kukuri_{VERSION}_deb-payload.json"
                report = json.loads(payload.read_text())
                if defect == "missing": deb.unlink()
                elif defect == "signature": (directory / (deb.name + ".sig")).unlink()
                elif defect == "tampered": deb.write_bytes(b"changed")
                elif defect == "arch": report["architecture"] = "arm64"
                elif defect == "version": report["version"] = "0.2.0"
                elif defect == "source": report["source_commit"] = "b" * 40
                elif defect == "notice": report["payload"] = []
                elif defect == "coverage": report["deb_sha256"] = "0" * 64
                elif defect == "format":
                    value["deb_updater_file"] = value["updater_file"]
                    metadata.write_text(json.dumps(value))
                if defect in {"arch", "version", "source", "notice", "coverage"}:
                    payload.write_text(json.dumps(report))
                    compliance = directory / f"kukuri_{VERSION}_linux-native-compliance.json"
                    native = json.loads(compliance.read_text())
                    native["deb_payload_sha256"] = release.file_record(directory, payload.name)["sha256"]
                    compliance.write_text(json.dumps(native))
                    value["files"] = [release.file_record(directory, row["name"]) for row in value["files"]]
                    metadata.write_text(json.dumps(value))
                with self.assertRaises(ValueError):
                    release.assembly_plan(root, TAG, "KingYoSun/kukuri", VERSION, SOURCE)

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
