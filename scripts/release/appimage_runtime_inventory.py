"""Collect AppDir package evidence on its Debian/Ubuntu build host; never run it.

This is not redistribution approval or a complete SBOM: the outer AppImage
runtime, static dependencies, non-ELF assets, and source delivery need review.
"""
import argparse
import hashlib
import json
import pathlib
import re
import shutil
import subprocess


def run(*args, acceptable=(0,)):
    result = subprocess.run(args, text=True, capture_output=True, check=False)
    if result.returncode not in acceptable:
        raise RuntimeError(f"{args[0]} failed ({result.returncode}): {result.stderr.strip()}")
    return result.stdout.strip()


def build_id(file):
    match = re.search(r"Build ID: ([0-9a-f]+)", run("readelf", "-n", str(file)))
    return match[1] if match else None


def package_directory(owner):
    if not re.fullmatch(r"[a-z0-9][a-z0-9+.-]*(?::[a-z0-9-]+)?", owner):
        raise ValueError(f"Unexpected dpkg package name: {owner}")
    return owner.replace(":", "_")


def matching_packages(file, ident):
    if not ident:
        return []
    # dpkg-query returns 1 when no package owns this filename. Other failures
    # still propagate; an unavailable dpkg database is not an empty inventory.
    rows = run("dpkg-query", "-S", "*/" + file.name, acceptable=(0, 1))
    owners = set()
    for row in rows.splitlines():
        if ": " not in row:
            continue
        owner, source = row.split(": ", 1)
        source = pathlib.Path(source)
        if source.is_file() and build_id(source) == ident:
            package_directory(owner)
            owners.add(owner)
    return sorted(owners)


def package_record(owner, output, doc_root=pathlib.Path("/usr/share/doc")):
    version, source, source_version = run(
        "dpkg-query", "-W", "-f=${Version}\t${source:Package}\t${source:Version}", owner
    ).split("\t")
    copyright_file = doc_root / owner.split(":")[0] / "copyright"
    copyright_text = copyright_file.read_text(errors="replace") if copyright_file.is_file() else ""
    relative = None
    if copyright_text:
        destination = output / "copyrights" / package_directory(owner) / "copyright"
        destination.parent.mkdir(parents=True)
        shutil.copyfile(copyright_file, destination)
        relative = destination.relative_to(output).as_posix()
    return {
        "version": version, "source_package": source, "source_version": source_version,
        "copyright_available": bool(copyright_text), "copyright_relative_path": relative,
        "copyright_source_path": str(copyright_file),
        "license_labels": sorted(set(re.findall(r"^License: (.+)$", copyright_text, re.M))),
    }


def matching_content_packages(file):
    """Match shipped schema/xdg-mime source by bytes, not filename alone."""
    digest = hashlib.sha256(file.read_bytes()).digest()
    rows = run("dpkg-query", "-S", "*/" + file.name, acceptable=(0, 1))
    owners = set()
    for row in rows.splitlines():
        if ": " not in row:
            continue
        owner, source = row.split(": ", 1)
        source = pathlib.Path(source)
        if source.is_file() and hashlib.sha256(source.read_bytes()).digest() == digest:
            package_directory(owner)
            owners.add(owner)
    return sorted(owners)


def collect(appdir, output):
    if output.exists():
        raise FileExistsError(f"Evidence directory already exists: {output}")
    appdir = appdir.resolve(strict=True)
    if not appdir.is_dir():
        raise ValueError("AppDir must be a directory")
    if output.resolve().is_relative_to(appdir):
        raise ValueError("Evidence output must be outside AppDir")
    output.mkdir(parents=True)
    records, packages, other, links, non_elf = [], {}, [], [], []
    for file in sorted(appdir.rglob("*")):
        relative = file.relative_to(appdir).as_posix()
        if file.is_symlink():
            links.append({"file": relative, "target": str(file.readlink())})
            continue
        if not file.is_file():
            continue
        with file.open("rb") as stream:
            is_elf = stream.read(4) == b"\x7fELF"
        if not is_elf:
            other.append(relative)
            owners = matching_content_packages(file) if (
                relative.startswith("usr/share/glib-2.0/schemas/")
                or relative == "usr/bin/xdg-mime"
            ) else []
            for owner in owners:
                if owner not in packages:
                    packages[owner] = package_record(owner, output)
            non_elf.append({
                "file": relative, "sha256": hashlib.sha256(file.read_bytes()).hexdigest(),
                "matching_build_host_packages": owners,
            })
            continue
        ident = build_id(file)
        first_party = relative == "usr/bin/kukuri-desktop-tauri"
        owners = [] if first_party else matching_packages(file, ident)
        for owner in owners:
            if owner not in packages:
                packages[owner] = package_record(owner, output)
        records.append({
            "file": relative, "sha256": hashlib.sha256(file.read_bytes()).hexdigest(),
            "build_id": ident, "matching_build_host_packages": owners, "first_party": first_party,
        })
    common = pathlib.Path("/usr/share/common-licenses")
    if common.is_dir():
        shutil.copytree(common, output / "common-licenses", symlinks=False)
    report = {
        "schema_version": 1, "redistribution_approved": False,
        "scope": "AppDir ELF matched by GNU build ID; schemas/xdg-mime by content hash on its build host; license labels are not approval.",
        "limitations": ["Outer AppImage runtime not included", "Static dependencies not enumerated",
                        "Non-ELF assets and source delivery obligations require separate review"],
        "elf_files": records, "packages": packages, "non_elf_paths": other,
        "non_elf_files": non_elf, "symlinks": links,
        "unmatched_elf": [r["file"] for r in records if not r["first_party"] and not r["matching_build_host_packages"]],
        "missing_copyright": [p for p, v in packages.items() if not v["copyright_available"]],
    }
    (output / "runtime-inventory.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return report


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("appdir", type=pathlib.Path)
    parser.add_argument("output", type=pathlib.Path)
    args = parser.parse_args()
    evidence = collect(args.appdir, args.output)
    print(json.dumps({"elf_count": len(evidence["elf_files"]), "package_count": len(evidence["packages"]),
                      "unmatched_elf": evidence["unmatched_elf"], "missing_copyright": evidence["missing_copyright"],
                      "redistribution_approved": False}, indent=2))
    if evidence["missing_copyright"]:
        raise SystemExit("Missing package copyright evidence")
