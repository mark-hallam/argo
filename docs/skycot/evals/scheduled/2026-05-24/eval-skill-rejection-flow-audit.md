# Eval: Skill Rejection Flow Audit

Date: 2026-05-24
Brief: `eval-skill-rejection-flow-audit`
Verdict: PASS - active substrate exists.

## Contract Checked

Mission claim 4 requires the human reviewer rejection path to leave an audit
trail and keep rejected learned skills out of runtime skill lookup. This audit
pins the path with an active argo-skill integration test:

- propose a learned skill via `SkillStore::propose_for_review`
- reject it via `SkillStore::transition_lifecycle(..., Rejected)`
- write and read first-class `AuditKind::SkillRejected` evidence
- assert `LayeredLookup::list_for_action("low")` excludes the rejected skill

The test is `crates/argo-skill/tests/skill_rejection_flow_audit.rs`.

## Existing Supporting Evidence

Current main already contains broader rejection coverage in
`crates/argo-skill/tests/skill_rejection_flow.rs`:

- judge rejection lands directly in `LifecycleState::Rejected`
- operator rejection emits a `skill.rejected` audit-shaped row
- pending-review skills do not propagate via vertical lookup
- rejected skills do not propagate via `LayeredLookup::list_for_action`
- rejected skills do not propagate via `LayeredLookup::find`
- rejection is isolated across independent workspace stores

The CLI review handler also maps rejected lifecycle transitions to
`AuditKind::SkillRejected` in `crates/argo-cli/src/handlers/skill_review.rs`.

## Gaps

No real substrate gap found. The first-class audit kind, lifecycle transition
seam, and runtime lookup filter all exist. No FINDINGS-ONLY ignored test was
needed.

## Follow-Up Briefs

None. No fix brief is queued from this audit.
