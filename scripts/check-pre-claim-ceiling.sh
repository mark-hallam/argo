#!/usr/bin/env bash
# check-pre-claim-ceiling.sh — refuse claim if a may-touch .rs file is >480 LOC.
#
# Hard guard against the #1066 pattern: brief author missed that
# crates/argo-agent-runner/src/turn.rs was at 496/500 LOC; worker shipped
# +57 LOC pushing it to 553, violating HARD RULE 6.
#
# This check fires BEFORE claim-with-retry commits the CLAIMED status,
# so a brief that needs a split-prep doesn't burn a claim cycle.
#
# Override: set ARGO_CLAIM_FORCE=1 to skip the check (rare — only when
# the conductor has explicitly authored a brief that knows it's pushing
# past the ceiling and accepts the consequences).
#
# Usage: scripts/check-pre-claim-ceiling.sh <brief-id>
# Exits 0 = OK to claim (no ceiling concerns)
# Exits 1 = REFUSE (some may-touch file is >480 LOC)
# Exits 2 = brief file not found / invocation error

set -uo pipefail

BRIEF_ID="${1:-}"
if [[ -z "$BRIEF_ID" ]]; then
    echo "usage: $0 <brief-id>" >&2
    exit 2
fi

if [[ "${ARGO_CLAIM_FORCE:-0}" == "1" ]]; then
    echo "check-pre-claim-ceiling: ARGO_CLAIM_FORCE=1 — skipping LOC check" >&2
    exit 0
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BRIEF="$REPO_ROOT/docs/build-plan/streams/$BRIEF_ID.md"

if [[ ! -f "$BRIEF" ]]; then
    echo "FAIL: brief file not found: $BRIEF" >&2
    exit 2
fi

# Auto-skip for split-prep / extract-helper / refactor briefs. These
# intentionally touch at-risk files because the brief's purpose is
# to reduce LOC. Detection mirrors validate-brief.sh's logic.
case "$BRIEF_ID" in
    *-split|*-split-*|*-extract-helper|*-extract|*-refactor|*-refactor-*)
        echo "check-pre-claim-ceiling: split/refactor brief — LOC check skipped" >&2
        exit 0
        ;;
esac
h1=$(head -1 "$BRIEF")
if [[ "$h1" =~ split|refactor|extract ]]; then
    echo "check-pre-claim-ceiling: split/refactor brief (H1) — LOC check skipped" >&2
    exit 0
fi

VIOLATIONS=""
in_may_touch=0
while IFS= read -r line; do
    if [[ "$line" =~ ^"## Files you may touch" ]]; then in_may_touch=1; continue; fi
    if [[ "$line" =~ ^"## " && $in_may_touch -eq 1 ]]; then in_may_touch=0; fi
    if [[ $in_may_touch -eq 1 && "$line" =~ \`(crates/[^\`]+\.rs)\` ]]; then
        path="${BASH_REMATCH[1]}"
        # Skip NEW-file markers (those start at 0 LOC)
        if [[ "$line" =~ \(NEW ]]; then continue; fi
        if [[ -f "$REPO_ROOT/$path" ]]; then
            loc=$(wc -l < "$REPO_ROOT/$path" 2>/dev/null || echo 0)
            if [[ "$loc" -gt 480 ]]; then
                VIOLATIONS="$VIOLATIONS  $loc LOC  $path\n"
            fi
        fi
    fi
done < "$BRIEF"

if [[ -z "$VIOLATIONS" ]]; then
    exit 0
fi

cat >&2 <<EOF
REFUSE: brief $BRIEF_ID has may-touch files at-risk of 500-LOC ceiling:

$(printf "$VIOLATIONS")
This is the #1066 / HARD RULE 6 pattern: brief author missed that the
target file has insufficient headroom for the planned addition.

Recommended actions:
  1. Conductor authors a split-prep brief (e.g., claude-<file>-split)
     to extract a sibling and re-pull headroom.
  2. After the split-prep merges, the original brief becomes claimable.
  3. If the addition genuinely fits (e.g., adding a 5-LOC import to a
     496-LOC file), set ARGO_CLAIM_FORCE=1 to override and accept the
     ceiling risk.

Falling through to HARD RULE 7 (pick a different brief).
EOF
exit 1
