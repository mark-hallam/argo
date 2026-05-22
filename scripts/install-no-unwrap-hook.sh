#!/usr/bin/env bash
# Install the no-unwrap pre-commit hook into the repo's Git hooks dir.
#
# The hook runs scripts/check-no-unwrap-outside-test.sh when staged Rust files
# under crates/ are present. Findings warn by default and block only when:
#   git config skycot.no-unwrap-enforce enforce

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CHECKER="${REPO_ROOT}/scripts/check-no-unwrap-outside-test.sh"

if [[ ! -x "$CHECKER" ]]; then
  echo "[install-no-unwrap-hook] missing executable checker: $CHECKER" >&2
  exit 1
fi

GIT_COMMON_DIR="$(git -C "$REPO_ROOT" rev-parse --git-common-dir)"
case "$GIT_COMMON_DIR" in
  /*) ;;
  *) GIT_COMMON_DIR="${REPO_ROOT}/${GIT_COMMON_DIR}" ;;
esac

HOOK_DIR="${GIT_COMMON_DIR}/hooks"
HOOK_PATH="${HOOK_DIR}/pre-commit"
MARKER="skycot:no-unwrap"
BACKUP=""

mkdir -p "$HOOK_DIR"

if [[ -f "$HOOK_PATH" ]] && grep -q "$MARKER" "$HOOK_PATH" 2>/dev/null; then
  echo "[install-no-unwrap-hook] hook already installed at $HOOK_PATH"
  exit 0
fi

if [[ -f "$HOOK_PATH" ]]; then
  BACKUP="${HOOK_PATH}.bak.no-unwrap.$(date +%s)"
  cp "$HOOK_PATH" "$BACKUP"
  echo "[install-no-unwrap-hook] existing pre-commit backed up to $BACKUP"
fi

cat >"$HOOK_PATH" <<HOOK_EOF
#!/usr/bin/env bash
# skycot:no-unwrap pre-commit hook - checks staged Rust changes.
set -uo pipefail

repo_root="\$(git rev-parse --show-toplevel)"
mode="\$(git config --get skycot.no-unwrap-enforce 2>/dev/null || echo warn)"
backup_hook="$BACKUP"

staged_rust="\$(git diff --cached --name-only --diff-filter=AM 2>/dev/null | grep -E '^crates/.+\\.rs$' || true)"

exit_code=0
if [[ -n "\$staged_rust" ]]; then
  echo "[skycot:no-unwrap] staged Rust detected; running no-unwrap scan"
  bash "\$repo_root/scripts/check-no-unwrap-outside-test.sh"
  check_status=\$?
  if [[ "\$check_status" -ne 0 ]]; then
    if [[ "\$mode" == "enforce" ]]; then
      exit_code="\$check_status"
      echo "[skycot:no-unwrap] failed in ENFORCE mode." >&2
      echo "  to bypass once: git commit --no-verify" >&2
      echo "  to switch to WARN: git config skycot.no-unwrap-enforce warn" >&2
    else
      echo "[skycot:no-unwrap] WARN mode: findings did not block commit." >&2
      echo "  to enforce: git config skycot.no-unwrap-enforce enforce" >&2
    fi
  fi
fi

if [[ -n "\$backup_hook" && -f "\$backup_hook" ]]; then
  bash "\$backup_hook"
  backup_status=\$?
  if [[ "\$backup_status" -ne 0 ]]; then
    exit_code="\$backup_status"
  fi
fi

exit "\$exit_code"
HOOK_EOF

chmod +x "$HOOK_PATH"
echo "[install-no-unwrap-hook] pre-commit hook installed at $HOOK_PATH"
echo "[install-no-unwrap-hook] mode: WARN (findings print but do not block)"
echo "[install-no-unwrap-hook] to enforce: git config skycot.no-unwrap-enforce enforce"
