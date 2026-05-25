#!/usr/bin/env bash
set -euo pipefail

repo_root="${1:-$(git rev-parse --show-toplevel)}"

python3 - "$repo_root" <<'PY'
from pathlib import Path
import re
import sys

root = Path(sys.argv[1])
api_dir = root / "web" / "lib" / "api"

pattern = re.compile(
    r"\b(?P<helper>api(?:Get|Post|Patch|Put|Delete))"
    r"\s*(?:<[^;()]*>)?\s*\(\s*"
    r"(?P<quote>[\"'`])(?P<path>/[^\"'`]*)(?P=quote)",
    re.DOTALL,
)

violations: list[tuple[Path, int, str, str]] = []
for path in sorted(api_dir.glob("*.ts")):
    text = path.read_text(encoding="utf-8")
    for match in pattern.finditer(text):
        api_path = match.group("path")
        if api_path.startswith("/api/v1/"):
            continue
        line = text.count("\n", 0, match.start()) + 1
        violations.append((path.relative_to(root), line, match.group("helper"), api_path))

if violations:
    print(
        "Un-prefixed gateway API helper paths found. Use /api/v1/... for "
        "apiGet/apiPost/apiPatch/apiPut/apiDelete paths.",
        file=sys.stderr,
    )
    for rel_path, line, helper, api_path in violations:
        print(f"{rel_path}:{line}: {helper}({api_path!r})", file=sys.stderr)
    sys.exit(1)
PY
