# eval-skill-rejection-flow-audit

**Owner**: codex
**Priority**: P2 - claim 4 substrate audit
**Authored against**: origin/main `7354f039d` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-skill-rejection-flow-audit`.

## Source

Mission claim 4 says a human reviewer can reject a learned skill and the skill is
not offered afterwards. The forward promotion path has coverage, but the rejection
path was flagged as unverified in the mission-vs-code gap manifest.

## Goal

Audit the skill rejection path with an end-to-end contract. If a first-class
audit kind or runtime filter is absent, leave a FINDINGS-ONLY ignored test and
queue a narrow fix brief.

## May Touch

- `crates/argo-skill/tests/skill_rejection_flow_audit.rs` (NEW)
- Existing `crates/argo-skill/tests/**` only for nearby helper reuse
- `docs/skycot/evals/scheduled/2026-05-24/eval-skill-rejection-flow-audit.md` (NEW report)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not change production skill store or gateway behavior in this audit brief.
- Do not add migrations.
- Do not add network or LLM calls.

## Acceptance

- [ ] Attempt the contract: submit/propose learned skill, reject it, assert rejection audit evidence, and assert runtime lookup excludes it.
- [ ] If the audit kind or store seam is missing, add a named FINDINGS-ONLY ignored test rather than mutating production.
- [ ] Write a report with verdict and exact follow-up brief names for real gaps.
- [ ] `cargo test -p argo-skill --tests` passes for active tests, or the PR body records any unrelated clean-main failure with a focused green command.

## Notes

Audit-only. This row was previously blocked because the stream file was missing.
