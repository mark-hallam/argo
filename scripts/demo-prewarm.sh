#!/usr/bin/env bash
# demo-prewarm.sh — curl each demo-critical SPA route once so Next.js
# Turbopack 16 has compiled every dynamic route pattern before the
# v1.0 mission demo recording starts.
#
# Why: the `/audit/action/[actionId]` route cold-compiles in ~203s on
# first hit, with a ~784s first-paint cost on top. That's a 3-13 minute
# blank state for whichever scene drills into the audit chain. Same
# class of problem on `/workers/[workerId]/[tab]` and the workflow
# viewer + builder routes.
#
# Next.js compiles per route pattern, not per concrete id, so a stub
# id like `prewarm` is enough to trigger the compile. The script does
# not require the gateway to return data — a 200/404 both count as
# "route warmed". Subsequent runs are <5 sec on a warm dev server.
#
# Run once before pressing record on the demo. Re-run after any
# `pnpm dev` restart.
#
# Usage:
#   bash scripts/demo-prewarm.sh
#   bash scripts/demo-prewarm.sh http://localhost:3000
#   SPA_BASE=http://localhost:3000 bash scripts/demo-prewarm.sh
#
# Inputs:
#   $1 (optional) — SPA base URL. Default: http://localhost:3000
#   SPA_BASE env var — overrides $1 if set.
#
# Output (one line per route, plus a summary):
#   warm /audit         200  0.42s
#   warm /audit/action/prewarm  200  203.18s   <-- first cold compile
#   ...
#   SUMMARY: 9/9 routes warmed; slowest = /audit/action/prewarm (203.18s)
#
# Exit codes:
#   0 — all routes returned a non-empty HTTP status (any 2xx/3xx/4xx).
#       The page compiled; data may or may not be present.
#   1 — one or more routes returned no status (SPA not running, or
#       request timed out past the per-route ceiling).

set -euo pipefail

SPA_BASE="${SPA_BASE:-${1:-http://localhost:3000}}"

# Per-route timeout. Cold-compile of /audit/action/[actionId] takes
# ~3 minutes on a fresh `pnpm dev`; 240s keeps headroom without
# letting a truly stuck dev server stall the script forever.
PER_ROUTE_TIMEOUT_S=240

# The nine demo-critical routes, in scene order.
ROUTES=(
    "/onboarding/welcome"
    "/audit"
    "/audit/action/prewarm"
    "/workers"
    "/workers/prewarm/overview"
    "/workers/prewarm/activity"
    "/workflows/sales/qualify_lead"
    "/workflows/sales/qualify_lead/build"
    "/approvals"
)

WARMED=0
FAILED=0
SLOWEST_ROUTE=""
SLOWEST_TIME=0

for route in "${ROUTES[@]}"; do
    url="${SPA_BASE}${route}"
    # -w prints "<status> <time_total>" on a separate line after the body
    # (which we discard with -o /dev/null). --max-time bounds the per-route
    # wait; a 0 status means the request timed out or the SPA isn't up.
    result="$(
        curl -sS \
            --max-time "$PER_ROUTE_TIMEOUT_S" \
            -o /dev/null \
            -w '%{http_code} %{time_total}' \
            "$url" 2>/dev/null || true
    )"
    status="${result%% *}"
    elapsed="${result##* }"
    # curl emits "000 <elapsed>" on connection-refused / timeout; treat
    # any non-2xx/3xx/4xx/5xx HTTP code as a warm-fail.
    if [[ -z "$status" || ! "$status" =~ ^[1-5][0-9][0-9]$ ]]; then
        printf 'WARM-FAIL %-40s  no response (status=%s, timeout>%ss?)\n' \
            "$route" "${status:-?}" "$PER_ROUTE_TIMEOUT_S"
        FAILED=$((FAILED + 1))
        continue
    fi
    printf 'warm      %-40s  %s  %ss\n' "$route" "$status" "$elapsed"
    WARMED=$((WARMED + 1))
    # Track slowest using awk so we can compare floats without bashisms.
    if awk -v a="$elapsed" -v b="$SLOWEST_TIME" 'BEGIN{exit !(a>b)}'; then
        SLOWEST_TIME="$elapsed"
        SLOWEST_ROUTE="$route"
    fi
done

TOTAL=${#ROUTES[@]}
if [[ -n "$SLOWEST_ROUTE" ]]; then
    printf 'SUMMARY: %s/%s routes warmed; slowest = %s (%ss)\n' \
        "$WARMED" "$TOTAL" "$SLOWEST_ROUTE" "$SLOWEST_TIME"
else
    printf 'SUMMARY: %s/%s routes warmed\n' "$WARMED" "$TOTAL"
fi

if (( FAILED > 0 )); then
    # Backticks below are part of a human-readable hint, not command substitution.
    # shellcheck disable=SC2016
    printf 'WARNING: %s route(s) failed to warm — is `pnpm dev` running on %s?\n' \
        "$FAILED" "$SPA_BASE" >&2
    exit 1
fi
exit 0
