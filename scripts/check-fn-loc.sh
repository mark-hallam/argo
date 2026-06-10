#!/usr/bin/env bash
#
# check-fn-loc.sh — fail when any NEW Rust function exceeds 80 LOC.
#
# Walks `crates/**/*.rs`, detects function definitions, measures
# each function's span (from the `fn NAME` signature line through
# the matching closing brace, inclusive), and exits non-zero with
# a list of NEW violators. CLAUDE.md §"Maintain"/§"Core-platform
# invariants" prescribe an 80-LOC ceiling per function; this gate
# enforces it on the merge path.
#
# Baseline ratchet
# ----------------
# The repo carries ~314 pre-existing over-budget functions. A gate
# that fails on all of them is permanently red on clean main, which
# trains "judge-around-it" (the gate gets ignored). So this script
# reads a committed baseline allowlist (`scripts/fn-loc-baseline.txt`)
# of the functions known to be over budget, and fails ONLY on
# violations NOT in that baseline — i.e. NEW over-budget functions a
# PR introduces. The baseline is meant to shrink to zero over time
# (burn-down); it must never grow except via a deliberate, reviewed
# `--update-baseline`.
#
# Baseline key is `relative_path<TAB>fn_name` — deliberately NOT the
# line number or LOC count, so an unrelated edit that shifts a
# function's line, or a refactor that changes its size, does not
# spuriously trip (or silently un-trip) the gate. The trade-off:
# growing an already-baselined function further still passes (it's a
# known offender), but adding any new over-budget function, or pushing
# a previously-OK function over 80 LOC, fails.
#
# Tests are NOT exempted. The same 80-LOC rule applies to every fn
# body in `crates/**`.
#
# Detection is approximate: a per-line brace counter with naive
# string-literal stripping. Macros, multi-line strings, and unusual
# literal forms may produce occasional false positives/negatives, but
# the gate is good enough to catch the obvious 100+ LOC offenders that
# motivated the rule.
#
# Usage:
#   scripts/check-fn-loc.sh                    verify (CI mode) — fail on NEW violations
#   scripts/check-fn-loc.sh --list             print every over-budget fn (baseline-agnostic)
#   scripts/check-fn-loc.sh --update-baseline  regenerate the baseline allowlist
#
# Tunables:
#   MAX_FN_LOC      override the 80-LOC ceiling (rarely needed).
#   FN_LOC_BASELINE override the baseline file path.
#
# Exit codes:
#   0  no NEW violations (baseline honored)
#   1  one or more NEW functions exceed the ceiling
#   2  invocation error
#
# Compatible with bash 3.2 (default macOS) — no associative arrays.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MAX_LOC="${MAX_FN_LOC:-80}"
BASELINE_FILE="${FN_LOC_BASELINE:-$REPO_ROOT/scripts/fn-loc-baseline.txt}"

MODE="verify"
case "${1:-}" in
    --list)            MODE="list" ;;
    --update-baseline) MODE="update" ;;
    "")                MODE="verify" ;;
    *)
        echo "unknown mode: $1" >&2
        echo "see header of: $0" >&2
        exit 2
        ;;
esac

if [[ ! -d "$REPO_ROOT/crates" ]]; then
    echo "FAIL: no crates/ directory at $REPO_ROOT" >&2
    exit 2
fi

# Emit one TAB-separated record per over-budget function:
#   relpath <TAB> line <TAB> fn_name <TAB> span
# relpath is relative to REPO_ROOT so the baseline is machine-independent.
violations=$(
    find "$REPO_ROOT/crates" -type f -name '*.rs' -not -path '*/target/*' -print0 \
    | xargs -0 awk -v maxloc="$MAX_LOC" -v root="$REPO_ROOT/" '
        FNR == 1 {
            in_fn = 0
            depth = 0
            body_started = 0
            fn_start = 0
            fn_name = ""
            relfile = FILENAME
            if (index(relfile, root) == 1) {
                relfile = substr(relfile, length(root) + 1)
            }
        }
        {
            line = $0
            # strip line comments — naive (does not honor string context)
            sub(/[[:space:]]*\/\/.*$/, "", line)
            # strip "..." string literals so braces inside dont count
            gsub(/"[^"]*"/, "\"\"", line)

            if (!in_fn) {
                if (match(line, /(^|[^A-Za-z0-9_])fn[[:space:]]+[A-Za-z_][A-Za-z0-9_]*/)) {
                    matched = substr(line, RSTART, RLENGTH)
                    sub(/^[^f]*/, "", matched)
                    sub(/^fn[[:space:]]+/, "", matched)
                    fn_name = matched
                    fn_start = FNR
                    in_fn = 1
                    depth = 0
                    body_started = 0
                }
            }
            if (in_fn) {
                n = length(line)
                for (i = 1; i <= n; i++) {
                    c = substr(line, i, 1)
                    if (c == "{") {
                        depth++
                        body_started = 1
                    } else if (c == "}") {
                        depth--
                        if (body_started && depth == 0) {
                            span = FNR - fn_start + 1
                            if (span > maxloc) {
                                printf "%s\t%d\t%s\t%d\n", relfile, fn_start, fn_name, span
                            }
                            in_fn = 0
                            body_started = 0
                            break
                        }
                    } else if (c == ";" && depth == 0 && !body_started) {
                        # trait method declaration without body, or fn pointer
                        # type alias — never enters the body, skip.
                        in_fn = 0
                        break
                    }
                }
            }
        }
    ' 2>/dev/null \
    | sort -u)

# Human-readable display for a record stream: "relpath:line  fn name  (NN LOC)".
display() {
    awk -F'\t' 'NF>=4 { printf "%s:%s  fn %s  (%s LOC)\n", $1, $2, $3, $4 }'
}

# Stable baseline key for a record stream: "relpath<TAB>fn_name".
keyify() {
    awk -F'\t' 'NF>=3 { print $1 "\t" $3 }' | sort -u
}

if [[ "$MODE" == "list" ]]; then
    printf '%s\n' "$violations" | grep -v '^$' | display
    exit 0
fi

if [[ "$MODE" == "update" ]]; then
    {
        printf '# scripts/fn-loc-baseline.txt\n'
        printf '#\n'
        printf '# Pre-existing functions over the %s-LOC ceiling, allowlisted so the\n' "$MAX_LOC"
        printf '# fn-LOC gate fails only on NEW over-budget functions. Regenerate with:\n'
        printf '#   scripts/check-fn-loc.sh --update-baseline\n'
        printf '#\n'
        printf '# Format: one entry per line, `relative_path<TAB>fn_name`. This list is\n'
        printf '# meant to SHRINK to zero over time (burn-down) and must never grow\n'
        printf '# except via a deliberate, reviewed --update-baseline. When you split a\n'
        printf '# baselined function under 80 LOC, drop its line here in the same PR.\n'
        printf '#\n'
        printf '# Adding a NEW function over %s LOC fails the gate; split it (preferred)\n' "$MAX_LOC"
        printf '# or, if genuinely unavoidable, regenerate the baseline with a rationale\n'
        printf '# in the PR body.\n\n'
        printf '%s\n' "$violations" | grep -v '^$' | keyify
    } > "$BASELINE_FILE"
    n=$(printf '%s\n' "$violations" | grep -cv '^$')
    echo "Wrote $BASELINE_FILE ($n functions over $MAX_LOC LOC)."
    exit 0
fi

# verify mode: compare current keys against the baseline allowlist.
ALLOWLIST_FILE="$(mktemp -t argo-fn-loc-allowlist.XXXXXX)"
CURRENT_KEYS_FILE="$(mktemp -t argo-fn-loc-current.XXXXXX)"
trap 'rm -f "$ALLOWLIST_FILE" "$CURRENT_KEYS_FILE"' EXIT

if [[ -f "$BASELINE_FILE" ]]; then
    sed -E 's/^[[:space:]]*#.*$//; s/[[:space:]]+$//' "$BASELINE_FILE" \
        | grep -v '^$' | sort -u > "$ALLOWLIST_FILE" || true
else
    : > "$ALLOWLIST_FILE"
    echo "WARN: no baseline at $BASELINE_FILE — every violation counts as new." >&2
fi

printf '%s\n' "$violations" | grep -v '^$' | keyify > "$CURRENT_KEYS_FILE"

# NEW violations = current keys not present in the baseline allowlist.
new_keys=$(comm -23 "$CURRENT_KEYS_FILE" "$ALLOWLIST_FILE")

# Burn-down progress: baseline entries that no longer violate (resolved).
resolved_count=$(comm -13 "$CURRENT_KEYS_FILE" "$ALLOWLIST_FILE" | grep -c . || true)

if [[ -n "$new_keys" ]]; then
    count=$(printf '%s\n' "$new_keys" | grep -c .)
    echo "FAIL: $count NEW function(s) exceeding $MAX_LOC LOC (not in baseline):"
    # Re-attach line + LOC for the offending keys, for a readable report.
    printf '%s\n' "$violations" | grep -v '^$' \
        | awk -F'\t' 'NR==FNR { seen[$0]=1; next } (($1 "\t" $3) in seen)' \
              <(printf '%s\n' "$new_keys") - \
        | display | sed 's/^/  /'
    echo
    echo "Split the function under $MAX_LOC LOC (preferred). If genuinely"
    echo "unavoidable, regenerate the baseline: scripts/check-fn-loc.sh --update-baseline"
    exit 1
fi

echo "OK: no new functions over $MAX_LOC LOC (baseline honored)."
if (( resolved_count > 0 )); then
    echo "     ($resolved_count baselined function(s) resolved — regenerate the"
    echo "      baseline to shrink it: scripts/check-fn-loc.sh --update-baseline)"
fi
exit 0
