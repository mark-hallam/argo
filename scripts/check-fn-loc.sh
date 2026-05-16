#!/usr/bin/env bash
#
# check-fn-loc.sh — fail when any Rust function exceeds 80 LOC.
#
# Walks `crates/**/*.rs`, detects function definitions, measures
# each function's span (from the `fn NAME` signature line through
# the matching closing brace, inclusive), and exits non-zero with
# a list of violators. CLAUDE.md §"Maintain"/§"Core-platform
# invariants" prescribe an 80-LOC ceiling per function; this gate
# enforces it.
#
# Tests are NOT exempted. The same 80-LOC rule applies to every
# fn body in `crates/**`. Per-brief decision documented in PR.
#
# Detection is approximate: a per-line brace counter with naive
# string-literal stripping. Macros, multi-line strings, and
# unusual literal forms may produce occasional false
# positives/negatives, but the gate is good enough to catch the
# obvious 100+ LOC offenders that motivated the rule.
#
# Tunables:
#   MAX_FN_LOC  override the 80-LOC ceiling (rarely needed).
#
# Exit codes:
#   0  no violations
#   1  one or more functions exceed the ceiling
#   2  invocation error

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MAX_LOC="${MAX_FN_LOC:-80}"

if [[ ! -d "$REPO_ROOT/crates" ]]; then
    echo "FAIL: no crates/ directory at $REPO_ROOT" >&2
    exit 2
fi

violations=$(
    find "$REPO_ROOT/crates" -type f -name '*.rs' -not -path '*/target/*' -print0 \
    | xargs -0 awk -v maxloc="$MAX_LOC" '
        FNR == 1 {
            in_fn = 0
            depth = 0
            body_started = 0
            fn_start = 0
            fn_name = ""
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
                    fn_file = FILENAME
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
                                printf "%s:%d  fn %s  (%d LOC)\n", fn_file, fn_start, fn_name, span
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
    | sort)

if [[ -n "$violations" ]]; then
    count=$(printf '%s\n' "$violations" | wc -l | tr -d ' ')
    echo "Found $count function(s) exceeding $MAX_LOC LOC:"
    printf '%s\n' "$violations"
    exit 1
fi

echo "  all functions ≤ $MAX_LOC LOC"
exit 0
