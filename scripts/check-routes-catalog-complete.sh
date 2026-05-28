#!/usr/bin/env bash
# check-routes-catalog-complete.sh — static-grep gate that asserts every
# `/api/v1/<segment>` prefix wired by a gateway dispatcher appears at
# least once in `crates/argo-cli/src/handlers/gateway/routes_catalog.rs`.
#
# Why this exists: F8 + F28 of ux-e2e-audit-T2309MEL (2026-05-26)
# found the catalog returned 143 routes, zero of which were `/api/v1/*`
# — every v1 endpoint silently invisible to new agents debugging the
# SPA → gateway wiring. The dispatchers register the routes; the
# catalog reads from a separate static registry; the two had drifted.
#
# This script catches the next drift episode by greping the dispatchers
# for their `starts_with("METHOD /api/v1/<seg>"` / `strip_prefix(
# "METHOD /api/v1/<seg>/"` literals, extracting the first segment
# under `/api/v1/`, and confirming each one appears in the catalog
# table.
#
# Usage:
#   bash scripts/check-routes-catalog-complete.sh
#
# Exit 0 = catalog covers every dispatched `/api/v1/<segment>` family.
# Exit 1 = at least one dispatcher prefix is missing from the catalog.
#
# Static, no gateway required, no network calls.

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

catalog="crates/argo-cli/src/handlers/gateway/routes_catalog.rs"
dispatcher_dir="crates/argo-cli/src/handlers/gateway"

if [[ ! -f "$catalog" ]]; then
  printf '[check-routes-catalog-complete] FAIL — catalog not found at %s\n' "$catalog" >&2
  exit 2
fi

python3 - "$catalog" "$dispatcher_dir" <<'PY'
import re
import sys
from pathlib import Path

catalog_path = Path(sys.argv[1])
dispatcher_dir = Path(sys.argv[2])

# 1. Extract the segments under /api/v1/ that the catalog already lists.
catalog_text = catalog_path.read_text(encoding="utf-8")
catalog_paths = re.findall(r'"(/api/v1/[^"]+)"', catalog_text)
catalog_segments = set()
for p in catalog_paths:
    # /api/v1/workers/<id> → "workers"; /api/v1/auth/csrf → "auth";
    # /api/v1/orchestration/runs/<id>/cancel → "orchestration".
    rest = p[len("/api/v1/"):]
    seg = rest.split("/", 1)[0]
    if seg:
        catalog_segments.add(seg)

# 2. Walk the dispatcher source for METHOD /api/v1/<seg>... literals.
#    Only count matches that sit inside a `starts_with(` or
#    `strip_prefix(` call — that is the dispatcher-registration
#    fingerprint. Freeform fixture strings inside `#[cfg(test)]`
#    blocks (e.g. cors.rs) and `*_tests.rs` siblings would otherwise
#    pollute the dispatched-segment set with paths that are not real
#    endpoints. `routes_catalog.rs` itself is excluded so it does not
#    count as a dispatcher registration of itself.
method_re = re.compile(
    r'(?:starts_with|strip_prefix)\s*\(\s*'
    r'"(?P<method>GET|POST|PATCH|PUT|DELETE|OPTIONS) /api/v1/(?P<seg>[a-zA-Z0-9_\-]+)'
)
# Many sub-modules bind their family root in a `PREFIX` constant and
# match against it via `path.strip_prefix(PREFIX)`. The method literal
# is not nearby in the source, so we treat the PREFIX as "this family
# is dispatched for the methods listed in the file's doc-comment
# table" — but we only care about presence here, not the verbs.
prefix_const_re = re.compile(
    r'const\s+\w+\s*:\s*&str\s*=\s*"/api/v1/(?P<seg>[a-zA-Z0-9_\-]+)'
)
dispatched = {}  # segment -> set of methods
for source in sorted(dispatcher_dir.glob("*.rs")):
    if source.name == catalog_path.name:
        continue
    if source.name.endswith("_tests.rs"):
        continue
    text = source.read_text(encoding="utf-8")
    for match in method_re.finditer(text):
        seg = match.group("seg")
        method = match.group("method")
        dispatched.setdefault(seg, set()).add(method)
    for match in prefix_const_re.finditer(text):
        seg = match.group("seg")
        dispatched.setdefault(seg, set())  # presence only

# 3. Diff dispatched vs catalog segments. Any dispatched segment that
#    isn't represented in the catalog is a drift hit.
missing = sorted(
    seg for seg in dispatched
    if seg not in catalog_segments
)

if missing:
    print(
        "[check-routes-catalog-complete] FAIL — the following /api/v1/<segment>",
        "families are dispatcher-registered but absent from the catalog:",
        file=sys.stderr,
    )
    for seg in missing:
        methods = ",".join(sorted(dispatched[seg]))
        print(f"  /api/v1/{seg}    (dispatched: {methods})", file=sys.stderr)
    print(
        "",
        "Add corresponding entries to "
        "crates/argo-cli/src/handlers/gateway/routes_catalog.rs.",
        sep="\n",
        file=sys.stderr,
    )
    sys.exit(1)

# Lightweight floor: the catalog must list at least 15 /api/v1/* paths
# total. Matches the brief acceptance criterion and guards against
# accidental deletion of the v1 block.
v1_total = sum(1 for p in catalog_paths if p.startswith("/api/v1/"))
if v1_total < 15:
    print(
        f"[check-routes-catalog-complete] FAIL — catalog lists only {v1_total} "
        "/api/v1/* paths (floor: 15).",
        file=sys.stderr,
    )
    sys.exit(1)

print(
    f"[check-routes-catalog-complete] OK — catalog lists {v1_total} /api/v1/* "
    f"paths covering {len(dispatched)} dispatched segments."
)
PY
