# Stream P2-11 — trust-ratchet UI

**Owner**: claude.
**Depends on**: mission-pm-approval-target-scope MERGED (so per-target streaks are meaningful).
**Authored against**: origin/main `80ffa124` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pm-trust-ratchet-ui`.
**Source**: skycot-pm-worker REPORT.md §6.2 + §11 P2 #11.

## Scope

When an operator approves the same `(worker, tool[, target_glob])` triple repeatedly with positive outcomes, the product should *offer* to promote it to auto-approve. Today the operator must manually edit `approval_policy` to grade up. This brief lands the offer.

Two surfaces:

1. **Backend rollup** — a `trust_streak` view (SQL view or materialised aggregator) that counts consecutive approve-with-positive-outcome decisions per `(worker_id, tool, target_glob)`. Exposed via `GET /api/v1/workers/{id}/trust-streaks`.
2. **Frontend ratchet card** — when a streak crosses a threshold (default 10), a card appears in the approvals inbox: "You've approved Riley's `update_roadmap` 10 times in a row for `Q3-*`. Auto-approve this pattern going forward?" One click promotes via the existing `approval_policy` write path.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/migrations/<next>__trust_streak_view.sql` (NEW) — `CREATE VIEW trust_streak AS ...` aggregating approval-granted events from the audit chain. If a view is too SQL-heavy for the existing pattern, prefer a materialised summary table with a small background updater.
- `crates/argo-store/src/trust_streak.rs` (NEW) — read-side accessor `pub async fn streaks_for_worker(worker_id) -> Vec<TrustStreak>`.
- `crates/argo-cli/src/handlers/gateway/routes_trust_streaks.rs` (NEW) — `GET /api/v1/workers/{id}/trust-streaks` route handler. Requires `approval.read`.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs` — narrowly scoped: add the dispatch arm. No other edits.
- `web/components/skycot/TrustRatchetCard.tsx` (NEW) — the offer card. ≤ 200 LOC.
- `web/app/(app)/page.tsx` — narrowly scoped: fetch trust streaks alongside the approval list; render `<TrustRatchetCard>` rows above the approvals when present.
- `web/components/skycot/TrustRatchetCard.test.tsx` (NEW).

## Files you must NOT touch

- `web/components/skycot/ApprovalCard.tsx` — ratchet is a separate card type, not a prop on approval card.
- `crates/argo-cli/src/handlers/approval_policy.rs` — promotion writes through the existing API; no edits to the policy types.
- Any audit kinds — this brief reads audit, doesn't write.

## Acceptance criteria

- [ ] `trust_streak` view (or aggregator) returns `(worker_id, tool, target_glob, streak_count, last_decided_at)` rows, computed from `approval.granted` audit kind with positive-outcome filter.
- [ ] Streak resets when (a) an `approval.denied` row appears for the same triple, OR (b) a downstream `worker_action.failed` audit row appears within 24h of the grant.
- [ ] `GET /api/v1/workers/{id}/trust-streaks?min=10` returns only streaks above the threshold.
- [ ] `TrustRatchetCard` renders one card per streak; "Auto-approve going forward" button POSTs to the existing `approval_policy` write path (or a thin wrapper) with a new auto-approve rule scoped to `(worker, tool, target_glob)`.
- [ ] Dismiss button hides the card for 7 days (client-localStorage; server-side persistence is out of scope).
- [ ] When the operator clicks "Auto-approve", an audit row `ApprovalPolicyPromoted` is emitted (NEW kind, coordinate with audit-events brief if it hasn't landed). Payload: `(worker, tool, target_glob, streak_count, operator)`.
- [ ] Tests: backend — streak count correct for 3 fixture scenarios (clean streak, broken-by-denial, broken-by-failure); frontend — card renders, click promotes, dismiss hides.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `pnpm lint && pnpm test && pnpm build` clean in `web/`.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-pm-trust-ratchet-ui`
- PR title: `trust: streak rollup + ratchet promotion card (mission-pm-trust-ratchet-ui)`

## Notes for the implementer

- Streak threshold is configurable via env `ARGO_TRUST_RATCHET_MIN` (default 10).
- The "positive outcome" check requires correlating the approval grant with a downstream worker-action success. Use the audit chain — if a `worker_action.completed` row appears with the matching action_id within 24h, count it as positive. If a `worker_action.failed` row appears, reset the streak.
- Do NOT auto-promote without operator click. The whole point is the offer is presented, the operator decides. Anything else is a trust violation.
- The card shows the last 3 approval timestamps as plain text so the operator remembers the context.

## Out of scope

- Per-tenant trust thresholds (one global env var is fine for v1).
- Auto-demote on subsequent denial (the policy edit handles this manually).
- Surfacing streaks for tools the operator has never seen approved (i.e. cold-start).
