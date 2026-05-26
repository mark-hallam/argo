# fix-audit-feed-missing-skill-invoked-and-workflow-step-executed-rows

**Owner**: codex.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-feed-missing-skill-invoked-and-workflow-step-executed-rows`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/scene-4-and-5-claim-4-and-5.md` F15 [P1].

🟠 **P1 — Scene 4 pass-cue is "a `SkillInvoked` audit row is written carrying the same provenance." Scene 5 pass-cue is "two `workflow.step.executed` rows for the run, one per backend." Current audit feed contains zero rows of either kind. After fixes F2 / F4 / F8 / F9 land, if the runtime doesn't actually emit these audit kinds the camera-drill-in has nothing to show.**

## Root cause

The audit feed in dev-bypass returns only `http.request` / `audit.detail.viewed` / `audit.replay` rows — every entry is SPA self-traffic. Real agent turns reportedly fire through the runtime per CLAUDE.md Claim 4 + 5 LIVE blocks, but the audit emissions either don't reach the gateway audit feed envelope or don't carry the kind tags the scene pass-cues require.

This is the gateway-side counterpart to `fix-demo-seed-cli-for-real-roster`. The seed CLI populates fixtures; this brief ensures the runtime emits the right kinds when real workflows fire.

## Code evidence

`curl /api/v1/audit?limit=5` against live gateway returns only meta-event rows — no `skill.invoked`, no `workflow.step.executed`.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F15 [P1] — Audit feed has no skill.invoked or workflow.step.executed rows; Scene 4 + 5 pass-cues won't fire

## Fix recipe

1. **Verify the runtime emits the right kinds**. Search:
   - `rg 'skill\.invoked|skill_invoked|SkillInvoked' crates/argo-runtime/ crates/argo-acp/`
   - `rg 'workflow\.step\.executed|workflow_step_executed' crates/argo-runtime/ crates/argo-acp/`
2. **If the runtime emits them but the gateway audit feed strips/filters them out**, fix the envelope passthrough. Likely path: `crates/argo-cli/src/handlers/gateway/routes_audit_list.rs` — confirm no kind-allowlist that blocks these emissions.
3. **If the runtime does NOT emit them**, add the emissions:
   - In the skill-invocation path (`crates/argo-agent-runner/src/skill_invoke.rs` or similar), emit `AuditKind::SkillInvoked` after the skill returns.
   - In the workflow-runtime step-execution path, emit `AuditKind::WorkflowStepExecuted` after each step completes, with `details.executor` distinguishing MCP / cu-macos / skill.
4. **Schema bump** for the audit-kind enum if these are new variants. Use the additive-migration pattern per `docs/skycot/operator/migrations.md`.
5. **Integration test**: a cassette-replay that fires a skill invocation + a 2-step workflow, then asserts the audit feed contains 1 `skill.invoked` row + 2 `workflow.step.executed` rows with correct provenance.
6. **Update the SPA's audit-row renderer** to colour-code these new kinds distinctly (small badge / icon) so the camera-drill-in can land on a row that visibly says "Skill invoked" vs "Step executed".

## Acceptance criteria

- [ ] Runtime emits `skill.invoked` audit rows for every skill turn, carrying `skill_id`, `version`, `tenant_id`, `worker_id`, `provenance` block.
- [ ] Runtime emits `workflow.step.executed` rows for every workflow step, carrying `workflow_id`, `step_index`, `step_kind`, `executor`, `provenance`.
- [ ] Gateway audit feed passes these through (no filtering).
- [ ] Integration test asserts both kinds land after a real cassette-replay.
- [ ] SPA audit-row colour-codes the new kinds.

## May-touch

- `crates/argo-runtime/src/` (skill + workflow runtime paths)
- `crates/argo-acp/src/audit_emit.rs` or equivalent (audit emission helpers)
- `crates/argo-types/src/audit_kind.rs` (enum extension if needed)
- `crates/argo-store/src/migrations/` (additive migration if enum is persisted)
- `crates/argo-cli/src/handlers/gateway/routes_audit_list.rs` (no filtering)
- `crates/argo-cli/tests/audit_emits_skill_invoked_and_workflow_step_executed.rs` (NEW)
- `web/components/skycot/ActivityRow.tsx` (badge for new kinds)

## Out of scope

- Decision-chain population in the audit-detail payload (separate; runtime should already thread chain state)
- Delegation chain rendering (separate brief: `fix-delegation-chain-card-visual-surface`)
- Skill marketplace install flow (separate brief: `fix-skills-detail-install-button-and-flow`)

## Effort

~250-400 LOC across runtime + audit emission + tests + web badge. ~6-8 hours. Mostly Rust; web side is ~30 LOC.
