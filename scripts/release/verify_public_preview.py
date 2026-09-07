"""Read-only CDN verification of the already verified release candidate."""
import argparse
import hashlib
import json
import pathlib
import urllib.request

from release_assets import file_record, release_input, validate_output


def download_hash(url):
    digest = hashlib.sha256()
    with urllib.request.urlopen(url, timeout=180) as response:
        while chunk := response.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def verify(root, tag, repository, source, fetch=download_hash):
    version = release_input("workflow_dispatch", tag, "")
    validate_output(root, tag, repository, version, source)
    base = f"https://github.com/{repository}/releases/download/{tag}/"
    latest = f"https://github.com/{repository}/releases/latest/download/latest-preview.json?verify={source}"
    if fetch(latest) != file_record(root, "latest-preview.json")["sha256"]:
        raise ValueError("Stable updater URL has not reached this candidate")
    provenance = json.loads((root / "release-provenance.json").read_text())
    names = {"SHA256SUMS.txt", "release-provenance.json"}
    for target, package in provenance["packages"].items():
        if target.startswith("cli-"):
            names.add(f"kukuri-cli_{version}_{target.removeprefix('cli-linux-')}-unknown-linux-gnu.tar.gz")
        else:
            names.add(package["updater_file"])
            if target == "linux-x86_64":
                names.add(package["deb_updater_file"])
    for name in sorted(names):
        if fetch(base + name) != file_record(root, name)["sha256"]:
            raise ValueError(f"Published artifact mismatch: {name}")
    # All uploaded files, including large source archives, were independently
    # checked against GitHub's upload digest before publication; do not redownload
    # those archives merely to repeat the same integrity check.
    return {"stable_manifest": "passed", "public_files": len(names), "source_commit": source}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=pathlib.Path, required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--source", required=True)
    args = parser.parse_args()
    print(json.dumps(verify(args.input, args.tag, args.repository, args.source)))
