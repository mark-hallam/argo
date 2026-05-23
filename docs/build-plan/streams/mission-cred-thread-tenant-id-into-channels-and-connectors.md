# mission-cred-thread-tenant-id-into-channels-and-connectors

**Authored against**: origin/main `b0afe9f99` on 2026-05-23
**Owner**: claude (manager-worker eligible — 4 children, one per crate cluster)
**Priority**: 🟡 P1 — mission-claim-1 follow-up to #2087 STALE
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cred-thread-tenant-id-into-channels-and-connectors`

## Why this is P1

PR #2087 (`mission-cred-get-tenant-parameter`) was the 91-callsite
sweep originally requested at tick-79. Tick-86 rebase discovered the
underlying API (`cred::get(tenant_id, &str)` + `get_unscoped(&str)` +
per-tenant TOML shape + lazy-migration helper) is already on `main`
via a chained zombie commit. The 91-callsite sweep itself, however,
**did not land** — every consumer still calls `cred::get_unscoped`,
which is a P1 cross-tenant credential-visibility leak in any
multi-tenant gateway.

Audit 2026-05-23T03-39 confirmed this as the follow-up shape
("RECOMMENDATION: close PR #2087 + author follow-up
`mission-cred-thread-tenant-id-into-channels-and-connectors`
manager-worker, 4 children — one per crate cluster").

## Pre-flight grep

```
rg -l "cred::get_unscoped|cred_get_unscoped|get_unscoped\b" crates/
```

Worker MUST verify count + cluster distribution before claiming.
Tick-87 inventory (`eval-mcp-connector-credential-scoping-sweep` PR
#2097) reports **31 call sites across 28 crates** for the MCP-
connector cluster alone. Channel crates + agent-runner add more.

## Scope (manager-worker — 4 children)

This brief is a MANAGER. The conductor (or claude manager-mode)
slices the work into ≤4 child briefs, one per cluster, each shippable
as its own PR. Suggested cuts:

- **child-1** — `crates/argo-mcp-connector-*/` (~28 connectors, ~28
  callsites). Mechanical: `get_unscoped(K)` → `get(tenant_id, K)`
  where `tenant_id` is already on the call stack (most
  connectors take a `tenant_id` arg at constructor or `request()`
  time per the round-10 + round-tick-87 audits). For connectors
  that do NOT have `tenant_id` on the call stack — flag in the
  child PR body and propagate from the caller-frame in a NEXT
  child.
- **child-2** — `crates/argo-channels*/` cluster (~5 channels).
  Same shape as child-1.
- **child-3** — `crates/argo-agent-runner/` + `crates/argo-cli/`
  call sites. May require new `tenant_id` parameter on a couple
  CLI handlers if not already plumbed.
- **child-4** — sweep + delete `cred::get_unscoped` from the public
  API once children 1-3 have landed (rename to `cred::get_admin_unscoped`
  + capability-gate to "admin" principal only). Catches any
  late-arriving caller via compile error.

Manager-worker mode: conductor authors each child as its own
ledger row when ready. Children claim independently.

## Acceptance per child

1. Child PR removes its assigned `get_unscoped` calls without
   touching other clusters.
2. Each replaced site gets a 1-line `tenant_id` source comment in
   the diff (`// tenant_id from <caller frame>`).
3. Cross-tenant negative test added per cluster (one per child PR
   minimum).
4. `bash scripts/conductor-pre-merge-gate.sh --skip-clippy` green
   (post-LOC-fix landing).

## Acceptance per manager (this brief)

1. All 4 children MERGED.
2. `rg "get_unscoped" crates/` returns only the renamed
   admin-escape-hatch + test fixtures.
3. Gap-manifest row `mcp-connector cred-axis` flipped from 🟡 →
   🟢.

## Must NOT touch (per child)

- Other clusters (each child stays in its lane).
- `cred::get` / `cred::get_unscoped` core API in
  `crates/argo-config/src/cred.rs` (children only call the API;
  child-4 renames it).
- Migration files.

## LOC budget

- child-1: ~300-500 LOC (28 callsites + tests)
- child-2: ~150-250 LOC
- child-3: ~150-300 LOC
- child-4: ~50-100 LOC (rename + delete-old-symbol)
- **Manager total**: ~700-1200 LOC across 4 PRs.

## Dependencies

- LOC-fix brief `claude-split-workflow-definition-tests-loc-fix`
  must land first (admin-merge gate prerequisite).
- `cred::get` API already on main (verified via #2087 rebase).
