#!/usr/bin/env python3
"""
Validate Markdown documentation date strings follow the YYYY年MM月DD日 format.

Usage:
  python scripts/check_date_format.py            # defaults to docs/ directory
  python scripts/check_date_format.py --check .  # explicitly run in check mode
  python scripts/check_date_format.py --fix      # rewrite files with zero-padded dates

The script scans for patterns like `2025年8月3日` and either reports them
(`--check`, default) or normalizes them to `2025年08月03日` (`--fix`).
"""

from __future__ import annotations

import argparse
import pathlib
import re
import sys
from typing import Iterable, List, Tuple

DATE_PATTERN = re.compile(r"(20\\d{2})年(\\d{1,2})月(\\d{1,2})日")
VALID_SUFFIXES = {".md", ".mdx"}


def iter_markdown_files(paths: Iterable[pathlib.Path]) -> Iterable[pathlib.Path]:
    for root in paths:
        if root.is_file():
            if root.suffix in VALID_SUFFIXES:
                yield root
            continue
        if root.is_dir():
            for file_path in root.rglob("*"):
                if file_path.is_file() and file_path.suffix in VALID_SUFFIXES:
                    yield file_path


def read_text_with_fallback(path: pathlib.Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        for encoding in ("utf-8-sig", "cp932", "shift_jis"):
            try:
                return path.read_text(encoding=encoding)
            except UnicodeDecodeError:
                continue
    return path.read_text(encoding="utf-8", errors="ignore")


def find_invalid_dates(path: pathlib.Path) -> List[Tuple[int, str]]:
    text = read_text_with_fallback(path)
    results: List[Tuple[int, str]] = []

    for line_no, line in enumerate(text.splitlines(), start=1):
        for match in DATE_PATTERN.finditer(line):
            month = match.group(2)
            day = match.group(3)
            if len(month) != 2 or len(day) != 2:
                results.append((line_no, match.group(0)))

    return results


def fix_dates(path: pathlib.Path) -> bool:
    text = read_text_with_fallback(path)

    def repl(match: re.Match[str]) -> str:
        year, month, day = match.groups()
        return f"{year}年{int(month):02d}月{int(day):02d}日"

    new_text = DATE_PATTERN.sub(repl, text)
    if new_text != text:
        path.write_text(new_text, encoding="utf-8")
        return True
    return False


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Check or fix YYYY年MM月DD日 date formatting in Markdown files."
    )
    parser.add_argument(
        "paths",
        nargs="*",
        type=pathlib.Path,
        default=[pathlib.Path("docs")],
        help="Files or directories to scan (default: docs/).",
    )
    parser.add_argument(
        "--fix",
        action="store_true",
        help="Rewrite files in place so month/day are zero-padded.",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Only check for violations (default when --fix is not provided).",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()

    paths = list(iter_markdown_files(args.paths))
    if not paths:
        print("No Markdown files found for the provided paths.", file=sys.stderr)
        return 0

    if args.fix:
        changed_files = [path for path in paths if fix_dates(path)]
        if changed_files:
            for path in changed_files:
                print(f"Fixed date formatting: {path}")
        else:
            print("No changes were necessary.")
        return 0

    # Default to check mode when --fix is not set.
    violations = []
    for path in paths:
        for line_no, value in find_invalid_dates(path):
            violations.append(f"{path}:{line_no}:{value}")

    if violations:
        print("Invalid date format detected (expected YYYY年MM月DD日):")
        for violation in violations:
            print(f"  {violation}")
        return 1

    print("All checked dates use YYYY年MM月DD日 format.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
