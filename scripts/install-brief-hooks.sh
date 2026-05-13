#!/usr/bin/env bash
# Install the brief-linter pre-commit hook into .git/hooks/.
#
# The hook runs scripts/lint-brief.sh in WARN mode (exit 0) on every new or
# modified docs/build-plan/streams/*.md staged for commit. Findings print to
# stderr but do not block the commit.
#
# To flip to enforce mode (blocking commits with findings), set
#   git config skycot.brief-lint enforce
# in the repo. The hook reads that config on each invocation.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HOOK_PATH="${REPO_ROOT}/.git/hooks/pre-commit"
EXISTING=""

if [[ -f "$HOOK_PATH" ]]; then
  # If we've already installed, exit fast. Otherwise back up.
  if grep -q 'skycot:brief-lint' "$HOOK_PATH" 2>/dev/null; then
    echo "[install-brief-hooks] hook already installed at $HOOK_PATH"
    exit 0
  fi
  EXISTING="${HOOK_PATH}.bak.$(date +%s)"
  cp "$HOOK_PATH" "$EXISTING"
  echo "[install-brief-hooks] existing pre-commit backed up to $EXISTING"
fi

cat >"$HOOK_PATH" <<'HOOK_EOF'
#!/usr/bin/env bash
# skycot:brief-lint pre-commit hook — runs scripts/lint-brief.sh on staged briefs.
set -uo pipefail

repo_root="$(git rev-parse --show-toplevel)"
mode="$(git config --get skycot.brief-lint 2>/dev/null || echo warn)"
enforce_flag=""
if [[ "$mode" == "enforce" ]]; then
  enforce_flag="--enforce"
fi

# Find staged briefs (added or modified) under docs/build-plan/streams/
mapfile_safe() {
  local arr_name="$1"; shift
  local result
  result="$("$@")"
  eval "${arr_name}=()"
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    eval "${arr_name}+=(\"\$line\")"
  done <<<"$result"
}

staged_files="$(git diff --cached --name-only --diff-filter=AM -- 'docs/build-plan/streams/*.md' 2>/dev/null || true)"
[[ -z "$staged_files" ]] && exit 0

exit_code=0
while IFS= read -r path; do
  [[ -z "$path" ]] && continue
  brief="$(basename "$path" .md)"
  if ! "$repo_root/scripts/lint-brief.sh" $enforce_flag "$brief"; then
    exit_code=1
  fi
done <<<"$staged_files"

if [[ "$exit_code" -ne 0 ]]; then
  echo "" >&2
  echo "[skycot:brief-lint] one or more briefs failed lint in enforce mode." >&2
  echo "  to bypass once: git commit --no-verify" >&2
  echo "  to switch to WARN: git config skycot.brief-lint warn" >&2
fi
exit "$exit_code"
HOOK_EOF

chmod +x "$HOOK_PATH"
echo "[install-brief-hooks] pre-commit hook installed at $HOOK_PATH"
echo "[install-brief-hooks] mode: WARN (findings print but do not block)"
echo "[install-brief-hooks] to enforce: git config skycot.brief-lint enforce"
