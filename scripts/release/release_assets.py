"""Small package/assembly contract shared by release jobs. No signing or publishing."""
import argparse
import hashlib
import json
import pathlib
import re

TARGETS = {"windows-x86_64", "linux-x86_64", "cli-linux-x86_64", "cli-linux-aarch64"}


def source_version(version, source):
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        raise ValueError("Invalid release version")
    if not re.fullmatch(r"[0-9a-f]{40}", source):
        raise ValueError("Release source must be a full commit SHA")


def release_input(event, tag, ref):
    if event not in {"push", "workflow_dispatch"}:
        raise ValueError("Untrusted release event")
    match = re.fullmatch(r"v([0-9]+\.[0-9]+\.[0-9]+)-preview\.([0-9]+)", tag)
    if not match or (event == "push" and ref != f"refs/tags/{tag}"):
        raise ValueError("Invalid preview tag or event ref")
    return match[1]


def file_record(directory, name):
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]*", name):
        raise ValueError("Unsafe asset filename")
    file = directory / name
    if file.is_symlink() or not file.is_file() or file.resolve().parent != directory.resolve():
        raise ValueError(f"Missing or indirect asset: {name}")
    return {"name": name, "sha256": hashlib.sha256(file.read_bytes()).hexdigest()}


def write_package(directory, target, version, source, files, updater=None, key=None, signing="none", deb=None):
    source_version(version, source)
    if target not in TARGETS or len(files) != len(set(files)) or not files:
        raise ValueError("Invalid package target or duplicate/empty inventory")
    records = [file_record(directory, name) for name in sorted(files)]
    if target.startswith("cli-"):
        arch = target.removeprefix("cli-linux-")
        expected = f"kukuri-cli_{version}_{arch}-unknown-linux-gnu.tar.gz"
        if expected not in files or updater or key:
            raise ValueError("CLI archive does not match version/target")
    elif not updater or updater not in files or f"{updater}.sig" not in files or key not in files:
        raise ValueError("GUI updater, signature and public key must be present")
    if target == "linux-x86_64":
        if deb != f"kukuri_{version}_amd64.deb" or not {deb, f"{deb}.sig"}.issubset(files):
            raise ValueError("Deb updater and signature must be present")
    elif deb:
        raise ValueError("Deb updater belongs only to the Linux GUI package")
    value = {
        "schema_version": 1, "target": target, "version": version, "source_commit": source,
        "signing_mode": signing, "files": records, "updater_file": updater,
        "public_key_file": key,
        "deb_updater_file": deb,
    }
    (directory / "release-package.json").write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
    return value


def assembly_plan(root, tag, repository, version, source):
    entries = [(json.loads(path.read_text(encoding="utf-8-sig")), path.parent)
               for path in sorted(root.rglob("release-package.json"))]
    return packages_plan(entries, tag, repository, version, source)


def packages_plan(entries, tag, repository, version, source):
    source_version(version, source)
    if release_input("workflow_dispatch", tag, "") != version:
        raise ValueError("Tag/version mismatch")
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
        raise ValueError("Invalid repository")
    packages, assets, names, platforms, public_keys = {}, [], set(), {}, set()
    for value, directory in entries:
        target = value.get("target")
        if target not in TARGETS or target in packages or value.get("schema_version") != 1:
            raise ValueError("Unknown/duplicate package target or schema")
        if value.get("version") != version or value.get("source_commit") != source:
            raise ValueError("Package version/source mismatch")
        files = value.get("files", [])
        package_names = []
        for record in files:
            name = record["name"]
            if name.casefold() in names or file_record(directory, name) != record:
                raise ValueError("Duplicate or changed package asset")
            names.add(name.casefold())
            package_names.append(name)
            assets.append({**record, "path": str((directory / name).resolve()), "target": target})
        if target.startswith("cli-"):
            expected = f"kukuri-cli_{version}_{target.removeprefix('cli-linux-')}-unknown-linux-gnu.tar.gz"
            if expected not in package_names:
                raise ValueError("Missing CLI archive")
        else:
            updater, key = value.get("updater_file"), value.get("public_key_file")
            if value.get("signing_mode") != "distribution":
                raise ValueError("Test-signed GUI artifact cannot be released")
            if not updater or updater not in package_names or key not in package_names or f"{updater}.sig" not in package_names:
                raise ValueError("Incomplete updater package")
            expected = (
                rf"kukuri_{re.escape(version)}_amd64\.AppImage" if target == "linux-x86_64"
                else rf"kukuri_{re.escape(version)}_x64(?:-setup)?\.(exe|zip)"
            )
            if not re.fullmatch(expected, updater):
                raise ValueError("Updater filename/version/target mismatch")
            signature = (directory / f"{updater}.sig").read_text(encoding="utf-8-sig").strip()
            public_key = (directory / key).read_text(encoding="utf-8-sig").strip()
            if not signature or not public_key:
                raise ValueError("Empty updater signature or public key")
            public_keys.add(public_key)
            if target == "linux-x86_64":
                deb = value.get("deb_updater_file")
                if deb != f"kukuri_{version}_amd64.deb" or not {deb, f"{deb}.sig"}.issubset(package_names):
                    raise ValueError("Deb updater and signature are missing or mismatched")
                deb_signature = (directory / f"{deb}.sig").read_text(encoding="utf-8-sig").strip()
                if not deb_signature:
                    raise ValueError("Deb updater signature is empty")
                platforms["linux-x86_64-deb"] = {
                    "signature": deb_signature,
                    "url": f"https://github.com/{repository}/releases/download/{tag}/{deb}",
                }
                validate_native_material(directory, value)
            platforms[target] = {
                "signature": signature,
                "url": f"https://github.com/{repository}/releases/download/{tag}/{updater}",
            }
        packages[target] = value
    if set(packages) != TARGETS or len(public_keys) != 1:
        raise ValueError("Incomplete release set or inconsistent updater keys")
    return {"source_commit": source, "assets": assets, "platforms": platforms, "packages": packages}


def validate_native_material(directory, package):
    prefix = f"kukuri_{package['version']}_linux-native"
    names = {row["name"] for row in package["files"]}
    compliance = f"{prefix}-compliance.json"
    required = {f"{prefix}-sources.tar.gz", f"{prefix}-notices.tar.gz"}
    if not (required | {compliance}).issubset(names):
        raise ValueError("Native source/notice material is missing")
    report = json.loads((directory / compliance).read_text(encoding="utf-8"))
    spec = json.loads(pathlib.Path(__file__).with_name("native-runtime-sources.json").read_text())
    if (report.get("source_material_complete") is not True or report.get("ubuntu_source_count", 0) < 1
            or report.get("static_source_count") != len(spec["sources"])
            or report.get("runtime_normalized_sha256") != spec["runtime"]["normalized_prefix_sha256"]
            or report.get("runtime_source") != spec["runtime"]["source_commit"]
            or report.get("appimage_sha256") != file_record(directory, package["updater_file"])["sha256"]):
        raise ValueError("Native compliance does not cover this AppImage")
    material = report.get("material", [])
    if len(material) != 2 or {row["name"] for row in material} != required:
        raise ValueError("Native material inventory is incomplete")
    for row in material:
        if file_record(directory, row["name"]) != row:
            raise ValueError("Native source/notice archive changed")
    validate_deb_material(directory, package, report)


def validate_deb_material(directory, package, native):
    name = f"kukuri_{package['version']}_deb-payload.json"
    if name not in {row["name"] for row in package["files"]}:
        raise ValueError("Deb payload inventory is missing")
    report = json.loads((directory / name).read_text(encoding="utf-8"))
    digest = file_record(directory, package["deb_updater_file"])["sha256"]
    if (report.get("deb_sha256") != digest or native.get("deb_sha256") != digest
            or report.get("source_commit") != package["source_commit"]
            or report.get("package") != "kukuri" or report.get("version") != package["version"]
            or report.get("architecture") != "amd64" or report.get("maintainer_scripts") != []
            or report.get("elf_paths") != ["usr/bin/kukuri-desktop-tauri"]
            or native.get("deb_payload_sha256") != file_record(directory, name)["sha256"]
            or native.get("deb_native_scope") != "first-party-elf-system-shared-libraries"):
        raise ValueError("Deb native compliance does not cover this payload/source")
    paths = {row["path"] for row in report.get("payload", [])}
    if not {"usr/bin/kukuri-desktop-tauri", "usr/share/doc/kukuri/copyright",
            "usr/share/doc/kukuri/THIRD_PARTY_NOTICES.md"}.issubset(paths):
        raise ValueError("Deb notice payload is incomplete")


def validate_output(root, tag, repository, version, source):
    manifest_bytes = (root / "latest-preview.json").read_bytes()
    if manifest_bytes.startswith(b"\xef\xbb\xbf"):
        raise ValueError("Manifest has a UTF-8 BOM")
    manifest = json.loads(manifest_bytes)
    provenance = json.loads((root / "release-provenance.json").read_text(encoding="utf-8-sig"))
    if provenance.get("source_commit") != source or provenance.get("tag") != tag:
        raise ValueError("Release provenance mismatch")
    plan = packages_plan([(value, root) for value in provenance["packages"].values()],
                         tag, repository, version, source)
    if manifest.get("version") != version or manifest.get("platforms") != plan["platforms"]:
        raise ValueError("Assembled manifest differs from verified package metadata")
    names = (root / "release-assets.txt").read_text(encoding="utf-8-sig").splitlines()
    actual = {path.name for path in root.iterdir() if path.is_file()}
    required = {"latest-preview.json", "release-provenance.json", "RELEASE_NOTES_DRAFT.md",
                "THIRD_PARTY_NOTICES.md", "manual-smoke-checklist.md", "SHA256SUMS.txt", "release-assets.txt"}
    if len(names) != len(set(names)) or set(names) != actual or not required.issubset(actual):
        raise ValueError("Release asset inventory is incomplete or duplicated")
    sums = {}
    for line in (root / "SHA256SUMS.txt").read_text(encoding="utf-8-sig").splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9][A-Za-z0-9._-]*)", line)
        if not match or match[2] in sums:
            raise ValueError("Invalid checksum inventory")
        sums[match[2]] = match[1]
    if set(sums) != actual - {"SHA256SUMS.txt"}:
        raise ValueError("Checksum inventory does not cover release files")
    for name, digest in sums.items():
        if file_record(root, name)["sha256"] != digest:
            raise ValueError(f"Release checksum mismatch: {name}")
    return sorted(names)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    package = commands.add_parser("package")
    package.add_argument("--directory", type=pathlib.Path, required=True)
    package.add_argument("--target", choices=sorted(TARGETS), required=True)
    package.add_argument("--version", required=True)
    package.add_argument("--source", required=True)
    package.add_argument("--file", action="append", required=True)
    package.add_argument("--updater")
    package.add_argument("--deb-updater")
    package.add_argument("--public-key-file")
    package.add_argument("--signing", choices=["none", "test", "distribution"], default="none")
    plan = commands.add_parser("plan")
    plan.add_argument("--input", type=pathlib.Path, required=True)
    plan.add_argument("--tag", required=True)
    plan.add_argument("--repository", required=True)
    plan.add_argument("--version", required=True)
    plan.add_argument("--source", required=True)
    resolve = commands.add_parser("resolve-input")
    resolve.add_argument("--event", required=True)
    resolve.add_argument("--tag", required=True)
    resolve.add_argument("--ref", required=True)
    resolve.add_argument("--draft", choices=["true", "false"], required=True)
    validate = commands.add_parser("validate-output")
    validate.add_argument("--input", type=pathlib.Path, required=True)
    validate.add_argument("--tag", required=True)
    validate.add_argument("--repository", required=True)
    validate.add_argument("--version", required=True)
    validate.add_argument("--source", required=True)
    args = parser.parse_args()
    if args.command == "package":
        write_package(args.directory, args.target, args.version, args.source, args.file,
                      args.updater, args.public_key_file, args.signing, args.deb_updater)
    elif args.command == "plan":
        print(json.dumps(assembly_plan(args.input, args.tag, args.repository, args.version, args.source)))
    elif args.command == "validate-output":
        print(json.dumps({"verified_files": len(validate_output(args.input, args.tag, args.repository, args.version, args.source))}))
    else:
        version = release_input(args.event, args.tag, args.ref)
        print(f"release_tag={args.tag}\nrelease_version={version}\nrelease_draft={args.draft}")


if __name__ == "__main__":
    main()
