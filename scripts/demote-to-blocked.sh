#!/usr/bin/env bash
# demote-to-blocked.sh — idempotent ledger flip from TODO → BLOCKED.
#
# Called by scripts/worker-supervisor.sh when scripts/check-runaway-zombie-claims.sh
# returns exit 3 (≥block-threshold zombie cycles on one brief in window).
# The supervisor must demote the row so the next iteration picks a
# different TODO instead of re-entering the storm. A BLOCKED row sits in
# the ledger until the conductor or operator manually investigates.
#
# Usage:
#   bash scripts/demote-to-blocked.sh <brief-id> <reason-text>
#
# Idempotency: running twice on the same row leaves a single BLOCKED row.
# If the row is already non-TODO (concurrent worker just claimed it, or a
# prior call already flipped it), this exits 0 silently with no push.

set -uo pipefail

BRIEF_ID="${1:-}"
REASON="${2:-runaway-zombie-claim guard}"

if [[ -z "$BRIEF_ID" ]]; then
  echo "usage: $0 <brief-id> [reason]" >&2
  exit 64
fi

REPO_ROOT="${ARGO_DEMOTE_REPO:-${REPO:-$(cd "$(dirname "$0")/.." && pwd)}}"
LEDGER="$REPO_ROOT/docs/build-plan/BUILD_STATUS.md"

if [[ ! -f "$LEDGER" ]]; then
  echo "demote-to-blocked: ledger not found at $LEDGER" >&2
  exit 1
fi

# Fail-safe under concurrent worker pushes: sync to latest origin/main
# before reading the row. Errors are non-fatal (the regex pass below is
# idempotent and a no-op if the row no longer matches).
git -C "$REPO_ROOT" fetch origin main >/dev/null 2>&1 || true
git -C "$REPO_ROOT" checkout main >/dev/null 2>&1 || true
git -C "$REPO_ROOT" pull --ff-only origin main >/dev/null 2>&1 || true

# Only flip if the row is currently TODO. Owner column is permissive
# ([^|]+) — matches the post-2026-05-27 code-doc alignment in
# scripts/worker-supervisor.sh and scripts/claude-worker-loop.sh.
if ! grep -qE "^\| ${BRIEF_ID} \| [^|]+ \| TODO \|" "$LEDGER" 2>/dev/null; then
  exit 0
fi

python3 - "$BRIEF_ID" "$LEDGER" "$REASON" <<'PY'
import pathlib
import re
import sys

brief_id, ledger_path, reason = sys.argv[1:4]
path = pathlib.Path(ledger_path)
text = path.read_text()
note = (
    f" **BLOCKED by {reason}:** worker exited without opening PR "
    "≥block-threshold times in window; manual conductor/operator "
    "review required before re-claiming. See "
    "scripts/check-runaway-zombie-claims.sh."
)
pattern = re.compile(
    rf"^(\| {re.escape(brief_id)} \| [^|]+) \| TODO \|(.*) \|$",
    re.MULTILINE,
)
new, count = pattern.subn(
    lambda m: f"{m.group(1)} | BLOCKED |{m.group(2)}{note} |",
    text,
    count=1,
)
if count:
    path.write_text(new)
PY

# If python made no edit (race with another writer between the grep and
# now), the working tree is clean and the commit will fail silently.
git -C "$REPO_ROOT" add docs/build-plan/BUILD_STATUS.md >/dev/null 2>&1 || true
if git -C "$REPO_ROOT" diff --cached --quiet >/dev/null 2>&1; then
  exit 0
fi

git -C "$REPO_ROOT" commit -m "status: BLOCKED ${BRIEF_ID} (${REASON})" >/dev/null 2>&1 || {
  echo "demote-to-blocked: commit failed for $BRIEF_ID" >&2
  exit 1
}
git -C "$REPO_ROOT" pull --rebase origin main >/dev/null 2>&1 || true
git -C "$REPO_ROOT" push origin main >/dev/null 2>&1 || {
  echo "demote-to-blocked: push failed for $BRIEF_ID (will retry next iter)" >&2
  exit 1
}
exit 0
