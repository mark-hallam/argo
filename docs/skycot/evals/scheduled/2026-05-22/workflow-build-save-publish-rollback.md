# Eval: Workflows — build → save → publish → run → rollback lifecycle

**Date**: 2026-05-22
**Pillar**: Workflows (build canvas + version control)
**Mission claim**: 3 — Platform builds workers
**Verdict**: 🔴 **FAIL** — gateway crashes on fresh-from-zero boot (same root cause as the skill scenario)

## Setup

Two paths were available:

1. **The live gateway at `:7400`.** Rejected — the gateway is 92h stale (schema v70 vs code v76), so the `/api/v1/verticals/{vid}/workflows` route doesn't even exist on it. See `gateway-process-stale-vs-codebase.md`.
2. **`workflow_edit_api_smoke.rs` integration test.** This spawns a fresh `argo gateway start --port <free>` against a tempdir-backed SQLCipher DB and exercises all 7 routes (list / head / versions / version / create / set-head / archive / rollback). Picked this path.

## Action

```bash
cargo test -p argo-cli --test workflow_edit_api_smoke workflow_edit_api_lifecycle -- --nocapture
```

(54-minute compile from a cold cache contested with the parallel skill test build.)

## Expected

From the test docstring (`workflow_edit_api_smoke.rs:1-22`):

- anonymous (no `X-Argo-Principal`) → 401 on GET and POST
- authenticated reader (`X-Argo-Role: user`) → 200 on GET, 403 on POST
- authenticated editor (`X-Argo-Role: owner`) → 201 on POST, 200 on subsequent GET
- `PUT /head` flips the head pointer; `GET /{name}` reflects it
- `DELETE /versions/{id}` on the current head clears the head pointer

## Observed

```
running 1 test

thread 'workflow_edit_api_lifecycle' panicked at
  crates/argo-cli/tests/workflow_edit_api_smoke.rs:87:47:
gateway listener signal: GatewayCrashed

test workflow_edit_api_lifecycle ... FAILED

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
```

The spawned gateway child crashed before announcing its listener. `GatewayCrashed` is defined at `crates/argo-cli/tests/common/listener_signal.rs:38` — it fires when the child's stderr closes before the `listening on port X` announce line.

## Anti-theater check + root cause

This is the **same root cause** as the skill cross-tenant test failure: PR [#1988](https://github.com/anthropics/argo/pull/1988) (commit `c92abfa7f`) introduced semicolons inside `-- ` line comments in 48 migration SQL files. The `split_terminator(';')` pattern in `crates/argo-store/src/migrations/v43.rs:25` chops the comment on the first inline `;` and hands raw English text (`operator-driven and not tenant-partitioned.`) to SQLite as a SQL statement.

For the `:memory:` skill test, this surfaces as `panicked: migrate store: Provider("v43 up: ... near \"operator\": syntax error")`.

For the workflow lifecycle smoke (which boots a real `argo gateway start` against a fresh SQLCipher tempdir DB), the same migration failure crashes the gateway during its boot-time `migrate()` call, so the child dies before opening its TCP listener. The test sees `GatewayCrashed`.

Both surfaces, one bug. See [`skill-cross-tenant-leak-blocked-by-migration-bug.md`](skill-cross-tenant-leak-blocked-by-migration-bug.md) for the full forensics.

## What I cannot verify because of this

- That `POST /api/v1/verticals/<vid>/workflows/<name>` writes a `workflow_definition` row + a `WorkflowDefinitionCreated` audit row.
- That `PUT .../head` actually flips the head pointer + writes `WorkflowHeadChanged`.
- That `POST .../rollback` writes `WorkflowRolledBack` with both `rolled_back_from_version_id` and `new_version_id` in the payload — the new mission-claim-3 audit kind that #1980 shipped two days ago.
- That the `X-Argo-Role: user` → 403 / `X-Argo-Role: owner` → 201 RBAC seam works end-to-end.

None of those are visible from `cargo build`-only CI. They need this test to pass.

## Independent finding — `cli_install_then_workflow_run_round_trip` separately FAILS

The other workflow-named test in the cargo run also failed:

```
thread 'cli_install_then_workflow_run_round_trip' panicked at
  crates/argo-cli/tests/vertical_install.rs:346:5:
workflow stderr: argo: provider error: workflow 'go':
                 one or more steps failed or were denied by the vertical's [tools].denied list
```

This is a DIFFERENT failure — not migration-related. The test installs a `greeter` vertical and runs a `go` workflow, but the workflow run hits a step that the vertical's `[tools].denied` list rejects. Either the test fixture's vertical TOML doesn't grant the tool it needs, or the per-vertical tool deny-list was tightened recently and the test fixture wasn't updated. Worth a separate triage line (lower P0 than the migration bug).

## Gap / follow-up brief candidates

**🔴 P0 (shared with skill scenario) — `claude-migration-comment-aware-splitter`**: The same fix unblocks BOTH the cross-tenant skill test AND this workflow lifecycle smoke. Acceptance:
- This test passes (`workflow_edit_api_lifecycle`).
- The skill test passes (`claim4_learned_skill_publishes_and_reloads_without_cross_tenant_leak`).

**🟡 P1 — `claude-vertical-install-greeter-tool-allowlist-fix`**: Tackle `cli_install_then_workflow_run_round_trip` separately. The fixture's vertical TOML at `crates/argo-cli/tests/fixtures/vertical_install/greeter/...` either needs the missing tool added to `[tools].allowed`, or the workflow needs to be changed to use only allowed tools. Low blast radius (test fixture + one TOML).

**🟢 INFO**: Once the P0 lands, re-run this scenario. The 7 routes the smoke covers are exactly the routes I tried to curl against the stale `:7400` gateway. Both paths converge once the migration unblocks.

## Static read of the workflow API surface (no execution)

I CAN attest from source-reading that the routes are wired and look correct:

- [`crates/argo-cli/src/handlers/gateway/routes_workflows.rs:1-40`](crates/argo-cli/src/handlers/gateway/routes_workflows.rs) — handler + audit emissions for all four mutating verbs (`WorkflowDefinitionCreated`, `WorkflowHeadChanged`, `WorkflowVersionArchived`, `WorkflowRolledBack`).
- [`crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs:34`](crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs) — GET dispatcher wires `routes_workflows::dispatch_get` as the last arm so any prior `/api/v1/...` handler can shadow it (none do).
- [`crates/argo-cli/src/handlers/gateway/routes_workflows.rs:parse_route()`](crates/argo-cli/src/handlers/gateway/routes_workflows.rs) — pure URL parser; pattern matches all 8 verb/path combinations (List / Head / Versions / Version / Create / SetHead / Archive / Rollback).
- [`crates/argo-cli/src/handlers/gateway/routes_workflows_tests.rs:1-30`](crates/argo-cli/src/handlers/gateway/routes_workflows_tests.rs) — three unit tests covering `parse_route` shape correctness (rollback recognition, non-POST rejection, existing-route preservation).

Architecturally this looks healthy. The blocker is purely the fresh-DB migration bug.
