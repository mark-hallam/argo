#!/usr/bin/env bash
# scripts/check-cli-help-examples.sh
#
# Asserts every leaf subcommand of `argo --help` exposes an
# `Examples:` block in its help text. Reads a baseline of
# known-missing leaves from `scripts/cli-help-baseline.txt` so the
# guard can land before every leaf is filled in. Fails if a leaf
# regresses — i.e., is missing AND not allowlisted.
#
# The baseline is intended to shrink to zero over time. New leaves
# added to the CLI MUST ship an Examples block; legacy leaves are
# tracked in the baseline and migrated incrementally.
#
# Environment:
#   ARGO_BIN          Path to a built `argo` binary. Auto-detected
#                     from $CARGO_TARGET_DIR/debug/argo or
#                     target/debug/argo; falls back to
#                     `cargo run -q -p argo-cli --bin argo --`.
#   BASELINE_FILE     Override the default baseline path.
#
# Usage:
#   scripts/check-cli-help-examples.sh                   verify (CI mode)
#   scripts/check-cli-help-examples.sh --list-missing    print every leaf missing Examples
#   scripts/check-cli-help-examples.sh --update-baseline regenerate the baseline file
#
# Compatible with bash 3.2 (default macOS) — no associative arrays.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BASELINE_FILE="${BASELINE_FILE:-$SCRIPT_DIR/cli-help-baseline.txt}"

resolve_argo_bin() {
    if [[ -n "${ARGO_BIN:-}" ]]; then
        return
    fi
    local candidates=(
        "${CARGO_TARGET_DIR:-target}/debug/argo"
        "${CARGO_TARGET_DIR:-target}/release/argo"
        "target/debug/argo"
        "target/release/argo"
    )
    for cand in "${candidates[@]}"; do
        if [[ -x "$cand" ]]; then
            ARGO_BIN="$cand"
            return
        fi
    done
    ARGO_BIN="cargo run -q -p argo-cli --bin argo --"
}

resolve_argo_bin

# Tempfile holds discovered leaves missing Examples, one path per line.
MISSING_FILE="$(mktemp -t argo-cli-help.XXXXXX)"
trap 'rm -f "$MISSING_FILE"' EXIT

# Normalised allowlist: strip comments + blanks. Empty file if no baseline.
ALLOWLIST_FILE="$(mktemp -t argo-cli-allowlist.XXXXXX)"
trap 'rm -f "$MISSING_FILE" "$ALLOWLIST_FILE"' EXIT
if [[ -f "$BASELINE_FILE" ]]; then
    sed -E 's/#.*$//; s/^[[:space:]]+//; s/[[:space:]]+$//' "$BASELINE_FILE" \
        | grep -v '^$' > "$ALLOWLIST_FILE" || true
fi

extract_subcommands() {
    awk '
        /^Commands:/ { in_cmds = 1; next }
        in_cmds && /^[A-Z]/ { in_cmds = 0 }
        in_cmds && NF == 0 { next }
        in_cmds && /^  [^ ]/ {
            name = $1
            if (name != "help") print name
        }
    '
}

walk() {
    local path="$1"
    local help
    if ! help=$($ARGO_BIN $path --help 2>/dev/null); then
        return 0
    fi

    local subs
    subs=$(printf '%s\n' "$help" | extract_subcommands)
    if [[ -n "$subs" ]]; then
        while IFS= read -r sub; do
            [[ -z "$sub" ]] && continue
            walk "${path:+$path }$sub"
        done <<< "$subs"
        return 0
    fi

    if grep -qF 'Examples:' <<<"$help"; then
        return 0
    fi

    printf '%s\n' "$path" >> "$MISSING_FILE"
}

walk ""

mode="${1:-verify}"

case "$mode" in
    --list-missing)
        sort -u "$MISSING_FILE"
        exit 0
        ;;
    --update-baseline)
        {
            printf '# scripts/cli-help-baseline.txt\n'
            printf '#\n'
            printf '# Leaf subcommands not yet carrying an `Examples:` after_help\n'
            printf '# block. Regenerate with:\n'
            printf '#   scripts/check-cli-help-examples.sh --update-baseline\n'
            printf '#\n'
            printf '# This list is meant to shrink to zero over time. Adding a new\n'
            printf '# leaf without an Examples block fails CI; either include the\n'
            printf '# block in the same PR or, if intentional, append the leaf path\n'
            printf '# below with a short rationale.\n\n'
            sort -u "$MISSING_FILE"
        } > "$BASELINE_FILE"
        echo "Wrote $BASELINE_FILE ($(wc -l < "$MISSING_FILE" | tr -d ' ') leaves)."
        exit 0
        ;;
    verify) ;;
    *)
        echo "unknown mode: $mode" >&2
        echo "see header of: $0" >&2
        exit 2
        ;;
esac

# Compare missing against allowlist; anything not in allowlist is a regression.
REGRESSIONS_FILE="$(mktemp -t argo-cli-regressions.XXXXXX)"
trap 'rm -f "$MISSING_FILE" "$ALLOWLIST_FILE" "$REGRESSIONS_FILE"' EXIT
sort -u "$MISSING_FILE" > "${MISSING_FILE}.sorted"
mv "${MISSING_FILE}.sorted" "$MISSING_FILE"
comm -23 "$MISSING_FILE" <(sort -u "$ALLOWLIST_FILE") > "$REGRESSIONS_FILE"

regression_count=$(wc -l < "$REGRESSIONS_FILE" | tr -d ' ')
allowlisted_still_missing=$(comm -12 "$MISSING_FILE" <(sort -u "$ALLOWLIST_FILE") | wc -l | tr -d ' ')

if (( regression_count > 0 )); then
    echo "FAIL: $regression_count leaf subcommand(s) missing an Examples: block:" >&2
    sed 's/^/  argo /' "$REGRESSIONS_FILE" >&2
    echo >&2
    echo "Fix by adding to the variant's clap derive:" >&2
    echo "    #[command(after_help = \"Examples:\\n  argo ...\")]" >&2
    echo >&2
    echo "If the omission is intentional, append the leaf path to:" >&2
    echo "    $BASELINE_FILE" >&2
    exit 1
fi

echo "OK: every non-allowlisted leaf carries an Examples: block."
if (( allowlisted_still_missing > 0 )); then
    echo "     ($allowlisted_still_missing leaves still on the baseline allowlist)"
fi
