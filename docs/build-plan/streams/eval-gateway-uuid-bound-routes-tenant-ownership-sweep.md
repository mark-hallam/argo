# eval-gateway-uuid-bound-routes-tenant-ownership-sweep

**Owner**: claude.
**Depends on**: nothing.
**Authored against**: origin/main `746b0e213` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-gateway-uuid-bound-routes-tenant-ownership-sweep`.

🟡 **P1 SUBSTRATE SWEEP — enumerate every gateway route that binds on a UUID alone without an upstream tenant-ownership pre-flight check.**

## Source

Evaluator round 35 (SHARED.md handoff 2026-05-23T12:30Z, scheduled-task hourly fire):

> Remaining unactioned brief candidates: (a) **`eval-gateway-uuid-bound-routes-tenant-ownership-sweep`** (P1, ~150-200 LOC) — round-33's `mission-approval-decide-request-tenant-pinned` + tick-110's `mission-gateway-session-export-tenant-bind` are two instances of the same pattern (gateway routes binding on UUID alone without tenant ownership pre-flight); a sweep brief should enumerate every such route systematically.

## What's wrong (the pattern)

UUIDs are global namespaces. When a gateway route is shaped like `/api/v1/<noun>/<uuid>/<verb>` and the handler binds the SQL on `WHERE id = ?` only (no `AND tenant_id = ?`), tenant_b can use a UUID it shouldn't have to mutate or read tenant_a's row. Two known instances:

- `approvals::decide_request` (round-33 finding, fix-brief `mission-approval-decide-request-tenant-pinned` TODO).
- `session_export` (tick-110 finding, fix-brief `mission-gateway-session-export-tenant-bind` TODO).

There are likely 5-15 more. This sweep produces the inventory.

## What this brief must do

1. **Enumerate** every route handler in `crates/argo-cli/src/handlers/gateway/routes_*.rs` whose URL path includes a UUID parameter (look for `Path((id,))` or `Path((uuid,))` and `Uuid::parse_str`).
2. **For each handler**, trace the SQL path it ultimately invokes (the store function). Categorize:
   - 🟢 **Pre-flight CHECKED** — handler calls a tenant-checking helper (e.g. `get_request` returns `tenant_id` and the handler asserts it matches `principal.tenant_id` before mutating).
   - 🟢 **Store FILTERED** — store function takes `tenant_id` and binds it in SQL.
   - 🔴 **GAP** — neither pre-flight nor store filter; tenant_b's principal could pass tenant_a's UUID and the mutation would land.
2. **Produce a report** at `docs/skycot/evals/scheduled/2026-05-23/eval-gateway-uuid-bound-routes-tenant-ownership-sweep.md` with a per-route table: route path, handler file:line, store fn called, pre-flight present?, store-filter present?, verdict.
3. **Add 1+ falsifiable pin tests** in `crates/argo-cli/tests/gateway_uuid_routes_tenant_ownership.rs`:
   - For each 🔴 GAP, write an `#[ignore]`'d FINDINGS-ONLY pin that calls the route with tenant_b's principal but tenant_a's UUID and asserts the leak (so a fix-brief can un-ignore later and invert).
   - For each 🟢 CHECKED, write a positive regression that asserts tenant_b gets a 404 (not 403 — existence-leak class).
4. **For every 🔴 GAP, author a sibling fix-brief** named `mission-gateway-<route>-tenant-bind`. List them in the report. Conductor will pick them up Step-8.

## Acceptance

- Report exists with per-route inventory.
- ≥1 pin test green (positive case for at least one 🟢 CHECKED handler).
- ≥1 sibling fix-brief authored per 🔴 GAP (zero is acceptable if sweep finds none).
- `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.

## Sizing

~150-300 LOC across 1 report + 1 test file + N sibling-brief stubs. ~2-4 hours.

## Why P1

Sweep audits frequently surface 3-10 latent leaks. Each is potentially a P0 if a careless caller wires it. Closing the inventory now is far cheaper than reacting to a found-in-prod leak per route.
