# mission-cross-tenant-test-worker-memory

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `917290b98` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cross-tenant-test-worker-memory`.

🟡 **P2 SECURITY HYGIENE.** Source: `mission-cross-tenant-audit-sweep` (this PR) — `routes_worker_memory` is the **top-2 data-exposure risk** in `docs/skycot/audit/cross-tenant-endpoint-coverage.md`. The handler covers four verbs (GET / POST / PUT / DELETE) over the worker-memory facts surface, and memory facts can carry PII (contact info, customer-side context, decision rationale text). The sibling test file `routes_worker_memory_tests.rs` exercises the happy path and tombstone semantics but does NOT include a cross-tenant negative test.

## Scope

Add cross-tenant negative tests to `routes_worker_memory_tests.rs` for each of the four verbs:

1. **GET /api/v1/workers/<id>/memory** — Tenant B GET on a worker owned by tenant A → 404 (the existing `resolve_worker_ctx` should already scope this; the test pins it).
2. **POST /api/v1/workers/<id>/memory/facts** — Tenant B POST to a tenant-A worker → 404; verify the underlying `memory_entry` table has zero new rows.
3. **PUT /api/v1/workers/<id>/memory/facts/<fact_id>** — Tenant B PUT against a fact created by tenant A → 404; original fact text unchanged.
4. **DELETE /api/v1/workers/<id>/memory/facts/<fact_id>** — Tenant B DELETE on a tenant-A fact → 404; fact still active (no tombstone marker written).

One bundled test per verb is fine — they share the same fixture
setup. Mirror the existing helper pattern (`env_lock()`,
`workspace_root()`, `fact_create_response`, etc.) — do not introduce
a parallel test infrastructure.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_worker_memory_tests.rs` — additive only; do not edit existing tests.
- `crates/argo-cli/src/handlers/gateway/routes_worker_memory.rs` — ONLY if a verb reveals a real cross-tenant bug AND the fix is ≤30 LOC and obviously correct (e.g. an `ensure_tenant_match` call already exists for one verb and not another). Otherwise: stop the brief, document the finding, queue a fix-brief.
- `crates/argo-cli/src/handlers/gateway/routes_worker_memory_helpers.rs` — ONLY a test-helper extension (e.g. `seed_fact_for_tenant`) if needed; ≤30 LOC.

## Files you must NOT touch

- Any other handler under `crates/argo-cli/src/handlers/gateway/`.
- `argo-memory` crate.
- `IdentityStore` / `argo-identity` paths.
- BUILD_STATUS.md outside the standard row flip.

## Acceptance criteria

- [ ] Four new `#[tokio::test]` fns in `routes_worker_memory_tests.rs`, one per verb, named with the `cross_tenant_` prefix.
- [ ] Tests use tenant ids `"tenant-a"` / `"tenant-b"` so the heuristic in `scripts/audit-cross-tenant-coverage.sh` flips this file to ✅.
- [ ] Each test asserts: the HTTP status is the documented isolation response (404 per handler comment), AND the underlying table state is unchanged (no rows added / mutated / tombstoned).
- [ ] `cargo test -p argo-cli routes_worker_memory` passes (including new fns).
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] If any verb surfaces a real cross-tenant bug, brief is BLOCKED with one-line finding; PR body's "Findings" section names the broken verb + the missing check; no production code edited unless the trivial-fix carve-out above applies.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-cross-tenant-test-worker-memory`
- PR title: `test(gateway): cross-tenant negative for /api/v1/workers/<id>/memory* (mission-cross-tenant-test-worker-memory)`

## Notes for the implementer

- ~250-400 LOC expected (four test fns + ~50 LOC of helper if the existing `seed_fact_*` helpers don't take a tenant argument).
- Read `routes_worker_freeze_tests.rs` first — it's the canonical worker-scoped cross-tenant test pattern in this directory; lines roughly 220-260 hold the tenant-B-on-tenant-A-worker assertion. Mirror that shape.
- The handler comment says the GET/POST/PUT/DELETE all resolve via `resolve_worker_ctx(tenant, worker_id)`. If true, the four tests should all pass on first run (the brief is then proof-of-coverage). If any pass-through to the store layer is missing a tenant filter, the bug is real and the brief BLOCKs.
- Memory facts are mutable; the test must capture pre-state (e.g. `fetch_active_facts(...)` returns N rows), run the cross-tenant verb, then assert post-state equals pre-state. Do NOT rely on the return body alone — a handler could return 404 but still have side-effects in a buggy code path.
- This is a TEST brief. Same fix-brief discipline as the parent audit
  sweep: don't ship a production fix in this PR unless trivially
  scoped.
