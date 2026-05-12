# Mission load-bearing wire — Approval-policy UI

**Owner**: claude. **Depends on**: E4-f Settings tab MERGED (extends it).
**Authored against**: origin/main `1cca6a69` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 6 #129. Approval policies (which action types require approval, which auto-approve, which always-refuse) live in the worker's `auto_approval.toml` today — editable only by file. Operators want a UI to inspect + tune policies without filesystem access.

## Scope

NEW Approval-Policy section in the worker E4-f Settings tab:

1. **Frontend** new tab section: `ApprovalPolicySection` with a 3-column table per action type: action_name, current_policy (auto-approve / always-ask / always-refuse / risk-gated), inferred-trust-grade.
2. **Edit affordance** — click a row → modal to change policy + add an operator-note explaining why.
3. **Backend** new endpoint `GET /api/v1/workers/<id>/policies` + `PATCH /api/v1/workers/<id>/policies/<action_type>` — reads/writes the worker's auto_approval.toml.
4. **Audit row** per policy change (kind: `worker.policy_changed`, payload: `{ worker_id, action_type, old_policy, new_policy, operator_note }`).
5. **Read-only mode** when `worker.is_paused = true` (changes blocked while worker is paused).

This is NOT in scope: cross-worker policy templates (separate brief); policy versioning beyond audit-row history; risk-gate threshold tuning (advanced; behind a "show advanced" toggle).

## Files you may touch

- `web/components/skycot/worker/settings/ApprovalPolicySection.tsx` (NEW, ≤ 280 LOC)
- `web/components/skycot/worker/settings/ApprovalPolicyEditModal.tsx` (NEW, ≤ 220 LOC)
- `web/lib/api/worker-policies.ts` (NEW, ≤ 180 LOC)
- `web/lib/hooks/use-worker-policies.ts` (NEW, ≤ 180 LOC)
- `web/components/skycot/worker/Settings.test.tsx` (extend, ≤ 480 LOC after — verify ceiling) — 4 new tests
- `crates/argo-cli/src/handlers/gateway/routes_worker_policies.rs` (NEW, ≤ 380 LOC) + tests
- `docs/skycot/RUNBOOK.md` — note in Settings section

## Files you must NOT touch

- The actual auto_approval evaluation logic (`worker_actions_auto_approval.rs`) — this brief reads + writes the .toml, doesn't change semantics
- E4-f tab shell

## Acceptance criteria

- [ ] Policy table renders all action types from the worker's auto_approval.toml
- [ ] Edit modal supports the 4 policy values + operator-note required for change
- [ ] PATCH validates the new policy is a known value
- [ ] Audit row per change
- [ ] Read-only when worker paused
- [ ] 4 vitest + 5 backend unit tests; `pnpm test` + `cargo clippy` clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-approval-policy-ui`
- PR title: `web+gateway: approval-policy UI in worker settings (mission-approval-policy-ui)`

## Notes

- The 4-policy vocabulary (auto-approve / always-ask / always-refuse / risk-gated) is the canonical set. Don't add new policy types in this brief; the runtime can't handle others yet.
- Operator-note is required not optional — policy changes have audit-evidence value. The note shows up in audit drill-in.
