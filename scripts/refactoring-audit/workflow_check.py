"""Actions adapter: arguments in, validated JSON/summary/artifact out."""
import json
import os
from pathlib import Path
import subprocess
import sys


def main():
    force = os.environ.get("AUDIT_FORCE", "false") or "false"
    if force not in ("true", "false"):
        raise ValueError("AUDIT_FORCE must be true or false")
    args = ["cargo", "run", "--locked", "--quiet", "--package", "xtask", "--no-default-features", "--",
            "refactoring-audit-check", "--format", "json", "--current", os.environ["GITHUB_SHA"]]
    reason = os.environ.get("AUDIT_REASON", "")
    if force == "true":
        args.append("--force-audit")
    if reason:
        args.append(f"--reason={reason}")
    try:
        result = subprocess.run(args, check=True, capture_output=True)
    except subprocess.CalledProcessError as error:
        if error.stderr:
            sys.stderr.buffer.write(error.stderr)
        raise
    report = json.loads(result.stdout)
    if (type(report.get("audit_required")) is not bool
            or report.get("current_commit") != os.environ["GITHUB_SHA"]
            or not isinstance(report.get("markdown"), str) or not report["markdown"]):
        raise ValueError("invalid command result")
    Path(os.environ["RUNNER_TEMP"], "refactoring-audit.json").write_bytes(result.stdout)
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        output.write(f"audit_required={str(report['audit_required']).lower()}\n")
    with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as summary:
        summary.write(report["markdown"])


if __name__ == "__main__":
    main()
