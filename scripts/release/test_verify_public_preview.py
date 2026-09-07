import json
import pathlib
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import verify_public_preview as public


class PublicTests(unittest.TestCase):
    def test_public_binaries_and_metadata_match_candidate(self):
        with tempfile.TemporaryDirectory() as work:
            root = pathlib.Path(work)
            packages = {
                "windows-x86_64": {"updater_file": "windows.exe"},
                "linux-x86_64": {"updater_file": "linux.AppImage", "deb_updater_file": "linux.deb"},
                "cli-linux-x86_64": {}, "cli-linux-aarch64": {},
            }
            names = ["latest-preview.json", "SHA256SUMS.txt", "windows.exe", "linux.AppImage", "linux.deb"]
            names += [f"kukuri-cli_0.1.8_{arch}-unknown-linux-gnu.tar.gz" for arch in ("x86_64", "aarch64")]
            for name in names:
                (root / name).write_text(name)
            (root / "release-provenance.json").write_text(json.dumps({"packages": packages}))
            calls = []

            def fetch(url):
                calls.append(url)
                name = url.rsplit("/", 1)[-1].split("?")[0]
                return public.file_record(root, name)["sha256"]

            with patch.object(public, "validate_output", return_value=[]):
                result = public.verify(root, "v0.1.8-preview.2", "KingYoSun/kukuri", "a" * 40, fetch)
                self.assertEqual(result["public_files"], 7)
                self.assertEqual(len(calls), 8)
                for broken in ("linux.AppImage", "linux.deb", "windows.exe", names[-1]):
                    with self.subTest(broken=broken), self.assertRaisesRegex(ValueError, "Published artifact mismatch"):
                        public.verify(root, "v0.1.8-preview.2", "KingYoSun/kukuri", "a" * 40,
                                      lambda url: "0" * 64 if url.endswith(broken) else fetch(url))

    def test_stale_latest_manifest_stops_before_bundle_download(self):
        with tempfile.TemporaryDirectory() as work:
            root = pathlib.Path(work)
            (root / "latest-preview.json").write_text("expected")
            calls = []

            def fetch(url):
                calls.append(url)
                return "0" * 64

            with patch.object(public, "validate_output", return_value=[]):
                with self.assertRaises(ValueError):
                    public.verify(root, "v0.1.8-preview.2", "KingYoSun/kukuri", "a" * 40, fetch)
            self.assertEqual(len(calls), 1)
            self.assertIn("/releases/latest/download/latest-preview.json", calls[0])


if __name__ == "__main__":
    unittest.main()
