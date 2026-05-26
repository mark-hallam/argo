# Round 85 — PR #2404 fix-onboarding-provision-demo-mode-bypass POST-MERGE anti-theater

- **Date**: 2026-05-26
- **Round**: 85
- **Pillar / Claim**: Scene 3 (onboarding wizard) v1.0 demo-MP4 P0 — surfaces the "Hire your first AI teammate" investor walk-through. Carries Claim 1 + Claim 4 (Skills install, vertical pack) and is the SPA-tier closer for `feedback_genesis_is_ai_teammate_onboarding_studio`.
- **PR head SHA / merge commit**: `6d4b823acc0e320f281ee3fdfe7dc315924c51bb`
- **Merged at**: `2026-05-26T11:32:21Z` (conductor T11-30, under permanent web-only auth, after conductor-personal rebase against #2343 + #2362 + #2376)
- **Verdict**: 🟡 **PASS-WITH-CAVEAT** (P2 typecheck regression introduced on main)

## Setup

- `git show --stat 6d4b823acc0e320f281ee3fdfe7dc315924c51bb` — 10-file +406/-13 web-only merge.
- `pnpm typecheck` from `~/code/argo/web` against current origin/main.
- Source-read of the demo-mode short-circuit in `web/lib/onboarding/use-provision.ts` + the scrub helper + the workspace page callsite.
- `git diff 04a51bbff..6d4b823ac -- web/lib/onboarding/use-provision.ts` (= #2376 baseline → #2404 head) to isolate the regression introduction.

## Observed — the wins

- **Demo-mode short-circuit lives at `web/lib/onboarding/use-provision.ts:82-86`**: when `isDemoMode()` is true, `provision()` calls `buildDemoProvisionResult(req.workspaceName)` (deterministic `demo-tenant-<slug>` / `demo-workspace-<slug>` ids) + `persistDemoIds(result)` (sessionStorage) and RETURNS — no gateway round-trip, no `human:<email>` principal required.
- **X-Argo-Principal leak scrubber at `web/app/(setup)/onboarding/_scrub-error.ts`**: regex-match against `/x-argo-principal/i`, `/\bprincipal\b/i`, `/x-argo-/i`, `/bearer\s+token/i`, `/authorization\s+header/i` → returns `"We can't save this in demo mode. Sign in to continue with a real workspace."` Pattern is paranoid — five regex slots — which is the correct trade-off for an investor-visible surface.
- **Workspace page consumes the scrubber at line 56**: `setError(scrubOnboardingErrorMessage(e).message)` — the form-card catch path no longer leaks internal protocol detail.
- **143-line vitest unit suite** (`web/tests/onboarding-demo-mode-bypass.test.ts`) + **108-line Playwright spec** (`web/tests/e2e/onboarding-demo-mode-step-2-continue.spec.ts`) cover the F5/F5a/F5b assertions from `docs/skycot/ux-audits/2026-05-26-2007/chrome-live-walk-and-playwright.md`.
- Closes the largest single Scene 3 demo-blocker. With #2405 (Scene 6) this is 2 of the 4 demo-MP4 P0s landed.

## Observed — the typecheck regression

```
lib/onboarding/use-provision.ts(58,3): error TS2741: Property 'created' is missing
in type '{ tenantId: string; workspaceId: string; }' but required in type 'ProvisionResult'.
```

`buildDemoProvisionResult()` at `web/lib/onboarding/use-provision.ts:56-62` returns:

```ts
return {
  tenantId: `demo-tenant-${slug}`,
  workspaceId: `demo-workspace-${slug}`,
};
```

…but the `ProvisionResult` interface at line 12-20 declares `created: boolean` as required (added by PR #2376, merged 04a51bbff earlier the same day). `pnpm typecheck` now fails on origin/main.

**Runtime impact**: NIL. The downstream consumer at `web/app/(setup)/onboarding/workspace/page.tsx:47` reads `if (res.created)` which is falsy when `created === undefined`, so the demo-mode workspace step quietly skips the "Workspace created — welcome to Skycot." celebration toast. That's arguably semantically correct (no real provisioning happened) so the UX is acceptable.

**Dev-loop impact**: real. Any local `pnpm typecheck` returns exit 2 + the error above. Pre-merge typecheck gating (if enabled) would block follow-up PRs until this is fixed.

**Why the conductor T11-30 rebase missed it**: the PR body claims `pnpm --dir web typecheck — no NEW errors (pre-existing ProvenanceBlock.tsx TS2345 also fails on origin/main without my changes)`. That claim was true against the PR's original base (pre-#2376). When #2376 went in (+`created: boolean` required), the conductor's personal rebase resolved the merge conflicts in `pick-vertical/page.tsx`, `connect-provider/page.tsx`, `workspace/page.tsx`, etc., but did NOT re-run typecheck against the rebased tree. The TS error only surfaces after the merge of #2376's interface change + #2404's new code path that doesn't satisfy it.

## Anti-theater check

- **Demo-mode bypass is real code, not a test fiction**: the `if (isDemoMode())` guard is a single source of truth at the data-layer (`use-provision.ts`) — both the unit test and the Playwright e2e exercise it through the actual `provision()` function, not a mocked shim.
- **The scrubber is paranoid by design**: five regex patterns instead of one means a future "X-Argo-Tenant must be set" or "Bearer token missing" leak still triggers the demo-safe copy. A test-only fix would have hard-coded one substring.
- **Five wizard step files touched in coordination**: workspace, pick-vertical, first-hire, connect-provider, plus the three `use-*.ts` hooks. Theatre would touch one file; this touches the whole step-cluster because the demo-mode contract is wizard-wide.

## Gap / follow-up brief candidates

- **🟡 P2 — `fix-pr-2404-use-provision-buildDemoProvisionResult-set-created-false`** (~5 LOC, ~5 min):
  - File: `web/lib/onboarding/use-provision.ts:56-62`
  - Change: add `created: false` to the demo `buildDemoProvisionResult` return object (semantically correct: nothing was really provisioned).
  - Reason: restore `pnpm typecheck` exit 0 on origin/main; close TS regression introduced by the merge order #2376→#2404.
  - Auth: permanent web-only ≤5 LOC fix-* lane.

- **🟢 P3 — `ops-conductor-personal-rebase-runs-typecheck-on-rebased-head`** (~30 LOC bash, preventive):
  - Add `pnpm --dir web typecheck` to the post-rebase verification step in the conductor's personal-rebase workflow (the current T11-30 workflow already runs vitest but not typecheck).
  - Reason: this is the second conductor-personal-rebase TS regression in 48h (the first was the #2369 SignupClient.tsx page-title merge). A pre-commit typecheck would have caught this in seconds.

## Recommendation

🟢 Scene 3 demo-blocker is **closed on the runtime path** — the investor walk-through works.

🟡 But the typecheck regression must be cleaned up before the demo MP4 records, OR the operator must accept a non-clean typecheck baseline. Recommend the P2 brief above ship under permanent web-only auth in the next conductor tick.

(71st anti-theater verification this week.)
