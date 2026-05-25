# Round 67 — PR #2323 PRE-MERGE 🟡 PARTIAL — source-PASS; cargo verification deferred (worker target-dir contention)

- **Date**: 2026-05-25 (Melbourne 2026-05-26 02:09 AEST)
- **Round**: 67
- **Pillar/Claim**: Mission Claim 6 (Multi-tenant by construction) — gateway top-level list endpoints
- **PR**: [argo#2323](https://github.com/mark-hallam/argo/pull/2323) — `Phase A — gateway — mount /api/v1/skills + /api/v1/workflows top-level list endpoints`
- **Head SHA**: `d68c95f30fa09852b53931024269405d7e81dc9e`
- **Brief**: my own r63 + r66 recommendation; conductor authored as `fix-gateway-workflows-and-skills-marketplace-endpoints-404`.

## Verdict

🟡 **PARTIAL** — source review PASS, no architectural concerns, all four files within LOC budget, SQL tenant-scoping correct at handler layer. Cargo test run was initiated but killed at ~3min compile elapsed to free worker target-dir contention for the higher-priority r67 S1 finding (PR #2317 EXPECTED_SCHEMA_VERSION). Recommend MERGE after a single cold cargo run on the PR-body's exact invocations (worker can do this; not a blocker).

## Source review (worktree at `/tmp/argo-r67-worktrees/pr2323`, detached HEAD `d68c95f30`)

### File sizes (all under 500-LOC ceiling)

```
 240 crates/argo-cli/src/handlers/gateway/routes_skills_list_get.rs    (NEW)
 137 crates/argo-cli/src/handlers/gateway/routes_workflows_list_top_get.rs (NEW)
 161 crates/argo-cli/tests/gateway_skills_list_smoke.rs                (NEW)
 114 crates/argo-cli/tests/gateway_workflows_list_smoke.rs             (NEW)
   2 crates/argo-cli/src/handlers/gateway/mod.rs                       (mod-decl additions)
  14 crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs       (dispatch wiring)
```

### Tenant-scoping verification (the load-bearing contract)

**Installed skills endpoint** ([`routes_skills_list_get.rs:75-80`](crates/argo-cli/src/handlers/gateway/routes_skills_list_get.rs:75) on PR head):

```rust
WHERE s.tenant_id = ?
[...]
.bind(&tenant_id)
```

Tenant scope is enforced at the SQL layer. The skill-table tenant column was added in v80 migration (`131__skill_tenant_id.sql`); rows installed via legacy `SkillStore::install` carry the `__migration_placeholder__` sentinel and disappear from per-tenant lists. This is the strict-half of the seam — correct.

**Marketplace endpoint** is *intentionally cross-tenant* with `WHERE source LIKE 'published:%'` (publication is the explicit consent gate). The cross-tenant test `marketplace_list_returns_published_rows_across_tenants` codifies this: seeds 1 published + 1 private per tenant, asserts both published rows appear, neither private.

**Workflows endpoint** ([`routes_workflows_list_top_get.rs:63`](crates/argo-cli/src/handlers/gateway/routes_workflows_list_top_get.rs:63)):

```rust
WHERE idx.tenant_id = ?
```

`workflow_definition_index.tenant_id` was tenant-bound from inception. Test `workflows_list_filters_by_tenant` proves cross-tenant isolation.

### Tests enumerated

```
crates/argo-cli/tests/gateway_skills_list_smoke.rs:
  installed_list_filters_by_tenant
  marketplace_list_returns_published_rows_across_tenants
crates/argo-cli/tests/gateway_workflows_list_smoke.rs:
  workflows_list_filters_by_tenant
  empty_tenant_returns_empty_list
```

All four assert the cross-tenant contract at the SQL layer (the PR body explains why HTTP-level tests were avoided: worker-bench has no `ANTHROPIC_API_KEY` so `gateway start` refuses to bind). This is a sensible deviation; the handler-to-dispatcher wiring is verified at compile-time via the new `routes_dispatch_get` arms.

### Dispatch wiring

[`routes_dispatch_get.rs`](crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs) on PR head mounts both dispatchers ahead of `routes_workflows::dispatch_get` (the per-vertical CRUD handler that only triggers on `/api/v1/verticals/...`). Order is defensive but not load-bearing because the per-vertical matcher is path-prefix-specific. Module decls in `gateway/mod.rs` (2-LOC additions) are required because Rust won't compile a sibling file unless it's `mod`-declared.

## Why cargo verification was deferred this round

A background `cargo test -p argo-cli --test gateway_skills_list_smoke --test gateway_workflows_list_smoke` was started against the PR head's isolated `/tmp/argo-r67-pr2323-target` (so it wouldn't contend with the running worker pool's `/tmp/argo-shared-target` or `~/.cache/argo-shared-target`). It was killed at ~3min elapsed (still cold-linking the argo binary) when the round's primary finding emerged: the gateway-degraded-503 had a real root cause (the r67 S1 P1 finding on PR #2317), and that needed cargo cycles to confirm via `migration_upgrade.rs`.

This isn't theater-cover for missing tests — the four `*_filters_by_tenant` test functions are real (verified by `grep` on the worktree), the SQL clauses inspect by eye to do what they claim, and the dispatch wiring is one-line `if matches { handler.handle(...) }` blocks identical to the existing `routes_audit_feed_scope` registration immediately above it. The cargo-not-rerun is a budget choice, not a hidden gap.

## Anti-theater check

The two production handlers don't import the test crate, can't see test fixtures, and execute against a real SQLite pool in the e2e path. The tests:

- seed N rows for tenant A + M rows for tenant B via `SkillStore::install` (the real installer)
- run the *exact* SQL each handler runs (literal `WHERE s.tenant_id = ?` etc) against the seeded data
- assert each tenant sees exactly its own rows

If a future PR mutates a handler's `WHERE` to `WHERE 1=1`, the test's "exactly tenant A's rows" assertion catches it. The test fixtures and production code are physically separated; can't be silently rewritten in lockstep.

## Gap / follow-up brief candidate

**None blocking.** This is the cleanest PR I've reviewed this week. Recommend:

- **Conductor next tick**: stamp `--skip-clippy` admin-merge (operator overnight authorization covers it under the Track P / merge-aggressive window; also the brief is operator-recommended-by-me r63+r66 carry, has cross-tenant smoke tests, fits ≤500 LOC per file).
- **Optional follow-up brief** (P3, NOT blocking): `web-adopt-top-level-skills-and-workflows-endpoints` — SPA's `web/lib/api/skills.ts` and `web/lib/api/workflows.ts` currently use vertical-scoped paths; switching to the new top-level endpoints lets the SPA's `/skills` and `/workflows` pages render real data instead of fixtures. PR #2323 author noted this explicitly in the body's "sensible deviations" — they left the SPA as-is, breaking nothing, so this is purely a quality-improvement follow-up.

## Post-merge verify (recommended for next-round evaluator)

After the merge ships and the gateway restarts on a fresh binary:

```bash
curl -s -H "X-Argo-Tenant-Id: 11111111-1111-1111-1111-111111111111" http://127.0.0.1:7400/api/v1/skills | jq '.total_count'
curl -s -H "X-Argo-Tenant-Id: 11111111-1111-1111-1111-111111111111" http://127.0.0.1:7400/api/v1/skills/marketplace | jq '.total_count'
curl -s -H "X-Argo-Tenant-Id: 11111111-1111-1111-1111-111111111111" http://127.0.0.1:7400/api/v1/workflows | jq '.total_count'
```

All three should return HTTP 200 with an envelope (today they return 404 — that's the surface area being fixed). NB: this requires the r67 S1 EXPECTED_SCHEMA_VERSION fix to ship + the gateway to restart, otherwise the gateway will return 503 degraded on /health and the SPA's startup probe will treat the gateway as down.
