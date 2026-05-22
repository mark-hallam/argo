# mission-onboarding-wizard-welcome-flake-fix

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `22af242b0` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-onboarding-wizard-welcome-flake-fix`.

🟢 **P3 SIGNAL HYGIENE.** Source: every recent web PR body (#2011 / #2012 / #2014 / #2016 / #2017 / #2019 / #2020 / #2028 / #2029) cites the same pre-existing flake: `onboarding/wizard > Welcome step renders the headline`. It's the dominant noise in PR verification claims. Fix it.

## Scope

Find the failing assertion in `web/__tests__/onboarding/wizard.test.tsx` (or the actual path — verify before claiming), figure out why it fails on clean `origin/main`, and fix it. Most likely causes (in order of probability):
1. The Welcome step's headline copy changed when D3-brand-consistency pass was applied (per memory `project_brand_consistency_pass.md`); the test expects the old copy.
2. A wrapper component changed and the headline is now inside a different DOM node the query doesn't reach.
3. The test relies on a hook or mock that was removed in a refactor.

Whichever it is, fix the test (NOT the production component) so it asserts against the current shipped behavior. If the production component IS broken (e.g. the headline really doesn't render), document that explicitly and split this brief into a sibling fix-brief.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `web/__tests__/onboarding/wizard.test.tsx` (verify path) — fix the failing test.
- `web/app/(setup)/wizard/welcome-step.tsx` or equivalent — narrowly scoped: ONLY if the test needs a `data-testid` or aria-label added to make it stable. No semantic component changes.
- `docs/build-plan/KNOWN_FLAKES.md` — narrowly scoped: remove the entry for this flake.

## Files you must NOT touch

- Any other test.
- Any other production code.
- BUILD_STATUS.md outside the standard row flip.

## Acceptance criteria

- [ ] `pnpm test --run web/__tests__/onboarding/wizard.test.tsx` exits 0 reliably (run 3× to confirm not just a single-pass).
- [ ] `pnpm test --run` total failure count on current main drops by exactly 1 (this test was the only one in the wizard suite that was failing — verify).
- [ ] If a `data-testid` was added: the prod component file diff is ≤3 lines, no semantic change.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm lint` clean.
- [ ] `KNOWN_FLAKES.md` entry for `onboarding/wizard > Welcome step renders the headline` removed (or marked RESOLVED with this PR's number).
- [ ] PR body explains the root cause in one short paragraph.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-onboarding-wizard-welcome-flake-fix`
- PR title: `web(onboarding): fix wizard Welcome-step test flake (mission-onboarding-wizard-welcome-flake-fix)`

## Notes for the implementer

- ~30-80 LOC expected. Small fix.
- DO NOT just `.skip()` the test. The point is honest signal — either fix the test or fix the component, but don't hide the failure.
- Don't refactor the test file beyond the failing assertion. One test, one fix. Keeps the diff reviewable.
- If during investigation you find a SECOND wizard flake hiding behind the first one, note it in the PR body as a follow-up — don't expand scope.
- Cross-check this brief against `claude-known-flakes-list-refresh` if it's in flight; if that brief lands first, this brief becomes a small follow-up against the refreshed list.
