#!/usr/bin/env bash
# Enforce the canonical tracing-span naming hierarchy
# (docs/skycot/TRACING_SPAN_NAMING.md).
#
# 1. Every `pub const X: &str = "…";` in the `argo_telemetry::spans`
#    module matches `^[a-z_]+\.[a-z_]+(\.[a-z_]+)?$`.
# 2. No `tracing::*_span!("literal")` or
#    `#[instrument(name = "literal")]` exists outside the
#    `argo-telemetry` crate. All spans must route through the
#    `argo_telemetry::spans` registry.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SPANS_FILE="$REPO_ROOT/crates/argo-telemetry/src/lib.rs"
REGEX='^[a-z_]+\.[a-z_]+(\.[a-z_]+)?$'

[[ -f "$SPANS_FILE" ]] || { echo "FAIL: missing $SPANS_FILE" >&2; exit 2; }

fail=0
in_mod=0
depth=0
checked=0
while IFS= read -r line; do
    if [[ $in_mod -eq 0 ]]; then
        [[ "$line" =~ ^[[:space:]]*pub[[:space:]]+mod[[:space:]]+spans[[:space:]]*\{ ]] \
            && { in_mod=1; depth=1; }
        continue
    fi
    opens=$(tr -cd '{' <<<"$line" | wc -c | tr -d ' ')
    closes=$(tr -cd '}' <<<"$line" | wc -c | tr -d ' ')
    depth=$((depth + opens - closes))
    if [[ "$line" =~ pub[[:space:]]+const[[:space:]]+[A-Z0-9_]+:[[:space:]]*\&str[[:space:]]*=[[:space:]]*\"([^\"]+)\" ]]; then
        value="${BASH_REMATCH[1]}"
        checked=$((checked + 1))
        if [[ ! "$value" =~ $REGEX ]]; then
            echo "FAIL: span constant '$value' does not match $REGEX" >&2
            fail=1
        fi
    fi
    [[ $depth -le 0 ]] && break
done < "$SPANS_FILE"

[[ $checked -gt 0 ]] || { echo "FAIL: no span constants found in argo_telemetry::spans" >&2; exit 2; }

RAW='(tracing::(info|debug|trace|warn|error)_span!\(\s*"|tracing::span!\([^,]+,\s*"|#\[(tracing::)?instrument\([^)]*name\s*=\s*")'
matches="$(cd "$REPO_ROOT" && rg --no-heading --line-number --type rust \
    --glob '!crates/argo-telemetry/**' "$RAW" || true)"

if [[ -n "$matches" ]]; then
    echo "FAIL: raw span literals found outside argo-telemetry:" >&2
    echo "$matches" >&2
    echo "All spans must route through argo_telemetry::spans constants." >&2
    echo "See docs/skycot/TRACING_SPAN_NAMING.md." >&2
    fail=1
fi

[[ $fail -eq 0 ]] || exit 1
echo "OK: $checked span constants validated, no raw span literals outside argo-telemetry."
