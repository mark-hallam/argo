#!/usr/bin/env bash
# bootstrap-machine.sh — validate this machine is ready to run Skycot workers.
#
# Usage:
#   bash $ARGO_REPO_ROOT/scripts/bootstrap-machine.sh
#
# Exits non-zero if anything is missing. Prints a ready-to-run worker
# launch command at the end if everything passes.
#
# Designed to be portable across macOS, Linux, and WSL2. Does NOT install
# anything — it only validates and reports.

set -uo pipefail

FAIL=0
WARN=0

red() { printf '\033[31m%s\033[0m\n' "$*"; }
yellow() { printf '\033[33m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }
bold() { printf '\033[1m%s\033[0m\n' "$*"; }

check_fail() { red "✗ $*"; FAIL=$((FAIL + 1)); }
check_warn() { yellow "⚠ $*"; WARN=$((WARN + 1)); }
check_ok() { green "✓ $*"; }

bold "=== SKYCOT bootstrap-machine check ==="
echo "Hostname: $(hostname)"
echo "OS:       $(uname -s) $(uname -r)"
echo "User:     ${USER:-unknown}"
echo "Date:     $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo

# --- Required tools ---------------------------------------------------------

bold "Required tools:"

command -v git >/dev/null && check_ok "git: $(git --version | awk '{print $3}')" \
  || check_fail "git not found — install with: macOS \`brew install git\` / Linux \`sudo apt install git\`"

command -v gh >/dev/null && check_ok "gh: $(gh --version | head -1 | awk '{print $3}')" \
  || check_fail "gh not found — install from https://cli.github.com/ then run \`gh auth login\`"

command -v cargo >/dev/null && check_ok "cargo: $(cargo --version | awk '{print $2}')" \
  || check_fail "cargo not found — install via rustup: \`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh\`"

if command -v cargo >/dev/null; then
  RUST_VER=$(cargo --version | awk '{print $2}' | cut -d. -f1-2)
  RUST_MAJOR_MINOR=$(echo "$RUST_VER" | awk -F. '{print $1*100+$2}')
  if [ "$RUST_MAJOR_MINOR" -ge 175 ]; then
    check_ok "rust version OK ($RUST_VER ≥ 1.75)"
  else
    check_warn "rust version $RUST_VER may be old; workspace requires ≥ 1.75 — \`rustup update stable\`"
  fi
fi

command -v claude >/dev/null && check_ok "claude CLI: $(claude --version 2>&1 | head -1)" \
  || check_fail "claude CLI not found — install per https://docs.anthropic.com/en/docs/claude-code"

command -v python3 >/dev/null && check_ok "python3: $(python3 --version | awk '{print $2}')" \
  || check_fail "python3 not found — needed by helper scripts"

command -v sccache >/dev/null && check_ok "sccache: $(sccache --version | awk '{print $2}') (optional, speeds builds 5-10×)" \
  || check_warn "sccache not installed (optional but recommended) — \`cargo install sccache\`"

echo

# --- Environment variables --------------------------------------------------

bold "Environment:"

if [ -n "${ANTHROPIC_API_KEY:-}" ]; then
  check_ok "ANTHROPIC_API_KEY set (last 6 chars: ...${ANTHROPIC_API_KEY: -6})"
else
  check_fail "ANTHROPIC_API_KEY not set — export in ~/.zshenv (macOS) or ~/.profile (Linux/WSL2). Non-interactive shells (Claude Code's Bash tool, scripts) do NOT source ~/.zshrc / ~/.bashrc, so vars set there will be invisible to swarm tooling. For the API key specifically, prefer direnv or a sourced gitignored env file rather than persisting a secret in ~/.zshenv."
fi

if [ -n "${ARGO_REPO_ROOT:-}" ]; then
  if [ -d "$ARGO_REPO_ROOT/.git" ]; then
    check_ok "ARGO_REPO_ROOT=$ARGO_REPO_ROOT (valid git repo)"
  else
    check_fail "ARGO_REPO_ROOT=$ARGO_REPO_ROOT does not contain a .git directory"
  fi
else
  check_warn "ARGO_REPO_ROOT not set — defaulting to current directory. Persist via ~/.zshenv (macOS) or ~/.profile (Linux/WSL2); ~/.zshrc / ~/.bashrc won't be seen by Claude Code's non-interactive Bash subshells."
  ARGO_REPO_ROOT="$(pwd)"
fi

if [ -n "${CARGO_TARGET_DIR:-}" ]; then
  check_ok "CARGO_TARGET_DIR=$CARGO_TARGET_DIR"
else
  check_warn "CARGO_TARGET_DIR not set — workers will create per-worktree target dirs (slower; ~28min cold each)"
  yellow "  recommended: export CARGO_TARGET_DIR=/tmp/argo-shared-target in ~/.zshenv (macOS) or ~/.profile (Linux/WSL2) so non-interactive shells inherit it"
fi

echo

# --- gh auth check ---------------------------------------------------------

bold "GitHub CLI auth:"
if command -v gh >/dev/null && gh auth status >/dev/null 2>&1; then
  GH_USER=$(gh api user --jq .login 2>/dev/null || echo "unknown")
  check_ok "gh authenticated as: $GH_USER"
else
  check_fail "gh not authenticated — run: \`gh auth login\`"
fi

echo

# --- Repo check ------------------------------------------------------------

bold "Repo state:"
if [ -d "$ARGO_REPO_ROOT/.git" ]; then
  cd "$ARGO_REPO_ROOT"
  if [ -n "$(git status --porcelain)" ]; then
    check_warn "working tree has uncommitted changes (may interfere with worker-loop)"
    git status --short | head -5
  else
    check_ok "working tree clean"
  fi

  CURR=$(git rev-parse --abbrev-ref HEAD)
  if [ "$CURR" = "main" ]; then
    check_ok "on main branch"
  else
    check_warn "on branch '$CURR' (workers expect main)"
  fi

  AHEAD=$(git rev-list --count origin/main..HEAD 2>/dev/null || echo "?")
  BEHIND=$(git rev-list --count HEAD..origin/main 2>/dev/null || echo "?")
  if [ "$AHEAD" = "0" ] && [ "$BEHIND" = "0" ]; then
    check_ok "in sync with origin/main"
  elif [ "$AHEAD" != "0" ]; then
    check_warn "$AHEAD unpushed local commit(s) — \`git push origin main\` before starting workers"
  elif [ "$BEHIND" != "0" ]; then
    check_warn "$BEHIND commit(s) behind origin/main — \`git pull --ff-only origin main\`"
  fi

  # Worker scripts present?
  if [ -x scripts/claude-worker-loop.sh ]; then
    check_ok "scripts/claude-worker-loop.sh executable"
  else
    check_fail "scripts/claude-worker-loop.sh not executable — \`chmod +x scripts/*.sh\`"
  fi
else
  check_fail "ARGO_REPO_ROOT does not have .git — clone the repo first"
fi

echo

# --- Disk free -------------------------------------------------------------

bold "Disk:"
TMP_FREE_GB=$(df -g /tmp 2>/dev/null | tail -1 | awk '{print $4}' || df -BG /tmp 2>/dev/null | tail -1 | awk '{print $4}' | tr -d 'G')
if [ -n "${TMP_FREE_GB:-}" ]; then
  if [ "$TMP_FREE_GB" -ge 50 ]; then
    check_ok "/tmp free: ${TMP_FREE_GB}GB"
  elif [ "$TMP_FREE_GB" -ge 20 ]; then
    check_warn "/tmp free: ${TMP_FREE_GB}GB — workers fill ~10-20GB per worktree, may need cleanup soon"
  else
    check_fail "/tmp free: ${TMP_FREE_GB}GB — too low to safely run workers; clean worktrees + cargo target dir"
  fi
else
  check_warn "/tmp free space could not be determined"
fi

echo

# --- Summary --------------------------------------------------------------

if [ "$FAIL" -eq 0 ] && [ "$WARN" -eq 0 ]; then
  green "✓✓✓ All checks passed. Ready to launch workers."
elif [ "$FAIL" -eq 0 ]; then
  yellow "⚠ $WARN warning(s) — usable but suboptimal."
else
  red "✗ $FAIL failure(s), $WARN warning(s) — fix failures before starting workers."
  exit 1
fi

echo
bold "To start a worker on this machine:"
echo "  cd \$ARGO_REPO_ROOT && bash scripts/claude-worker-loop.sh"
echo
bold "To start the conductor on this machine (only ONE conductor across ALL machines):"
echo "  The conductor runs as a Claude Code session, not a shell script."
echo "  1. Open the canonical conductor tick prompt: \$ARGO_REPO_ROOT/docs/build-plan/PROTOCOL_LOOP_PROMPT.md"
echo "  2. Substitute the Part-B operator-specific blocks (machine, accounts, focus)."
echo "  3. In a Claude Code session, run:  /loop 5m <paste the substituted prompt>"
echo "  Re-read PROTOCOL_LOOP_PROMPT.md before each new conductor session — it evolves."

exit 0
