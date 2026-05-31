#!/usr/bin/env bash
# Pre-record roster guard for scripts/demo-dry-run.sh.
#
# Validates that the demo gateway has been seeded with the canonical
# eight-worker roster (Mariah / Peta / Priya / Jamal / Nori / Portia /
# Grant / Marcus — read from scripts/lib/demo-roster.txt) AND
# non-zero skills + verticals catalogs. Without this guard, a
# roster-mismatched gateway (e.g. one stuck on the "Maya" boot-seed)
# passes the dry-run silently and the operator only finds out
# mid-take.
#
# Closes the F1 [P0] finding from
# docs/skycot/ux-audits/2026-05-27-0623/scene-1-and-2-claim-1-and-2.md.
#
# Exit codes:
#   0  GREEN — roster matches + skills > 0 + verticals > 0
#   1  ROSTER-FAIL — workers missing/wrong OR skills/verticals empty
#   2  UNREACHABLE — curl to a probed endpoint failed
#   3  PARSE — JSON malformed or missing expected shape
#
# Usage:
#   scripts/check-demo-roster.sh [GATEWAY_URL]
#     defaults to http://127.0.0.1:7400 (canonical demo port).
#
# Test hooks (set by scripts/check-demo-roster-tests.sh; UNSET in prod):
#   FAKE_TEAM_BODY        — substitute body for /api/v1/team
#   FAKE_SKILLS_BODY      — substitute body for /api/v1/skills
#   FAKE_VERTICALS_BODY   — substitute body for /api/v1/verticals
#   FAKE_TEAM_STATUS      — substitute HTTP status (default 200)
#   FAKE_SKILLS_STATUS    — substitute HTTP status (default 200)
#   FAKE_VERTICALS_STATUS — substitute HTTP status (default 200)
#
# A non-empty FAKE_*_BODY short-circuits the curl entirely; the
# corresponding endpoint is not contacted.

set -uo pipefail

GATEWAY="${1:-http://127.0.0.1:7400}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROSTER_FILE="${REPO_ROOT}/scripts/lib/demo-roster.txt"

command -v curl >/dev/null 2>&1 || { echo "FAIL: curl not on PATH" >&2; exit 2; }
command -v jq   >/dev/null 2>&1 || { echo "FAIL: jq not on PATH (brew install jq)" >&2; exit 2; }
[[ -f "$ROSTER_FILE" ]] || { echo "FAIL: roster file missing: $ROSTER_FILE" >&2; exit 3; }

# Read expected names into an array (skips blank lines + # comments).
EXPECTED_NAMES=()
while IFS= read -r line || [[ -n "$line" ]]; do
    line="${line%%#*}"
    line="${line#"${line%%[![:space:]]*}"}"
    line="${line%"${line##*[![:space:]]}"}"
    [[ -n "$line" ]] && EXPECTED_NAMES+=("$line")
done <"$ROSTER_FILE"
EXPECTED_COUNT="${#EXPECTED_NAMES[@]}"

print_seed_hint() {
    {
        echo ""
        echo "Run the canonical seed CLI against the demo tenant, then re-run:"
        echo ""
        echo "    argo tenant list | grep -i demo"
        echo "    export ARGO_DEMO_TENANT_ID=<uuid-from-demo-row>"
        echo "    argo demo seed --tenant \$ARGO_DEMO_TENANT_ID"
        echo ""
        echo "    bash scripts/demo-dry-run.sh --demo-mode"
    } >&2
}

# fetch_body URL FAKE_BODY FAKE_STATUS BODY_OUT STATUS_OUT
# Writes the response body to file BODY_OUT and the HTTP status to
# file STATUS_OUT. Returns 0 on success. Cannot set caller-visible
# variables via $(...) capture (subshell), so we use tempfiles.
fetch_body() {
    local url="$1" fake_body="$2" fake_status="$3" body_out="$4" status_out="$5"
    if [[ -n "$fake_body" ]]; then
        printf '%s' "$fake_body" >"$body_out"
        printf '%s' "${fake_status:-200}" >"$status_out"
        return 0
    fi
    local code
    code="$(curl -sS -o "$body_out" -w '%{http_code}' "$url" 2>/dev/null)"
    [[ -n "$code" ]] || code="000"
    printf '%s' "$code" >"$status_out"
}

TMP_BODY="$(mktemp)"
TMP_STATUS="$(mktemp)"
trap 'rm -f "$TMP_BODY" "$TMP_STATUS"' EXIT

# --- 1. /api/v1/team roster check ------------------------------------
fetch_body "$GATEWAY/api/v1/team" "${FAKE_TEAM_BODY:-}" "${FAKE_TEAM_STATUS:-}" "$TMP_BODY" "$TMP_STATUS"
TEAM_BODY="$(cat "$TMP_BODY")"
RESP_STATUS="$(cat "$TMP_STATUS")"
case "$RESP_STATUS" in
    200) ;;
    000)
        echo "FAIL: /api/v1/team unreachable at $GATEWAY (curl exit non-zero)" >&2
        print_seed_hint
        exit 2
        ;;
    *)
        echo "FAIL: /api/v1/team returned HTTP $RESP_STATUS at $GATEWAY" >&2
        echo "      body: $TEAM_BODY" >&2
        print_seed_hint
        exit 2
        ;;
esac

ACTUAL_NAMES_JSON="$(printf '%s' "$TEAM_BODY" | jq -c '.workers // empty | map(.name)' 2>/dev/null || true)"
if [[ -z "$ACTUAL_NAMES_JSON" || "$ACTUAL_NAMES_JSON" == "null" ]]; then
    echo "FAIL: /api/v1/team response missing .workers[].name array" >&2
    echo "      body: $TEAM_BODY" >&2
    exit 3
fi

actual_count="$(printf '%s' "$ACTUAL_NAMES_JSON" | jq 'length')"
missing=()
for name in "${EXPECTED_NAMES[@]}"; do
    found="$(printf '%s' "$ACTUAL_NAMES_JSON" | jq --arg n "$name" 'any(. == $n)')"
    [[ "$found" == "true" ]] || missing+=("$name")
done

if (( actual_count < EXPECTED_COUNT )) || (( ${#missing[@]} > 0 )); then
    {
        echo "FAIL: /api/v1/team returned $actual_count worker(s); need $EXPECTED_COUNT (${EXPECTED_NAMES[*]})."
        if (( ${#missing[@]} > 0 )); then
            echo "      missing personas: ${missing[*]}"
        fi
        printf '      actual roster: '
        printf '%s' "$ACTUAL_NAMES_JSON" | jq -r 'join(", ")'
    } >&2
    print_seed_hint
    exit 1
fi

# --- 2. /api/v1/skills non-empty -------------------------------------
fetch_body "$GATEWAY/api/v1/skills" "${FAKE_SKILLS_BODY:-}" "${FAKE_SKILLS_STATUS:-}" "$TMP_BODY" "$TMP_STATUS"
SKILLS_BODY="$(cat "$TMP_BODY")"
RESP_STATUS="$(cat "$TMP_STATUS")"
case "$RESP_STATUS" in
    200) ;;
    000)
        echo "FAIL: /api/v1/skills unreachable at $GATEWAY" >&2
        exit 2
        ;;
    *)
        echo "FAIL: /api/v1/skills returned HTTP $RESP_STATUS at $GATEWAY" >&2
        echo "      body: $SKILLS_BODY" >&2
        exit 2
        ;;
esac
skills_len="$(printf '%s' "$SKILLS_BODY" | jq '.skills // [] | length' 2>/dev/null || echo 0)"
if (( skills_len < 1 )); then
    echo "FAIL: /api/v1/skills returned 0 skills (expected at least 1 after seed)." >&2
    print_seed_hint
    exit 1
fi

# --- 3. /api/v1/verticals non-empty ----------------------------------
fetch_body "$GATEWAY/api/v1/verticals" "${FAKE_VERTICALS_BODY:-}" "${FAKE_VERTICALS_STATUS:-}" "$TMP_BODY" "$TMP_STATUS"
VERTS_BODY="$(cat "$TMP_BODY")"
RESP_STATUS="$(cat "$TMP_STATUS")"
case "$RESP_STATUS" in
    200) ;;
    000)
        echo "FAIL: /api/v1/verticals unreachable at $GATEWAY" >&2
        exit 2
        ;;
    *)
        echo "FAIL: /api/v1/verticals returned HTTP $RESP_STATUS at $GATEWAY" >&2
        echo "      body: $VERTS_BODY" >&2
        exit 2
        ;;
esac
verts_len="$(printf '%s' "$VERTS_BODY" | jq '.verticals // [] | length' 2>/dev/null || echo 0)"
if (( verts_len < 1 )); then
    echo "FAIL: /api/v1/verticals returned 0 verticals (expected at least 1 after seed)." >&2
    print_seed_hint
    exit 1
fi

printf 'roster=%d skills=%d verticals=%d STATUS: GREEN\n' \
    "$actual_count" "$skills_len" "$verts_len"
exit 0
