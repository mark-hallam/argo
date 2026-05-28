#!/usr/bin/env bash
# Install the conductor/worker pre-commit hooks into .git/hooks/.
#
# The hook chains two checks on every commit:
#
#   1. scripts/lint-brief.sh on every new/modified docs/build-plan/streams/*.md
#      staged for commit (WARN by default; enforce via
#      `git config skycot.brief-lint enforce`).
#
#   2. scripts/check-ledger-not-eaten-by-stash-pop.sh --staged whenever
#      docs/build-plan/BUILD_STATUS.md is staged. Refuses commits that have
#      silently dropped a TODO/CLAIMED ledger row (the classic
#      `git stash pop`-after-worker-push failure mode documented in
#      streams/ops-conductor-protect-against-stash-pop-eating-ledger-rows.md).
#      WARN downgrade available via `git config skycot.ledger-guard warn`.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# Use --git-common-dir so installs from inside a worktree (where .git is a
# file pointing back to a worktree subdir) land in the shared hooks dir.
HOOKS_DIR="$(git -C "$REPO_ROOT" rev-parse --git-common-dir 2>/dev/null)/hooks"
HOOK_PATH="${HOOKS_DIR}/pre-commit"
mkdir -p "$HOOKS_DIR"
EXISTING=""

# Treat a previous-generation install (brief-lint only, no ledger-guard) as
# upgrade-eligible: back it up so this run replaces it with the chained hook.
if [[ -f "$HOOK_PATH" ]]; then
  if grep -q 'skycot:brief-lint' "$HOOK_PATH" 2>/dev/null \
     && grep -q 'skycot:ledger-guard' "$HOOK_PATH" 2>/dev/null; then
    echo "[install-brief-hooks] hook already installed at $HOOK_PATH"
    exit 0
  fi
  EXISTING="${HOOK_PATH}.bak.$(date +%s)"
  cp "$HOOK_PATH" "$EXISTING"
  echo "[install-brief-hooks] existing pre-commit backed up to $EXISTING"
fi

cat >"$HOOK_PATH" <<'HOOK_EOF'
#!/usr/bin/env bash
# skycot:brief-lint + skycot:ledger-guard pre-commit hook.
set -uo pipefail

repo_root="$(git rev-parse --show-toplevel)"
exit_code=0

# ---- skycot:brief-lint ---------------------------------------------------
brief_mode="$(git config --get skycot.brief-lint 2>/dev/null || echo warn)"
brief_enforce_flag=""
if [[ "$brief_mode" == "enforce" ]]; then
  brief_enforce_flag="--enforce"
fi

staged_briefs="$(git diff --cached --name-only --diff-filter=AM -- 'docs/build-plan/streams/*.md' 2>/dev/null || true)"
if [[ -n "$staged_briefs" ]]; then
  while IFS= read -r path; do
    [[ -z "$path" ]] && continue
    brief="$(basename "$path" .md)"
    if ! "$repo_root/scripts/lint-brief.sh" $brief_enforce_flag "$brief"; then
      exit_code=1
    fi
  done <<<"$staged_briefs"

  if [[ "$exit_code" -ne 0 ]]; then
    echo "" >&2
    echo "[skycot:brief-lint] one or more briefs failed lint in enforce mode." >&2
    echo "  to bypass once: git commit --no-verify" >&2
    echo "  to switch to WARN: git config skycot.brief-lint warn" >&2
  fi
fi

# ---- skycot:ledger-guard -------------------------------------------------
# Script may be absent on branches predating its addition; skip silently
# in that case so the hook stays installable on any checkout.
ledger_guard="$repo_root/scripts/check-ledger-not-eaten-by-stash-pop.sh"
staged_ledger="$(git diff --cached --name-only --diff-filter=AM -- 'docs/build-plan/BUILD_STATUS.md' 2>/dev/null || true)"
if [[ -n "$staged_ledger" && -x "$ledger_guard" ]]; then
  ledger_mode="$(git config --get skycot.ledger-guard 2>/dev/null || echo enforce)"
  ledger_flags=(--staged)
  if [[ "$ledger_mode" == "warn" ]]; then
    ledger_flags+=(--warn)
  fi
  if ! "$ledger_guard" "${ledger_flags[@]}"; then
    exit_code=1
    echo "" >&2
    echo "[skycot:ledger-guard] BUILD_STATUS.md commit refused." >&2
    echo "  to bypass once:    git commit --no-verify" >&2
    echo "  to soft-mode globally: git config skycot.ledger-guard warn" >&2
  fi
fi

exit "$exit_code"
HOOK_EOF

chmod +x "$HOOK_PATH"
echo "[install-brief-hooks] pre-commit hook installed at $HOOK_PATH"
echo "[install-brief-hooks] brief-lint:   WARN (set git config skycot.brief-lint enforce to block)"
echo "[install-brief-hooks] ledger-guard: ENFORCE (set git config skycot.ledger-guard warn to soft-mode)"
