#!/usr/bin/env bash
# validate-brief.sh — pre-authoring sanity check for new briefs.
#
# The conductor authors briefs from full context, but without an
# automated cross-check the same patterns keep biting:
#   - turn.rs at 496/500 missed → #1066 HARD RULE 6 violation
#   - ArgoError::NotFound cited but variant doesn't exist → multiple BLOCKED
#   - Migration slot 41 cited as NEW but already taken → near-miss
#   - May-touch file path moved post-cli-split → worker premise mismatch
#   - "X is a stub" claim but stub became real impl → BLOCKED briefs
#
# This script runs before the brief is committed/queued and STRICTLY
# fails on any of those patterns. Differs from check-brief-freshness.sh
# (advisory, exit 0 on warnings) — this one BLOCKS authoring on real
# problems.
#
# Usage:
#   scripts/validate-brief.sh docs/build-plan/streams/claude-foo.md
#   scripts/validate-brief.sh --strict <brief-file>    # also fail on warnings
#
# Exit codes:
#   0 — brief is valid, OK to commit + queue
#   1 — brief has structural problems; fix before committing
#   2 — invocation error

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
STRICT=0
BRIEF=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --strict) STRICT=1; shift ;;
        --help|-h) sed -n '2,22p' "$0" | sed 's/^# \?//'; exit 0 ;;
        *) BRIEF="$1"; shift ;;
    esac
done

if [[ -z "$BRIEF" ]]; then
    echo "usage: $0 <path-to-brief.md>" >&2
    exit 2
fi

if [[ ! -f "$BRIEF" ]]; then
    echo "FAIL: brief not found: $BRIEF" >&2
    exit 2
fi

ERRORS=0
WARNINGS=0
err()  { echo "  ❌ $*" >&2; ERRORS=$((ERRORS+1)); }
warn() { echo "  ⚠️  $*" >&2; WARNINGS=$((WARNINGS+1)); }

echo "=== Validating brief: $(basename "$BRIEF") ===" >&2

# 1. Required structural sections
echo "[1/8] Checking required sections..." >&2
for section in "## Why" "## Approach" "## Files you may touch" "## Acceptance criteria" "## Branch + PR"; do
    if ! grep -q "^${section}" "$BRIEF"; then
        err "missing required section: ${section}"
    fi
done

# 2. Owner + dependencies declared
echo "[2/8] Checking ownership header..." >&2
if ! grep -qE '\*\*Owner\*\*:[[:space:]]*(claude|codex)' "$BRIEF"; then
    err "missing **Owner**: claude|codex header"
fi
if ! grep -qE '\*\*Depends on\*\*:' "$BRIEF"; then
    err "missing **Depends on**: header (use 'nothing' if no deps)"
fi

# 3. Authored-against freshness anchor
echo "[3/8] Checking freshness anchor..." >&2
if ! grep -qE '\*\*Authored against\*\*:[[:space:]]*origin/main[[:space:]]+`?[a-f0-9]{7,40}`?' "$BRIEF"; then
    warn "missing **Authored against**: origin/main <sha> on <YYYY-MM-DD> anchor"
fi

# 4. May-touch file paths exist on current main (or marked NEW)
echo "[4/8] Validating may-touch paths..." >&2
in_may_touch=0
while IFS= read -r line; do
    if [[ "$line" =~ ^"## Files you may touch" ]]; then in_may_touch=1; continue; fi
    if [[ "$line" =~ ^"## " && $in_may_touch -eq 1 ]]; then in_may_touch=0; fi
    if [[ $in_may_touch -eq 1 && "$line" =~ \`(crates/[^\`]+\.(rs|toml|sql))\` ]]; then
        path="${BASH_REMATCH[1]}"
        if [[ "$line" =~ \(NEW ]]; then continue; fi
        if [[ ! -f "$REPO_ROOT/$path" ]]; then
            err "may-touch file does not exist on current main: $path (mark as (NEW) if intended)"
        fi
    fi
done < "$BRIEF"

# 5. May-touch LOC ceiling (≤ 480 LOC headroom; HARD)
# Skip for split-prep briefs — they intentionally touch at-risk files to
# reduce LOC. Detection: title or filename contains "-split", "-extract-",
# or first H1 includes "split"/"refactor".
echo "[5/8] Checking LOC ceiling on may-touch files..." >&2
is_split_prep=0
brief_basename=$(basename "$BRIEF" .md)
case "$brief_basename" in
    *-split|*-split-*|*-extract-helper|*-extract|*-refactor|*-refactor-*) is_split_prep=1 ;;
esac
if [[ "$is_split_prep" -eq 0 ]]; then
    h1=$(head -1 "$BRIEF")
    if [[ "$h1" =~ split|refactor|extract ]]; then
        is_split_prep=1
    fi
fi
if [[ "$is_split_prep" -eq 1 ]]; then
    echo "  (split/refactor brief — LOC ceiling check skipped; brief's purpose is to reduce LOC)" >&2
else
    in_may_touch=0
    while IFS= read -r line; do
        if [[ "$line" =~ ^"## Files you may touch" ]]; then in_may_touch=1; continue; fi
        if [[ "$line" =~ ^"## " && $in_may_touch -eq 1 ]]; then in_may_touch=0; fi
        if [[ $in_may_touch -eq 1 && "$line" =~ \`(crates/[^\`]+\.rs)\` ]]; then
            path="${BASH_REMATCH[1]}"
            if [[ "$line" =~ \(NEW ]]; then continue; fi
            if [[ -f "$REPO_ROOT/$path" ]]; then
                loc=$(wc -l < "$REPO_ROOT/$path" 2>/dev/null || echo 0)
                if [[ "$loc" -gt 480 ]]; then
                    err "may-touch file at $loc LOC (>480) — author a split-prep brief first: $path"
                fi
            fi
        fi
    done < "$BRIEF"
fi

# 6. ArgoError variants exist
echo "[6/8] Validating ArgoError variants..." >&2
ARGO_ERROR_FILE="$REPO_ROOT/crates/argo-types/src/error.rs"
if [[ -f "$ARGO_ERROR_FILE" ]]; then
    grep -oE 'ArgoError::[A-Z][A-Za-z]+' "$BRIEF" 2>/dev/null \
      | sort -u | while read -r variant; do
          variant_name="${variant#ArgoError::}"
          if ! grep -qE "^\s+${variant_name}(\(|\s|,|$)" "$ARGO_ERROR_FILE"; then
              echo "  ❌ brief cites $variant but no such variant in argo-types/src/error.rs" >&2
          fi
      done
    # Detect if any errors fired in the subshell — count by re-running
    bad_variants=$(grep -oE 'ArgoError::[A-Z][A-Za-z]+' "$BRIEF" 2>/dev/null | sort -u | while read -r v; do
        [[ -z "$v" ]] && continue
        vn="${v#ArgoError::}"
        grep -qE "^\s+${vn}(\(|\s|,|$)" "$ARGO_ERROR_FILE" || echo "$v"
    done | wc -l | xargs)
    ERRORS=$((ERRORS + bad_variants))
fi

# 7. Migration slot collisions (HARD: brief claims slot N as NEW but it's taken)
echo "[7/8] Checking migration slot collisions..." >&2
collisions=0
while read -r slot; do
    [[ -z "$slot" ]] && continue
    [[ "$slot" -lt 10 || "$slot" -gt 99 ]] && continue
    raw_match=$(ls "$REPO_ROOT"/crates/argo-store/migrations/${slot}__*.sql 2>/dev/null | head -1)
    ver_match=$(ls "$REPO_ROOT"/crates/argo-store/src/migrations/v${slot}.rs 2>/dev/null | head -1)
    if [[ -n "$raw_match" || -n "$ver_match" ]]; then
        if grep -qE "(NEW migration|NEW \*\*\*).*${slot}" "$BRIEF" \
           || grep -qE "${slot}__[a-z_]+\.sql.*NEW" "$BRIEF" \
           || grep -qE "v${slot}\.rs.*NEW" "$BRIEF"; then
            err "brief claims migration slot $slot as NEW but raw=$raw_match versioned=$ver_match exists"
            collisions=$((collisions + 1))
        fi
    fi
done < <(grep -oE '(migrations/|/v|slot )[0-9]{1,3}(__|\.rs|[^0-9])' "$BRIEF" 2>/dev/null | grep -oE '[0-9]{1,3}' | sort -un)

# 8. Stub claims still hold (any "is a stub" claim must match a file with stub markers)
echo "[8/8] Validating stub claims..." >&2
grep -nE 'is (currently )?a stub|stub (executor|handler|impl)|placeholder (executor|impl)' "$BRIEF" 2>/dev/null \
  | while IFS=: read -r lineno text; do
      while [[ "$text" =~ \`([a-zA-Z_/.]+\.rs)\` ]]; do
          path="${BASH_REMATCH[1]}"
          text="${text#*\`*\`}"
          full="$REPO_ROOT/$path"
          if [[ -f "$full" ]] && ! grep -qE 'unimplemented!|todo!\(\)|not yet implemented|placeholder' "$full"; then
              echo "  ❌ brief claims $path is a stub (line $lineno) but file has no stub markers" >&2
          fi
      done
  done

# Summary
echo "" >&2
if [[ "$ERRORS" -eq 0 && ( "$STRICT" -eq 0 || "$WARNINGS" -eq 0 ) ]]; then
    echo "✅ Brief validates clean ($WARNINGS warning(s), 0 error(s)). OK to commit + queue." >&2
    exit 0
else
    echo "❌ Brief has $ERRORS error(s) and $WARNINGS warning(s). Fix before committing." >&2
    if [[ "$STRICT" -eq 1 && "$WARNINGS" -gt 0 ]]; then
        echo "   (--strict: warnings escalated to errors)" >&2
    fi
    exit 1
fi
