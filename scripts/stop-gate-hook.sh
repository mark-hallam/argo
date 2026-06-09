#!/usr/bin/env bash
# stop-gate-hook.sh — DEC-042 Stage-1 B-2 defence-in-depth Stop hook (the
# faithful-to-DEC layer: the worker session itself refuses to finish "done" when
# the gate fails).
#
# WHY THIS LIVES IN scripts/ AND NOT .claude/: the repo's .gitignore excludes
# `.claude/` (it is intentionally operator-local, per-machine). So the hook
# SCRIPT is tracked here, and the wiring stays in an operator-local
# `.claude/settings.json` (not committed). Wiring snippet:
#
#   { "hooks": { "Stop": [ { "hooks": [
#       { "type": "command", "command": "bash scripts/stop-gate-hook.sh" } ] } ] } }
#
# CONTRACT + CAVEAT: a Claude Code Stop hook reads a JSON event on stdin and may
# print {"decision":"block","reason":"..."} to ask the session not to stop. In
# headless `claude -p` mode this MAY be advisory only (unverified upstream as of
# this commit — see the brief's grep-before-write item). Therefore this hook is
# DEFENCE-IN-DEPTH, not the enforcer: the load-bearing block lives in the bash
# wrapper scripts/stop-gate-enforce.sh, which runs OUTSIDE the graded session.
#
# SAFETY: no-op (exit 0, allow stop) unless ARGO_STOP_GATE=1. Even with the flag
# on, it only grades when the brief + branch are known via env, and it only runs
# the (potentially minutes-long) verifier when ARGO_STOP_GATE_HOOK_ENFORCE=1 —
# otherwise it logs an advisory note and allows the stop, so it can never wedge a
# session mid-stream. Reliability-first (CLAUDE.md priority #1).
#
# Env it reads (exported by the worker loop when it opts in):
#   ARGO_STOP_GATE=1                 — master flag (default OFF)
#   ARGO_STOP_GATE_HOOK_ENFORCE=1    — actually run the verifier + block (default OFF)
#   ARGO_STOP_GATE_BRIEF=<id>        — brief under grade
#   ARGO_STOP_GATE_BRANCH=<branch>   — pushed feature branch

set -uo pipefail

# Drain stdin (the Stop event JSON); we don't currently need its fields.
cat >/dev/null 2>&1 || true

# Disabled → allow the stop, byte-identical to no hook at all.
if [ "${ARGO_STOP_GATE:-0}" != "1" ]; then
  exit 0
fi

BRIEF="${ARGO_STOP_GATE_BRIEF:-}"
BRANCH="${ARGO_STOP_GATE_BRANCH:-}"
REPO="$(cd "$(dirname "$0")/.." && pwd)"

# Without brief context we cannot grade — allow the stop (the wrapper backstop
# will still catch a failing gate after the session ends).
if [ -z "$BRIEF" ]; then
  exit 0
fi

# Advisory mode (default): do not run the heavy verifier inside the live session.
if [ "${ARGO_STOP_GATE_HOOK_ENFORCE:-0}" != "1" ]; then
  echo "[stop-gate-hook] advisory: ARGO_STOP_GATE on; the bash-wrapper backstop will gate $BRIEF after exit." >&2
  exit 0
fi

# Enforce mode: run the verifier; emit a block decision on failure.
out="$(ARGO_STOP_GATE=1 bash "$REPO/scripts/stop-gate-verify.sh" \
        --brief "$BRIEF" --branch "$BRANCH" --repo "$REPO" 2>&1)"
rc=$?
if [ "$rc" -eq 0 ]; then
  exit 0
fi
reason="$(printf '%s\n' "$out" | grep -E '^STOP_GATE_FAIL:' | head -1 | sed 's/^STOP_GATE_FAIL: *//')"
[ -n "$reason" ] || reason="stop-gate verifier failed (rc=$rc)"
# Block decision: ask the session not to finish "done".
printf '{"decision":"block","reason":"stop-gate (DEC-042 B-2): %s"}\n' \
  "$(printf '%s' "$reason" | sed 's/\\/\\\\/g; s/"/\\"/g')"
exit 0
