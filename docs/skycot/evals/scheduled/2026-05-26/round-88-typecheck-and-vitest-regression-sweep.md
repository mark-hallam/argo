# Round 88 — Typecheck + Vitest regression sweep on origin/main

- **Date**: 2026-05-26
- **Round**: 88
- **Pillar**: Process — pre-merge gate coverage of `web/` regressions.
- **Verdict**: 🟡 **PARTIAL** — typecheck carry now 4-round unclaimed; **NEW**: 6 vitest failures across 3 files newly surfaced on main.

## Setup

- Two independent worktrees at `/tmp/eval-r88-wt/pr-{2413,2418}` both detached on recent PR heads (descended from origin/main).
- Each ran `pnpm exec tsc --noEmit` and `pnpm vitest run`.
- Identical results on both PRs confirms the issues are on origin/main, not introduced by either PR.

## Observed

### Typecheck — 2 errors persisting from r85/r86/r87

```
components/skycot/audit/ProvenanceBlock.tsx(36,56): error TS2345:
  Argument of type 'string | undefined' is not assignable to parameter of type 'string'.

lib/onboarding/use-provision.ts(58,3): error TS2741:
  Property 'created' is missing in type '{ tenantId: string; workspaceId: string; }'
  but required in type 'ProvisionResult'.
```

- `fix-pr-2404-use-provision-builddemoprovisionresult-set-created-false` (ledger line 319) → **4th-round unclaimed** (r85 → r86 → r87 → r88).
- `fix-pr-2366-provenance-block-hastext-accepts-optional` (ledger line 320) → **3rd-round unclaimed** (r86 → r87 → r88).
- `ops-conductor-pre-merge-gate-web-typecheck-pin` (ledger line 321) → still TODO since r86.

### Vitest — **NEW** — 6 failures / 1641 across 3 files

```
__tests__/settings/page.test.tsx                       4 failed
  × renders the settings shell and workspace panel on a fresh workspace
  × renders the canonical settings tabs with live routes as links
  × renders the danger-zone tab as a security anchor link
  × mounts without throwing on a fresh workspace (no providers required)

components/auth/__tests__/WizardEditorialShell.onboarding.test.tsx   1 failed
  × welcome step renders the shell chrome + mission-pillar copy

app/(app)/audit/page.test.tsx                          1 failed
  × groups rows under TODAY / YESTERDAY headers in the feed
```

The 4 `settings/page.test.tsx` failures all share the same stacktrace head: `useTenantTheme components/skycot/ThemeProvider.tsx:19:19` → `ThemePicker components/skycot/ThemePicker.tsx:16:52`. This is **likely root-caused by PR #2368 "fix(settings): surface theme picker in workspace"**, which inserted `<ThemePicker>` into the settings shell without ensuring a `ThemeProvider` is mounted in the test setup, or without giving `useTenantTheme` a safe default when no provider is present.

The remaining 2 failures (`WizardEditorialShell` + `audit page TODAY/YESTERDAY`) need separate triage but were also not flagged in r87. Likely candidates: #2400 (Genesis welcome framing), #2366 (audit empty fallbacks), or #2353 (audit feed hash envelope).

### Process — 29-round 🟢 streak

- Halt sentinels: ABSENT.
- Gateway: HTTP 200, schema v83 == codebase v83 (FRESH), uptime ~9.6h.
- Findings-only fix-by sweep: 4 / 4 within budget (all due 2026-05-30, 4 days remaining).
- Drift detector: 0 rows.
- Unread handoffs: 1 (my own r87, which the conductor will mark read after actioning).

The runtime "🟢 streak" measures gateway / drift / sentinels — **NOT** typecheck or vitest. Both can degrade in parallel without breaking the streak. This is itself a gap.

## Anti-theater check

The 4-round-unclaimed typecheck briefs ARE briefs the conductor authored, but workers haven't picked them up. Reason hypothesised in r87 — freshness header collisions OR the brief paths point at the wrong App Router location. Either way, the operator should know.

The vitest failures are dispositive — `vitest run` does not have a `--bail-on-first-failure` masking effect; all 6 failures rendered fully.

**85th anti-theater verification this week.**

## Recommendation

- (a) 🔴 **P1 SURFACE TO OPERATOR** — `fix-pr-2404-use-provision-builddemoprovisionresult-set-created-false` is now 4 rounds (r85→r88) unclaimed. Recommend the conductor either (i) bump the brief's `Authored against` SHA via `scripts/check-brief-freshness.sh` to re-grant claimability, (ii) convert to a 1-LOC mechanical fix and self-author the PR, or (iii) ask the operator whether the brief should be cancelled.
- (b) 🔴 **P1 SURFACE TO OPERATOR** — same situation for `fix-pr-2366-provenance-block-hastext-accepts-optional` (3 rounds unclaimed).
- (c) 🔴 **P1 NEW** — author `fix-settings-themepicker-missing-themeprovider-in-tests` (~10 LOC web-only fix-* lane). Root-cause hypothesis: `<ThemePicker>` calls `useTenantTheme()` which throws when no `ThemeProvider` is mounted; test setups need either a default-mode mock provider OR the hook needs a fallback. Suspect commit `c6697c9c4` (PR #2368).
- (d) 🟡 **P2 ESCALATE** — `ops-conductor-pre-merge-gate-web-typecheck-pin` (line 321) escalate **P3 → P1**. Three consecutive same-root-cause rounds + a new vitest-regression class means the systemic cost is now strictly lower than the per-PR leaf cost. Pin both `pnpm tsc --noEmit` AND `pnpm vitest run` into the conductor pre-merge gate when the PR diff touches `web/**`.
- (e) 🟡 **P2 NEW** — author `fix-wizard-editorial-shell-onboarding-test-restoration` and `fix-audit-page-today-yesterday-grouping-test-restoration` as paired briefs (~15 LOC each) for the remaining 2 vitest failures.

29-round 🟢 streak r57→r88 HOLDING on the runtime axis. But the test-suite axis has now degraded for 3+ rounds in a row with no closure motion. Process-wise this is a tracked-but-unmitigated risk.

## Files generated

- `docs/skycot/evals/scheduled/2026-05-26/round-88-pr-2413-sessionprovider-demo-bypass-pre-merge.md`
- `docs/skycot/evals/scheduled/2026-05-26/round-88-pr-2418-demo-banner-sidebar-overlap-pre-merge.md`
- This file.
