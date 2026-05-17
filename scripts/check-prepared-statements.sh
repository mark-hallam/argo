#!/usr/bin/env bash
# Enforce sqlx prepared-statement caching on production query sites.
#
# Rule: production Rust must not build SQL via format!("SELECT|INSERT|
# UPDATE|DELETE ...") unless the interpolation is a documented dynamic
# identifier / PRAGMA exception. Values must be passed through sqlx
# bind parameters so prepared statements stay cached and injection-safe.
#
# Exception: identifiers and SQLite/SQLCipher PRAGMAs cannot always be
# parameterised. Mark the dynamic format! with `audit-allow:` on the
# same line or within the three lines immediately above it, and keep a
# fixed allowlist / enum / validated literal at the callsite.
#
# Inventory tracks brief claude-perf-store-prepared-statements.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

python3 - <<'PY'
from pathlib import Path
import re
import sys

roots = [
    Path("crates/argo-store/src"),
    Path("crates/argo-audit/src"),
    Path("crates/argo-cli/src/handlers"),
]
handler_re = re.compile(r"(audit_|session_|message_).*\.rs$")
format_re = re.compile(
    r'format!\s*\(\s*(r#*".*?"#*|"(?:\\.|[^"\\])*?")',
    re.DOTALL,
)
sql_re = re.compile(r"^\s*(SELECT|INSERT|UPDATE|DELETE|PRAGMA|ALTER\s+TABLE|DROP\s+COLUMN)\b")


def candidate_files():
    for root in roots:
        if not root.exists():
            continue
        for path in root.rglob("*.rs"):
            name = path.name
            if name == "tests.rs" or name.endswith("_tests.rs") or name.endswith("_test_helpers.rs"):
                continue
            if name in {"gap_tests.rs", "filter_dims_tests.rs", "sink_tenant_tests.rs", "archive_tests.rs"}:
                continue
            if root.as_posix().endswith("/handlers") and not handler_re.match(name):
                continue
            yield path


def literal_body(lit: str) -> str:
    lit = lit.lstrip("r#").strip('#"')
    return lit.replace("\\\n", "")


offences = []
allowed = 0
scanned = 0
for path in sorted(candidate_files()):
    scanned += 1
    text = path.read_text()
    lines = text.splitlines()
    for match in format_re.finditer(text):
        body = literal_body(match.group(1))
        if not sql_re.search(body):
            continue
        line_no = text.count("\n", 0, match.start()) + 1
        line = lines[line_no - 1] if line_no <= len(lines) else ""
        window = "\n".join(lines[max(0, line_no - 4):line_no])
        if "audit-allow:" in line or "audit-allow:" in window:
            allowed += 1
            continue
        sql = " ".join(body.split())
        offences.append(f"{path}:{line_no}: {sql[:140]}")

if offences:
    print(
        f"FAIL: undocumented SQL interpolation via format!() ({len(offences)} site(s)):",
        file=sys.stderr,
    )
    for offence in offences:
        print(f"  {offence}", file=sys.stderr)
    print(
        "\nUse sqlx bind parameters for values. For dynamic identifiers or non-bindable "
        "PRAGMA literals, add `// audit-allow: ...` near the format! and keep the "
        "callsite gated by a fixed allowlist, enum, or validator.",
        file=sys.stderr,
    )
    sys.exit(1)

print(
    f"OK: count: 0 undocumented SQL interpolation site(s); "
    f"{allowed} documented exception(s) across {scanned} production file(s)."
)
PY
