"""Create a new draft, upload verified files, then optionally publish. Never overwrite."""
import argparse
import json
import pathlib
import subprocess

from release_assets import file_record, release_input, validate_output


class GitHubError(RuntimeError):
    def __init__(self, status):
        self.status = status
        super().__init__(f"GitHub request failed ({status}); existing assets were not replaced")


def github(path, method="GET", data=None, file=None):
    command = ["gh", "api", path, "--method", method]
    body = None
    if file is not None:
        command += ["--input", str(file), "-H", "Content-Type: application/octet-stream"]
    elif data is not None:
        command += ["--input", "-"]
        body = json.dumps(data)
    result = subprocess.run(command, input=body, capture_output=True, text=True, check=False)
    if result.returncode:
        try:
            status = int(json.loads(result.stdout).get("status", 0))
        except (ValueError, TypeError):
            status = 0
        raise GitHubError(status)
    return json.loads(result.stdout)


def publish(root, tag, repository, source, draft=True, api=github):
    version = release_input("workflow_dispatch", tag, "")
    names = validate_output(root, tag, repository, version, source)
    if api(f"repos/{repository}/commits/{tag}")["sha"] != source:
        raise ValueError("Remote tag changed after release validation")
    notes = (root / "RELEASE_NOTES_DRAFT.md").read_text(encoding="utf-8-sig")
    created = False
    try:
        release = api(f"repos/{repository}/releases/tags/{tag}")
    except GitHubError as error:
        if error.status != 404:
            raise
        release = api(f"repos/{repository}/releases", "POST", {
            "tag_name": tag, "target_commitish": source, "name": f"kukuri {tag}", "body": notes,
            # GitHub's prerelease flag is distinct from our preview tag/channel.
            "draft": True, "prerelease": False,
        })
        created = True
    release_id = int(release["id"])
    existing = {asset["name"]: asset for asset in release.get("assets", [])}
    if not created and release.get("body") != notes:
        raise ValueError("Existing release notes differ; refusing to change it")
    if release.get("assets"):
        if not set(existing).issubset(names):
            raise ValueError("Existing release does not match this candidate; refusing to change it")
        for name, asset in existing.items():
            if asset.get("digest") != "sha256:" + file_record(root, name)["sha256"]:
                raise ValueError("Existing release asset differs; refusing replacement")
    if release.get("draft") is False:
        if set(existing) != set(names):
            raise ValueError("Published release is incomplete; automatic modification is forbidden")
        return {"release_id": release_id, "draft": False, "source_commit": source, "unchanged": True}
    for name in names:
        if name in existing:
            continue  # Resume the same candidate without uploading/replacing an existing asset.
        digest = file_record(root, name)["sha256"]
        uploaded = api(f"https://uploads.github.com/repos/{repository}/releases/{release_id}/assets?name={name}",
                       "POST", file=root / name)
        if uploaded.get("digest") != f"sha256:{digest}":
            raise ValueError("Uploaded asset digest mismatch; release remains a draft")
    if not draft:
        if api(f"repos/{repository}/commits/{tag}")["sha"] != source:
            raise ValueError("Tag moved during upload; release remains a draft")
        api(f"repos/{repository}/releases/{release_id}", "PATCH",
            {"draft": False, "prerelease": False, "make_latest": "true"})
    return {"release_id": release_id, "draft": draft, "source_commit": source}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=pathlib.Path, required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--source", required=True)
    parser.add_argument("--draft", choices=["true", "false"], default="true")
    args = parser.parse_args()
    print(json.dumps(publish(args.input, args.tag, args.repository, args.source, args.draft == "true")))
