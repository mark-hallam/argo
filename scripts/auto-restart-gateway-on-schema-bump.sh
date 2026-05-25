#!/usr/bin/env bash
# auto-restart-gateway-on-schema-bump.sh — conductor post-merge helper.
#
# Usage:
#   bash scripts/auto-restart-gateway-on-schema-bump.sh <merged-commit-sha>
#
# If the merged commit bumps EXPECTED_SCHEMA_VERSION, restart the local
# gateway only when its /health endpoint is reachable and stale.

set -euo pipefail

sha="${1:-}"
if [[ -z "$sha" || "$sha" == "-h" || "$sha" == "--help" ]]; then
    grep '^#' "$0" | sed 's/^# \{0,1\}//'
    exit 0
fi

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$repo_root" ]]; then
    echo "auto-restart-gateway: not inside an argo git checkout" >&2
    exit 2
fi
cd "$repo_root"

if ! git cat-file -e "$sha^{commit}" 2>/dev/null; then
    echo "auto-restart-gateway: commit not found: $sha" >&2
    exit 2
fi

shared_log="${AUTO_RESTART_GATEWAY_SHARED_LOG:-docs/skycot/agents/SHARED.md}"
base_url="${AUTO_RESTART_GATEWAY_BASE_URL:-http://127.0.0.1:7400}"
restart_script="${AUTO_RESTART_GATEWAY_RESTART_SCRIPT:-scripts/restart-live-gateway.sh}"
curl_timeout="${AUTO_RESTART_GATEWAY_CURL_TIMEOUT:-3}"

commit_body="$(git show -s --format=%B "$sha")"
pr_label="$(
    printf '%s\n' "$commit_body" \
        | sed -nE 's/.*#([0-9]+).*/#\1/p' \
        | head -1
)"
[[ -n "$pr_label" ]] || pr_label="#unknown"

append_shared() {
    local message="$1"
    mkdir -p "$(dirname "$shared_log")"
    printf '%s [conductor-auto-restart-gateway] post %s schema-bump merge — %s\n' \
        "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" "$pr_label" "$message" >> "$shared_log"
}

if ! git show --name-only --format= "$sha" | grep -qxF 'crates/argo-store/src/lib.rs'; then
    echo "auto-restart-gateway: no schema bump in $sha (argo-store lib.rs untouched)"
    exit 0
fi

if ! git show --format= "$sha" -- crates/argo-store/src/lib.rs \
    | grep -qE '^\+pub const EXPECTED_SCHEMA_VERSION'; then
    echo "auto-restart-gateway: no EXPECTED_SCHEMA_VERSION addition in $sha"
    exit 0
fi

expected="$(
    grep -E '^pub const EXPECTED_SCHEMA_VERSION: u32' crates/argo-store/src/lib.rs \
        | sed -E 's/.*= *([0-9]+).*/\1/'
)"
if [[ -z "$expected" ]]; then
    echo "auto-restart-gateway: could not parse EXPECTED_SCHEMA_VERSION" >&2
    exit 2
fi

probe_health() {
    local response body code schema
    response="$(curl -sS --max-time "$curl_timeout" -w $'\n%{http_code}' "$base_url/health" 2>/dev/null || true)"
    code="$(printf '%s\n' "$response" | tail -n1)"
    body="$(printf '%s\n' "$response" | sed '$d')"
    if [[ -z "$code" || "$code" == "000" ]]; then
        printf 'UNREACHABLE'
        return 1
    fi
    schema="$(
        printf '%s' "$body" \
            | sed -nE 's/.*"schema_version"[[:space:]]*:[[:space:]]*([0-9]+).*/\1/p' \
            | head -1
    )"
    if [[ -n "$schema" ]]; then
        printf 'HTTP %s schema_version=v%s' "$code" "$schema"
    else
        printf 'HTTP %s schema_version=unknown' "$code"
    fi
}

health_status="$(probe_health || true)"
if [[ "$health_status" == "UNREACHABLE" ]]; then
    append_shared "gateway not running locally, no-op"
    echo "auto-restart-gateway: gateway unreachable at $base_url; no-op"
    exit 0
fi

live_schema="$(
    printf '%s' "$health_status" \
        | sed -nE 's/.*schema_version=v([0-9]+).*/\1/p' \
        | head -1
)"
if [[ -n "$live_schema" && "$live_schema" -ge "$expected" ]]; then
    echo "auto-restart-gateway: gateway already fresh ($health_status, expected v$expected); no-op"
    exit 0
fi

echo "auto-restart-gateway: schema bump detected in $sha; restarting local gateway ($health_status, expected v$expected)"
if ! bash "$restart_script" --no-prompt; then
    append_shared "restart attempted but FAILED, prior /health: $health_status"
    echo "auto-restart-gateway: restart failed" >&2
    exit 1
fi

new_status="$(probe_health || true)"
append_shared "restarted local gateway, new /health: $new_status"
echo "auto-restart-gateway: restarted local gateway, new /health: $new_status"
