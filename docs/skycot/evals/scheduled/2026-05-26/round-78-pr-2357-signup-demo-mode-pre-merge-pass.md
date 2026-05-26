# Round 78 — PR #2357 signup demo-mode PRE-MERGE PASS

- **Date**: 2026-05-26
- **Round**: 78
- **Pillar / Claim**: Commercial usability (priority 5) — Scene 3 of v1.0 demo MP4 recording
- **Verdict**: 🟢 **PASS** — eligible for admin-merge under overnight `--skip-clippy` ops-* authorization (window expires 22:00Z AEST 2026-05-26)
- **Brief**: `docs/build-plan/streams/fix-signup-demo-mode-env-flag-and-runbook-prerequisites.md` (T00-28 UX-audit P0)

## Setup

```
gh pr view 2357 --json title,mergeable,mergeStateStatus,headRefOid,additions,deletions,changedFiles
git fetch origin pull/2357/head:pr-2357-eval
git worktree add /tmp/pr-2357-wt 47d1bf78aa1455d9530c195e55a809b4bb9b4976
ln -sf ~/code/argo/web/node_modules /tmp/pr-2357-wt/web/node_modules
cd /tmp/pr-2357-wt/web && npx vitest run __tests__/auth/signup-page.test.tsx
```

## Observed

### PR metadata

| field | value |
|---|---|
| title | `fix: signup demo-mode env-flag + runbook prereqs` |
| head SHA | `47d1bf78aa1455d9530c195e55a809b4bb9b4976` |
| base | `main` |
| mergeable | `MERGEABLE` |
| mergeStateStatus | `CLEAN` |
| diff | +134 / -8 across 4 files (web-only, no Rust crates) |

### Vitest on PR head

```
 RUN  v3.2.4 /private/tmp/pr-2357-wt/web
 ✓ __tests__/auth/signup-page.test.tsx (8 tests) 291ms

 Test Files  1 passed (1)
      Tests  8 passed (8)
   Duration  1.15s
```

The 3 net-new tests live under the describe block `SignupPage — demo-mint secondary button (runbook fallback)` and pin the brief's three acceptance bullets:

1. `renders the secondary demo-mint button by default in non-production builds` — asserts `getByTestId("signup-demo-mint")` is in the document with no env stubbing.
2. `hides the secondary button on production builds unless NEXT_PUBLIC_ALLOW_DEMO_MINT=1` — stubs `NEXT_PUBLIC_ENV=production`, asserts button hidden; then stubs `NEXT_PUBLIC_ALLOW_DEMO_MINT=1`, asserts button visible again.
3. `clicking the secondary button calls /api/auth/demo-mint and never /api/auth/signup (SMTP-bypass)` — mocks `fetchMock`, throws on any URL other than `/api/auth/demo-mint`, clicks the button, asserts no SMTP signup call is made.

### Vitest on `origin/main` (anti-theater)

```
 RUN  v3.2.4 ~/code/argo/web
 ✓ __tests__/auth/signup-page.test.tsx (5 tests) 235ms

 Test Files  1 passed (1)
      Tests  5 passed (5)
```

Only 5 tests on main vs. 8 tests on PR head — confirms the PR **adds** 3 net-new tests (not test-rewrite). The diff at `web/__tests__/auth/signup-page.test.tsx` shows an append-only `+ describe("SignupPage — demo-mint secondary button (runbook fallback)", () => { ... })` block.

### Implementation review

- `web/app/auth/signup/page.tsx` — adds the secondary button rendered via the existing `submitDemoMint` handler (already wired to `/api/auth/demo-mint` → `signIn("gateway-bearer")` → `/onboarding/welcome`). No new client-side state, no new HTTP routes. Reuses production-grade plumbing.
- `web/.env.example` — documents both env vars.
- `docs/skycot/demos/v1-mission-mvp-runbook.md` — adds pre-record checklist note about the runbook-fallback button.

### Sensible deviations (called out in PR body)

- Gating rule defaults to enabled when `NEXT_PUBLIC_ENV !== "production"` (instead of strictly `NEXT_PUBLIC_ALLOW_DEMO_MINT === "1"`). Rationale: the brief literally writes the latter, but the brief's stated motivation ("makes the demo path discoverable without needing the operator to remember env-var dance") is only satisfied by the default-on behavior. The production-build hide-by-default invariant is preserved.
- Test file path: brief lists `web/tests/auth-signup.test.ts`; the actual file is `web/__tests__/auth/signup-page.test.tsx`. PR extends the real file.

Both deviations are explained in the PR description with rationale.

## Anti-theater check

- Vitest run against the **PR head SHA** in a fresh worktree, not against `origin/main` (which would skip the new tests entirely).
- Re-ran vitest against `origin/main` to baseline the test count (5) — confirms the PR's 3 new tests don't already exist there.
- The third test explicitly asserts the **network layer** — it throws on any URL other than `/api/auth/demo-mint`. A regression that re-introduced the SMTP call to `/api/auth/signup` would fail this assertion, not be silently allowed.
- The brief's acceptance criterion ("Production builds do NOT show the demo bypass button") is pinned by an actual env-stub test, not just a code comment.

## Merge recommendation

- **Action**: admin-squash-merge via `bash scripts/conductor-pre-merge-gate.sh --skip-clippy <PR-or-head>` per the overnight ops-* authorization window (T00-28 UX-audit synthesis batch P0).
- **Cold-cargo cost**: zero — web-only PR; pre-merge gate Rust steps no-op or skip.
- **Carry-burndown context**: this is the second T00-28 P0 PR to clear PRE-MERGE PASS (after `fix-spa-no-cu-macos-executor-status-surface` at #2358 which is currently DIRTY per T03 tick). Closes Scene 3 demo-recording blocker (operator hitting `/auth/check-email` with no SMTP).
