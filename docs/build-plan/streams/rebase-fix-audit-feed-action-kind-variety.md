# rebase-fix-audit-feed-action-kind-variety

**Owner**: claude.
**Authored against**: origin/main `a5df90c41` on `2026-05-29`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh rebase-fix-audit-feed-action-kind-variety`.
**Supersedes**: PR #2603 (closed for re-author 2026-05-29) + original brief `streams/fix-audit-feed-action-kind-variety.md`.

🔴 **P1/P0 — `/audit` page renders 13 visible rows that are all `action_type: http.request` from actor `UN` (unknown). Reads as log spam, not "audit-first AI workforce". Directly contradicts the landing-page promise the v1.0 demo MP4 is meant to prove.**

**Re-implement against current main; PR #2603's 448-LOC three-layer fix conflicts on `argo-demo-seed/lib.rs/seed.rs/plan.rs` (reshaped by #2584) and `routes_audit_feed.rs` (touched by #2635, #2589, #2527, #2440, #2577).**

## What changed on main since PR #2603 was authored

**Demo-seed crate** — same drift as the retro/orchestration brief:
- PR #2584 (`766f6b0d0`) added `dept_filler.rs` + `personas.rs` and reshaped lib.rs/seed.rs/plan.rs
- Current layout: `dept_filler.rs`, `lib.rs`, `marketplace.rs`, `personas.rs`, `plan.rs`, `seed.rs`, `skills.rs`, `workflow.rs`
- PR #2603's `audit_backstory.rs` does NOT exist on main

**Gateway audit-feed route** — multiple touches:
- `bf840b42f` (#2635 — "align audit list/detail un-headered tenant fallback")
- `e9fc8d287` (#2589 — "align gateway audit list/detail tenant resolver via shared extract_tenant")
- `cec1331c9` (#2527 — "audit feed accepts ?kind= alias + 400 on unknown query params")
- `e44e23eed` (#2440 — "surface seeded workflow + verticals + audit rows on /api/v1/* endpoints")
- `b311de551` (#2577 — "resolve audit feed worker.name via worker/persona join")

The `include_meta` filter PR #2603 proposes does NOT exist on main — grep confirms no `include_meta`, no `audit.detail.viewed` filter, no `audit.feed.queried` filter.

The feature is genuinely unshipped. SPA Playwright spec `audit-feed-has-variety.spec.ts` doesn't exist either.

## What this brief MUST ship

Re-implement PR #2603's full three-layer posture against current `origin/main` HEAD:

**Layer 1 — Seed** (`crates/argo-demo-seed/src/audit_backstory.rs` NEW):
Emit 34 varied action-type rows for the canonical demo workspace per the brief recipe:
- 8 × `skill.invoked` (Claim 4 evidence)
- 6 × `approval.granted` + 2 × `approval.escalated`
- 4 × `delegation.assigned` (cross-persona; `delegated_by` populated — Claim 2 evidence)
- 3 × `worker.hired`
- 4 × `cu.action.taken` (Claim 5 substrate)
- 3 × `mcp.tool.call` (Claim 5 substrate)
- Filler: `workflow.completed` / `memory.write` / `skill.installed` / `policy.evaluated`

Each row carries 4 populated decision-chain stages (`memory_loaded_summary` / `skill_summary` / `plan_selected_summary` / `outcome_summary`) plus provenance fields (`model_id` / `prompt_template_hash` / `policy_hash` / `signature`), so `/audit` drill-in renders ≥3 non-empty timeline stages. `provenance.hash_chain_anchor` is filled by the real chain hash the sink computes — not synthetic.

`created_at` is now-stamped (same compromise the existing Pass B dressing makes); `payload.synthetic_ts` carries the intended 7-day spread for `/audit` detail display.

**Layer 2 — Gateway meta-filter** in `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs`:
Exclude `action_type ∈ {audit.detail.viewed, audit.feed.queried}` from the default `/audit` view. Caller opts SPA self-traffic back in with `?include_meta=1`. Surgical addition; do not refactor the existing query-param parsing introduced by #2527.

**Layer 3 — SPA regression spec** at `web/tests/e2e/audit-feed-has-variety.spec.ts` (NEW):
Pin the SPA's rendering: ≥5 distinct action_type values, no meta rows, no `http.request`, ≥3 distinct workers.

**Wiring path**: in current `crates/argo-demo-seed/src/lib.rs`, follow the same pattern PR #2584 used for `dept_filler` + `personas` — declare the new `audit_backstory` module, plumb the seed call from `seed.rs::seed_demo_workspace()` Pass B.

## Acceptance criteria

- [ ] `bash scripts/check-brief-freshness.sh rebase-fix-audit-feed-action-kind-variety` passes.
- [ ] `/audit` first 20 rows contain ≥5 distinct action_type values.
- [ ] `skill.invoked` + `approval.granted` rows seeded.
- [ ] No `audit.detail.viewed` rows in default view (gateway filter verifies; SPA regression spec pins).
- [ ] At least one `delegation.assigned` row with `delegated_by` ≠ actor.
- [ ] Drill-in decision_chain ≥3 stages.
- [ ] `?include_meta=1` re-surfaces the meta rows.
- [ ] Playwright spec at `web/tests/e2e/audit-feed-has-variety.spec.ts`.
- [ ] `cargo clippy -p argo-demo-seed --tests -- -D warnings` clean.
- [ ] `cargo clippy -p argo-cli --tests -- -D warnings` clean.
- [ ] `cargo test -p argo-demo-seed` + `cargo test -p argo-cli --bins handlers::gateway::routes_audit_feed` green (modulo any pre-existing flakes on origin/main — confirm via stash-test-restore-test).
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.

## May-touch

- `crates/argo-demo-seed/src/audit_backstory.rs` (NEW — 34-row recipe)
- `crates/argo-demo-seed/src/lib.rs` (module declaration)
- `crates/argo-demo-seed/src/seed.rs` (Pass B wire-up)
- `crates/argo-demo-seed/src/plan.rs` (audit_backstory plan row)
- `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs` (include_meta filter)
- `web/tests/e2e/audit-feed-has-variety.spec.ts` (NEW)

## Out of scope

- SPA-side filter as fallback (`web/lib/api/audit.ts`) — the gateway-side change is tractable; fallback would be dead code.
- Real 7-day timestamp spread on `created_at` (matches existing Pass B dressing compromise).
- Touching `personas.rs` / `dept_filler.rs`.
- Re-anchoring the hash chain (the seed lets the sink compute the real anchor).

## Effort

~448 LOC across one new seed module + gateway filter + Playwright spec. ~2-3 hours.

## Source

- Original brief: `streams/fix-audit-feed-action-kind-variety.md`
- Closed PR: #2603 (closed 2026-05-29 as REAL-CONFLICT-NEEDS-REAUTHOR; 448 LOC three-layer fix straddling demo-seed reshape + gateway audit-feed touches)
- UX audit findings: chrome-live-walk-and-playwright F3 [P1] + scene-1-and-2-claim-1-and-2 F3 [P0] (partial)
