# Round 44 — `mission-skill-share-tenant-pinned` P0 status check (60-min ageing watch)

- **Date**: 2026-05-23
- **Round**: 44 (scheduled-task hourly fire)
- **Pillar / Claim**: Claim 6 (Multi-tenant-by-construction) — live cross-tenant write-leak via marketplace install
- **Brief**: `docs/build-plan/streams/mission-skill-share-tenant-pinned.md`
- **Ledger row**: BUILD_STATUS.md line 1203
- **Headline**: 🔴 **P0 AGEING — 60 min since elevation, no claim, no PR. Production leak still live.**

## Why this scenario

Round 43 (2026-05-23T20:27Z) elevated this from POSSIBLE-LATENT to LIVE-EXPLOIT. The conductor (tick-127, ledger sha `deaf469ba`) inserted the ledger row at TODO status. One hour later (round 44), I check: has a worker claimed? Has a PR opened?

## Setup

```
grep -B 1 -A 2 "mission-skill-share-tenant-pinned" docs/build-plan/BUILD_STATUS.md
gh pr list --search "skill-share" --state all
sed -n '60,90p' crates/argo-store/src/skill_share.rs                     # current SQL
sed -n '230,270p' crates/argo-cli/src/handlers/gateway/marketplace.rs   # current handler
```

## Observed

### Ledger row — still TODO

```
| mission-skill-share-tenant-pinned | claude | TODO | — | 🔴 P0 LATENT-BUT-LIVE CROSS-TENANT MUTATION (post-#2092 sweep gap). NEW 2026-05-24 by conductor (tick-127, … evaluator round 43 explicitly elevated this to P0 …) …
```

No worker name in the assignee column (just `claude` family); no PR number; no `CLAIMED` state.

### gh pr list — no in-flight PR matching "skill-share"

`gh pr list --search "skill-share" --state all` returns 20 historical PRs all MERGED ≥ 24h ago, NONE matching this brief name. `gh pr list --state open --limit 30` returns 1 open PR (#2131 `mission-orchestrator-cross-tenant-handoff-audit-kind` — unrelated).

### Production code — leak still live

`crates/argo-store/src/skill_share.rs:78-86`:

```rust
sqlx::query(
    "UPDATE skill SET shared = ?, shared_with_cluster = ? \
     WHERE name = ? AND version = ?",
)
```

WHERE clause STILL has no tenant_id bind. Pre-shipping #2092 (which added `tenant_id` column to `skill` table on 2026-05-23T02:00Z), this was masked by sparse population — `(name, version)` collisions across tenants were rare. Post-#2092, the column exists and the population gap closes over time, so the exploit window widens with each marketplace install.

### Caller — handler has a partial guard, not a full one

`crates/argo-cli/src/handlers/gateway/marketplace.rs:230-260`:

```rust
let target = …query.get("tenant_id")…or_else(|| body_tenant_id(body));
if let Some(target) = target.as_deref() {
    if !target.is_empty() && target != auth.tenant_id {
        return ("403 Forbidden", "tenant mismatch — cannot install into another tenant");
    }
}
…
argo_store::skill_share::set_shared(
    store,
    &skill.name,
    &skill.version,
    true,
    Some(&auth.tenant_id),  // ← cluster value, NOT a SQL filter
)
```

The 403 guard at line 232-238 ONLY fires when the caller passes an explicit `tenant_id` query param or body field. **If the caller omits `tenant_id`** — the simpler exploit path — `target == None`, the guard is skipped, and the unfiltered UPDATE runs against any `(name, version)` it can find. Whoever owns that `(name, version)` gets their `shared_with_cluster` flipped to the attacker's cluster name.

## Anti-theater check

Three independent sources agree:

1. **Ledger row read at 21:27Z**: `TODO`, no PR.
2. **`gh pr list`**: no matching open PR.
3. **Production SQL still missing `AND tenant_id = ?`**: read direct from disk on `origin/main` HEAD `6aba0455f`.

No theater — the P0 is real and untouched. The conductor inserted the row 60 min ago and no worker has claimed it yet.

## Gap / follow-up brief candidates

🔴 **P0 (re-surface for next conductor tick)**: claim + ship `mission-skill-share-tenant-pinned`. Recipe per ledger:
- Additive `set_shared(store, tenant_id, name, version, shared, cluster)` — same canonical pattern PR #2168 just shipped for the session-routes cluster
- WHERE `name = ? AND version = ? AND tenant_id = ?` — match the bind shape from `session_query.rs:184`
- Thread `auth.tenant_id` from `marketplace.rs:252` (caller already has it)
- Cross-tenant pin test + same-tenant baseline (mirror `routes_session_tenant_bind_tests.rs:91-117`)
- ~200-350 LOC per the original ledger estimate

🟡 **Recommended P1 sibling brief**: `mission-skill-share-tenant-pinned-belt-and-braces-401-on-missing-tenant-id` — even with the additive constructor, the marketplace handler should reject installs that omit `tenant_id` entirely (or default it to `auth.tenant_id`). The current handler's optional-target guard is too lenient.

## Operational note for conductor

Round 43 explicitly flagged this as "pre-v1.0-demo blocker if marketplace install is in the recorded surface." The 2026-05-30 demo gate is 6 days away. The brief is authored, the recipe is canonical (PR #2168 ships exactly this pattern), and the LOC budget is small. The blocker is queue replenishment — pick a `claude` worker, claim this row.

**Re-surface to operator + conductor**: this is the second-highest-aged P0 row in the ledger. Round 43 said "TENTH P0 elevated this week"; round 44 confirms ageing already.
