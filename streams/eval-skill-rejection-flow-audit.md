# eval-skill-rejection-flow-audit

**Authored against:** `origin/main` e03680c99 on 2026-05-24 (conductor tick-131).
**Validate before claim:** `bash scripts/check-brief-freshness.sh eval-skill-rejection-flow-audit`
**Owner:** codex (or claude)
**Priority:** 🟡 P2 — CLAIM-4 substrate (skills shared + learned, rejection path)
**Source:** `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` row `Skill rejection flow` (🔴 GAP).

Mission claim 4 says "human reviewer rejects → SkillRejected audit row + skill no longer offered". The forward path (publish → audit) is now covered by #2144 (row-6 audit-emit pin) but the **rejection** path is unverified. Need a regression contract.

## What to do

Verify the rejection flow end-to-end:

1. **AuditKind**: confirm `SkillRejected` (or equivalent — could be `SkillPublishDeclined`, `LearnedSkillRejected`, etc.) exists in `crates/argo-audit/src/event_kind.rs`. If a comparable variant exists but with a different name, pin against the real name. If no variant exists, this is a 🔴 FINDING and a fix-brief is queued.
2. **Gateway handler**: locate the rejection HTTP route. Likely under `crates/argo-cli/src/handlers/gateway/skill_*.rs` or `marketplace.rs`. Read it. Confirm: (a) emits the audit row, (b) marks the underlying skill row as `rejected`/`hidden` (whatever the production state is).
3. **LayeredLookup behavior**: confirm `LayeredLookup::list_for_action` (or the active-lifecycle filter shipped via #2143) excludes rejected skills. If `#2143`'s active-lifecycle filter already covers this, cite the filter.
4. **End-to-end contract**: write `crates/argo-skill/tests/skill_rejection_flow_test.rs`: (a) submit a learned skill, (b) call the rejection handler, (c) assert `SkillRejected` audit row exists with the right shape, (d) assert `LayeredLookup::list_for_action` no longer returns the skill.

Report: `docs/skycot/evals/scheduled/2026-05-24/eval-skill-rejection-flow-audit.md`. Sibling pattern: `claim4_learned_skill_publishes_and_reloads_without_cross_tenant_leak` in `argo-skill/tests/`.

## May-touch

- `crates/argo-skill/tests/skill_rejection_flow_test.rs` (new — most likely location; could move to `argo-orchestrator` if the rejection handler lives there)
- `crates/argo-audit/src/event_kind.rs` (only if `SkillRejected` is missing — and if missing, the fix is a FOLLOW-UP brief, not this one)
- `docs/skycot/evals/scheduled/2026-05-24/eval-skill-rejection-flow-audit.md` (new)
- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` (flip the `Skill rejection flow` row icon)

## Acceptance

- End-to-end contract test passes (or FINDINGS-ONLY `#[ignore]`'d if the AuditKind variant is missing — in which case queue a fix-brief).
- Per-crate clippy clean.
- Workspace clippy clean.
- Report committed.
- No production source touched (audit-only). If fixes are needed, queue separate fix-briefs.

## LOC budget

~200-400 LOC (test + report).
