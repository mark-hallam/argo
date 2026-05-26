# fix-audit-feed-action-kind-variety

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-feed-action-kind-variety`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/chrome-live-walk-and-playwright.md` F3 [P1] + scene-1-and-2-claim-1-and-2.md F3 [P0].

🟠 **P1 — Audit log is homogeneous demo-spam. 13 visible rows in the TODAY section are EVERY SINGLE ONE action type `http.request` from actor `UN` (unknown) with status `Succeeded`. No `skill.invoked`, no `approval.granted`, no `worker.hired`, no `delegation.assigned`, no `mcp.tool.call`, no `cu.action.taken`. Reads as log spam, not as "audit-first AI workforce" — directly contradicts the landing-page promise.**

## Root cause

The live gateway's only real audit traffic is the SPA's own browsing events (every `/audit?range=7d` GET produces an `audit.detail.viewed` row attributed to `worker = {id:"", name:"unknown"}`). There's no seed-time population of action-kind-varied audit rows representing a workforce doing real work.

Per audit Scene-1/2 F3: "There are still **no agent-attributed rows in the audit feed**. Every row is `action_type: audit.detail.viewed`, `worker: {id:"", name:"unknown"}`, `channel: web`, `verb: http.request`. So if Scene 2 lands on /audit and clicks any row, the camera reads 'Decision chain unavailable' — better than 8 rows of empty templates, but still doesn't prove the Claim 2 talk-track."

This is downstream of the missing workflow-seed (covered by `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`) AND the missing skill-seed (covered by `fix-skills-marketplace-perpetual-loading`) — without workflow runs and skill installs there's nothing to audit. But the demo-seed should ALSO emit a synthetic 7-day backstory of varied audit rows directly, so the page reads as a living workforce even when no fresh runs have fired today.

## Findings this brief closes

- chrome-live-walk-and-playwright F3 [P1] — Audit log is homogeneous demo-spam (all http.request from UN)
- scene-1-and-2-claim-1-and-2 F3 [P0] partial — audit feed has only HTTP meta-events with no agent-attributed rows
- (also relieves Scene 6 narrative where camera lands on /audit and reads a wall of identical rows)

## Fix recipe

1. **Add a `seed_audit_backstory` step to the demo-seed crate.** Emit ≥30 audit rows spread across the last 7 days, one row per minute on weekdays-9-5 with the following distribution:
   - 8× `skill.invoked` (from Mariah / Peta / Priya / Jamal / Nori across various skills)
   - 6× `approval.granted` (per the SAMPLE_APPROVALS list, with approver = the operator's name)
   - 2× `approval.escalated` (high-severity ones routed to manager)
   - 4× `delegation.assigned` (Mariah → Priya, Peta → Patrick, Jamal → Priya, etc.) with `delegated_by` populated to prove Claim 2
   - 3× `worker.hired` (the canonical 5 personas across the last 2 weeks)
   - 4× `cu.action.taken` (Notes.app, browser, Slack — proves Claim 5 substrate)
   - 3× `mcp.tool.call` (Zendesk fetch, Salesforce search — proves MCP substrate)
   - The rest spread across `workflow.completed` / `memory.write` / `skill.installed` / `policy.evaluated`

2. **Ensure each seeded row has populated decision_chain + provenance.** Each `skill.invoked` row should have a 3-5 stage decision chain (e.g. "Trigger received → Memory lookup → Skill match → Tool call → Response"), populated `provenance.model.id` (anthropic-claude-sonnet-4-5 or similar), `provenance.skill.id`, `provenance.hash_chain_anchor` (real sha256-shape hex), and `provenance.signature` (HMAC over a stable canonical form).

3. **Filter out `audit.detail.viewed` rows from the default `/audit` view.** They're meta-telemetry and pollute the workforce-doing-real-work narrative. Add a default filter at `web/lib/api/audit.ts` (or in the gateway query) that excludes `verb=audit.detail.viewed` unless an explicit `?include_meta=1` query param is set.

4. **Add a Playwright spec** at `web/playwright/audit/feed-has-variety.spec.ts`:
   - Load `/audit`
   - Assert ≥5 distinct action_type values are visible in the first 20 rows
   - Assert no more than 30% of visible rows are `http.request` or `audit.detail.viewed`
   - Assert at least one row each: `skill.invoked`, `approval.granted`, `delegation.assigned`

## Acceptance criteria

- [ ] `/audit` first 20 rows contain ≥5 distinct action_type values
- [ ] Both `skill.invoked` and `approval.granted` rows are visible without scrolling
- [ ] No `audit.detail.viewed` rows appear in default view (only with `?include_meta=1`)
- [ ] At least one `delegation.assigned` row visible to prove Claim 2 has on-camera evidence
- [ ] Drill-in on any seeded row returns a populated decision_chain with ≥3 stages
- [ ] New Playwright spec passes

## May-touch

- `crates/argo-demo-seed/src/audit_backstory.rs` (NEW)
- `crates/argo-demo-seed/src/lib.rs` (wire the new step)
- `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs` (default filter for meta rows)
- `web/lib/api/audit.ts` (client-side filter as fallback if gateway-side change is heavy)
- `web/playwright/audit/feed-has-variety.spec.ts` (NEW)

## Out of scope

- Hash-chain hover affordance (covered by `fix-audit-hash-chain-prev-this-hover-and-live-rows`)
- Audit detail envelope deserialization (covered by `fix-audit-detail-adapter-deserialize-envelope`)
- Audit drill-in 500 demo-mode fallback (covered by `fix-audit-action-detail-demo-mode-fallback`)
- Cross-tenant scope leak (covered by `fix-audit-list-row-in-tenant-scope-legacy-leak`)

## Effort

~400-600 LOC across seed + filter + spec. ~8-12 hours. P1 because Scene 6 narrative requires audit variety to land as proof.
