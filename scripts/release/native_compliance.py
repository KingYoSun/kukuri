"""Collect exact native sources/notices without executing upstream build recipes."""
import argparse
import hashlib
import json
import pathlib
import re
import shutil
import struct
import subprocess
import tarfile
import tempfile
import urllib.request

from release_assets import file_record, write_package
from deb_package import inspect as inspect_deb

SPEC_FILE = pathlib.Path(__file__).with_name("native-runtime-sources.json")


def bind_runtime(data, spec):
    size, offset, width = spec["runtime_size"], spec["digest_md5_offset"], spec["digest_md5_size"]
    prefix = bytearray(data[:size])
    if len(prefix) != size or prefix[:6] != b"\x7fELF\x02\x01" or width != 16:
        raise ValueError("Unknown AppImage runtime")
    table = struct.unpack_from("<Q", prefix, 40)[0]
    entry_size, count, names_index = struct.unpack_from("<HHH", prefix, 58)
    if entry_size != 64 or names_index >= count or table + count * entry_size > size:
        raise ValueError("Invalid runtime section table")
    names_offset, names_size = struct.unpack_from("<QQ", prefix, table + names_index * entry_size + 24)
    names = prefix[names_offset:names_offset + names_size]
    found = []
    for index in range(count):
        entry = table + index * entry_size
        name_offset = struct.unpack_from("<I", prefix, entry)[0]
        name = bytes(names[name_offset:]).split(b"\0", 1)[0]
        if name == b".digest_md5":
            found.append(struct.unpack_from("<QQ", prefix, entry + 24))
    if found != [(offset, width)] or offset + width > size:
        raise ValueError("Unexpected mutable runtime section")
    prefix[offset:offset + width] = bytes(width)
    if hashlib.sha256(prefix).hexdigest() != spec["normalized_prefix_sha256"]:
        raise ValueError("Runtime differs outside the validated digest section")
    return spec["normalized_prefix_sha256"]


def download(entry, root):
    package, name = entry["package"], entry["name"]
    if not re.fullmatch(r"[A-Za-z0-9._-]+", package) or not re.fullmatch(r"[A-Za-z0-9_+.-]+", name):
        raise ValueError("Unsafe source filename")
    if package in {".", ".."} or name in {".", ".."} or not entry["url"].startswith("https://"):
        raise ValueError("Invalid source location")
    algorithm = "sha512" if "sha512" in entry else "sha256"
    target = root / package / name
    target.parent.mkdir(parents=True, exist_ok=True)
    digest = hashlib.new(algorithm)
    with urllib.request.urlopen(entry["url"], timeout=120) as response, target.open("xb") as output:
        while chunk := response.read(1024 * 1024):
            digest.update(chunk)
            output.write(chunk)
    if digest.hexdigest() != entry[algorithm]:
        raise ValueError(f"Upstream source checksum mismatch: {package}/{name}")
    return target


def source_file_record(directory, name):
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._+~-]*", name):
        raise ValueError("Unsafe source archive name")
    file = directory / name
    if file.is_symlink() or not file.is_file():
        raise ValueError("Source file is missing or indirect")
    return {"name": name, "sha256": hashlib.sha256(file.read_bytes()).hexdigest()}


def verify_dsc(directory, expected_name=None, expected_version=None):
    descriptors = list(directory.glob("*.dsc"))
    if len(descriptors) != 1:
        raise ValueError("Expected one source descriptor")
    text = descriptors[0].read_text(encoding="utf-8")
    if expected_name and (f"Source: {expected_name}\n" not in text or f"Version: {expected_version}\n" not in text):
        raise ValueError("Source descriptor identity mismatch")
    match = re.search(r"(?m)^Checksums-Sha256:\n((?: [^\n]+\n)+)", text)
    if not match:
        raise ValueError("Source descriptor lacks SHA-256 inventory")
    for row in match[1].splitlines():
        digest, size, name = row.split()
        record = source_file_record(directory, name)
        if record["sha256"] != digest or (directory / name).stat().st_size != int(size):
            raise ValueError("Ubuntu source archive does not match descriptor")
    return [source_file_record(directory, path.name) for path in sorted(directory.iterdir()) if path.is_file()]


def source_packages(inventory, destination):
    packages = sorted({(value["source_package"], value["source_version"]) for value in inventory["packages"].values()})
    records = []
    for name, version in packages:
        if not re.fullmatch(r"[a-z0-9][a-z0-9+.-]*", name) or not re.fullmatch(r"[A-Za-z0-9.+:~\-]+", version):
            raise ValueError("Invalid source package identity")
        directory = destination / name / version.replace(":", "_")
        directory.mkdir(parents=True)
        result = subprocess.run(["apt-get", "source", "--download-only", "--only-source", "--yes", f"{name}={version}"],
                                cwd=directory, capture_output=True, text=True, timeout=900)
        if result.returncode:
            raise RuntimeError(f"Exact Ubuntu source unavailable: {name}={version}")
        records.append({"package": name, "version": version, "files": verify_dsc(directory, name, version)})
    if not records:
        raise ValueError("Empty native source inventory")
    return records


def copy_archive_notices(archive, destination):
    if not tarfile.is_tarfile(archive):
        return
    with tarfile.open(archive) as tar:
        for member in tar:
            name = pathlib.PurePosixPath(member.name).name.upper()
            if member.isfile() and (name.startswith("LICENSE") or name.startswith("COPYING") or name == "COPYRIGHT"):
                # Do not extract paths or execute anything from source archives.
                output = destination / (hashlib.sha256(member.name.encode()).hexdigest()[:16] + ".txt")
                destination.mkdir(parents=True, exist_ok=True)
                output.write_bytes(tar.extractfile(member).read())


def archive_directory(source, target):
    temporary = target.with_suffix(target.suffix + ".partial")
    with tarfile.open(temporary, "w:gz", compresslevel=1) as tar:
        tar.add(source, arcname=source.name)
    if temporary.stat().st_size > 1900 * 1024 * 1024:
        raise ValueError("Source archive exceeds the release part size; split before publication")
    temporary.replace(target)


def collect(package_dir, appdir):
    metadata = json.loads((package_dir / "release-package.json").read_text())
    if metadata["target"] != "linux-x86_64" or metadata["signing_mode"] != "distribution":
        raise ValueError("Native source collection requires the distribution package")
    spec = json.loads(SPEC_FILE.read_text())
    appimage = package_dir / metadata["updater_file"]
    runtime = bind_runtime(appimage.read_bytes(), spec["runtime"])
    if hashlib.sha256((appdir / "AppRun.wrapped").read_bytes()).hexdigest() != "f30140a43a0a59e46db21bdefdf749b9e9f2c6946e92afabbacf98b8ae73fb4f":
        raise ValueError("Unreviewed AppRun binary")
    evidence = package_dir / "runtime-evidence"
    deb = package_dir / metadata["deb_updater_file"]
    deb_payload_name = f"kukuri_{metadata['version']}_deb-payload.json"
    deb_payload = inspect_deb(deb, metadata["version"])
    deb_payload["source_commit"] = metadata["source_commit"]
    if json.loads((package_dir / deb_payload_name).read_text()) != deb_payload:
        raise ValueError("Deb payload differs from the build inventory")
    # Deb contains only the first-party Rust ELF, desktop/icon and notices. Its
    # dynamically linked OS libraries are resolved by Depends, not redistributed.
    # The AppImage runtime and Ubuntu source inventory do not claim to cover Deb.
    shutil.copyfile(package_dir / deb_payload_name, evidence / "deb-payload.json")
    inventory = json.loads((evidence / "runtime-inventory.json").read_text())
    if inventory["missing_copyright"]:
        raise ValueError("Native copyright material is incomplete")
    version = metadata["version"]
    prefix = f"kukuri_{version}_linux-native"
    source_archive = package_dir / f"{prefix}-sources.tar.gz"
    compliance_file = package_dir / f"{prefix}-compliance.json"
    if source_archive.exists() or compliance_file.exists():
        raise ValueError("Native source output already exists")
    with tempfile.TemporaryDirectory(prefix="kukuri-native-sources-") as work:
        sources = pathlib.Path(work) / "sources"
        sources.mkdir()
        static = sources / "static"
        notices = evidence / "static-notices"
        shutil.copytree(pathlib.Path(__file__).with_name("native-licenses"), notices)
        for entry in spec["sources"]:
            file = download(entry, static)
            if entry["kind"] == "notice":
                shutil.copyfile(file, notices / file.name)
            else:
                copy_archive_notices(file, notices / entry["package"])
        ubuntu = source_packages(inventory, sources / "ubuntu")
        shutil.copyfile(SPEC_FILE, sources / "runtime-source-manifest.json")
        shutil.copyfile(pathlib.Path(__file__).with_name("native-source-readme.md"), sources / "SOURCE-README.md")
        # Include the actual public schema/wrapper/xdg source shipped in this AppDir.
        public_files = evidence / "shipped-non-elf"
        for name in inventory["non_elf_paths"]:
            file = appdir / name
            if not file.resolve().is_relative_to(appdir.resolve()) or file.is_symlink():
                raise ValueError("Indirect bundled source asset")
            output = public_files / name
            output.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(file, output)
        (sources / "ubuntu-source-manifest.json").write_text(json.dumps(ubuntu, indent=2) + "\n")
        archive_directory(sources, source_archive)
    notices_archive = package_dir / f"{prefix}-notices.tar.gz"
    archive_directory(evidence, notices_archive)
    report = {
        "schema_version": 1, "source_material_complete": True,
        "appimage_sha256": file_record(package_dir, appimage.name)["sha256"],
        "runtime_normalized_sha256": runtime, "runtime_source": spec["runtime"]["source_commit"],
        "ubuntu_source_count": len(ubuntu), "static_source_count": len(spec["sources"]),
        "material": [file_record(package_dir, source_archive.name), file_record(package_dir, notices_archive.name)],
        "deb_sha256": file_record(package_dir, deb.name)["sha256"],
        "deb_payload_sha256": file_record(package_dir, deb_payload_name)["sha256"],
        "deb_native_scope": "first-party-elf-system-shared-libraries",
    }
    compliance_file.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    files = sorted({row["name"] for row in metadata["files"]} | {source_archive.name, notices_archive.name, compliance_file.name})
    write_package(package_dir, metadata["target"], version, metadata["source_commit"], files,
                  metadata["updater_file"], metadata["public_key_file"], metadata["signing_mode"], metadata["deb_updater_file"])
    print(json.dumps({"ubuntu_source_count": len(ubuntu), "static_source_count": len(spec["sources"]), "complete": True}))


def verify_static(appimage, output):
    spec = json.loads(SPEC_FILE.read_text())
    bind_runtime(appimage.read_bytes(), spec["runtime"])
    output.mkdir(parents=True, exist_ok=False)
    for entry in spec["sources"]:
        download(entry, output)
    print(json.dumps({"scope": "static-only", "runtime_match": True, "verified_files": len(spec["sources"])}))


def verify_ubuntu(inventory_file, output):
    output.mkdir(parents=True, exist_ok=False)
    inventory = json.loads(inventory_file.read_text())
    records = source_packages(inventory, output / "ubuntu")
    (output / "ubuntu-source-manifest.json").write_text(json.dumps(records, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"scope": "ubuntu-only", "verified_source_packages": len(records)}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", type=pathlib.Path)
    parser.add_argument("--appdir", type=pathlib.Path)
    parser.add_argument("--verify-static-only", action="store_true")
    parser.add_argument("--verify-ubuntu-only", action="store_true")
    parser.add_argument("--inventory", type=pathlib.Path)
    parser.add_argument("--appimage", type=pathlib.Path)
    parser.add_argument("--output", type=pathlib.Path)
    args = parser.parse_args()
    if args.verify_ubuntu_only:
        if not args.inventory or not args.output: parser.error("Ubuntu verification needs --inventory and --output")
        verify_ubuntu(args.inventory, args.output)
    elif args.verify_static_only:
        if not args.appimage or not args.output: parser.error("static verification needs --appimage and --output")
        verify_static(args.appimage, args.output)
    else:
        if not args.package or not args.appdir: parser.error("collection needs --package and --appdir")
        collect(args.package, args.appdir)
