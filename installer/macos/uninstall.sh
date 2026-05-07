#!/usr/bin/env bash
# uninstall.sh — remove the launchd agent, the binary, and the
# user-side argo data dir (with confirmation).
set -e

echo "argo uninstall"
echo "  This will:"
echo "    1. unload + remove ~/Library/LaunchAgents/com.skycot.argo.plist"
echo "    2. remove /usr/local/bin/argo (if present)"
echo "    3. PROMPT before removing ~/.argo (workspace data)"
echo ""
read -r -p "Continue? [y/N] " yn
case "$yn" in
    [Yy]*) ;;
    *) echo "aborted"; exit 0 ;;
esac

if command -v argo >/dev/null 2>&1; then
    argo daemon uninstall || true
else
    LA="$HOME/Library/LaunchAgents/com.skycot.argo.plist"
    if [[ -f "$LA" ]]; then
        launchctl unload "$LA" 2>/dev/null || true
        rm -f "$LA"
    fi
fi

if [[ -f /usr/local/bin/argo ]]; then
    sudo rm -f /usr/local/bin/argo || rm -f /usr/local/bin/argo
fi

if [[ -d "$HOME/.argo" ]]; then
    read -r -p "Remove ~/.argo (workspace data)? [y/N] " yn
    case "$yn" in
        [Yy]*) rm -rf "$HOME/.argo" ;;
        *) echo "kept ~/.argo" ;;
    esac
fi

echo "uninstall complete"
