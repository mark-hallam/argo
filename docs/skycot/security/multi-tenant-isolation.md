# Multi-tenant isolation — runtime invariants

Runtime proof of CLAUDE.md core invariant #2 (*Multi-tenant by
construction*). Enterprise procurement asks: "prove tenant A's data
cannot leak to tenant B under any concurrent-request pattern." This
runbook covers the test that answers that, lives the assertions
catalog, and explains how to extend it when a new isolation surface
ships.

## Run

```bash
cargo test -p argo-cli --test multi_tenant_isolation_load -- --nocapture
```

Wall-clock budget: <5 min on an M-series laptop. Boots `argo gateway
start` in a subprocess against a fresh `ARGO_HOME`, provisions 10
tenants, runs the concurrent load + assertions, tears the gateway
down on test exit.

## What it covers

Six assertions, each catching a different cross-tenant failure mode:

1. **Audit isolation** — `audit_record.tenant_id` partitions every
   tenant's rows. The `payload_json.tenant_id` denorm field must
   match the column (the audit-feed handler filters on the denorm
   for hot reads).
2. **Memory isolation** — `memory_entry.workspace_id` resolves to
   exactly one tenant; row count matches the seed + load total.
3. **Worker isolation** — `worker.tenant_id` partitions; exact-count
   match per tenant.
4. **Skill namespace isolation** — v1 `skill` rows are tenant-global
   (no `tenant_id` column); the test asserts seeded names don't
   collide. The library-scope F-11 selector check
   ("library skill from A not visible in B's prompt context") lives
   in `tests/cross_tenant_skill_discovery_negative.rs` — when a
   tenant_id column lands on `skill`, fold that check in here.
5. **Audit-chain integrity** — `argo_audit::verify_all` re-walks
   every per-session hash chain after the concurrent load and
   reports `ok = true` with zero breaks.
6. **Capability denial** — `GET /api/v1/audit?as_tenant=<other>`
   issued WITHOUT the `X-Argo-Audit-Cross-Tenant: 1` capability
   header must NOT surface the other tenant's rows. The handler
   silently filters to the caller's workspace tenant (returning a
   status code would leak the target tenant's existence — see
   `routes_audit_feed.rs:39`).

## What to do on failure

- **A specific assertion fails.** The test prints the tenant pair
  and the row count. Investigate that surface first; if the failure
  is `cross_tenant_leak`, file a P0.
- **`verify_all` reports broken chains.** That's a P0 — the audit
  trail is the legal evidence trail. Bisect against `git log
  -p crates/argo-audit/src/sink.rs` and the `audit_record` schema
  migrations.
- **Capability denial breached.** That's the worst class of bug.
  Run the negative test in isolation
  (`cargo test -p argo-cli --test multi_tenant_isolation_load
  capability_denial`) to confirm and then escalate.
- **Test times out.** `boot_gateway_with_home` polls a TCP connect
  for 30s. If the gateway is failing to bind, inspect stderr from
  the spawned child (uncapture with `RUST_LOG=debug` set in the
  test env).

## How to extend

Adding a new isolation invariant:

1. Add the assertion as a `assert_<name>` function in
   `tests/multi_tenant_isolation_load.rs`. Follow the existing
   pattern: pull per-tenant counts from `store.pool()`, compare
   against the seed + load total.
2. If the assertion needs new seed data, extend `SeedCounts` and
   `seed_tenant_rows` in `tests/fixtures/multi_tenant_seed.rs`.
3. List the assertion in the catalog above with a one-line
   description of the failure mode it catches.

The deliberately-narrow may-touch list (test + fixture + this doc +
`Cargo.toml`) is the structural defense — production code stays a
read-only consumer of the test, so the test cannot mask a
gateway-side regression by changing what the gateway does.
