#!/usr/bin/env python3
"""Remove redundant version literals from internal path dependencies."""

from __future__ import annotations

import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CRATE_TOMLS = sorted((ROOT / "crates").glob("*/Cargo.toml"))
INTERNAL_PATH = re.compile(r'path\s*=\s*"\.\./argo-[^"]+"')
VERSION_FIELD = re.compile(r',\s*version\s*=\s*"0\.0\.1"')


def rewrite_line(line: str) -> tuple[str, int]:
    if INTERNAL_PATH.search(line):
        return VERSION_FIELD.subn("", line)
    return line, 0


def main() -> int:
    files_changed = 0
    fields_removed = 0

    for manifest in CRATE_TOMLS:
        original = manifest.read_text()
        rewritten_lines = []
        removed_in_file = 0
        for line in original.splitlines(keepends=True):
            rewritten, removed = rewrite_line(line)
            rewritten_lines.append(rewritten)
            removed_in_file += removed

        if removed_in_file:
            manifest.write_text("".join(rewritten_lines))
            files_changed += 1
            fields_removed += removed_in_file
            print(f"{manifest.relative_to(ROOT)}: removed {removed_in_file}")

    print(f"summary: {fields_removed} version field(s) removed across {files_changed} file(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
