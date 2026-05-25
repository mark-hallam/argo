# `argo-team-headline` crate surface audit

**Date**: 2026-05-24
**Brief**: `eval-team-headline-crate-surface`
**Mission link**: Claim 1 ("Teammate, not tool") — last unaudited cell in
the gap manifest's Claim 1 row. Failure mode is stale-headline UX
on the Team page rather than data loss; classified P3.
**Verdict**: 🟡 **PARTIAL.** Surface is small, coherent, and now pinned
by an external contract test. Production consumers tenant-gate at
the route layer before calling, but the crate's own API and SQL are
worker-id scoped — tenant isolation is implicit, not enforced inside
this crate. One follow-up brief named below.

## Public Surface Inventory

`crates/argo-team-headline/src/lib.rs` exposes 7 names. The new
contract test at `crates/argo-team-headline/tests/surface_pin_test.rs`
imports every one and asserts concrete shape.

| Slice | Re-exports |
|---|---|
| Constants | `MAX_BULLETS` (= 3), `MAX_BULLET_CHARS` (= 60) |
| Headline result | `TeamHeadline { bullets: Vec<String>, tomorrow: Option<String> }` (Serialize + Deserialize, both fields public) |
| Generator | `generate_team_headline(WorkerId, &str, &Arc<SessionStore>) -> ArgoResult<TeamHeadline>` |
| Activity rollup | `ActivityCounts { assistant_messages: i64, tool_calls: i64, pending_escalations: i64 }` (Default + Copy) |
| KPI table | `bullets_for_vertical(&str) -> &'static [BulletKind]`, `BulletKind { source, singular, plural }` |

The `BulletSource` enum is `pub` inside `kpi_table` but not
re-exported from `lib.rs`; it is reachable only through the
`BulletKind { source: BulletSource, .. }` field type at downstream
use-sites. `argo-cli::handlers::gateway::routes_worker_overview_kpis`
relies on that pattern.

## Production Consumers

| Consumer | Symbols used | Tenant context |
|---|---|---|
| `argo-cli::handlers::gateway::routes_team_list` | `generate_team_headline`, `TeamHeadline` | Workspace tenant resolved upfront (`resolve_workspace_tenant`); cross-tenant query rejected 403 before any worker is iterated. The crate call itself is passed only `WorkerId`. |
| `argo-cli::handlers::gateway::routes_worker_detail` | `generate_team_headline` (line 345) | Worker is loaded by id within an already tenant-scoped principal context; same implicit-tenant pattern. |
| `argo-cli::handlers::gateway::routes_worker_overview_kpis` | `bullets_for_vertical`, `ActivityCounts`, `BulletKind` | Pure shaping use; no DB call from this crate, tenant orthogonal. |

All consumers live in `crates/argo-cli`; no other workspace crate
depends on `argo-team-headline`.

## Test Coverage Added

`tests/surface_pin_test.rs` adds 7 active assertions over the external
surface:

| Contract | Pin |
|---|---|
| Re-export inventory | Compile-fence imports + concrete-shape asserts for all 7 public names |
| KPI ordering (`gtm-engineer`) | `bullets_for_vertical` returns exactly 3 slots; plural template renders `"8 replies sent"` |
| Singular templates | `BulletKind::format` with `n=1` renders `"1 reply sent"` (not `"1 replies sent"`) |
| Zero-count filter | All KPIs for `recruiter` return `None` when counts are zero |
| Fallback shape | Unknown vertical id renders the `"N actions taken today"` fallback |
| `TeamHeadline` JSON shape | Two top-level keys (`bullets`, `tomorrow`); `Option::None` serializes as JSON `null` |
| Tomorrow line shape | `Some(...)` serializes as a flat string the SPA reads directly |

The crate already has 17 internal unit tests under
`src/lib_tests.rs` covering `render_bullets`, DB-backed
`activity_counts_for_worker` window math, and `tomorrow.rs` cron
parsing. The external surface pin is additive; nothing internal is
duplicated.

## Tenant-Safety Assessment (Claim 1 invariant)

Mission invariant #2 (multi-tenant by construction): `tenant_id`
must be plumbed through every primitive that touches stored data.

Findings:

1. **`generate_team_headline` API does not accept a `TenantId`.**
   It takes `WorkerId + vertical_id + &Arc<SessionStore>`. Tenant
   safety relies on the caller having already resolved the correct
   `WorkerId` for the current tenant.
2. **The SQL in `compute::activity_counts_for_worker` joins
   `audit_record` on `session.worker_id = ?` only.** No `tenant_id`
   predicate. If a route bug ever passed a `WorkerId` belonging to a
   different tenant, the headline would silently leak that worker's
   activity bullets. `WorkerId` is a UUIDv7 generated per worker, so
   collisions are astronomically improbable — but the safety is
   structural-by-uniqueness, not enforced.
3. **`tomorrow::next_duty_for_worker` has the same shape.** Reads
   `role_duty WHERE worker_id = ?` without a `tenant_id` filter.

Production callers do tenant-gate at the route layer
(`routes_team_list.rs:54–64` resolves the workspace tenant and
rejects cross-tenant query strings 403), so today's wire is safe.
But the crate's contract does not prevent a future caller — say, a
worker-detail route that takes `?worker_id=` from a query string —
from passing an attacker-controlled id and leaking another tenant's
last-24-hour activity counts and KPI summary.

This is the same shape codex's earlier audit (PR #2199) reached;
naming the follow-up consistently keeps the queue de-duplicated.

## Findings

**F-1 — `argo-team-headline` is worker-id scoped, not tenant-bound.**
Crate-internal SQL filters by `worker_id` alone. Production callers
tenant-gate first, but a new gateway route that forwards an
externally-supplied `WorkerId` without re-validating tenant ownership
would surface cross-tenant activity bullets and Tomorrow lines. The
fix is additive — accept an optional `TenantId` in
`generate_team_headline` and add `AND s.tenant_id = ?` /
`AND r.tenant_id = ?` predicates to both queries. Existing callers
can either pass the resolved workspace tenant or use a back-compat
shim that retains today's behavior during the transition.

## Follow-Up Briefs

- `mission-team-headline-tenant-bind` — extend
  `generate_team_headline` to accept `TenantId`, add tenant
  predicates to the `audit_record` and `role_duty` queries, and
  update the two `argo-cli` route callers to pass the resolved
  workspace tenant. ~120–200 LOC. Audit-only today, fix-brief
  tomorrow.

## Verification

Required command:

```bash
cargo test -p argo-team-headline --test surface_pin_test
```

Crate-scoped clippy + the full crate test suite are also run; the
PR body reports both.
