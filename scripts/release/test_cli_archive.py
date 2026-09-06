import importlib.util
import pathlib
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).parent))
SPEC = importlib.util.spec_from_file_location("cli_archive", pathlib.Path(__file__).with_name("cli_archive.py"))
cli = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(cli)


class CliArchiveTests(unittest.TestCase):
    def test_archive_contains_requested_arch_and_retains_executable(self):
        with tempfile.TemporaryDirectory() as work:
            root = pathlib.Path(work)
            (root / "docs/runbooks").mkdir(parents=True)
            for path in ("LICENSE", "docs/runbooks/linux-cli.md", "docs/THIRD_PARTY_NOTICES.md"):
                (root / path).write_text("fixture")
            binary = root / "binary"
            data = bytearray(64)
            data[:6] = b"\x7fELF\x02\x01"
            struct.pack_into("<H", data, 18, 183)
            binary.write_bytes(data)
            archive = cli.create_archive(binary, "aarch64", "0.1.8", "a" * 40, root / "out", root)
            restored = cli.unpack_cli(archive, root, "aarch64", "0.1.8")
            self.assertEqual(restored.read_bytes(), data)
            with self.assertRaises(ValueError): cli.check_elf(data, "x86_64")
            with self.assertRaises(ValueError): cli.create_archive(binary, "aarch64", "0.1.8", "a" * 40, root / "out", root)

    def test_non_elf_is_not_a_distribution_binary(self):
        with self.assertRaises(ValueError): cli.check_elf(b"not executable", "x86_64")


if __name__ == "__main__":
    unittest.main()
