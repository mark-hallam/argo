#!/usr/bin/env bash
# Sweep every `*_tests.rs` under `crates/argo-cli/src/handlers/gateway/`
# and every `*.rs` under `crates/argo-cli/tests/`, flagging which carry
# a cross-tenant negative test (two tenants seeded + isolation
# assertion). Idempotent — re-running with no source changes
# regenerates the same table.
#
# Output: markdown table on stdout, ready to paste under
# "Test-file coverage" in `docs/skycot/audit/cross-tenant-endpoint-coverage.md`.
# Endpoint → test-file mapping is held in the report by hand, because
# many gateway endpoints are dispatched through path-prefix helpers
# (not single `req.starts_with` literals), so a fully-automatic
# endpoint → owner map would miss real-world coverage.
#
# Heuristic (approximate; see Known limits in the report):
#   - two-tenant marker: file mentions both /tenant[-_ ]?a/ AND
#     /tenant[-_ ]?b/ (or "t1" AND "t2").
#   - isolation assertion: file mentions one of
#     /leak|403|Forbidden|excluded?|isolation|cross[-_]tenant|other[_-]tenant/.
#   - The two markers do not have to occur in the same fn (the report
#     flags suspect rows as "needs manual review").

set -euo pipefail

REPO="${ARGO_REPO_ROOT:-$(git rev-parse --show-toplevel)}"
GW="$REPO/crates/argo-cli/src/handlers/gateway"
CLI="$REPO/crates/argo-cli/tests"

has_xtest() {
  local f="$1"
  [ -f "$f" ] || return 1
  grep -qE '(tenant[-_ ]?a|tenant-a|"t1")' "$f" 2>/dev/null || return 1
  grep -qE '(tenant[-_ ]?b|tenant-b|"t2")' "$f" 2>/dev/null || return 1
  grep -qE '(leak|403|Forbidden|excluded?|isolation|cross[-_]tenant|other[_-]tenant)' "$f" 2>/dev/null || return 1
}

emit_rows() {
  local dir="$1" kind="$2"
  for f in "$dir"/*.rs; do
    [ -f "$f" ] || continue
    # gateway scope = inline *_tests.rs siblings only.
    # cli scope     = every integration-test binary in the crate.
    case "$kind" in
      gateway) [[ "$f" == *_tests.rs ]] || continue ;;
    esac
    if has_xtest "$f"; then printf '%s\t%s\t%s\n' YES "$kind" "$(basename "$f")"
    else                    printf '%s\t%s\t%s\n' NO  "$kind" "$(basename "$f")"
    fi
  done
}

echo "| Has cross-tenant test? | Scope | Test file |"
echo "|---|---|---|"
{ emit_rows "$GW" gateway; emit_rows "$CLI" cli; } | sort | \
  awk -F'\t' 'BEGIN{OFS="|"} { icon=($1=="YES")?"✅":"❌"; printf "| %s | %s | `%s` |\n", icon, $2, $3 }'
