"""Deb payload contracts; fixtures are never installed or executed."""
import io
import pathlib
import sys
import tarfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import deb_package as deb


def archive(files):
    output = io.BytesIO()
    with tarfile.open(fileobj=output, mode="w") as tar:
        for name, (data, mode) in files.items():
            entry = tarfile.TarInfo("./" + name)
            entry.size, entry.mode = len(data), mode
            tar.addfile(entry, io.BytesIO(data))
    return output.getvalue()


class DebTests(unittest.TestCase):
    def fixture(self):
        elf = bytearray(64)
        elf[:6] = b"\x7fELF\x02\x01"
        elf[18:20] = (62).to_bytes(2, "little")
        return {
            "usr/bin/kukuri-desktop-tauri": (bytes(elf), 0o755),
            "usr/share/applications/kukuri.desktop": (b"[Desktop Entry]\nType=Application\nExec=kukuri-desktop-tauri %U\nIcon=kukuri\nMimeType=x-scheme-handler/kukuri;\n", 0o644),
            "usr/share/icons/hicolor/128x128/apps/kukuri.png": (b"\x89PNG\r\n\x1a\nfixture", 0o644),
            "usr/share/doc/kukuri/copyright": (b"license", 0o644),
            "usr/share/doc/kukuri/THIRD_PARTY_NOTICES.md": (b"notices", 0o644),
            "usr/share/doc/kukuri/native-licenses/README.md": (b"native notices", 0o644),
        }

    def metadata(self):
        return {"Package": "kukuri", "Version": "0.1.8", "Architecture": "amd64",
                "Depends": ", ".join(sorted(deb.REQUIRED_DEPENDS))}

    def test_real_payload_inventory_distinguishes_system_dependencies(self):
        report = deb.inspect_payload(self.metadata(), archive(self.fixture()), archive({}), "0.1.8")
        self.assertEqual(report["elf_paths"], ["usr/bin/kukuri-desktop-tauri"])
        self.assertEqual(set(report["system_dependencies"]), deb.REQUIRED_DEPENDS)
        self.assertGreater(len(report["payload"]), 3)

    def test_metadata_and_payload_defects_are_rejected(self):
        for defect in ("arch", "version", "package", "depends", "elf", "extra-elf", "notice", "deep-link", "setuid", "user-data", "script"):
            with self.subTest(defect=defect):
                metadata, files, scripts = self.metadata(), self.fixture(), {}
                if defect == "arch": metadata["Architecture"] = "arm64"
                elif defect == "version": metadata["Version"] = "0.1.9"
                elif defect == "package": metadata["Package"] = "another-app"
                elif defect == "depends": metadata["Depends"] = ""
                elif defect == "elf": files["usr/bin/kukuri-desktop-tauri"] = (b"not ELF", 0o755)
                elif defect == "extra-elf": files["usr/lib/libextra.so"] = files["usr/bin/kukuri-desktop-tauri"]
                elif defect == "notice": del files["usr/share/doc/kukuri/copyright"]
                elif defect == "deep-link": files["usr/share/applications/kukuri.desktop"] = (b"[Desktop Entry]\nExec=another-app\n", 0o644)
                elif defect == "setuid": files["usr/bin/kukuri-desktop-tauri"] = (files["usr/bin/kukuri-desktop-tauri"][0], 0o4755)
                elif defect == "user-data": files["home/user/.local/share/kukuri/db"] = (b"bad", 0o644)
                elif defect == "script": scripts["postinst"] = (b"#!/bin/sh\nrm -r /home/user\n", 0o755)
                with self.assertRaises(ValueError):
                    deb.inspect_payload(metadata, archive(files), archive(scripts), "0.1.8")


if __name__ == "__main__":
    unittest.main()
