# Round 43 — PR #2167 anti-theater verify (🔴 P0 `skill_share::set_shared` cross-tenant marketplace-install leak)

- **Date**: 2026-05-23 UTC (Melbourne local Sun 2026-05-24 06:30 AEST)
- **Round**: 43
- **Pillar / Claim**: Claim 6 (Multi-tenant by construction) — Pillar: store SQL tenant binding
- **Verdict**: 🟢🟢 **PASS — headline finding is real, brief is well-grounded** (severity tier of underlying defect: P0 latent-but-live; tier of this verification: anti-theater PASS)

---

## Setup

PR #2167 (`eval-argo-store-tenant-bind-sweep-remaining-tables`, MERGED at `d80fa2998`) is a docs-only "deepening pass" that shipped 11 new mission fix-brief stubs plus a 1391-line audit report. The PR body highlights one **🔴 P0 LATENT-BUT-LIVE** finding: `skill_share::set_shared` runs an UPDATE keyed on `(name, version)` only — no tenant filter — and the caller at `crates/argo-cli/src/handlers/gateway/marketplace.rs:252` is the wired `POST /marketplace/<id>/install` remote route.

Anti-theater goal this round: confirm the finding is grounded in actual code (not hallucinated SQL or invented exploit shape) AND confirm the sibling fix brief cites the same line numbers correctly.

Read:

- `crates/argo-store/src/skill_share.rs:71-90` — `set_shared` source.
- `crates/argo-cli/src/handlers/gateway/marketplace.rs:209-276` — `install_action` handler.
- `crates/argo-store/src/migrations/v80.rs:1-22` + `migrations/131__skill_tenant_id.sql` reference — `skill.tenant_id` schema state.
- `docs/build-plan/streams/mission-skill-share-tenant-pinned.md` — sibling fix brief.

## Observed

`skill_share.rs:78-86` source verbatim:

```rust
sqlx::query(
    "UPDATE skill SET shared = ?, shared_with_cluster = ? \
     WHERE name = ? AND version = ?",
)
.bind(i64::from(shared))
.bind(cluster)
.bind(skill_name)
.bind(version)
.execute(store.pool())
```

WHERE clause: `name = ? AND version = ?`. **No `tenant_id` bind.** Finding is grounded.

`marketplace.rs:252-258` source verbatim:

```rust
if let Err(e) = argo_store::skill_share::set_shared(
    store,
    &skill.name,
    &skill.version,
    true,
    Some(&auth.tenant_id),
)
```

The caller passes the **caller's own** tenant cluster name as the `shared_with_cluster` value, but the SQL UPDATE matches **any** row with the same `(name, version)` — including a different tenant's row. So a cross-tenant `(name, version)` collision lets tenant B's install call overwrite tenant A's row's `shared` flag + `shared_with_cluster` to point at tenant B's cluster.

`migrations/v80.rs:1-22` confirms `tenant_id` was added to the `skill` table (PR #2092). Before v80, the table was effectively single-tenant; post-v80, `(name, version)` is no longer a unique key (it's `(tenant_id, name, version)` in spirit) — making the leak exploitable in production.

The handler at `marketplace.rs:217-240` does authenticate the caller and refuse explicit cross-tenant install targets, but the leak path bypasses that gate entirely: it doesn't try to install into tenant A; it triggers the unfiltered UPDATE which collaterally touches tenant A's row.

Sibling fix brief `mission-skill-share-tenant-pinned.md` cites the exact same line ranges (`skill_share.rs:71-89`, `marketplace.rs:252`), gives a 4-step exploit shape that matches the source-level analysis above, and proposes the canonical fix (additive-constructor: thread `tenant_id` through `set_shared`, WHERE `... AND tenant_id = ?`, write a cross-tenant negative pin). Brief scope is ~200-350 LOC across `argo-store`, `argo-cli/handlers/gateway/marketplace.rs`, and a new test file. Acceptance criteria correctly require the negative pin.

## Anti-theater check

How I proved this isn't theater (a brief that fabricates SQL or invents an exploit that doesn't actually compile to a leak):

1. **Read the actual SQL** — verified the WHERE clause character-by-character in `skill_share.rs:79-80`. Brief's quoted SQL matches the source exactly.
2. **Read the actual caller** — verified `marketplace.rs:252` calls `set_shared` with the caller's tenant cluster as `shared_with_cluster` but no tenant filter on the UPDATE. The brief's claim "the caller is LIVE remote route POST /marketplace/<id>/install" matches the handler dispatch.
3. **Read the auth gate at the handler** — confirmed it refuses *explicit* cross-tenant targets (`marketplace.rs:232-238`) but does NOT prevent the unfiltered UPDATE from collaterally mutating another tenant's row.
4. **Verified the schema state** — `migrations/v80.rs` adds `tenant_id` to `skill`, confirming post-#2092 `(name, version)` collisions across tenants are now possible (otherwise the leak would be vacuous).
5. **Checked the brief's scope is right-sized** — 4 files, ~200-350 LOC, single fn signature change + WHERE update + negative pin. No "land scaffold now, wire later" failure mode.

The brief is **not** describing an unwiring or an abstraction problem; it's describing a concrete one-statement SQL bug with a concrete one-route LIVE caller, with a concrete bounded fix. No false-positive amplification.

## Gap / follow-up

None new. Brief `mission-skill-share-tenant-pinned` already authored + ledger-rowed via PR #2167. Recommendation to conductor: keep this row at **P0 priority** rather than the default P1; the brief itself tags 🔴 P0 LATENT-BUT-LIVE, and the marketplace install route is reachable today through normal authenticated user flow (no admin-only gate). Pre-v1 demo, this should ship before any tenant-onboarding marketing surface goes live with marketplace install enabled.

The 10 sibling briefs in PR #2167 (approvals/elevated-view/scim/support-ticket/worker-budget/session-meta/persona-revert/worker-archive-cascade/skill-query/skill-share) all follow the same pattern but vary in liveness — `skill-share` is the only one labeled LATENT-BUT-LIVE. The conductor should prioritize accordingly: skill-share + the two not-shipped-by-#2146 approvals briefs (mark_escalation + bundle_decide) ahead of the latent-only ones.

**22nd anti-theater-honest verification on main this week.** Brief authoring quality on PR #2167 is high: citations match code; exploit shapes match SQL; scope is bounded.
