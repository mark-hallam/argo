#!/usr/bin/env bash
# Smoke tests for scripts/check-demo-roster.sh.
#
# Exercises the FAKE_TEAM_BODY / FAKE_SKILLS_BODY / FAKE_VERTICALS_BODY
# hooks so the harness can assert exit codes without binding a real
# gateway. Six scenarios, covering every documented exit code:
#   (0) GREEN — 5 expected workers + skills + verticals → exit 0
#   (1) ROSTER-FAIL — 1 worker ("Maya") → exit 1, missing personas listed
#   (2) ROSTER-FAIL — 5 workers but wrong names → exit 1
#   (3) ROSTER-FAIL — 0 skills → exit 1
#   (4) ROSTER-FAIL — 0 verticals → exit 1
#   (5) UNREACHABLE — port has no listener → exit 2

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null)"
[[ -n "$REPO_ROOT" ]] || { echo "must run inside argo git repo" >&2; exit 1; }
cd "$REPO_ROOT"

SCRIPT="$REPO_ROOT/scripts/check-demo-roster.sh"
[[ -x "$SCRIPT" ]] || { echo "script not executable: $SCRIPT" >&2; exit 1; }

# Pick a port that is almost-certainly free for the UNREACHABLE case.
PORT=$(( (RANDOM % 5000) + 41000 ))
for _ in $(seq 1 10); do
    if ! lsof -nP -iTCP:"$PORT" -sTCP:LISTEN -t >/dev/null 2>&1; then
        break
    fi
    PORT=$((PORT + 1))
done

OK_SKILLS='{"skills":[{"id":"intro_to_voicemail","name":"intro_to_voicemail"}],"count":1}'
OK_VERTS='{"verticals":[{"id":"marketplace","name":"Marketplace"}]}'
EMPTY_SKILLS='{"skills":[],"count":0}'
EMPTY_VERTS='{"verticals":[]}'

build_team() {
    # $1 = comma-separated worker names. Empty -> empty workers[].
    local names="$1"
    if [[ -z "$names" ]]; then
        printf '{"workers":[]}'
        return
    fi
    printf '{"workers":['
    local first=1
    local IFS=','
    for n in $names; do
        [[ $first -eq 1 ]] || printf ','
        printf '{"name":"%s","id":"fake-%s"}' "$n" "$n"
        first=0
    done
    printf ']}'
}

run_check() {
    local team="$1" skills="$2" verts="$3" url="${4:-http://127.0.0.1:$PORT}"
    set +e
    OUT="$(FAKE_TEAM_BODY="$team" FAKE_SKILLS_BODY="$skills" FAKE_VERTICALS_BODY="$verts" \
        "$SCRIPT" "$url" 2>&1)"
    RC=$?
    set -e
}

run_unreachable() {
    set +e
    OUT="$("$SCRIPT" "http://127.0.0.1:$PORT" 2>&1)"
    RC=$?
    set -e
}

pass() { printf '  ✅ %s\n' "$*"; }
fail() { printf '  ❌ %s\n' "$*" >&2; printf '%s\n' "$OUT" | sed 's/^/      /' >&2; exit 1; }

# ---------- scenario 0: GREEN ---------------------------------------

echo "Scenario 0: canonical 5-worker roster + non-empty skills + verticals → exit 0."
TEAM="$(build_team 'Mariah,Peta,Priya,Jamal,Nori')"
run_check "$TEAM" "$OK_SKILLS" "$OK_VERTS"
[[ $RC -eq 0 ]] || fail "expected exit 0, got $RC"
printf '%s\n' "$OUT" | grep -q "STATUS: GREEN" || fail "missing STATUS: GREEN"
printf '%s\n' "$OUT" | grep -q "roster=5 skills=1 verticals=1" || fail "missing summary line"
pass "GREEN path → exit 0 + summary line"

# ---------- scenario 1: ROSTER-FAIL — 1 worker ("Maya") -------------

echo "Scenario 1: gateway returns single 'Maya' worker → exit 1 with all 5 missing."
TEAM="$(build_team 'Maya')"
run_check "$TEAM" "$OK_SKILLS" "$OK_VERTS"
[[ $RC -eq 1 ]] || fail "expected exit 1, got $RC"
printf '%s\n' "$OUT" | grep -q "returned 1 worker" || fail "missing 'returned 1 worker' line"
for name in Mariah Peta Priya Jamal Nori; do
    printf '%s\n' "$OUT" | grep -q "missing personas:.*${name}" \
        || fail "missing-personas line did not include ${name}"
done
printf '%s\n' "$OUT" | grep -q "argo demo seed --tenant" \
    || fail "remediation did not include the seed command verbatim"
pass "Maya-only → exit 1 + names all 5 missing + seed hint"

# ---------- scenario 2: ROSTER-FAIL — 5 wrong names ------------------

echo "Scenario 2: 5 workers but wrong names → exit 1, only wrong ones listed."
TEAM="$(build_team 'Mariah,Peta,Priya,Bob,Alice')"
run_check "$TEAM" "$OK_SKILLS" "$OK_VERTS"
[[ $RC -eq 1 ]] || fail "expected exit 1, got $RC"
printf '%s\n' "$OUT" | grep -q "missing personas:.*Jamal" || fail "Jamal not listed missing"
printf '%s\n' "$OUT" | grep -q "missing personas:.*Nori"  || fail "Nori not listed missing"
printf '%s\n' "$OUT" | grep -q "missing personas:.*Mariah" \
    && fail "Mariah should NOT be listed missing (was in actual roster)"
pass "5 wrong names → exit 1 + only the missing ones listed"

# ---------- scenario 3: ROSTER-FAIL — skills empty -------------------

echo "Scenario 3: 5-worker roster but 0 skills → exit 1."
TEAM="$(build_team 'Mariah,Peta,Priya,Jamal,Nori')"
run_check "$TEAM" "$EMPTY_SKILLS" "$OK_VERTS"
[[ $RC -eq 1 ]] || fail "expected exit 1, got $RC"
printf '%s\n' "$OUT" | grep -q "/api/v1/skills returned 0 skills" \
    || fail "missing skills-empty message"
pass "skills empty → exit 1"

# ---------- scenario 4: ROSTER-FAIL — verticals empty ----------------

echo "Scenario 4: 5-worker roster + skills but 0 verticals → exit 1."
run_check "$TEAM" "$OK_SKILLS" "$EMPTY_VERTS"
[[ $RC -eq 1 ]] || fail "expected exit 1, got $RC"
printf '%s\n' "$OUT" | grep -q "/api/v1/verticals returned 0 verticals" \
    || fail "missing verticals-empty message"
pass "verticals empty → exit 1"

# ---------- scenario 5: UNREACHABLE ----------------------------------

echo "Scenario 5: nothing listening on \$PORT → exit 2 (UNREACHABLE)."
run_unreachable
[[ $RC -eq 2 ]] || fail "expected exit 2, got $RC"
printf '%s\n' "$OUT" | grep -q "unreachable" || fail "missing 'unreachable' substring"
pass "no listener → exit 2"

echo "✅ All 6 scenarios passed."
