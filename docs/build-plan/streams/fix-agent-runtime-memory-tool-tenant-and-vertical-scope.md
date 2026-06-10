# fix-agent-runtime-memory-tool-tenant-and-vertical-scope

**Owner**: claude (WS1 / security — conductor adversarial-review before merge, do NOT self-merge).
**Depends on**: none (`first_workspace_for_tenant` + `list_for_vertical` already exist).
**Authored against**: origin/main `6e8149f19` on `2026-06-10`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-agent-runtime-memory-tool-tenant-and-vertical-scope

## Goal

During a live agent run, the in-core `memory` tool must bind to the **session's
own tenant workspace** (not the global oldest/demo workspace) and filter
reads/supersede by the **bound vertical** — so one tenant's (and one role's)
memory cannot be read or overwritten by another.

## Why now (live e2e finding, 2026-06-10)

A freshly-signed-up account's `customer-support` teammate, on its FIRST run, read
a `completed_jobs` memory record belonging to a **different role from a different
prior session** — a `gtm-engineer` outreach draft dated 2026-06-08 (the embedded
record carried `"vertical_id":"gtm-engineer"`). Repro:
`.skycot/outputs/019eaf2f-8509-7e13-bb31-baf027901ad4/{result.json,transcript.md}`.
A fresh user is supposed to get its OWN empty workspace (the
`sprint-e2e-new-user-journey-smoke` test #2705 asserts exactly this), yet its
worker saw the demo/cwd workspace's prior data. This is the tenant-isolation
moat — the platform's core invariant ("multi-tenant by construction") — leaking
in the agent runtime.

## Root cause (two layered defects, both VERIFIED)

**3a — tenant binding (the severe one).** `build_turn_dispatch_ctx` binds the
memory tool to `store.first_workspace()`
(`crates/argo-agent-runner/src/turn_helpers_run.rs:116`), which is
`SELECT id FROM workspace ORDER BY created_at ASC LIMIT 1`
(`crates/argo-store/src/workspace.rs:18`) = the **globally-oldest (demo/cwd)
workspace**, IGNORING `session.tenant_id` (which is in scope, used lines
128/133/158/164). The tenant-scoped variant
`first_workspace_for_tenant(tenant_id)` ALREADY exists
(`workspace.rs:37`) and was built for WS1 precisely to "bind a new session to
the signed-in tenant's own workspace instead of the cwd/demo workspace." In
single-binary dev (one workspace) this masks as a cross-VERTICAL bleed; in
multi-tenant it is **cross-TENANT memory read AND write** of
`MEMORY/User/Soul/Agents` + `completed_jobs`. Same WS1 isolation class as
#2676/#2708/#2710/#2777, but in the agent-runtime turn path that the WS1
read-route sweep did not cover.

**3b — vertical filter.** Even within the correct workspace, the memory tool's
read + supersede actions use `store.list_active(workspace_id, file)`
(`crates/argo-tool/src/tools/memory.rs:292` list, `:312` read, `:330` write
supersede-check), which returns ALL verticals' rows for the workspace and
ignores the bound `vertical_id`. The write CREATE path already correctly
branches to `append_for_vertical` when `vertical_id` is `Some`
(`memory.rs:340-344`) — proving the pattern — but reads and the
supersede-existing check don't. The vertical-scoped query `list_for_vertical`
exists (`crates/argo-memory/src/store_vertical.rs:70`). Consequence: one role
reads another role's `MEMORY.md` (the observed leak) AND a write can
**supersede another role's entry** (`list_active().first()` may return a
different vertical's row → cross-vertical overwrite/corruption, worse than a
read).

## Fix

1. **(3a)** In `build_turn_dispatch_ctx`, bind `memory_ctx` to
   `store.first_workspace_for_tenant(session.tenant_id)` instead of
   `store.first_workspace()` (`turn_helpers_run.rs:116`). **Fail-closed:** if the
   tenant has no workspace, bind nothing rather than fall back to the global
   workspace.
2. **(3b)** In `memory.rs` `list_action` (292), `read_action` (312), and
   `write_action`'s existing-entry check (330): when `wired.vertical_id.is_some()`,
   use `store.list_for_vertical(workspace, file, vid)` instead of `list_active`;
   keep `list_active` only for the legacy `vertical_id == None` path (CLI without
   `--vertical`). Mirror the write CREATE branch already at `memory.rs:340-344`.

No schema change (reuses existing `first_workspace_for_tenant` +
`list_for_vertical`). `memory.rs` is 392 LOC; ~30-40 LOC of branching keeps it
<500.

## May touch

- `crates/argo-agent-runner/src/turn_helpers_run.rs` (workspace binding, line
  116) + tests
- `crates/argo-tool/src/tools/memory.rs` (392 LOC) + memory tests

## Acceptance / verifier

1. **(3a) Cross-tenant test:** tenant B's worker run does NOT read or supersede
   tenant A's `MEMORY`/`completed_jobs` even when A's workspace is the older
   (demo/cwd) one. Build two tenants + workspaces; assert isolation. Must FAIL on
   the current `first_workspace()` binding (regression pin), mirroring the WS1
   negative-test pattern (#2696).
2. **(3b) Cross-vertical test in the agent-runner path:** a
   `customer-support`-bound memory list/read does NOT return a `gtm-engineer`
   entry in the same workspace, and a write does not supersede another vertical's
   same-named file. (`cli_memory_list_with_vertical_excludes_other_verticals_rows`
   exists at the CLI layer; add the worker-tool / agent-runner-path equivalent —
   the gap the investigation flagged.)
3. Legacy CLI (no `--vertical`) still sees workspace-global rows (backward compat).
4. `cargo test -p argo-agent-runner -p argo-tool -p argo-memory` green; clippy
   clean; LOC budget respected.

## Size / owner / priority

~80-150 LOC incl. tests across 2 files. **🔴 P1** — WS1 tenant-isolation class:
latent **cross-tenant** memory read+write in multi-tenant, masked in
single-binary dev as the observed cross-vertical bleed. DEC-034 WS1 keystone
surface; the moat is tenant isolation + truthful audit. Distinct from the
already-merged gateway `/memory/<id>` route tenant-bind (#2180) — this is the
agent-RUNTIME memory binding. Sibling:
`fix-worker-run-memory-namespace-unbound-team-actions` (same
`build_turn_dispatch_ctx` under-scopes worker identity). Cross-ref WS1 program
(`sprint-ws1-tenant-resolve-from-session-jwt-not-cwd` #2676 keystone).

## Open question (possible separate finding)

WHY did a fresh signup's run resolve to the demo/cwd workspace at all — is
fresh-signup → own-workspace provisioning (WS1-3 #2694) not reached by the
gateway *run* path, or is this purely the `first_workspace()` binding masking
it? The 3a fix addresses the memory-tool symptom regardless; flag for the
operator/conductor if the run path's tenant→workspace resolution needs the same
WS1 treatment the read routes already got.
