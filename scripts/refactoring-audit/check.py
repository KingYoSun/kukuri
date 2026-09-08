"""Read-only audit recommendation. All product inputs come from Git objects."""
import argparse
from collections import Counter
from datetime import date, datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

from jsonschema import Draft7Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[2]
BASELINE = "xtask/refactoring-audit-baseline.json"
SCHEMA = ROOT / "docs/schemas/refactoring-audit-baseline.schema.json"


class AuditError(Exception):
    """An invalid input or failed read, never a successful recommendation."""


def require(condition, message):
    if not condition:
        raise AuditError(message)


def git(root, *args):
    result = subprocess.run(
        ["git", "--no-pager", *args], cwd=root, capture_output=True,
        env={**os.environ, "GIT_NO_REPLACE_OBJECTS": "1", "GIT_OPTIONAL_LOCKS": "0"},
    )
    require(result.returncode == 0, f"git_read_failed: {args[0]}: {result.stderr.decode('utf-8', errors='replace').strip()}")
    return result.stdout.decode("utf-8")


def blob(root, commit, path):
    return git(root, "show", f"{commit}:{path}")


def oid(root, commit, path):
    return git(root, "rev-parse", "--verify", f"{commit}:{path}").strip()


def paths_at(root, commit):
    return git(root, "ls-tree", "-r", "--name-only", "-z", commit).rstrip("\0").split("\0")


def digest(paths):
    return hashlib.sha256(("\n".join(sorted(paths)) + "\n").encode("utf-8")).hexdigest()


def json_value(text):
    def unique_object(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, f"duplicate_json_key: {key}")
            result[key] = value
        return result
    return json.loads(text, object_pairs_hook=unique_object)


def unique(items, key, label):
    values = [item[key] for item in items]
    require(len(values) == len(set(values)), f"duplicate_{label}")


def candidate(path, scope):
    return (path.rsplit(".", 1)[-1].lower() in scope["extensions"]
            and path not in scope["excluded_exact_paths"]
            and not any(path.startswith(prefix) for prefix in scope["excluded_path_prefixes"]))


def ratchet(text):
    value = json_value(text)
    require(isinstance(value, dict) and set(value) == {"files"} and isinstance(value["files"], list), "invalid_ratchet")
    result = {}
    for entry in value["files"]:
        require(isinstance(entry, dict) and set(entry) == {"path", "lines"}, "invalid_ratchet_entry")
        path, lines = entry["path"], entry["lines"]
        require(isinstance(path, str) and path and not path.startswith("/") and "\\" not in path
                and ".." not in path.split("/") and not re.match(r"^[A-Za-z]:", path)
                and "\0" not in path, "invalid_ratchet_path")
        require(type(lines) is int and lines >= 1000, "invalid_ratchet_cap")
        require(path not in result, "duplicate_ratchet_path")
        result[path] = lines
    return result


def line_count(text):
    # str::lines: LF delimiter, optional CR, no extra line for a final LF.
    return len(text.split("\n")) - (1 if text.endswith("\n") else 0) if text else 0


def metric_value(root, commit, metric, side):
    total = 0
    for path in metric[f"{side}_paths"]:
        text = blob(root, commit, path)
        scope = metric["measurement_scope"]
        if scope["kind"] == "struct_fields":
            symbol = scope[f"{side}_symbol"]
            # Current baseline describes named Rust structs, not arbitrary code execution.
            match = re.search(r"\bstruct\s+" + re.escape(symbol) + r"\b[^\{]*\{([^}]*)\}", text, re.S)
            require(match is not None, f"metric_struct_missing: {metric['id']}:{symbol}")
            text = match[1]
        total += len(re.findall(metric["measurement_regex"], text, re.M))
    return total


def validate_baseline(root, current, baseline):
    schema = json_value(SCHEMA.read_text(encoding="utf-8"))
    validator = Draft7Validator(schema, format_checker=FormatChecker())
    errors = list(validator.iter_errors(baseline))
    require(not errors, "baseline_schema_invalid: " + "; ".join(error.message for error in errors[:3]))
    base, start = baseline["baseline_commit"], baseline["audit_start_commit"]
    git(root, "cat-file", "-e", f"{base}^{{commit}}")
    git(root, "cat-file", "-e", f"{start}^{{commit}}")
    git(root, "merge-base", "--is-ancestor", start, base)
    git(root, "merge-base", "--is-ancestor", base, current)
    for path, expected in baseline["code_tree_oids"].items():
        require(oid(root, base, path) == expected, f"baseline_tree_mismatch: {path}")

    scope = baseline["scope"]
    require(scope["source_policy"] == "xtask/src/oversized.rs::should_scan_oversized_file", "unsupported_source_policy")
    require(oid(root, base, "xtask/src/oversized.rs") == scope["source_policy_blob"], "source_policy_blob_mismatch")
    before, after = paths_at(root, start), paths_at(root, base)
    candidates = [path for path in after if candidate(path, scope)]
    require(len(candidates) == scope["tracked_candidate_count"]
            and digest(candidates) == scope["tracked_candidate_paths_sha256"], "candidate_population_mismatch")
    unique(scope["groups"], "id", "group")
    for group in scope["groups"]:
        selector = re.compile(group["selector_regex"])
        old = {path for path in before if selector.search(path)}
        new = {path for path in after if selector.search(path)}
        require(len(old) == group["before_count"] and len(new) == group["member_count"]
                and digest(new) == group["member_paths_sha256"]
                and new - old == set(group["added_paths"])
                and old - new == set(group["removed_paths"]), f"group_population_mismatch: {group['id']}")

    oversized = baseline["oversized"]
    require(oid(root, base, oversized["ratchet_path"]) == oversized["ratchet_blob"], "ratchet_blob_mismatch")
    caps = ratchet(blob(root, base, oversized["ratchet_path"]))
    unique(oversized["files"], "path", "oversized_path")
    require(oversized["count"] == len(oversized["files"]), "oversized_count_mismatch")
    require(caps == {item["path"]: item["ratchet_lines"] for item in oversized["files"]}, "ratchet_snapshot_mismatch")
    for item in oversized["files"]:
        require(item["path"] in candidates, "oversized_path_outside_scope")
        require(line_count(blob(root, base, item["path"])) == item["observed_lines"], "observed_lines_mismatch")
        require(line_count(blob(root, start, item["path"])) == item["before_lines"], "before_lines_mismatch")
        require(item["observed_lines"] <= item["ratchet_lines"], "baseline_ratchet_violation")
    unique(baseline["structural_metrics"], "id", "metric")
    for metric in baseline["structural_metrics"]:
        for side, commit in [("before", start), ("after", base)]:
            require(metric_value(root, commit, metric, side) == metric[side], f"metric_mismatch: {metric['id']}:{side}")
    unique(baseline["candidates"], "id", "candidate")
    counts = Counter(item["classification"] for item in baseline["candidates"])
    for field, label in [("selected", "実施"), ("deferred", "延期"), ("rejected", "却下"), ("other_type", "別種別")]:
        require(counts[label] == baseline["candidate_counts"][field], "candidate_classification_mismatch")
    unique(baseline["children"], "issue", "child")
    return caps


def evaluate(value, threshold):
    require(type(value) is int and value >= 0, "invalid_signal_value")
    return value > threshold


def diagnostics(root, base, current, scope, now, audit_date):
    history_range = f"{base}..{current}"
    commits = int(git(root, "rev-list", "--count", history_range).strip())
    # One numstat row per path/commit, merges compared to first parent; renames
    # are deletion+addition. Binary rows count as path touches, not lines.
    history = git(root, "log", "--format=", "--numstat", "-z", "--no-renames",
                  "--diff-merges=first-parent", "--no-ext-diff", "--no-textconv", history_range, "--")
    counts = Counter()
    for row in history.split("\0"):
        if not row:
            continue
        parts = row.split("\t", 2)
        require(len(parts) == 3, "invalid_git_numstat")
        path = parts[2]
        if candidate(path, scope):
            counts[path] += 1
    return dict(commits_since_baseline=commits, changed_candidate_paths=len(counts),
                max_path_change_commits=max(counts.values(), default=0),
                days_since_audit=(now.date() - date.fromisoformat(audit_date)).days,
                hotspots=[dict(path=path, commits=count) for path, count in sorted(counts.items(), key=lambda item: (-item[1], item[0]))[:10]])


def markdown_text(text):
    # Keep input visible without allowing it to create HTML markers or tables.
    return str(text).replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace("|", "&#124;").replace("\r", "").replace("\n", "<br>")


def render(result):
    lines = ["## 監査トリガー判定", "",
             f"- 比較起点: `{result['baseline_commit']}`", f"- 評価commit: `{result['current_commit']}`",
             f"- 判定日時: {result['evaluated_at']}", f"- audit_required: `{str(result['audit_required']).lower()}`",
             f"- 直前の監査／campaign: #{result['tracking_issue']}、規範: #871", "",
             "| signal | 値 | 閾値 | 発火 | 根拠 |", "| --- | ---: | --- | --- | --- |"]
    for signal in result["signals"]:
        lines.append(f"| {signal['id']} | {signal['value']} | > {signal['threshold']} | {str(signal['triggered']).lower()} | {markdown_text(signal['reason'])} |")
    lines += ["", "### 手動入力", "", markdown_text(result["manual_reason"]) or "指定なし",
              "", "### 人間による確認項目", ""]
    lines += [f"- {markdown_text(item)}" for item in result["human_only_triggers"]]
    lines += ["", "### 対象候補path", ""]
    lines += [f"- {markdown_text(path)}" for path in result["candidate_paths"]] or ["- 自動候補なし。手動理由から有限scopeを固定する。"]
    diag = result["diagnostics"]
    lines += ["", "### 観測情報（自動閾値なし）", "",
              f"commit数: {diag['commits_since_baseline']}、変更path数: {diag['changed_candidate_paths']}、最大変更commit数: {diag['max_path_change_commits']}、監査後日数: {diag['days_since_audit']}", ""]
    lines += [f"- {markdown_text(item['path'])}: {item['commits']} commits" for item in diag["hotspots"]]
    return "\n".join(lines) + "\n"


def check(root, current="HEAD", now=None, force=False, reason=""):
    require(not force or bool(reason.strip()), "force_reason_required")
    current = git(root, "rev-parse", "--verify", "--end-of-options", f"{current}^{{commit}}").strip()
    require(re.fullmatch(r"[0-9a-f]{40}", current) is not None, "invalid_current_commit")
    baseline = json_value(blob(root, current, BASELINE))
    old = validate_baseline(root, current, baseline)
    new = ratchet(blob(root, current, baseline["oversized"]["ratchet_path"]))
    timestamp = datetime.fromisoformat(now.replace("Z", "+00:00")) if now else datetime.now(timezone.utc)
    require(timestamp.tzinfo is not None, "evaluation_timezone_required")
    timestamp = timestamp.astimezone(timezone.utc).replace(microsecond=0)
    require(timestamp.date() >= date.fromisoformat(baseline["audit_date"]), "evaluation_before_audit")
    new_paths = sorted(new.keys() - old.keys())
    increased = sorted(path for path in old.keys() & new.keys() if new[path] > old[path])
    signals = []
    for name, paths in [("ratchet_new_paths", new_paths), ("ratchet_increased_caps", increased)]:
        config = baseline["signals"][name]
        signals.append(dict(id=name, value=len(paths), threshold=config["threshold"],
                            triggered=evaluate(len(paths), config["threshold"]), reason=config["reason"], paths=paths))
    result = dict(schema_version=1, baseline_commit=baseline["baseline_commit"], current_commit=current,
                  tracking_issue=baseline["tracking_issue"], evaluated_at=timestamp.isoformat().replace("+00:00", "Z"),
                  audit_required=force or any(signal["triggered"] for signal in signals),
                  force_audit=force, manual_reason=reason, signals=signals,
                  candidate_paths=sorted(set(new_paths + increased)), human_only_triggers=baseline["human_only_triggers"],
                  diagnostics=diagnostics(root, baseline["baseline_commit"], current, baseline["scope"], timestamp, baseline["audit_date"]))
    result["markdown"] = render(result)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=ROOT)
    parser.add_argument("--current", default="HEAD")
    parser.add_argument("--now", help="ISO-8601 timezone-aware evaluation time; default UTC now")
    parser.add_argument("--force-audit", action="store_true")
    parser.add_argument("--reason", default="")
    parser.add_argument("--format", choices=["json", "markdown"], default="markdown")
    args = parser.parse_args()
    try:
        result = check(args.repo, current=args.current, now=args.now, force=args.force_audit, reason=args.reason)
        output = json.dumps(result, ensure_ascii=False, indent=2) + "\n" if args.format == "json" else result["markdown"]
        sys.stdout.buffer.write(output.encode("utf-8"))
        return 0
    except (AuditError, ValueError, KeyError, TypeError, OSError, re.error) as error:
        sys.stderr.buffer.write(f"refactoring_audit_error: {error}\n".encode("utf-8"))
        return 1


if __name__ == "__main__":
    sys.exit(main())
