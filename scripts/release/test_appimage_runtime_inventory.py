"""Contracts for the read-only AppDir evidence collector (no system packages needed)."""
import importlib.util
import pathlib
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "inventory", pathlib.Path(__file__).with_name("appimage_runtime_inventory.py")
)
inventory = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(inventory)


class RuntimeInventoryTests(unittest.TestCase):
    def test_non_elf_owner_requires_matching_content(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            bundled, source = root / "bundled.xml", root / "host.xml"
            bundled.write_text("public schema")
            source.write_text("different schema")
            with patch.object(inventory, "run", return_value=f"schemas: {source}"):
                self.assertEqual(inventory.matching_content_packages(bundled), [])
                source.write_bytes(bundled.read_bytes())
                self.assertEqual(inventory.matching_content_packages(bundled), ["schemas"])

    def test_schema_package_copyright_is_included_in_evidence(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            appdir = root / "AppDir"
            schema = appdir / "usr/share/glib-2.0/schemas/example.xml"
            schema.parent.mkdir(parents=True)
            schema.write_text("public schema")
            record = {"copyright_available": True}
            with patch.object(inventory, "matching_content_packages", return_value=["schemas"]), \
                    patch.object(inventory, "package_record", return_value=record) as package:
                report = inventory.collect(appdir, root / "out")
            package.assert_called_once_with("schemas", root / "out")
            self.assertEqual(report["packages"], {"schemas": record})
            self.assertEqual(report["non_elf_files"][0]["matching_build_host_packages"], ["schemas"])

    def test_package_paths_are_portable_and_cannot_escape(self):
        self.assertEqual(inventory.package_directory("libx:amd64"), "libx_amd64")
        with self.assertRaises(ValueError):
            inventory.package_directory("../../escape")

    def test_only_matching_build_id_is_owned(self):
        with tempfile.TemporaryDirectory() as temporary:
            library = pathlib.Path(temporary) / "libx.so"
            library.write_bytes(b"\x7fELFfixture")
            with patch.object(inventory, "run", return_value=f"libx:amd64: {library}\n"), \
                    patch.object(inventory, "build_id", return_value="aaa"):
                self.assertEqual(inventory.matching_packages(library, "aaa"), ["libx:amd64"])
                self.assertEqual(inventory.matching_packages(library, "bbb"), [])
                self.assertEqual(inventory.matching_packages(library, None), [])

    def test_existing_output_is_never_overwritten(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            sentinel = root / "keep"
            sentinel.write_text("unchanged")
            with self.assertRaises(FileExistsError):
                inventory.collect(root, root)
            self.assertEqual(sentinel.read_text(), "unchanged")

    def test_unmatched_elf_and_non_elf_are_visible_without_execution(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            appdir = root / "AppDir"
            appdir.mkdir()
            (appdir / "AppRun.wrapped").write_bytes(b"\x7fELFfixture")
            (appdir / "AppRun").write_text("#!/bin/sh\nexit 99\n")
            with patch.object(inventory, "build_id", return_value="aaa"), \
                    patch.object(inventory, "matching_packages", return_value=[]):
                report = inventory.collect(appdir, root / "out")
            self.assertEqual(report["unmatched_elf"], ["AppRun.wrapped"])
            self.assertEqual(report["non_elf_paths"], ["AppRun"])
            self.assertFalse(report["redistribution_approved"])

    def test_missing_copyright_is_not_success(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            with patch.object(inventory, "run", return_value="1.2\tsource-x\t1.2"):
                result = inventory.package_record("libx:amd64", root, root / "missing-docs")
            self.assertFalse(result["copyright_available"])
            self.assertIsNone(result["copyright_relative_path"])

    def test_copyright_is_copied_without_changing_source(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            source = root / "docs" / "libx" / "copyright"
            source.parent.mkdir(parents=True)
            source.write_text("Copyright fixture\nLicense: MIT\n", encoding="utf-8")
            with patch.object(inventory, "run", return_value="1.2\tsource-x\t1.1"):
                result = inventory.package_record("libx:amd64", root / "out", root / "docs")
            self.assertEqual(result["source_version"], "1.1")
            self.assertEqual(result["license_labels"], ["MIT"])
            self.assertEqual(result["copyright_relative_path"], "copyrights/libx_amd64/copyright")
            self.assertEqual((root / "out" / result["copyright_relative_path"]).read_bytes(), source.read_bytes())

    def test_output_inside_appdir_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            with self.assertRaises(ValueError):
                inventory.collect(root, root / "out")
            self.assertFalse((root / "out").exists())

    def test_tool_failure_is_not_an_empty_inventory(self):
        result = subprocess.CompletedProcess(["dpkg-query"], 2, "", "database failure")
        with patch.object(inventory.subprocess, "run", return_value=result):
            with self.assertRaises(RuntimeError):
                inventory.matching_packages(pathlib.Path("libx.so"), "aaa")

    def test_missing_package_is_unmatched(self):
        result = subprocess.CompletedProcess(["dpkg-query"], 1, "", "no path found")
        with patch.object(inventory.subprocess, "run", return_value=result):
            self.assertEqual(inventory.matching_packages(pathlib.Path("libx.so"), "aaa"), [])


if __name__ == "__main__":
    unittest.main()
