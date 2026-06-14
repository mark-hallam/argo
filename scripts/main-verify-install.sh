#!/usr/bin/env bash
# main-verify-install.sh — install/update the main-verify daemon.
#
# Copies the daemon to a STABLE runtime dir (~/.argo/main-verify/bin) and the
# launchd plist to ~/Library/LaunchAgents, then (re)loads. Pulled from
# origin/main via `git show`, so it works even when the shared working tree is
# stale/dirty (the daemon must NOT depend on the volatile checkout it guards).
# Idempotent — re-run to update after a script change.
set -uo pipefail
REPO="${MAIN_VERIFY_REPO:-~/code/argo}"
BIN="$HOME/.argo/main-verify/bin"
LA="$HOME/Library/LaunchAgents"
PLIST="$LA/com.skycot.main-verify.plist"
mkdir -p "$BIN" "$LA" "$HOME/.argo/logs"

git -C "$REPO" fetch origin main -q 2>/dev/null || true
git -C "$REPO" show origin/main:scripts/main-verify-sweep.sh > "$BIN/main-verify-sweep.sh" || { echo "FAILED to read sweep from origin/main"; exit 1; }
chmod +x "$BIN/main-verify-sweep.sh"
git -C "$REPO" show origin/main:deploy/launchd/com.skycot.main-verify.plist > "$PLIST" || { echo "FAILED to read plist from origin/main"; exit 1; }

launchctl unload "$PLIST" 2>/dev/null || true
launchctl load -w "$PLIST"
echo "main-verify installed + loaded:"
echo "  daemon: $BIN/main-verify-sweep.sh"
echo "  plist : $PLIST"
echo "  status: bash $REPO/scripts/main-verify-status.sh   (first run does a cold compile ~15min)"
