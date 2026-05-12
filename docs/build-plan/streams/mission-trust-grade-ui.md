# Mission load-bearing wire — Trust-grade UI surface

**Owner**: claude.
**Authored against**: origin/main `1cca6a69` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 6 #200. Trust grades drive what a worker is allowed to do without operator approval. Today the grade exists in `TrustGrade` enum but isn't surfaced in the SPA. Operators want to see the current grade + the criteria (signals + thresholds) + the path to upgrade.

## Scope

NEW Trust-Grade section on worker profile (E4 tab family):

1. **Frontend** new tab `TrustGrade` (E4 8/9 slot) — shows current grade (Bootstrap / Reviewed / Tenant / Verified / Published-Verified — verify enum at claim time), the signals contributing (action_count, success_rate, refused_count, time_since_first_action), and the gap to the next grade.
2. **Visual** — grade displayed as a tone-aware badge (Published-Verified=success, Tenant=warning, Bootstrap=destructive). Progress bar to next grade with criteria-met markers.
3. **Backend** `GET /api/v1/workers/<id>/trust-grade` returns `{ current_grade, signals: {...}, next_grade_criteria: [{ name, current_value, target_value, met: bool }] }`.
4. **No edit affordance** — trust grade is derived from observed behavior, not operator-tunable. The UI is purely informational + transparent about the model.

This is NOT in scope: trust-grade edit (intentionally read-only); cross-worker trust signals (sibling brief); historical-trend graph (could be added in follow-up).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_worker_trust_grade.rs` (NEW, ≤ 320 LOC) + tests
- `web/components/skycot/worker/TrustGradeCard.tsx` (NEW, ≤ 280 LOC)
- `web/components/skycot/worker/TrustGradeProgress.tsx` (NEW, ≤ 200 LOC)
- `web/app/(app)/workers/[workerId]/trust/page.tsx` (NEW, ≤ 280 LOC)
- `web/lib/api/worker-trust.ts` (NEW, ≤ 160 LOC) + hook
- `web/lib/sample-worker-trust.ts` (NEW, ≤ 180 LOC)
- Worker-profile tabs nav (existing) — add /trust tab entry
- `docs/skycot/RUNBOOK.md` — small note

## Files you must NOT touch

- The actual trust-grade evaluation logic in `argo-identity`; the underlying TrustGrade enum

## Acceptance criteria

- [ ] Trust-grade tab renders for every worker
- [ ] Current grade visible + tone-aware badge
- [ ] Progress bar to next grade with criteria-met indicators
- [ ] Read-only (no edit affordance)
- [ ] 5 backend tests + 6 frontend tests; clippy + pnpm test clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-trust-grade-ui`
- PR title: `web+gateway: worker trust-grade tab (mission-trust-grade-ui)`

## Notes

- Transparency matters here — operators trust the system more when they can see WHY a worker is at Bootstrap vs Tenant. The criteria + current-value display is the load-bearing UX detail.
- Tone-aware: Published-Verified = green badge, Tenant = yellow, Bootstrap = red. Operators understand the color language from approval-card severity.
- Verify TrustGrade variant names at claim time (`crates/argo-identity/src/store_trust_grade.rs` or similar). The brief lists Bootstrap/Reviewed/Tenant/Verified/PublishedVerified but verify against the actual enum.
