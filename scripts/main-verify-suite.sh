#!/usr/bin/env bash
# main-verify-suite.sh — the curated critical-path verification of ONE checkout.
#
# Phase A: compile guard — `cargo check --workspace --tests` (catches the #1
#          failure mode: a merge that does not even build, on any crate/test).
# Phase B: curated critical verifiers from `main-verify-critical-tests.txt`.
#          Each line MUST yield >=1 passed and 0 non-allowlisted failures.
#          A critical filter that matches ZERO tests is treated as RED — a
#          clobbered/renamed verifier is EXACTLY the failure mode this guards
#          (the #2936 convergence fix was silently reverted once; its verifier
#          is the canary line below).
#
# Exit 0 = GREEN, 1 = RED. Prints a summary; the CALLER (the sweep daemon or the
# conductor) owns status.json + the merge-freeze sentinel. Run hermetically — only
# tests with no external deps belong in the critical list.
#
# Usage: bash scripts/main-verify-suite.sh --worktree <path>
#        MAIN_VERIFY_INCLUDE_CU=1  also runs the slow computer-use claim_5 test.
set -uo pipefail

WT="."
while [ $# -gt 0 ]; do case "$1" in
  --worktree) WT="$2"; shift 2;;
  --worktree=*) WT="${1#*=}"; shift;;
  *) shift;;
esac; done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRIT="$SCRIPT_DIR/main-verify-critical-tests.txt"
ALLOW="$SCRIPT_DIR/main-verify-flake-allowlist.txt"
INCLUDE_CU="${MAIN_VERIFY_INCLUDE_CU:-0}"

cd "$WT" || { echo "MV-SUITE: cannot cd '$WT'"; exit 1; }
[ -f "$CRIT" ] || { echo "MV-SUITE: missing critical-tests config $CRIT"; exit 1; }

# Ephemeral DB key — the macOS keychain-hang workaround (gateway-spawn tests block
# on the Keychain otherwise). Caller may pre-set one.
export SKYCOT_DB_KEY_BASE64="${SKYCOT_DB_KEY_BASE64:-$(python3 -c 'import base64,os;print(base64.b64encode(os.urandom(32)).decode())')}"

red=0; fails=""

allowlisted() {  # is failing test name $1 substring-listed in the flake allowlist?
  [ -f "$ALLOW" ] || return 1
  grep -vE '^[[:space:]]*#|^[[:space:]]*$' "$ALLOW" | grep -qiF -- "$1"
}

echo "MV-SUITE: Phase A — cargo check --workspace --tests"
if ! cargo check --workspace --tests >/tmp/mv-phaseA.log 2>&1; then
  echo "MV-SUITE: PHASE A FAIL — compile break:"
  grep -E 'error(\[|:)' /tmp/mv-phaseA.log | head -12
  echo "MV-SUITE: RESULT RED (compile)"
  echo "FAILS=compile:workspace-tests"
  exit 1
fi
echo "MV-SUITE: Phase A OK (everything compiles)"

echo "MV-SUITE: Phase B — curated critical verifiers"
while IFS= read -r raw; do
  line="${raw%%#*}"
  # shellcheck disable=SC2116
  line="$(echo $line)"   # trim surrounding whitespace via word-splitting
  [ -z "$line" ] && continue
  case "$line" in
    CU:*) if [ "$INCLUDE_CU" != "1" ]; then echo "  skip (CU-gated): ${line#CU:}"; continue; fi; line="${line#CU:}";;
  esac
  echo "  run: cargo test $line"
  out="$(cargo test $line 2>&1)"; ec=$?
  # failing test names (the "test <name> ... FAILED" lines)
  fnames="$(printf '%s\n' "$out" | sed -nE 's/^test (.+) \.\.\. FAILED$/\1/p')"
  if [ -n "$fnames" ]; then
    while IFS= read -r fn; do
      [ -z "$fn" ] && continue
      if allowlisted "$fn"; then echo "    tolerated known-flake: $fn"
      else echo "    FAIL (not allowlisted): $fn"; red=1; fails="$fails $fn"; fi
    done <<< "$fnames"
  fi
  # 0 tests passed for a critical line => the verifier vanished / was renamed /
  # was clobbered, OR the target failed to build. Either way: RED.
  if ! printf '%s\n' "$out" | grep -qE '[1-9][0-9]* passed'; then
    echo "    CRITICAL VERIFIER MATCHED 0 PASSING TESTS — RED (clobbered/renamed? cargo exit=$ec)"
    printf '%s\n' "$out" | grep -E 'error(\[|:)|test result:|warning: unused' | head -6
    red=1; fails="$fails zero-pass:[$line]"
  fi
done < "$CRIT"

if [ "$red" -eq 0 ]; then echo "MV-SUITE: RESULT GREEN"; exit 0; fi
echo "MV-SUITE: RESULT RED —$fails"
echo "FAILS=$fails"
exit 1
