# fix-audit-detail-render-empty-decision-chain-and-provenance-fallback

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-detail-render-empty-decision-chain-and-provenance-fallback`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/scene-6-claim-6-trust-and-approvals.md` F8 [P2] + `scene-1-and-2-claim-1-and-2.md` F4 [P0] partial overlap (the decision-chain UI half).

🟠 **P1 — DecisionChainCard renders 8 rows but every body is empty and every details object is null when the underlying audit row is an `audit.detail.viewed` meta-event (the SPA self-auditing). The locked B3 design promises "Memory loaded → snapshot abc · Skill chosen → review-skill v3 · …"; live gateway gives 8 hollow placeholders. Camera reads "trust theatre."**

## Root cause

Live gateway response on the detail endpoint for SPA-emitted self-audit rows:
```json
"decision_chain": [
  {"body":"","details":{"snapshot_id":null},"n":1,"title":"Memory loaded"},
  {"body":"","details":{"skill_id":null,"skill_trust":null,"skill_version":null},"n":2,"title":"Skill chosen"},
  ...
]
"provenance": {"model":{"id":"","version":""}, ...}
```

Two compounding issues:
1. The SPA renders the 8 stages unconditionally with empty body strings, producing a hollow skeleton.
2. The audit feed in dev-bypass contains only SPA-emitted `http.request` / `audit.detail.viewed` rows — there are no real agent-turn rows to decision-chain into. Even after seed-CLI lands, current rows are theatrical placeholders.

The DecisionChainCard should render a "Not applicable for this event kind" empty-state for meta-events instead of pretending an 8-stage chain exists. For real agent-turn rows where the chain IS populated, the existing rendering is fine.

## Code evidence

`web/components/skycot/audit/DecisionChainCard.tsx` — renders all chain entries unconditionally.
Live response shows 8 stages with `body:""` and `details:null` for the `audit.detail.viewed` event kind.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F8 [P2] (escalated to P1 because Scene 2 + Scene 6 both drill in here) — DecisionChainCard renders hollow skeleton for meta-events
- scene-1-and-2-claim-1-and-2 F4 [P0] partial — the decision-chain UI half (the delegation surface half goes to `fix-delegation-chain-card-visual-surface`)

## Fix recipe

1. **In `DecisionChainCard.tsx`**, detect "empty chain" by checking: all `body === ""` AND all `details` keys are null/absent. If so, render a single muted row instead of 8:
   ```tsx
   if (isEmptyChain(chain)) {
     return <ChainEmptyState eventKind={detail.action_type} />;
   }
   ```
2. **`ChainEmptyState`** renders a single card: "Decision chain unavailable — this row records a {event_kind} meta-event, not an agent turn. Drill into an agent-attributed row to see the full chain."
3. **For real agent-turn rows where 1-2 stages are populated and 6 are not**, render the populated stages fully and the rest as "Not reached" muted placeholders rather than empty body strings.
4. **Update `ProvenanceBlock.tsx`** similarly: detect all-empty fields and render "Provenance unavailable for this event kind" instead of 5 empty rows.
5. **Web test** with two fixtures: (a) `audit.detail.viewed` meta-event → empty-state renders, (b) real `agent.turn` event with 3 of 8 stages populated → 3 rendered + 5 "Not reached".

## Acceptance criteria

- [ ] DecisionChainCard does NOT render 8 empty rows for meta-events.
- [ ] DecisionChainCard renders a clear "meta-event — no chain" empty state in that case.
- [ ] For partially populated chains, unpopulated stages render as muted "Not reached" not empty body strings.
- [ ] ProvenanceBlock applies the same logic.
- [ ] Web test covers both fixtures.

## May-touch

- `web/components/skycot/audit/DecisionChainCard.tsx` (empty-detection + empty-state)
- `web/components/skycot/audit/ProvenanceBlock.tsx` (same)
- `web/components/skycot/audit/ChainEmptyState.tsx` (NEW)
- `web/tests/audit-detail-empty-chain.test.ts` (NEW)

## Out of scope

- Seeding real agent-turn audit rows (separate brief: `fix-demo-seed-cli-for-real-roster` + `fix-audit-feed-missing-skill-invoked-and-workflow-step-executed-rows`)
- Delegation-chain rendering (separate brief: `fix-delegation-chain-card-visual-surface`)
- Hash-chain anchor row (separate brief: `fix-audit-provenance-block-render-hash-chain-anchor`)

## Effort

~80-120 LOC + 1 test. ~2 hours.
