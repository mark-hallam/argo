# fix-delegation-chain-card-visual-surface

**Owner**: claude.
**Authored against**: origin/main `5c4abb23e1f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-delegation-chain-card-visual-surface`.
**Source**: e2e-ux-audit (scene-1-and-2-claim-1-and-2) finding F5 — `docs/skycot/ux-audits/2026-05-26-0820/scene-1-and-2-claim-1-and-2.md`.

🔴 **P0 DEMO-BLOCKER — Claim 2 "Delegation is the unit" has zero visible UI surface. Scene 2 talk-track is unprovable by drill-in. Investor asks "where do I see who delegated what to whom?" — answer: nowhere.**

## Root cause

The runtime threads `ToolContext.current_handoff_id` through every delegated call (per CLAUDE.md Claim 2 LIVE block, shipped via #2256). The data IS being captured at the runtime layer. But the SPA renders NONE of it:

- `web/components/skycot/audit/DecisionChainCard.tsx:38` renders a generic 8-step trace ("this action followed") — no parent-action link, no who-delegated-to-whom edge, no delegation timeline
- `web/components/skycot/ActivityRow.tsx` renders only the avatar of the acting worker — no parent / origin / delegated-by chip
- `rg -rnE 'delegat|handoff|delegated_by|delegated_to|parent.*action|by.*worker' web/components/skycot/audit/ web/components/skycot/ActivityRow.tsx` returns **zero** non-test hits

So Scene 2's narration "delegation is the unit — see the chain of who delegated what to whom" is not provable in the UI. The viewer sees a generic 8-step Decision Chain that looks identical regardless of whether the action originated from a human, a cron, or a delegating peer worker.

The fix requires three coordinated changes:
1. **Gateway**: extend `/api/v1/audit/<id>` to surface the delegation chain (an ordered list of `{worker_id, action_id, kind: "delegated"|"executed", timestamp}`)
2. **SPA component**: build (or extend) a `DelegationChainCard` that renders the chain as a vertical timeline with worker chips + arrow connectors
3. **Activity row**: add a "delegated by" chip on rows where `current_handoff_id` indicates a delegation

This is the **single biggest investor-narrative gap** for Claim 2.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F5 [P0] — No delegation-chain rendering anywhere in audit surfaces; Claim 2 has no visual proof

## Fix recipe

**1. Gateway surface** (`crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs` or wherever the action-detail handler lives):
- Add a `delegation_chain` field to the audit-detail response envelope:
  ```json
  {
    "id": "019e...",
    "delegation_chain": [
      { "worker_id": "019e5...", "worker_name": "Mariah", "action_id": "019e6...", "role": "originator", "ts": "..." },
      { "worker_id": "019e7...", "worker_name": "Peta", "action_id": "019e8...", "role": "delegated_to", "ts": "..." },
      { "worker_id": "019e9...", "worker_name": "Nori", "action_id": "019ea...", "role": "executed", "ts": "..." }
    ],
    ... existing fields ...
  }
  ```
- Resolve the chain by walking `current_handoff_id` back to its root action — likely a recursive store query (`audit_record WHERE handoff_id = ?` ordered by `ts`).
- Cap depth at 10 to prevent runaway queries.
- Return empty array `[]` when the action has no delegation history (most actions).

**2. New component** `web/components/skycot/audit/DelegationChainCard.tsx`:
- Props: `chain: DelegationStep[]` (from the new gateway field)
- Visual: vertical timeline; each step is a row with `<WorkerChip>` (avatar + name + L-level), an arrow connector, a small "delegated" or "executed" pill, and the timestamp
- Empty-state: hide the card entirely (don't render an empty timeline)
- Click on any worker chip → navigate to `/workers/<worker_id>/overview`
- Match the C2 personality-card visual register (gradient banner, warm ink color)

**3. Activity-row delegated-by chip** (`web/components/skycot/ActivityRow.tsx`):
- When the row's audit payload includes a non-null `handoff_id`, render a small "via Mariah" chip beside the worker avatar
- Clicking the chip navigates to the originating worker's overview page

**4. Wire into action-detail page** (`web/app/(app)/audit/action/[actionId]/page.tsx`):
- Render `<DelegationChainCard chain={detail.delegation_chain} />` above the existing `<DecisionChainCard>` when `chain.length > 1`
- DO NOT replace `DecisionChainCard` — the two cards answer different questions (delegation chain = "who handed this off to whom"; decision chain = "what were the per-step model/tool outputs").

**5. Tests**:
- Gateway: integration test at `crates/argo-cli/tests/audit_detail_delegation_chain.rs` — seed a 3-worker handoff, GET `/audit/<id>`, assert `delegation_chain` array length 3 with correct worker_ids
- SPA: component test at `web/components/skycot/audit/DelegationChainCard.test.tsx` — render with 3-step chain → assert 3 timeline rows + 2 arrows
- Visual: storybook entry at `web/stories/DelegationChainCard.stories.tsx` with empty / 2-step / 5-step variants

## Acceptance criteria

- [ ] `GET /api/v1/audit/<id>` response includes a `delegation_chain` field (array of `{worker_id, worker_name, action_id, role, ts}`).
- [ ] Actions with no delegation return `delegation_chain: []` (or omit the field — be consistent).
- [ ] Actions originating from a `delegate-task` tool call return the full chain (originator → ... → executor).
- [ ] New `DelegationChainCard` component renders a vertical timeline with worker chips + arrows + role pills.
- [ ] `ActivityRow` shows a "via {worker_name}" chip on delegated rows.
- [ ] Clicking any worker chip in either component navigates to that worker's overview page.
- [ ] Scene 2 drill-in: click into an audit row originating from `delegate-task` → see the chain rendered above the decision-chain card.
- [ ] Storybook entry visible with empty / short / long chain variants.
- [ ] Gateway integration test + SPA component test pass.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs` (or wherever action-detail handler lives)
- `crates/argo-cli/src/handlers/gateway/audit_detail_envelope.rs` (envelope type if separate)
- `crates/argo-audit/src/chain.rs` (delegation walker if not present)
- `crates/argo-cli/tests/audit_detail_delegation_chain.rs` (NEW)
- `web/components/skycot/audit/DelegationChainCard.tsx` (NEW)
- `web/components/skycot/audit/DelegationChainCard.test.tsx` (NEW)
- `web/components/skycot/ActivityRow.tsx` (delegated-by chip)
- `web/app/(app)/audit/action/[actionId]/page.tsx` (wire the new card in)
- `web/lib/api/audit-detail.ts` (type for the new field)
- `web/stories/DelegationChainCard.stories.tsx` (NEW)

## Out of scope

- Replacing or refactoring `DecisionChainCard` (kept as-is; complementary surface)
- Backfill of `handoff_id` on legacy audit rows (separate brief if needed)
- Delegation visualization in `/workflows/build` (out of scope — focus is audit drill-in for Scene 2)

## Effort

~250-400 LOC across gateway + SPA. ~6-8 hours. Single biggest UI unlock for Claim 2.
