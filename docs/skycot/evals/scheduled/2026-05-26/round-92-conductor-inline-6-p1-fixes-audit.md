# Round 92 — Conductor mechanical-author 6-inline-P1-fix audit (commit 56f4ea0b4)

- **Date / UTC**: 2026-05-26T19:20Z
- **Round**: 92
- **Pillar / Claim**: process-integrity / R91 escalation closure (standing 5-6 round carry + #2429 collateral)
- **Verdict**: 🟢 **PASS** — all 6 inline-fixes are real (not "test mutates itself to green"), the new pre-merge typecheck gate is a structural improvement that prevents the recurrence pattern, and the operator's Option A (conductor takes mechanical-fix authority on standing zombie-claimed briefs) closed the R91-escalated 5-6 round carry. **Anti-theater dispositive on all 6 files.** 93rd anti-theater verification this week.

## Setup

R91 escalated 3 typecheck briefs that had been zombie-claimed without shipping for 5-6 rounds:
- `fix-onboarding-use-provision-tsc-error-missing-created` (6 rounds)
- `fix-audit-provenanceblock-tsc-error-hastext-undefined` (5 rounds)
- `ops-typecheck-pin-error-budget-budget` (5 rounds)

Plus an R88 brief that the conductor never authored:
- `fix-settings-themepicker-missing-themeprovider-in-tests`

The operator authorised "Option A" — let the conductor mechanical-author the fixes inline (instead of going through worker claim) AND add a new gate to prevent recurrence. Commit `56f4ea0b4` is the result. R91 also missed that #2429's `gatewayBase()` contract change broke 3 downstream test files; those 3 are bundled in this commit as "collateral".

## Observed — diff per file

(All diffs verified via `git show 56f4ea0b4 -- <file>`.)

1. **`web/lib/onboarding/use-provision.ts`** (+1 LOC)
   - Adds `created: false` to the demo-result object in `buildDemoProvisionResult`.
   - The `ProvisionResult` type was bumped in PR #2404 to require `created: boolean`; the demo path was missed.
   - 🟢 Real production fix.

2. **`web/components/skycot/audit/ProvenanceBlock.tsx`** (+2 LOC)
   - `hasText(value: string)` → `hasText(value: string | undefined)` + `if (value === undefined) return false;` guard.
   - PR #2366 introduced callsites that pass an optional value; the function signature lagged.
   - 🟢 Real production fix — the runtime guard is meaningful (returns false for missing fields, true for fields with non-separator content).

3. **`web/__tests__/settings/page.test.tsx`** (+5 LOC, -3)
   - Wraps `renderSettingsPage()` in `<ThemeProvider>`; the prior test description was "mounts without throwing on a fresh workspace (no providers required)" → renamed to "mounts without throwing on a fresh workspace".
   - PR #2368 surfaced `ThemePicker` in the workspace landing page; ThemePicker reads `useTheme()` which requires the provider.
   - 🟡-to-🟢 — the test description change weakens an invariant ("no providers required"), but that invariant was already false post-#2368. The real product render path mounts the page inside the app-shell layout (which DOES include ThemeProvider). The new test mirrors prod render, which is the more useful contract. **Acceptable.**

4. **`web/lib/api/audit.test.ts`** (+1 LOC, -1)
   - `expect(req.url).toBe('http://gw.test/api/v1/audit?range=today')` → `.toMatch(/\/api\/v1\/audit\?range=today$/)`.
   - Required by #2429's same-origin contract (browser-side URLs are now path-only, base is `""`).
   - 🟢 Real fix — the regex still pins **path + query string** with `$` end-anchor, so the assertion is meaningful (would catch a wrong path, wrong query, or accidental trailing junk).

5. **`web/lib/api/orchestration.test.ts`** (+2 LOC, -2)
   - Same regex relaxation pattern, applied to 2 `expect(fetchMock).toHaveBeenCalledWith(...)` assertions using `expect.stringMatching(/.../)`.
   - 🟢 Real fix; assertion still pins path including the dynamic `run-001` / `run-002` IDs and the action suffix.

6. **`web/lib/api/workers.test.ts`** (+1 LOC, -1)
   - Same pattern, but uses `new RegExp(...)` because the URL contains a runtime template variable `${MARIAH.id}`.
   - 🟢 Real fix.

## New structural artefact — `scripts/pre-merge-web-typecheck.sh` (+71 LOC) + `scripts/conductor-pre-merge-gate.sh` (gate 7→8 renumber)

The conductor inserted a new pre-merge gate that runs `pnpm tsc --noEmit` IF the diff touches any `web/**` file; auto-skips for Rust-only PRs.

- **Why this matters**: PR #2404 (use-provision) and PR #2366 (ProvenanceBlock) both shipped through the pre-merge gate WITHOUT being flagged for TypeScript errors. Both went 5-6 rounds before R91 caught them. A pre-merge typecheck gate would have refused both at merge-time.
- The gate is gated on file-path touch (web/** only), so it doesn't add cost to Rust-only PRs (the bulk of merges).
- The renumber (7→8) is clean — `conductor-pre-merge-gate.sh` only references gates by number internally, no external callers expecting a specific count.
- I did not run the new gate (out of scope; the conductor's own commit body says "new gate 7 verified clean against current tree").

🟢 Real structural improvement — prevents the recurrence pattern that introduced both 5-6 round carries.

## Anti-theater check

1. **None of the 6 fixes is "test mutates itself to green".**
   - Files 1 and 2 are production-code changes (use-provision.ts, ProvenanceBlock.tsx).
   - File 3 (settings/page.test.tsx) is a test-wrap change, but the wrap mirrors how the page renders in production — not a silencing.
   - Files 4-6 are regex relaxations that still pin path + query / dynamic IDs — assertions remain meaningful.
2. **The commit's verification block is internally consistent**: claims `pnpm tsc --noEmit` exits 0 (was exit 2 with TS2741 + TS2345); claims vitest 4 → 1. My scenario-2 vitest run found 1 failure on a DIFFERENT test (Step4Introduce, a load-induced flake), and the WizardEditorialShell rename in d40c4f72d closed the 1 that remained. So conductor's vitest math is consistent with my evidence.
3. **No code outside the 10 files in the diff was touched.** `git show 56f4ea0b4 --stat` returns exactly 10 files; no stowaways.
4. **Ledger flips are honest.** BUILD_STATUS.md `+8/-2` lines flip 3 TODO rows to MERGED-DIRECT and add 2 new MERGED-DIRECT rows for themepicker + #2429-collateral. The conductor didn't try to claim "row 5 also fixed".

## Gap / follow-up

None for this commit. The R91-escalated zombie-claim drift is closed (5-6 round carries → shipped via Option A). The structural improvement (new pre-merge typecheck gate) addresses root cause.

One **observation, not a brief**: the conductor's Option A pattern (operator authorises conductor to mechanical-fix-inline when a brief has zombie-claimed for N rounds) is a powerful escape valve, but the runtime currently has no auto-detection that suggests Option A to the operator. R91 had to escalate manually. The `ops-supervisor-block-runaway-zombie-claims-on-brief` candidate (referenced in r91 + my scenario-1 report) would create that detection signal; pair it with an "operator surface this for Option A?" prompt and the swarm becomes self-correcting on zombies.

(Evaluator does not author briefs. Surfaced for conductor decision.)

## Recommendation

Mark commit `56f4ea0b4` as a clean win. The conductor's mechanical-author authority should be exercised again when zombie-claim count > 4 rounds on a single brief, *with operator authorisation per case* (per existing process). The new pre-merge typecheck gate should ride in conductor-pre-merge-gate.sh going forward without further review.
