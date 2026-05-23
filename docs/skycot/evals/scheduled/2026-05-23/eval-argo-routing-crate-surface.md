# `argo-routing` crate surface inventory

**Authored**: 2026-05-23 (evaluator follow-on to gap manifest
`argo-routing` row).
**Brief**: `docs/build-plan/streams/eval-argo-routing-crate-surface.md`.
**Verdict (short form)**: 🟡 Surface inventory is clean and stable
(7 re-exports, 4 `Router` impls, single production consumer). Two
real wire-gaps surfaced and pinned as FINDINGS-ONLY tests with
named fix-briefs:

1. **`fix-routing-decision-tenant-scoping`** — `RoutingContext` /
   `RoutingDecision` / `SessionStore::persist_routing_decision` /
   the `routing_decision` table all elide `tenant_id`. Tenant
   scoping is implicit-via-session and unenforced at the store
   boundary.
2. **`fix-routing-decision-persist-wire`** — the only production
   caller (`argo_agent_runner::turn_routing::maybe_route_turn_with_
   transport`) builds `TieredRouter::new(...)` directly and emits an
   `AuditKind::RoutingDecision` audit row, but does NOT wrap with
   `PersistingRouter`, so the `routing_decision` table receives zero
   writes in production today. The §24.4 spec acceptance ("all
   non-Fixed decisions persisted in `routing_decision`") is
   currently met by the audit log only.

A third clarification: the gap manifest's annotation that
`argo-routing` is the substrate for `mission-wire-api-action-
executor` is a misattribution. `ApiActionExecutor` is an HTTP REST
executor for worker actions
(`crates/argo-cli/src/handlers/worker_actions_executor_api.rs`); it
shares no abstraction with `argo-routing`'s cost-tier `Router`
trait. The two crates do not interact at the type level. The fix
brief for `ApiActionExecutor` can proceed independently.

## §1 — Public surface inventory

The crate exposes exactly **7 re-exports** from `lib.rs:31-36`:

| Re-export             | Source module     | Kind  | Purpose |
|-----------------------|-------------------|-------|---------|
| `TierCache`           | `cache`           | struct | TTL cache for `(session_id, hash(msg)) → tier` (§24.3, 60 s default). |
| `DEFAULT_TTL`         | `cache`           | const  | 60 s — spec default for the tier cache. |
| `Router`              | `router`          | trait  | `async fn route(&self, ctx) -> ArgoResult<RoutingDecision>` — the cost-tier seam (§24.1). |
| `RoutingContext`      | `router`          | struct | Per-turn input (session_id, turn, messages, user_message, tools_in_scope, default_model, policy, deadline). |
| `FixedRouter`         | `fixed`           | struct | Zero-overhead: always returns the default model with `policy="fixed"` / `tier="unrouted"` (§24.2). |
| `TieredRouter`        | `tiered`          | struct | Manifest-driven classifier router (§24.2). |
| `PersistingRouter`    | `persist`         | struct | Wrapper that writes non-`fixed` decisions to `routing_decision` (§24.4). |
| `PolicyPluginRouter`  | `policy_plugin`   | struct | **Permanent v1 stub** — returns `ArgoError::RoutingFailed` until the Phase 4 plugin host lands. |

LOC count (production source, no tests): **375 LOC across 7 files**.

| File                             | LOC | Notes |
|----------------------------------|-----|-------|
| `src/lib.rs`                     |  36 | Crate header + 7 re-exports. |
| `src/router.rs`                  |  44 | `Router` trait + `RoutingContext` struct. |
| `src/fixed.rs`                   |  75 | `FixedRouter` + 1 unit test. |
| `src/persist.rs`                 |  43 | `PersistingRouter` wrapper. |
| `src/policy_plugin.rs`           |  37 | Stub returning `RoutingFailed`. |
| `src/cache.rs`                   | 140 | `TierCache` + 5 unit tests. |
| `src/tiered.rs`                  | 236 | `TieredRouter` classifier loop. |
| `src/tiered/tests.rs`            | 282 | 9 in-module tests (mock classifier). |
| `tests/persist.rs`               | 157 | 3 integration tests (persist → routing_decision). |
| `tests/surface_inventory_test.rs`| ~350 | This brief: 9 passing pins + 2 FINDINGS-ONLY. |

All production files are well under the 500-LOC ceiling. No file
splits required.

## §2 — Consumer trace

Workspace consumers of `argo-routing` symbols
(`rg "use argo_routing|argo_routing::" --type rust`):

| Crate                                 | Symbols used                                  | Role |
|---------------------------------------|------------------------------------------------|------|
| `argo-agent-runner::turn_routing`     | `Router, RoutingContext, TieredRouter`         | **Sole production caller.** |
| `argo-routing::tiered::tests`         | (intra-crate)                                  | Unit tests. |
| `argo-routing::tests::persist`        | `FixedRouter, PersistingRouter, Router, RoutingContext` | Integration test. |
| `argo-routing::tests::surface_inventory_test` | All 7 re-exports                       | **This brief.** |

The only production-side touchpoint is **one function**:
`crates/argo-agent-runner/src/turn_routing.rs:67`
(`maybe_route_turn_with_transport`). Its shape:

```rust
let router = TieredRouter::new(classifier);   // ← line 87
// …
let decision = router.route(&ctx).await?;
// …
let _ = audit.write(Some(session.id), AuditKind::RoutingDecision, payload, None).await;
```

`PersistingRouter::new` is referenced **zero** times in production
(grep verified). The integration test at `tests/persist.rs` is the
only workspace consumer.

## §3 — Tenant-scoping gap (FINDINGS-ONLY pin #1)

### Observation

Tenancy plumbing is absent from every layer of the routing
subsystem:

- **`RoutingContext` (router.rs:11-31)** carries `session_id`,
  `turn`, `messages`, `user_message`, `tools_in_scope`,
  `default_model`, `policy`, `deadline` — no `tenant_id`.
- **`Router::route` (router.rs:43)** signature returns
  `ArgoResult<RoutingDecision>` — no tenant binding.
- **`RoutingDecision`** (in `argo-types`) carries `id`,
  `session_id`, `turn`, `policy`, `tier`, `classified_at`,
  `classifier_latency_ms`, `chosen_provider_model`, `reason` — no
  `tenant_id`.
- **`SessionStore::persist_routing_decision`
  (session_query.rs:265)** takes `&RoutingDecision` and binds only
  the fields above — no tenant column to bind to.
- **`routing_decision` table (v1.rs:145-156)** has no `tenant_id`
  column; the only tenancy link is the `session_id BLOB NOT NULL
  REFERENCES session(id)` foreign key. The `session` table is
  tenant-scoped, so the JOIN to recover tenancy is intact, but is
  not used by any current read path.

### Implication

`SessionStore::list_routing_decisions(session_id)` accepts any
caller-supplied `session_id` and returns the matching rows. The
store layer enforces nothing. A handler that takes a caller's
`session_id` parameter and forwards it without first checking the
requesting principal's tenant ownership of that session will leak
cross-tenant routing decisions.

This is a known pattern in `argo-store` more broadly — tenant
checks happen at the handler boundary, not the store. The
finding is that **the routing subsystem lacks a tenant-aware
store API**, so handlers cannot use a tenant-keyed call even if
they want to defense-in-depth-check.

### Risk assessment

- **Severity**: 🟡 P1, not P0. The only production caller
  (`maybe_route_turn_with_transport`) gets its `session_id` from
  `SessionHeader`, which is loaded from the session table by the
  agent loop's own session-resolution path (which is tenant-
  scoped via `session.tenant_id` lookups upstream). The router
  itself never reads back its decisions, so today no read-path
  leak exists.
- **Future risk**: an audit-inspection surface (`argo route
  inspect <session>`) that exposes per-session routing decisions
  would inherit the gap unless it implemented its own
  tenant-gate.

### Fix-brief shape

**`fix-routing-decision-tenant-scoping`** —

1. Add a `tenant_id BLOB NOT NULL REFERENCES tenant(id)` column
   to `routing_decision` (additive migration per `docs/skycot/
   operator/migrations.md`).
2. Backfill from the joined session row.
3. Extend `RoutingContext` with `pub tenant_id: TenantId` and
   require it at construction.
4. Extend `RoutingDecision` (in `argo-types`) with
   `pub tenant_id: TenantId`.
5. Replace `persist_routing_decision(&decision)` with
   `persist_routing_decision_for_tenant(&tenant, &decision)`,
   asserting the bound tenant matches `decision.tenant_id` and
   refusing on mismatch.
6. Replace `list_routing_decisions(session_id)` with
   `list_routing_decisions_for_tenant(tenant, session_id)`,
   filtering by both.
7. Flip the two FINDINGS-ONLY tests in
   `tests/surface_inventory_test.rs` from `#[ignore]` to live.

Pinned by: `persist_routing_decision_does_not_check_tenant`
(passes today, fails when (5) lands) +
`routing_decision_table_has_no_tenant_id_column` (passes today,
fails when (1) lands).

## §4 — Production persistence wire-gap (FINDINGS-ONLY pin #2)

### Observation

`crates/argo-agent-runner/src/turn_routing.rs:67-116` is the only
production caller. Its construction (line 87) is
`TieredRouter::new(classifier)` — bare, not wrapped:

```rust
let router = TieredRouter::new(classifier);
// …
let decision = router.route(&ctx).await?;
// payload built …
let _ = audit
    .write(Some(session.id), AuditKind::RoutingDecision, payload, None)
    .await;
```

Persistence is **only** to the audit log
(`AuditKind::RoutingDecision`, an enum variant in
`argo-audit/src/event_kind.rs`). The `routing_decision` table is
NEVER written-to from production code on origin/main.

### Implication

`docs/plan/argo-build-plan.md` §24.4 states the acceptance
criterion: *"all non-Fixed decisions persisted in
`routing_decision`"*. On origin/main today this is met only
indirectly via the audit log. Two downstream surfaces this
matters for:

- `argo route inspect` (if it queries `routing_decision`, will
  see an empty table for any session not exercised by the test
  suite).
- Future cost-rollup / per-tier-spend reporting that reads the
  `routing_decision` table.

### Risk assessment

- **Severity**: 🟡 P1, not P0. The audit log preserves the
  forensic information today. The risk is divergence between the
  spec's named persistence target and the observable storage
  shape — i.e. anyone who reads §24.4 will look in the wrong
  place.

### Fix-brief shape

**`fix-routing-decision-persist-wire`** —

1. In `turn_routing.rs:87`, change
   ```rust
   let router = TieredRouter::new(classifier);
   ```
   to
   ```rust
   let router = PersistingRouter::new(TieredRouter::new(classifier), store.clone());
   ```
2. Decide: keep the `AuditKind::RoutingDecision` write or rely
   on the routing_decision table? Currently both could land — the
   audit row gives event-stream consumers a cheap watch; the
   table gives queryable per-session history. Recommend keeping
   both.
3. Flip the FINDINGS-ONLY test
   `production_turn_routing_wraps_tier_router_in_persisting_router`
   from `#[ignore]` to live.

Pinned by:
`production_turn_routing_wraps_tier_router_in_persisting_router`
(today: `#[ignore]`'d FINDINGS).

## §5 — `PolicyPluginRouter` is a permanent v1 stub

`PolicyPluginRouter::route` (policy_plugin.rs:29-37) returns
`ArgoError::RoutingFailed { reason: "PolicyPluginRouter(<plugin>)
requires the Phase 4 plugin host (Agents I/J)" }` for every call.
The crate uses it nowhere in production (grep verified: only the
re-export in `lib.rs:34` references the type). This is the
intentional v1 shape — the cost-tier routing seam is reserved
for the plugin host (`§24.2`) that lands with the Phase 4 plugin
ABI work. No action required for v1.

Pinned by: `policy_plugin_router_is_a_stub_returning_routing_failed`.

## §6 — `ApiActionExecutor` is a separate subsystem

The gap-manifest cell says:

> `argo-routing` … Per MISSION.md status: "ApiExecutor remains
> BLOCKED on schema-vs-acceptance mismatch (mission-wire-api-
> action-executor)".

This cite is a misattribution. `ApiActionExecutor` lives in
`crates/argo-cli/src/handlers/worker_actions_executor_api.rs` and
dispatches HTTP REST API calls for worker actions. It shares no
type, no trait, and no callsite with `argo-routing`'s cost-tier
`Router`. The two crates do not interact.

The `mission-wire-api-action-executor` brief at
`docs/build-plan/streams/mission-wire-api-action-executor.md`
lists its may-touch as `argo-routing/src/, argo-runtime/src/,
argo-cli/handlers/gateway/` — but the implementing
work is entirely under the `argo-cli/handlers/` tree
(`worker_actions_executor_api.rs`, `worker_actions_executor.rs`,
etc.). The `argo-routing` line in the may-touch list appears to
be over-broad and can likely be dropped from the brief.

**Recommendation**: the conductor amends the
`mission-wire-api-action-executor` brief's may-touch list to
drop `argo-routing/src/` and replace it with the actual API-
executor location. The two gaps surfaced in this audit
(`fix-routing-decision-tenant-scoping` +
`fix-routing-decision-persist-wire`) are independent work.

## §7 — What this PR ships

1. `crates/argo-routing/tests/surface_inventory_test.rs` (NEW) —
   9 passing pin tests + 2 `#[ignore]`'d FINDINGS-ONLY tests
   that capture the two wire gaps described in §3 and §4. The
   ignored tests are explicitly named after their fix-briefs so
   they're grep-able for the next worker that lands either fix.

2. This report
   (`docs/skycot/evals/scheduled/2026-05-23/eval-argo-routing-
   crate-surface.md`) — the canonical writeup.

3. `docs/build-plan/BUILD_STATUS.md` — row flip to `IN_REVIEW`
   with the PR number.

No production source edits. The crate's behavior is unchanged on
origin/main; the audit observes and pins, it does not modify.

## §8 — Citations

- `crates/argo-routing/src/lib.rs:31-36` — the 7 re-exports.
- `crates/argo-routing/src/router.rs:11-31` — `RoutingContext`
  fields.
- `crates/argo-routing/src/persist.rs:14-35` — `PersistingRouter`
  wraps `R: Router`, writes via `store.persist_routing_decision`.
- `crates/argo-routing/src/policy_plugin.rs:29-37` — stub
  `RoutingFailed` return.
- `crates/argo-store/src/session_query.rs:265-286` —
  `persist_routing_decision` takes no tenant arg.
- `crates/argo-store/src/migrations/v1.rs:145-156` —
  `routing_decision` table schema (no `tenant_id`).
- `crates/argo-agent-runner/src/turn_routing.rs:67-116` — sole
  production caller; constructs `TieredRouter::new` bare.
- `crates/argo-audit/src/event_kind.rs` — `AuditKind::RoutingDecision`.
- `docs/build-plan/streams/mission-wire-api-action-executor.md` —
  the brief whose may-touch over-broadly includes `argo-routing/src/`.
- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-
  manifest.md` (claim 5, `argo-routing` row) — the gap-manifest
  cell this report resolves.
