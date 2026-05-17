# codex-multi-tenant-isolation-bench — verify tenant isolation under concurrent load (P1 operational-audit + security)

**Owner**: codex. **Sizing hint**: GPT-5.5 Pro extra-high — judgment on what isolation invariants to assert.
**Depends on**: none (consumes existing tenant + auth surfaces).
**Authored against**: origin/main `37f268a6` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-multi-tenant-isolation-bench`.
**Source**: 2026-05-17 audit blind-spot — multi-tenant is core architecture (CLAUDE.md invariant #2). Static `TenantId` plumbing is verified; runtime tenant-data-isolation under concurrent load is unverified. Enterprise procurement asks: "prove that tenant A's data cannot leak to tenant B under any concurrent-request pattern."

## Mission alignment

- **Multi-tenant by construction** (CLAUDE.md core invariant #2) — without proof at load, this is a claim, not a property.
- **Security** (CLAUDE.md priority #4) — cross-tenant data leak is the worst possible bug class.

## Scope

P1. Integration test in `crates/argo-cli/tests/multi_tenant_isolation_load.rs`:

### Setup

1. Spawn gateway + initialize 10 tenants (A-J) via the substrate APIs (#1777).
2. Seed each tenant with: 100 audit rows, 50 memory rows, 20 workers, 10 skills (workspace-scoped + library-eligible).

### Concurrent load

3 spawn 10 concurrent threads, each acting as a different tenant (A-J):
   - Issue `GET /audit/feed` for OWN tenant only.
   - Issue `POST /api/v1/workers` to spawn own worker.
   - Issue `GET /workers/:id/memory` for own worker.
   - Issue `POST /dispatch` against own worker.
   - Each thread runs for 60 seconds at sustained 10 RPS.

### Assertions

After load completes, verify:

1. **Audit isolation**: no audit row from tenant A exists with `tenant_id = B` (cross-rg).
2. **Memory isolation**: no memory row from tenant A is queryable by tenant B.
3. **Worker isolation**: tenant A's worker count = 21 (initial 20 + 1 spawned during load); tenant B's = 21; etc. No drift.
4. **Skill library**: a library-scope skill from tenant A's library is NOT visible in tenant B's prompt context (per F-11 selector).
5. **Audit-chain isolation**: chain hashes do not collide across tenants (each tenant's chain is independent).
6. **Capability denial**: any cross-tenant action (e.g. tenant A trying to read tenant B's `/workers/:id`) returns 403 in audit row.

### Reporting

PR body includes per-test pass/fail. If any cross-tenant leak is detected, PR body flags it as P0 with the specific tenant pair + row count.

LOC: ~300.

## Files you may touch

- `crates/argo-cli/tests/multi_tenant_isolation_load.rs` (NEW) — integration test. ≤280 LOC.
- `crates/argo-cli/tests/fixtures/multi_tenant_seed.rs` (NEW or extend) — seeding helper. ≤120 LOC.
- `crates/argo-cli/Cargo.toml` — narrowly: ensure tokio + reqwest dev-deps (likely already present). ≤4 modified LOC.
- `docs/skycot/security/multi-tenant-isolation.md` (NEW) — operator runbook + assertions catalog + how to extend. ≤80 LOC.

## Files you must NOT touch

- Production gateway code — pure consumer of HTTP surface.
- Existing audit/memory/skill primitives — read-only via the HTTP API + DB queries.
- `argo-audit::verify` API — consume only.
- Tenant substrate (#1777) — consume only.

## Acceptance criteria

- [ ] Test runs end-to-end via `cargo test -p argo-cli --test multi_tenant_isolation_load -- --nocapture`.
- [ ] Test takes < 5 minutes wall-clock on a typical dev laptop (M-series).
- [ ] All 6 isolation assertions pass on current main.
- [ ] If any assertion fails, the test reports the specific cross-tenant pair + row count.
- [ ] Test ignores expected pre-existing flakes via `#[ignore]` markers or skip-logic.
- [ ] Operator runbook documents: how to run, what to do on failure, how to add a new isolation invariant.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-multi-tenant-isolation-bench`
- Title: `test(multi-tenant): concurrent-load isolation invariants — 6 cross-tenant assertions (codex-multi-tenant-isolation-bench)`
- Worker verification: crate-scoped clippy + integration test + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Pre-claim check: confirm #1796 magic-link MERGED.** Without auth, the test would need to bypass JWT — and the audit explicitly flagged signup security as a launch blocker. If #1796 hasn't merged at claim time, this brief is gated; worker should self-exit.
- **Don't reuse test tenant_ids across runs.** Generate per-run (e.g. `test-tenant-{nonce}`) so re-running doesn't conflict with prior state.
- **Cleanup teardown.** After the test, delete the 10 test tenants + all their workers/audit/memory/skill rows. The test SHOULD be idempotent.
- **Don't assert on timing.** This is a correctness test, not a perf test. It uses concurrent load to expose isolation bugs that synchronous tests miss.
- **Don't shortcut the auth path.** Each thread acts as a different tenant via a real session JWT (issued via the magic-link signup flow OR via an admin-issued JWT for testing).
- **Cross-tenant capability denial** is the most-load-bearing assertion. If a worker for tenant A tries to read tenant B's `/workers/:id`, the audit row MUST show capability denial. If the API just returns 200 with B's data, that's the worst possible bug.
- **Closes audit blind-spot** for the "multi-tenant by construction" invariant — provides runtime proof.
