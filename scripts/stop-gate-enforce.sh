#!/usr/bin/env bash
# stop-gate-enforce.sh — DEC-042 Stage-1 B-2 load-bearing backstop (the layer the
# thing-being-graded cannot skip). Sourced by scripts/claude-worker-loop.sh.
#
# It defines ONE function, stop_gate_enforce, called by the worker loop AFTER a
# session returns and AFTER the existing ship-check. By then the row is either
# already released to TODO (zombie, no PR — handled by the ship-check) or it is
# IN_REVIEW (a PR was opened). For the IN_REVIEW case this re-runs the single
# checkable verifier (scripts/stop-gate-verify.sh) over the pushed branch; if the
# gate FAILS, the row must NOT present as done — it is moved IN_REVIEW->NEEDS-FIX
# with the reason and the reason is posted as a PR comment.
#
# No-op (return 0, no side effects) unless ARGO_STOP_GATE=1 — so with the flag
# unset the worker loop is byte-identical to pre-B-2.
#
# Sourcing contract: relies on the loop's with_ledger_lock + ledger_commit_push
# for the serialized commit+push, but is testable standalone — when those are
# absent the row is still mutated on disk and the commit step is skipped (the
# meta-verifier scripts/test-stop-gate.sh exercises exactly this path).
#
# Test seams (env):
#   STOP_GATE_VERIFY_CMD     — replace the stop-gate-verify.sh invocation
#   STOP_GATE_SKIP_PR_COMMENT=1 — do not call `gh pr comment`

# $1 = brief id, $2 = branch name, $3 = repo path
stop_gate_enforce() {
  local brief="$1" branch="$2" repo="$3"
  [ "${ARGO_STOP_GATE:-0}" = "1" ] || return 0

  local ledger="$repo/docs/build-plan/BUILD_STATUS.md"
  [ -f "$ledger" ] || return 0

  # Only grade a row that is IN_REVIEW (a PR exists). CLAIMED/TODO/BLOCKED have
  # no opened PR to verify; the existing ship-check owns the zombie case.
  local row
  row="$(grep -E "^\| ${brief} \|" "$ledger" 2>/dev/null | head -1)"
  printf '%s' "$row" | grep -qE "\| IN_REVIEW \|" || return 0

  local pr
  pr="$(printf '%s' "$row" | grep -oE '#[0-9]+' | head -1 | tr -d '#')"

  echo "[$(date +%H:%M:%S)] 🔎 stop-gate: re-verifying $brief (branch $branch) before accepting IN_REVIEW..." >&2

  local out rc reason
  if [ -n "${STOP_GATE_VERIFY_CMD:-}" ]; then
    out="$(ARGO_STOP_GATE=1 bash -c "$STOP_GATE_VERIFY_CMD" 2>&1)"; rc=$?
  else
    out="$(ARGO_STOP_GATE=1 bash "$repo/scripts/stop-gate-verify.sh" \
            --brief "$brief" --branch "$branch" --repo "$repo" 2>&1)"; rc=$?
  fi

  if [ "$rc" -eq 0 ]; then
    echo "[$(date +%H:%M:%S)] ✅ stop-gate: $brief passed gate-verify; IN_REVIEW stands." >&2
    return 0
  fi

  reason="$(printf '%s\n' "$out" | grep -E '^STOP_GATE_FAIL:' | head -1 | sed 's/^STOP_GATE_FAIL: *//')"
  [ -n "$reason" ] || reason="gate failed (rc=$rc)"
  echo "[$(date +%H:%M:%S)] 🛑 stop-gate: $brief FAILED gate-verify — $reason; moving IN_REVIEW->NEEDS-FIX." >&2

  # Flip the row so it can never *present* as done. NEEDS-FIX is deliberately
  # NOT TODO: the awk picker only re-claims TODO, so a held row waits for a human
  # / conductor to address the gate failure rather than being silently re-run.
  STOP_GATE_REASON="$reason" python3 - "$brief" "$repo" <<'PY'
import re, os, sys, pathlib
brief, repo = sys.argv[1:3]
reason = os.environ.get("STOP_GATE_REASON", "gate failed")
p = pathlib.Path(repo) / "docs/build-plan/BUILD_STATUS.md"
text = p.read_text()
note = (f" **HELD by stop-gate (DEC-042 B-2 — done != verified):** PR opened but "
        f"the checkable gate failed: {reason}. Row moved IN_REVIEW->NEEDS-FIX; fix "
        f"the gate failure and refresh the PR.")
new, n = re.subn(
    rf"^(\| {re.escape(brief)} \| (?:claude|codex)) \| IN_REVIEW \|(.*) \|$",
    lambda m: f"{m.group(1)} | NEEDS-FIX |{m.group(2)}{note} |",
    text, count=1, flags=re.MULTILINE,
)
if n:
    p.write_text(new)
PY

  # Serialized commit+push only when the loop's helpers are present (they are in
  # the worker loop; absent in the standalone meta-verifier, which asserts the
  # on-disk row directly).
  if command -v with_ledger_lock >/dev/null 2>&1 && command -v ledger_commit_push >/dev/null 2>&1; then
    with_ledger_lock ledger_commit_push "status: stop-gate held $brief (gate failed: $reason)"
  fi

  # Post the reason on the PR so a failing-gate change never silently looks ready.
  if [ "${STOP_GATE_SKIP_PR_COMMENT:-0}" != "1" ] && [ -n "$pr" ] && command -v gh >/dev/null 2>&1; then
    gh pr comment "$pr" --body "🛑 **stop-gate (DEC-042 B-2) held this PR.** \`done\` binds to gate-verified, and the gate failed:

\`\`\`
${reason}
\`\`\`

The ledger row was moved IN_REVIEW→NEEDS-FIX. Fix the failure above and refresh the PR." 2>&1 | tail -2 || true
  fi

  return 1
}
