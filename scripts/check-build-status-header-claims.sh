#!/usr/bin/env bash
# check-build-status-header-claims.sh -- lint stale claims in BUILD_STATUS.md header.
#
# Intentionally narrow: this checks explicit brief-id mentions before the first
# table row and flags phrase classes that have already drifted in practice.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LEDGER=""

usage() {
    cat <<'USAGE' >&2
usage: scripts/check-build-status-header-claims.sh [--root <repo>] [--ledger <path>]

Checks the BUILD_STATUS.md header only. It fails when header prose says a
brief is pending ledger insertion even though a ledger row exists, or when a
header "STATUS row(s)" phrase disagrees with the live row status.
USAGE
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --root)
            [[ $# -ge 2 ]] || { usage; exit 2; }
            ROOT="$2"
            shift 2
            ;;
        --ledger)
            [[ $# -ge 2 ]] || { usage; exit 2; }
            LEDGER="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            usage
            exit 2
            ;;
    esac
done

if [[ -z "$LEDGER" ]]; then
    LEDGER="$ROOT/docs/build-plan/BUILD_STATUS.md"
fi

python3 - "$LEDGER" <<'PY'
import re
import sys
from dataclasses import dataclass
from pathlib import Path

ledger = Path(sys.argv[1])
STATUS_WORDS = ("TODO", "CLAIMED", "IN_REVIEW", "MERGED", "BLOCKED")
BRIEF_RE = re.compile(
    r"(?<![A-Za-z0-9_])([A-Za-z][A-Za-z0-9]+(?:-\s*[A-Za-z0-9]+)+)(?![A-Za-z0-9_])"
)


@dataclass(frozen=True)
class Reference:
    brief: str
    line: int
    start: int
    end: int


def parse_row(line: str):
    if not line.startswith("| "):
        return None
    cells = line.rstrip("\n").split("|")
    if len(cells) < 6:
        return None
    brief = cells[1].strip()
    owner = cells[2].strip()
    status = cells[3].strip()
    if not brief or brief.lower() == "brief" or brief.startswith("**"):
        return None
    if not owner or not status:
        return None
    return brief, status


try:
    lines = ledger.read_text(encoding="utf-8").splitlines()
except OSError as exc:
    print(f"error: cannot read {ledger}: {exc}", file=sys.stderr)
    sys.exit(2)

header_lines: list[tuple[int, str]] = []
rows: dict[str, str] = {}
in_table = False
for line_no, line in enumerate(lines, 1):
    parsed = parse_row(line)
    if parsed is not None:
        in_table = True
        brief, status = parsed
        rows.setdefault(brief, status)
        continue
    if not in_table:
        header_lines.append((line_no, re.sub(r"^>\s?", "", line)))

if not rows:
    print(f"error: no ledger rows found in {ledger}", file=sys.stderr)
    sys.exit(2)

header_text = "\n".join(text for _, text in header_lines)
line_starts = []
pos = 0
for line_no, text in header_lines:
    line_starts.append((pos, line_no))
    pos += len(text) + 1


def line_for(offset: int) -> int:
    current = header_lines[0][0] if header_lines else 1
    for start, line_no in line_starts:
        if start > offset:
            break
        current = line_no
    return current


def normalize_brief(raw: str) -> str:
    return re.sub(r"-\s+", "-", raw.strip())


cleaned_by_line = {line_no: text for line_no, text in header_lines}


def line_window(ref: Reference) -> str:
    parts = []
    for line_no in range(ref.line - 1, ref.line + 3):
        text = cleaned_by_line.get(line_no)
        if text is not None:
            parts.append(text.strip())
    return " ".join(parts)


refs: list[Reference] = []
seen_refs = set()
for match in BRIEF_RE.finditer(header_text):
    brief = normalize_brief(match.group(1))
    if brief not in rows:
        continue
    key = (brief, match.start())
    if key in seen_refs:
        continue
    seen_refs.add(key)
    refs.append(Reference(brief, line_for(match.start()), match.start(), match.end()))

failures: list[str] = []
seen_failures = set()


def add(ref: Reference, message: str):
    key = (ref.brief, ref.line, message)
    if key in seen_failures:
        return
    seen_failures.add(key)
    failures.append(f"{ledger}:{ref.line}: {message}")


for ref in refs:
    status = rows[ref.brief]
    context = line_window(ref)
    context_lower = context.lower()

    if "pending ledger insertion" in context_lower:
        add(
            ref,
            f"header says {ref.brief} is pending ledger insertion, but row exists with status {status}",
        )

    for word in STATUS_WORDS:
        if not re.search(rf"\b{re.escape(word)}\s+rows?\b", context):
            continue
        if word != status:
            add(
                ref,
                f"header says {ref.brief} is {word}, but ledger status is {status}",
            )

if failures:
    for failure in failures:
        print(failure)
    sys.exit(1)

print("ok build status header claims")
PY
