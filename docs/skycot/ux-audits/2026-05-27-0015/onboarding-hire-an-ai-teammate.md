# Onboarding "Hire an AI teammate" — investor narrative e2e UX audit

**Date**: 2026-05-27 00:15 AEST (Melbourne)
**Auditor**: e2e-ux-journey-test-agent (read-only, demo-mode SPA on :3000 + gateway :7400 schema v83)
**Scope**: `/landing` → `/auth/signup` → 5-step `/onboarding/*` wizard → `/workers/{id}` (C2-personality detail).
**Lens**: POC + investor demo (per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md` — function side must be real, dressing side may be fixture).
**Prior audit**: `docs/skycot/ux-audits/2026-05-26-2309/onboarding-hire-an-ai-teammate.md`. **9 of 15 prior findings remain UNCHANGED on `origin/main`** — the in-flight remediations (PRs #2411, #2412, #2413, #2418) are all in the "review" lane but not yet merged. This audit re-verifies the rejected/unmerged P0s and surfaces 3 additional findings the prior pass didn't catch.

---

## TL;DR

12 findings: 4 P0, 4 P1, 3 P2, 1 P3. Three brand-new findings (env-var typo F5-new, fake provider-key validation F6-new, "we install skills" subtitle theater F7-new). Six prior findings unchanged. Three prior findings have fixes authored on branches but not yet on main; their behavior in the running dev server today is unchanged.

The narrative the v1.0 demo is supposed to record — **"in four minutes, hire your first AI teammate, then walk into the workspace and meet them"** — is end-to-end broken on `origin/main` today in two compounding ways:

1. **The hire commit produces a teammate the next page literally says doesn't exist.** In demo mode `useFirstHire` now returns `demo-worker-<verticalId>-<timestamp>` (recent fix; better than the prior `stub-<uuid>`). But the sample roster the SPA falls back to (`web/lib/sample-worker-overview.ts:175-181`) only knows the five canonical UUIDs (Mariah / Peta / Priya / Jamal / Nori). So the redirect to `/workers/demo-worker-receptionist-mclr0xqz` ends in the "Worker not found — The worker 'demo-worker-receptionist-mclr0xqz' doesn't exist or you don't have access. [Back to team]" empty state (`web/components/skycot/WorkerNotFound.tsx:16`). The camera reads this as a failure.

2. **An env-var typo in `.claude/launch.json` (NEW THIS AUDIT)** silently disables the workers/team/overview mock-fallback path. `launch.json:9` sets `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`; every code site reads `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` (with the `ARGO_` prefix). The two paths drift: onboarding short-circuits via `isDemoMode()` (which reads `NEXT_PUBLIC_SKYCOT_DEMO_MODE` correctly), but the post-hire worker-overview path runs through `isAuthBypassed()` which never trips, so the SPA hits the real gateway, gets unauthorized, falls through to mock-on-404 — and only then discovers there's no sample row for a `demo-worker-*` id.

Also new: the connect-provider step's "$0.001 test call confirms the key works" sub-headline is a function-side promise the demo breaks. In demo mode `validateProviderKey()` returns `{validated: true, testCost: 0.001}` without any HTTP call (`web/lib/onboarding/use-provider-key.ts:40-42`). Any sk-ant- prefix string passes — there is no test call. The camera sees the green "Key validated. Forwarding…" with no real call having been made. Same shape for pick-vertical: the "We install the skills, memory schema, and approval gates" promise is bypassed in demo mode.

**Recommendation for the demo recording**: Do NOT walk through `/onboarding/first-hire` → `/workers/<id>` live. Click through to step 5, then narrate "and here's the workspace with my hire alongside me" while showing `/team` (which already has the SAMPLE_TEAM personas). Or merge PRs #2411 + #2413 + the pending demo-mode worker-roster brief and re-record after.

---

## Headline blockers (top 4 P0s)

| # | Finding | Demo-narrative impact |
|---|---|---|
| **F1** | Demo-mode hire still ends at "Worker not found" — new id shape `demo-worker-<verticalId>-<ts>` is not seeded in the sample roster, so the SPA renders the 404 empty state. | The "meet your new teammate" payoff IS a function-side fabrication the camera sees: a `[Back to team]` panel where the personality card should be. Violates `feedback_no_theater_in_demo`. |
| **F2** | Genesis page is still orphaned on main — no nav link, no /onboarding/welcome CTA, no marketing footer entry. Gateway endpoint still 503s with `genesis_provider_key_missing`. | "AI Teammate Onboarding Studio" — the operator-locked investor moment (`feedback_genesis_is_ai_teammate_onboarding_studio`) — has no discoverable surface and no working backend on `origin/main`. |
| **F3** | "$0.001 test call confirms the key works" is function-side theater in demo mode. `validateProviderKey()` short-circuits to `{validated: true, testCost: 0.001}` with no HTTP call (`use-provider-key.ts:40-42`). | The camera sees "Key validated. Forwarding…" appear after the buyer types `sk-ant-bogus`. Investors read this as a real key check. Same shape for pick-vertical's "We install the skills…" subtitle when nothing is installed. |
| **F4** | `.claude/launch.json` env-var typo — `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` (without `ARGO_` prefix). Every code site reads `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`. NEW THIS AUDIT. | The workers/team/overview mock-bypass branch (`isAuthBypassed()`) never trips. Manifests in F1 (the not-found page is reached because the worker-overview call hits the gateway first, then falls through to a sample roster that doesn't have the synthetic id). Trivial one-line fix; large blast radius. |

---

## Did the agent answer the brief's critical questions?

| Question | Answer |
|---|---|
| Does the hired worker exist in the gateway after the wizard completes? | **No** in demo mode (the recording configuration). `firstHire()` short-circuits at `web/lib/onboarding/use-first-hire.ts:120-122` and returns `demo-worker-<verticalId>-<ts>` without a gateway round-trip. Outside demo mode it does POST to `/api/v1/onboarding/first-hire` AND verifies readability with up to 3 retries (`verifyWorkerReadable`); if the read fails it surfaces the honest `FIRST_HIRE_VERIFY_FAILED_MESSAGE` and stays on step 5. That outside-demo path is actually well-engineered; the demo-mode shortcut is what's broken. |
| Does the new worker get a real charter / persona / memory schema? | **In demo mode: no — and the next page openly says so.** The `demo-worker-*` synthetic id has no row in the gateway, no row in `sample-worker-overview.ts`, no row in `sample-team.ts`. The SPA renders `WorkerNotFound` — UserX icon + "The worker '<raw id>' doesn't exist". Outside demo mode the wizard's worker_draft + answers DO go to the gateway and a real worker row is written (and verified) — the `/team` deck still doesn't optimistically insert though, so the new row appears on the next refetch tick. |
| Does the flow feel like onboarding a human teammate? | **/onboarding does on the surface** (F1 editorial-center holds up, brand voice consistent, copy is teammate-first across welcome/workspace/first-hire). But the step-3 "test call" theater + step-4 "we install skills" theater + step-5 "meet your new teammate" 404 break the trust ladder one step before payoff. The shape is right; the substance is hollow in demo mode. |
| Is there a Genesis entry point? | **Still no.** `grep -rn 'href="/genesis"' web/` returns zero matches. The `data-testid="onboarding-pillar-genesis-studio"` tile on `/onboarding/welcome` (which DESCRIBES Genesis behavior — "voice or text, upload SOPs, we verify they understand before they start") is not a link. "Get started" still routes to `/onboarding/workspace`. |

---

## Prior-audit findings — status on `origin/main` now

| Prior # | Status now | Notes |
|---|---|---|
| 2309-F1 (Genesis orphan + 503) | **UNCHANGED** | F2 in this audit |
| 2309-F2 (parallel /hire + /onboarding wizards) | **PARTIAL** | Marketing CTAs now route to /auth/signup, not /hire (good). But /team, /workers, dashboard "hire" button + CommandPalette all still push to /hire. Two flows still ship side-by-side. F8 in this audit. |
| 2309-F3 (stub-uuid post-hire) | **UNCHANGED in effect** | Synthetic id shape changed from `stub-<uuid>` to `demo-worker-<verticalId>-<ts>`. Sample roster still doesn't seed it. Same 404 destination. F1 in this audit. |
| 2309-F4 (welcome pillar promises Genesis, doesn't link) | **UNCHANGED** | F9 in this audit. |
| 2309-F5 (Hire teammate vs Hire worker copy) | **UNCHANGED** | `/for/_components/VerticalLandingShell.tsx:600` still says "Hire your first worker". F10 in this audit. |
| 2309-F6 (/landing CTA bypasses signup) | **FIXED on main** | `/landing` CTAs now all route to `/auth/signup`. Verified via SSR + grep. |
| 2309-F7 (/workers redirects to /team) | **UNCHANGED** | F11 in this audit. |
| 2309-F8 (`/api/v1/workers` missing from /routes catalog) | UNCHANGED (out of POC scope) | Not re-listed; v1.1 work. |
| 2309-F9 (low-contrast label on /hire step 1) | UNCHANGED (out of scope — /hire is not the canonical demo flow) | Not re-listed. |

The "in review" PRs (#2411, #2412, #2413, #2418) that the BUILD_STATUS ledger lists as remediation are all still pending merge as of `git log origin/main` at audit time. Confirmed by `git merge-base --is-ancestor` for each.

---

## All findings

### F1 [P0] — Demo-mode hire redirects to "Worker not found"

`web/lib/onboarding/use-first-hire.ts:102-112` builds a synthetic id of the shape `demo-worker-<verticalId>-<timestamp-base36>`. The wizard then redirects to `/workers/<id>` (line 57 of `first-hire/page.tsx`), which falls through `/workers/<id>/overview/layout.tsx` setting the page title to the raw id, then the WorkerLayoutShell mounts `useWorker(workerId)` which calls `fetchWorker()` → tries the gateway → fails → falls back to `findSampleWorker()` (`web/lib/api/workers.ts:140`) → returns undefined → throws `WorkerNotFoundError` → layout renders the `WorkerNotFound` empty state ("The worker 'demo-worker-receptionist-mclr0xqz' doesn't exist or you don't have access.") with a `[Back to team]` button.

This is exactly the function-side theater the operator's `feedback_no_theater_in_demo` rule forbids: the camera clicks "Hire them" and the next page literally tells the buyer their hire doesn't exist.

The earlier audit (2026-05-26-2309 F3) flagged this with the `stub-<uuid>` shape; the fix changed only the id format, not the underlying issue.

**Reproduction**:
1. With `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` set on the dev server (as `.claude/launch.json` configures), open `/onboarding/welcome` → click through workspace → connect-provider (type any `sk-ant-*` key, click validate) → pick-vertical (click any tile) → first-hire (type any description, click "Hire them").
2. Wait for redirect.
3. SSR-verify the destination renders the not-found shell: `curl http://127.0.0.1:3000/workers/demo-worker-receptionist-anyid/overview` returns 200 with `<title>demo-worker-receptionist-anyid · Overview</title>` and 404 markers in the body.

**Files**:
- `web/lib/onboarding/use-first-hire.ts:102-112` (`buildDemoFirstHireResult`)
- `web/lib/sample-worker-overview.ts:175-181` (only seeds the 5 canonical UUIDs)
- `web/components/skycot/WorkerNotFound.tsx:16` (the empty-state component)
- `web/app/(app)/workers/[workerId]/overview/layout.tsx:10` (page title = raw id)

**Suggested fix** (pick one): (a) make `buildDemoFirstHireResult` return one of the SAMPLE_TEAM uuids (Mariah/Peta/Priya/Jamal/Nori) — choose based on `verticalId` — so the redirect lands on an existing persona; (b) author a 6th sample-worker-overview entry keyed `^demo-worker-` that uses the buyer's typed description as the charter and a "new hire" persona; (c) ditch the demo-mode short-circuit and run the demo against a real gateway with the workspace seeded.

---

### F2 [P0] — Genesis still orphaned + still 503 on the running gateway

Repeat finding from 2026-05-26-2309 F1. Status confirmed:

- `grep -rn 'href="/genesis"' web/` → zero matches.
- `grep -rn '"/genesis"' web/components web/app | grep -v test` → zero matches.
- NAV_ITEMS (`web/components/skycot/nav-items.tsx:22-30`) lists Approvals / Team / Workers / Cohorts / Skills / Workflows / Audit; no Genesis.
- `curl -X POST http://127.0.0.1:7400/api/v1/genesis` returns `{"error":{"code":"genesis_provider_key_missing","message":"Set ARGO_GENESIS_PROVIDER_KEY to enable POST /api/v1/genesis (LLM-driver cost guardrail)"}}`. The `ARGO_GENESIS_PROVIDER_KEY` env var is unset on the running gateway.
- The `/onboarding/welcome` "Onboard them like a human" pillar (`data-testid="onboarding-pillar-genesis-studio"`) — the literal Genesis pitch — still has no click handler.

Per `feedback_genesis_is_ai_teammate_onboarding_studio` this is the investor-moment surface. The fix-brief is claimed (PR #2410 status: claim) but not yet merged.

**Suggested fix** (operator decision): (a) add `/genesis` to NAV_ITEMS as item 1 (above Approvals) when in demo mode; (b) wire the `genesis-studio` pillar tile itself to navigate to `/genesis`; (c) make `/genesis` the canonical first-hire route, redirect `/onboarding/welcome` → `/genesis`; (d) set `ARGO_GENESIS_PROVIDER_KEY` on the dev gateway so the streaming pipeline works.

---

### F3 [P0] — Connect-provider step performs no test call in demo mode

`web/lib/onboarding/use-provider-key.ts:40-42`:

```ts
if (isDemoMode()) {
  return { validated: true, testCost: 0.001 };
}
```

The page subtitle reads "A **$0.001 test call** confirms the key works — or skip and use a mock provider for now." (`connect-provider/page.tsx:139-140`). In demo mode there is no HTTP call. Any string starting with `sk-ant-` passes. The "Key validated. Forwarding…" success message + the $0.001 cost figure are both confabulated.

This is more egregious than F1 because the buyer types a value (their actual Anthropic key, in a real demo session) and sees a "validated" confirmation — establishing trust in a verification that didn't happen. Per `feedback_no_theater_in_demo`, this is function-side fabrication the camera sees. NEW THIS AUDIT.

**Reproduction**:
1. Open `/onboarding/connect-provider`. Type `sk-ant-bogus-key-xyz` into the input.
2. Click "Validate & continue".
3. Watch the green "Key validated. Forwarding…" appear.
4. Confirm via `curl` or DevTools-network that no POST to `/api/v1/onboarding/provider-key` was made.

**Files**:
- `web/lib/onboarding/use-provider-key.ts:40-42` (the demo-mode short-circuit)
- `web/app/(setup)/onboarding/connect-provider/page.tsx:137-141` (the headline copy that promises a test call)

**Suggested fix**: in demo mode, either (a) actually call a no-op gateway endpoint and pass through whatever it returns, or (b) honestly relabel the step in demo mode — change the subtitle to "**Demo mode** — any `sk-ant-` value advances you to the next step. We'll skip the real test call." Option (b) is cheaper and is what the prior `connect-provider-page` audit at 2026-05-26-2007 flagged earlier (F5b protocol-leak fix). The honest-relabel is the rule already established for `/auth/signup`'s "Demo mode is active — no email is sent" banner; carry it through.

---

### F4 [P0] — env-var name typo in `.claude/launch.json` silently disables auth-bypass mocks

`.claude/launch.json:9`:

```
NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1 NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:7400 NEXT_PUBLIC_ARGO_DEMO_MODE=1 NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 pnpm --dir web dev
```

But every code site checks the prefixed name:

- `web/lib/api/workers.ts:103` — `process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"`
- `web/lib/api/team.ts:49` — `process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"`
- `web/lib/api/worker-overview.ts:94` — `process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"`
- `web/middleware.ts:58` — `process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"`
- `web/playwright.config.ts:50` — `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS: "1"`
- `web/.env.example:19` — `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=0`
- `web/vercel.json:13,21` — `"NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS": "0"`

Without the `ARGO_` prefix the var is just inert. Effects on the demo journey:

- `isAuthBypassed()` returns false in `lib/api/workers.ts` (and team / worker-overview). On every fetch the SPA tries the real gateway first, gets unauthorized (no X-Argo-Principal — the demo-mint cookie isn't surfaced cross-route in some navigations), then runs the catch branch. For `fetchWorker` and `fetchWorkerOverview` the catch falls back to mock-on-404 — but unauthorized is a different error class, so the fallback only fires when the gateway returns 404 not 401.
- Compounds F1: the `demo-worker-*` synthetic id walks all the way through `apiGet` instead of being intercepted up-front.

Middleware bypass survives because of the separate `NEXT_PUBLIC_ARGO_DEMO_MODE === "1"` branch (middleware.ts:72-77). That branch + the launch.json having `NEXT_PUBLIC_ARGO_DEMO_MODE=1` set correctly is the only reason the SPA isn't redirecting every page to /auth/signin.

NEW THIS AUDIT.

**Reproduction**:
1. `cat .claude/launch.json | grep AUTH_BYPASS` — confirm the typo.
2. `rg 'NEXT_PUBLIC_(ARGO_)?GATEWAY_AUTH_BYPASS' web/` — confirm every callsite is prefixed.
3. Stop the dev server, fix the typo to add the `ARGO_` prefix, restart. Verify the F1 not-found page now renders the sample-roster mock instead of the empty state. (Won't fully fix F1 — still need the sample-roster to know the `demo-worker-*` id — but it's a layered defense.)

**File**: `.claude/launch.json:9`

**Suggested fix**: one-character (well, 5-character) change. Either rename in the launch.json or rename in the code (but the latter requires 7 sites edited + tests). The launch.json edit is the right one. **Highest leverage P0 in this audit.**

---

### F5 [P1] — Signup demo-mint button violates its own visible promise (still broken on main)

`web/app/auth/signup/page.tsx:203` AND `:221` (both the primary submit and the secondary "no email required" button):

```
disabled={pending || email.trim().length === 0}
```

Both buttons are disabled until the user types something into the email field. The secondary button's label is **"Or try a demo workspace · no email required"**. A user clicking it without typing an email sees nothing happen.

The handler `submitDemoMint()` also sets `setDemoError("Enter an email to continue.")` if the field is blank — explicitly contradicting the button's own copy.

PR #2412 (`fix-signup-no-email-required-button-honest-copy`) ships the correct fix (drop the disabled gate + mint synthetic `demo+<ts>@skycot.test`) but is in review only — not on main. Confirmed via `git merge-base --is-ancestor caa2315eb origin/main` → false.

**Reproduction**:
1. Open `/auth/signup` in a non-demo dev environment OR with `NEXT_PUBLIC_ALLOW_DEMO_MINT=1` so the secondary button renders.
2. With email field empty, hover the "Or try a demo workspace · no email required" button. It's `cursor: not-allowed` and `opacity: 0.55`.
3. Type an email. The button enables.

**File**: `web/app/auth/signup/page.tsx:67-71, 220-221, 232-233`. Fix is `caa2315eb` (claude-fix-signup-no-email-required-button-honest-copy branch).

**Suggested fix**: merge #2412.

---

### F6 [P1] — pick-vertical step also skips the install in demo mode (function-side theater on subtitle promise)

`web/app/(setup)/onboarding/pick-vertical/page.tsx:226-229`:

```ts
if (isDemoMode()) {
  setBusy(false);
  router.push("/onboarding/first-hire");
  return;
}
```

The page subtitle reads "We install the **skills, memory schema, and approval gates**. You can swap or extend later." (line 264-266). In demo mode none of that happens. The buyer clicks a tile, the wizard advances, and pretends to have installed the vertical pack. The skill counts on each card (8 for gtm-engineer, 5 for marketing-intern, etc.) come from the bundled fallback list (lines 108-158) which is honest about being a fallback — but the post-click promise IS broken.

Same shape as F3 (provider-key). NEW THIS AUDIT.

**Reproduction**: Click any vertical tile in demo mode → instant redirect to /first-hire → no `POST /api/v1/onboarding/install-vertical` fired (verify via DevTools-network or `tail` the gateway logs).

**File**: `web/app/(setup)/onboarding/pick-vertical/page.tsx:226-229, 264-266`.

**Suggested fix**: same options as F3. Honest relabel: "**Demo mode** — pick a starting template. We'll install the real skills + memory schema once you create a real workspace."

---

### F7 [P1] — In-product "hire" CTAs still push to /hire (the parallel wizard); only /landing was rewired

`grep -rnE '(href="/hire"|router\.push\("/hire"\))' web/app web/components` returns 12 callsites. The /landing rewrite to /auth/signup (closing prior 2309-F6) was clean, but every IN-PRODUCT entry point still points at /hire:

- `web/app/(app)/page.tsx:460` (dashboard)
- `web/app/(app)/team/page.tsx:268` + `:469` (team page CTA + empty-state)
- `web/app/(app)/team/error.tsx:38` (team error page CTA)
- `web/components/skycot/CommandPalette.tsx:449` (⌘K → "Hire a worker")
- `web/components/skycot/TeamEmptyState.tsx:54`
- `web/components/skycot/workflows/WorkflowEdgeStates.tsx:173`

So once a buyer signs up and is inside the app, every "hire" affordance routes to the OTHER 5-step wizard (`/hire/step/1..5` — different copy, different state shape, different completion shape). The brief mentions the dual-flow problem (prior 2309-F2); status now is "marketing side fixed, in-product side untouched".

PR #2411 (`fix-hire-demo-mode-stub-uuid-worker-roster-invisibility`) touches workers.ts but not these callsites. No open brief consolidates them as far as `find docs/build-plan/streams -name '*hire*' | xargs grep -l '/onboarding\|/hire'` shows.

**Suggested fix**: pick one wizard. Either delete `/hire` and rewire all in-product callsites to `/onboarding/first-hire`, or delete `/onboarding/*` and rewire the demo entry to `/hire`. Both wizards exist, both ship; this is real drift.

---

### F8 [P1] — Both NAV_ITEMS "Team" and "Workers" lead to the same deck

`web/app/(app)/workers/page.tsx:23-29`: if `workers.length > 0`, `router.replace("/team")`. In demo mode SAMPLE_TEAM has 5 rows; the condition is always true; `/workers` always bounces to `/team`. But NAV_ITEMS (`web/components/skycot/nav-items.tsx:22-30`) renders BOTH `/team` and `/workers` as separate nav rows.

Result: the sidebar shows two adjacent nav items that take the buyer to the exact same page. Brand-restraint nav should never have a "decoy" item.

The brief's reference to "/workers" as the destination of the post-hire deck is itself stale — the deck lives on /team. Brief copy + brief wording should be updated to "/team".

**Reproduction**: Open the app shell, click "Workers" in the sidebar. Observe URL silently change to `/team`. Click "Team". Same destination.

**File**: `web/app/(app)/workers/page.tsx:23-29`, `web/components/skycot/nav-items.tsx:24-25`.

**Suggested fix**: drop one of the two nav items. Either rename /workers → /team in code (a big move) or hide the /workers nav row when the roster is non-empty (small move; keeps the empty-state route accessible).

---

### F9 [P2] — `/onboarding/welcome` Genesis-pillar tile is a static `<li>`, not a button

`web/app/(setup)/onboarding/welcome/page.tsx:43-49` renders the "Onboard them like a human" tile with `data-testid="onboarding-pillar-genesis-studio"`. The tile describes Genesis ("voice or text, upload SOPs, we verify they understand before they start") but has no click handler. Repeat of prior 2309-F4; status unchanged.

**Suggested fix**: make the tile into a `<Link href="/genesis">` once F2 is resolved. Currently no destination exists.

---

### F10 [P2] — Inconsistent primary CTA wording: "Hire your first teammate" everywhere except per-vertical landing pages

The `/landing` hero + `/landing` top-bar + `/auth/signup` hero + `/team` deck CTA + `/compare` CTA + customer-detail page CTA all say "Hire your first teammate" (or "AI teammate"). But `/for/[vertical]` pages (e.g. `/for/sales`) render the same CTA as **"Hire your first worker"** via `web/app/for/_components/VerticalLandingShell.tsx:600`. Repeat of prior 2309-F5; status unchanged.

`/for/_data/verticals.ts` does use "teammate" inside per-vertical headlines ("Hire your first sales teammate", "Hire your first recruiting teammate", etc.), so the CTA copy is the only outlier.

**Suggested fix**: `web/app/for/_components/VerticalLandingShell.tsx:600` — "Hire your first worker" → "Hire your first teammate" (or per-vertical "Hire your first sales teammate"). Single-char.

---

### F11 [P2] — Page title on /workers/{id}/overview shows the raw id when the worker doesn't resolve

`web/app/(app)/workers/[workerId]/overview/layout.tsx:8-10`:

```ts
return { title: `${workerId} · Overview` };
```

When F1 hits (demo-worker-* id not resolvable), the browser tab title becomes `demo-worker-receptionist-mclr0xqz · Overview`. Not catastrophic — the camera doesn't usually read the title bar — but worth fixing alongside F1.

**Suggested fix**: in `generateMetadata`, attempt the gateway/sample lookup and fall back to "Worker · Skycot" on miss. Or use a generic title and let the body component render the worker name.

---

### F12 [P3] — `buildDemoProvisionResult` omits the required `created` field

`web/lib/onboarding/use-provision.ts:56-62` returns `{tenantId, workspaceId}` only. The `ProvisionResult` interface declares `created: boolean` as required (line 19). TypeScript-wise the function should fail to compile; it doesn't because the file is `.ts` and the `ProvisionResult` return type isn't enforced on the helper.

Effect: `res.created` is `undefined` (falsy) in the workspace step's `if (res.created)` check (`workspace/page.tsx:47`), so the "Workspace created — welcome to Skycot." celebration toast never fires in demo mode. The wizard silently skips the moment of celebration the operator wrote.

**Suggested fix**: add `created: true` to the returned object in `buildDemoProvisionResult`. One-line.

---

## Resolved since the 2026-05-26-2309 audit

- **2309-F6 (/landing CTA bypasses signup)** — FIXED on main. `/landing`'s "Hire your first teammate" pill now routes to `/auth/signup`, not `/hire`. The demo-mint button on /auth/signup then carries the buyer into onboarding.

---

## Orientation score

**40 / 100** for the demo recording lens.

Why: the journey shape is right (5-step F1-locked editorial-center, brand-voice copy, single primary action per step, magic-link signup-bypass for the demo). The end-to-end function side is broken in three accumulating ways (F1 + F3 + F6 are all "the SPA pretends the gateway did something it didn't"). The "meet your new teammate" payoff is a hard 404 — exactly what no-theater-on-function forbids. The investor-moment surface (Genesis) has no entry point and no working backend.

This is not a deep design problem; it's a "the last 10% wasn't wired" problem. **F1 + F4 fix together with about 30 lines of code changes** (rewire `buildDemoFirstHireResult` to return a known sample-roster uuid + fix the .claude/launch.json typo). **F3 + F6 fix with two copy edits** (the "test call" and "we install skills" subtitles need a demo-mode honest-relabel branch). **F2 is the only one with substantive scope** — Genesis needs both nav-entry wiring AND the dev gateway needs `ARGO_GENESIS_PROVIDER_KEY` set; without those, the investor moment can't be demoed.

For the demo MP4 today: skip `/onboarding/first-hire → /workers/<id>` live. Show /landing → /auth/signup → "Enter demo workspace" → /onboarding/welcome → narrate through workspace + connect-provider (don't click validate — just talk over it) → pick-vertical (don't click — pan across the cards) → cut to /team showing the SAMPLE_TEAM deck → click Mariah → narrate her C2-personality page. That arc tells the same story without hitting any of F1/F3/F6/F11 on screen.
