# eval-argo-skill-sdk-author-flow

**Owner**: codex
**Priority**: P2 - claim 4 SDK audit
**Authored against**: origin/main `7354f039d` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-skill-sdk-author-flow`.

## Source

The learning loop has multiple pins, but the operator-authored skill SDK surface
has not been audited. `argo-skill-sdk` should make it clear what an authored
skill declares, what capability metadata it carries, and what examples remain
honest.

## Goal

Audit the SDK author flow with small smoke skills and a report. Queue fix briefs
for any missing capability/audit/tenant contracts rather than broadening scope.

## May Touch

- `crates/argo-skill-sdk/tests/author_flow_smoke_test.rs` (NEW)
- Existing `crates/argo-skill-sdk/tests/**` only for helper reuse
- `docs/skycot/audit/2026-05-24-argo-skill-sdk-author-flow/audit.md` (NEW report)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not change production SDK builders unless a compile-only drift fix is tiny.
- Do not edit `argo-skill` runtime loading.
- Do not add network or LLM calls.

## Acceptance

- [ ] Inventory public authoring surface and current examples.
- [ ] Build 2-3 minimal skills through the SDK: echo, capability-declared, and audit-hook or metadata-bearing.
- [ ] Report whether capability declarations, version metadata, and tenant assumptions are explicit.
- [ ] Name follow-up brief(s) for any real SDK contract gap.
- [ ] `cargo test -p argo-skill-sdk --tests` passes, or the PR body records any unrelated clean-main failure with a focused green command.

## Notes

Audit-only by default.
