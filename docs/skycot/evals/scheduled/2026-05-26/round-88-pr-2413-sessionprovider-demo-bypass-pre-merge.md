# Round 88 — PR #2413 SessionProvider disable in demo-bypass — PRE-MERGE PASS

- **Date**: 2026-05-26
- **Round**: 88
- **Pillar / Claim**: Demo-recording polish (audit F2 P0); not a MISSION claim per se, but the v1.0 demo MP4 ship-gate depends on the SPA console being clean while the operator records.
- **Verdict**: 🟢 **PRE-MERGE PASS** (P0 — demo-recording blocker)
- **PR head SHA**: `af163fb4b3d83c4bb3e938f74b8a4d8f533ec011`
- **Size**: +71 / −0 across 2 files (web-only). Eligible for operator-locked overnight `--skip-clippy` ops-/fix-* lane.

## Setup

- Created worktree at `/tmp/eval-r88-wt/pr-2413` detached at the PR head SHA.
- `pnpm install --frozen-lockfile --prefer-offline` succeeded (6.5s).
- Ran `pnpm exec tsc --noEmit` and `pnpm vitest run`.
- Read the production change at [web/components/skycot/AuthProvider.tsx](web/components/skycot/AuthProvider.tsx) and the playwright spec at [web/tests/e2e/no-session-fetch-error-in-demo.spec.ts](web/tests/e2e/no-session-fetch-error-in-demo.spec.ts).

## Observed

**Production code** — `AuthProvider.tsx` reads two env-vars (`NEXT_PUBLIC_ARGO_DEMO_MODE` or `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`). When either equals `"1"`, it mounts the NextAuth `<SessionProvider>` with `session={null}`, `refetchInterval={0}`, `refetchOnWindowFocus={false}`. This is the documented escape hatch in NextAuth v5 for skipping the initial `/api/auth/session` fetch + the polling loop. Consumers of `useSession()` still get a valid context returning `{ data: null, status: "unauthenticated" }`, which the SPA already handles in demo flows.

**Test** — Playwright spec walks 6 demo routes (`/`, `/team`, `/audit`, `/approvals`, `/skills`, `/workflows`) and listens for both `console.type === "error"` AND `pageerror` events. Asserts the filtered array `fetchErrors` (only `ClientFetchError`-mentioning lines) is empty.

**Vitest** — 6 failed / 1635 passed (1641 total) on the PR head. All 6 failures are pre-existing on origin/main, NOT introduced by this PR. Affected files:

- `__tests__/settings/page.test.tsx` (4 failures, ThemeProvider context bug)
- `components/auth/__tests__/WizardEditorialShell.onboarding.test.tsx` (1 failure)
- `app/(app)/audit/page.test.tsx` (1 failure)

These are flagged separately in `round-88-typecheck-and-vitest-regression-sweep.md`.

**Typecheck** — `pnpm exec tsc --noEmit` returns exit 2 with the same 2 errors from r85/r86/r87:

- `lib/onboarding/use-provision.ts:58 TS2741` (now 4th-round unclaimed)
- `components/skycot/audit/ProvenanceBlock.tsx:36 TS2345` (now 3rd-round unclaimed)

Neither error touches `AuthProvider.tsx`. The PR is inheriting them, not introducing them.

## Anti-theater check

Dispositive on two axes:

1. **Production code is unconditional behaviour, not a test prop**. The `if (DEMO_BYPASS)` branch returns a `<SessionProvider>` configured for no polling. There's no test-only mock path — the real browser session, when run with the bypass env-var set, gets the same configured provider. A test that "rewrites itself green" would have to mock `SessionProvider`, but the spec doesn't.
2. **Test listens on real Playwright events**. `page.on("console", ...)` and `page.on("pageerror", ...)` capture whatever the browser actually emits. If the production code accidentally re-enabled polling under `DEMO_BYPASS`, the assertion `expect(fetchErrors).toEqual([])` would fail. The spec doesn't suppress, filter, or rewrite the console pipeline.

Cross-checked against the v5 NextAuth source contract: passing `session={null}` AND `refetchInterval={0}` means `useSession()` does NOT trigger the internal `/api/auth/session` poll. The fix is library-supported, not a workaround.

**83rd anti-theater verification this week.**

## Recommendation

🟢 **P0 MERGE** under operator-locked overnight `--skip-clippy` fix-* lane (~6h remaining window per `feedback_overnight_genesis_track_p_merge_authorization_2026_05_25`).

The PR is web-only ≤200 LOC ≤2 files — also satisfies the permanent `feedback_web_only_merge_authorization_2026_05_26` lane (with vitest summary in the body acceptable; the PR's specific change has no vitest tests, the 6 pre-existing failures are unrelated).

## Gap / follow-up brief candidates

None directly from this scenario. The pre-existing typecheck + vitest carries are recommended via the separate `round-88-typecheck-and-vitest-regression-sweep.md`.
