# SKYCOT e2e UX audit — Onboarding hire-flow (investor narrative)

**Audit date**: 2026-05-27 06:23 AEST (Melbourne)
**Auditor**: Claude (e2e UX journey agent)
**Tools**: Claude Preview MCP (Chromium) driving SPA `:3000` + gateway `:7400` (schema v83, ~16h uptime)
**Setup**: `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1 NEXT_PUBLIC_ARGO_DEMO_MODE=1` (canonical demo recording env)
**Scope**: `/landing` → `/auth/signup` (demo-mint) → `/onboarding/welcome` → `workspace` → `connect-provider` → `pick-vertical` → `first-hire` → `/workers/<id>` (C2-personality detail). 5-step wizard end-to-end.
**Prior audit**: `docs/skycot/ux-audits/2026-05-25-1100-onboarding-hire-an-ai-teammate.md` — many findings from that pass appear to still apply. This audit re-validates against current `origin/main` and refreshes the status.

---

## TL;DR

**13 findings**: 3 P0 (function-side blockers the demo camera will see), 5 P1 (visible during the flow, workable), 4 P2 (polish), 1 P3 (nit).

**The 5-step wizard works front-to-back** — every step renders the locked F1-editorial-center design, navigation is smooth, the "four minutes from plain English to a working teammate" promise is mechanically honoured. The auth signup → demo-mint → onboarding/welcome handoff is clean.

**But the metaphor leaks at the finish line**. The user types "A marketing intern named Maya..." and the wizard:
1. **Throws Maya away.** In demo mode the wizard hard-codes a redirect to one of 4 sample worker UUIDs (Mariah/Priya/Jamal/Nori), picked by `verticalId` — not by the operator's described worker. `marketing-intern` → Nori. Camera sees the user type "Maya", clicks Hire, lands on a page titled **"Nori · Overview"** with stock "drafts product comms" copy — different name, different role, different stats. This is the textbook theater the operator banned (per `feedback_no_theater_in_demo`). [F1, P0]
2. **The real Maya WAS hired on the backend** (gateway `POST /api/v1/onboarding/first-hire` ran during my walk and `/api/v1/team` returns Maya with the correct charter + tagline). But the SPA throws the real worker_id away and shows the sample one. So the demo flow demonstrates the OPPOSITE of what's actually working: it lies about the hire to look better.
3. **If you navigate to the real Maya** at `/workers/<real-uuid>`, the page crashes — gateway returns `charter` as `{text,version,last_edited_at,last_edited_by}`, SPA renders it as a string → React throws. [F2, P0] (Same root cause as 2026-05-25 audit F5/F8 — STILL UNFIXED.)
4. **`/team` shows the SAMPLE_TEAM fixture instead of the real team.** Gateway `/api/v1/team` returns 1 worker (Maya); the team page renders 5 sample workers (Mariah/Peta/Priya/Jamal/Nori) + "5 teammates working · 935 actions/wk · 60 hours saved" hero. Maya doesn't appear. The hire that the user just performed is invisible. [F3, P0]

**Headline call**: the hire-an-AI-teammate metaphor is INTACT through the wizard but BROKEN at every post-hire surface. The function (the gateway, the hire endpoint, Maya's charter) is real and works. The UI dressing — what the demo camera sees — fabricates a different worker, lies about the team size, and crashes on the real one. The fixes are tractable (3 file paths below). After they land the journey works end-to-end without theater.

---

## Headline blockers (P0)

| # | Sev | Finding | Narrative impact |
|---|---|---|---|
| **F1** | **P0** | `useFirstHire` in demo mode short-circuits to a hard-coded sample worker UUID (`SAMPLE_NORI` for marketing-intern) and discards the operator's typed name + the real gateway response. Camera sees "Maya" typed → lands on "Nori · Overview". | The investor narrative's reveal beat. The user's first hire shows up as a DIFFERENT worker. Pure theater. |
| **F2** | **P0** | `/workers/<real-uuid>/overview` crashes for any newly-hired worker. Gateway returns `charter: {text, version, last_edited_at, last_edited_by}`; SPA's `WhatSheDoesCard` types and renders it as `string`. React throws `Objects are not valid as a React child`. Page falls through to error boundary 500 state. | Carried over from 2026-05-25 audit F5/F8 — unfixed. If the demo ever drills into the real hire, it crashes. |
| **F3** | **P0** | `/team` page shows hardcoded SAMPLE_TEAM (5 workers) + a banner with "935 actions/wk · 60 hours saved" instead of the gateway's real team (1 worker: Maya, 0 actions). The hire the user just made does not appear. | The "now your AI workforce is real" moment shows a fictional workforce that has nothing to do with the actions just performed. |

---

## All findings

### F1 [P0] — First-hire wizard discards user input + redirects to wrong worker

In demo mode, `useFirstHire.firstHire()` short-circuits before calling the gateway and returns one of four hard-coded sample worker UUIDs picked by `verticalId`. The operator's typed description (e.g. "A marketing intern named Maya who drafts blog posts...") is thrown away. The wizard then `router.push("/workers/<sample-uuid>")` → user lands on **"Nori · Overview"**, a sample persona with completely different stats, role description (`drafts product comms and announcements`), and personality fields (the WorkerPersonalityCard falls to DEFAULT because Nori isn't in the PERSONALITIES map either).

Worse: the real backend hire DID happen (the gateway logged `/api/v1/onboarding/first-hire` 200 during my walk and the resulting worker — Maya — is queryable at `/api/v1/team` and `/api/v1/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218/overview`). The SPA just throws away the real `worker_id` and redirects to the fake one.

**Repro**: `NEXT_PUBLIC_ARGO_DEMO_MODE=1` → `/landing` → Hire your first teammate → `/auth/signup` → fill email → Enter demo workspace → walk wizard → step 4 click "Marketing Intern" → step 5 type "A marketing intern named Maya..." → Hire them. URL goes to `/workers/018f67d0-0000-7000-8000-000000000004/overview` (the static Nori UUID) instead of the real `/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218/overview`.

**File**: `~/code/argo/web/lib/onboarding/use-first-hire.ts:120-143` (the `VERTICAL_TO_SAMPLE_WORKER` map + `buildDemoFirstHireResult` + `if (isDemoMode()) return buildDemoFirstHireResult(...)` block at L152).

**Suggested fix**: in demo mode either (a) actually call the gateway (auth-bypass + demo-mint already provides a verified principal — the gateway hire DOES work in this env), then redirect to the REAL worker_id; OR (b) keep the sample-UUID shortcut but parse the user's name out of `req.description` and stash it for `WorkerPersonalityCard` to use. Per the operator's "no theater in the demo" rule (memory `feedback_no_theater_in_demo`), option (a) is the right answer.

### F2 [P0] — `/workers/<real-uuid>/overview` crashes on real worker (charter shape mismatch)

Same root cause flagged in the 2026-05-25 audit F5/F8. Still unfixed. Gateway response:

```
GET /api/v1/workers/<id>/overview → 200
{ "charter": {"text": "Drafts blog posts...", "version": 1,
              "last_edited_at": "...", "last_edited_by": "system:hire-endpoint"},
  "duties": [], "kpis": [...], "permissions": {...}, ... }
```

SPA contract: `web/lib/api/worker-overview.ts:62` declares `charter: string;` and `WorkerOverviewContent.tsx:144` passes it directly to `<WhatSheDoesCard charter={overview.charter}>` which embeds it inside a `<p>`. React: `Objects are not valid as a React child (found: object with keys {last_edited_at, last_edited_by, text, version})`. Error boundary catches, shows the page-wide "We couldn't load this worker. 500" state.

**Repro**: hire any real worker (or pre-seed one), then visit `/workers/<uuid>/overview`. The page header card renders the name + tagline (because the `useWorker` hook only reads top-level fields), then the body throws.

**File**: contract mismatch between `crates/argo-cli/src/handlers/gateway/routes_workers_overview.rs` (returns charter as an object) and `~/code/argo/web/lib/api/worker-overview.ts:62` (declares `charter: string`).

**Suggested fix**: pick one. Easiest = SPA-side: change `WorkerOverview.charter` to the object shape and pass `.text` (with a fallback for empty/null) into `WhatSheDoesCard`. The version + last_edited_* fields are useful in the "Adjust" affordance already on the page.

### F3 [P0] — `/team` page shows fixture (5 workers, fake stats) instead of real team

After the demo-mint signup + hire, `/api/v1/team` returns `{total_count: 1, workers: [{name: "Maya", ...}]}`. But `/team` in the SPA renders 5 sample workers (Mariah, Peta, Priya, Jamal, Nori) plus a hero strip "5 teammates, one team · 0 working right now · 935 actions this week · saving you ~60 hours." Maya doesn't appear at all. The hire that just succeeded is invisible.

In demo / auth-bypass mode the team page appears to be hard-falling-back to the SAMPLE_TEAM fixture rather than reading the real gateway data. Likely related to the `isAuthBypassed()` branches in `web/lib/api/workers.ts:409` (the demo-fallback used by the `/hire` wizard) but the `/team` data path also seems to ignore real gateway responses in this env.

**Repro**: walk the full 5-step wizard in `NEXT_PUBLIC_ARGO_DEMO_MODE=1`, then click "Team" in the sidebar. Compare against `fetch('/api/v1/team')` from the console — gateway returns Maya only.

**Suggested fix**: in demo-mode, splice the real gateway team (Maya) onto the SAMPLE_TEAM fixture so the just-hired worker actually appears. Drop the hardcoded `935 actions / 60 hours` and compute from actual data. Per memory `feedback_no_theater_in_demo` — these aggregate KPIs are camera-visible function-side, not decorative dressing.

### F4 [P1] — Approvals dashboard "Hire teammate" button skips steps 1–4

On `/` (Approvals dashboard, the default landing page in demo mode), the top-right "Hire teammate" button calls `router.push("/onboarding/first-hire")` — i.e. straight to step 5. The wizard's progress bar then shows "STEP 05 OF 05" but the prior 4 steps (Welcome / Workspace / Provider / Vertical) have never been completed, so `useOnboardingDraft` has `verticalId: null` → the submit returns "Pick a template first" without ever rendering an error visible to the user, and the wizard stalls.

**Repro**: from `/`, click the purple "Hire teammate" button. URL is `/onboarding/first-hire` with the wizard progressbar mid-flow. Type a prompt and click Hire — nothing happens (silent fail).

**File**: `~/code/argo/web/app/(app)/page.tsx:459`.

**Suggested fix**: change `router.push("/onboarding/first-hire")` to `router.push("/onboarding/welcome")` (or `/hire` if the intent is to use the standalone hire wizard, not the onboarding one). The onboarding wizard is for the FIRST hire only — buttons that surface from the dashboard later should go to `/hire`, not `/onboarding/first-hire`.

### F5 [P1] — WorkerPersonalityCard PERSONALITIES map is hardcoded by NAME — anyone outside (Mariah/Priya/Jamal) gets default

`web/components/skycot/worker/WorkerPersonalityCard.tsx:35` declares a static `PERSONALITIES` record keyed by worker name. Only Mariah / Priya / Jamal are listed. Peta and Nori (both in the SAMPLE_TEAM fixture) fall to DEFAULT_PERSONALITY (`Reliable / Curious / Helpful`). Maya (real hire) also falls to default. So the C2-personality investor moment — "your AI teammate has a real personality" — is theatre for 3 of 5 sample workers and ALL real hires.

The brief at the top of the file says "v1 seeds traits from a static personality map keyed by worker name. The character/charter system on the gateway will surface these in a follow-up brief." That brief hasn't shipped.

**File**: `~/code/argo/web/components/skycot/worker/WorkerPersonalityCard.tsx:35-111`.

**Suggested fix**: at minimum, add Peta + Nori to PERSONALITIES (matches the 5-card team page). Long-term: surface personality from the gateway's `/workers/<id>` response (the runtime charter system already carries persona + signature behaviors; the SPA just doesn't read them).

### F6 [P1] — Workspace step succeeds without calling backend in demo mode, but `setProvision` still stores nulls

`workspace/page.tsx:45` calls `provision.mutateAsync(...)` which on demo-mode short-circuits (no real gateway call), and `setProvision(res.tenantId, res.workspaceId)` runs with stub values. The stored `tenantId` is the synthetic demo-tenant from the demo-mint cookie, not the real backend tenant. This isn't visibly broken in the demo path because nothing else relies on the real tenant, but it means the wizard claims "workspace created" while the backend has no record of it (sessionStorage shows `skycot:demo-onboarding-tenant-id` is populated but it's a synthetic principal id, not a real tenant row).

**Repro**: complete step 2; check `sessionStorage.getItem('skycot:demo-onboarding-tenant-id')` — it's the demo-mint id. No corresponding tenant row exists in the gateway's `tenants` table.

**Suggested fix**: this is acceptable for the demo path AS LONG AS step 5 (first-hire) uses the same demo-mint principal. Currently F1 confirms it does NOT — the demo first-hire short-circuits the gateway entirely. Once F1 is fixed (gateway is actually called), this becomes load-bearing: the workspace step must provision a real tenant. Until then it's harmless theater.

### F7 [P1] — Onboarding wizard progressbar reads "STEP 05 OF 0 5" (split number)

Accessibility snapshot shows the progressbar accessible name as `"Hire wizard — step 5 of 5"` (correct) but the visible glyph reads "STEP 05 OF 0 5" with a visible space between the "0" and "5" in "0[ ]5". This is a layout/typography issue in `WizardEditorialShell` — the "OF" separator inserts a non-breaking element that splits the final "5". Camera will catch this on the first frame of step 5.

**Repro**: visit any step; the progressbar header reads e.g. "STEP 03 OF 0 5".

**Suggested fix**: trace through `components/auth/WizardEditorialShell.tsx`'s step indicator JSX — the "OF" label likely sits in a flex container that's force-wrapping the "5".

### F8 [P1] — Step-3 connect-provider Skip button writes `providerSkipped=true` but the demo banner already says "Skip and use mock provider for now" — UX gives both options simultaneously

The page sub-headline says "Demo mode — workers run with a **mock provider** (simulated responses). Type any sk-ant- key to continue; no Anthropic call is made." Below the input is an inline warning: "Mock provider — workers run but responses are simulated. Skipping uses a mock provider until you add a real key in Settings · Providers." Then TWO buttons: "Skip for now" + "Validate & continue". In demo mode both paths land you on a mock provider. The distinction is invisible to the user but the UI dramatises a choice.

**File**: `~/code/argo/web/app/(setup)/onboarding/connect-provider/page.tsx:128-239`.

**Suggested fix**: in demo mode, collapse to a single "Continue with mock provider" CTA. Hide the input + Skip button. The validate path also still tries to hit the gateway in demo (will 4xx the typed key) — confusing.

### F9 [P2] — Step-4 vertical-pick "Skill count · DEMO MODE · INSTALL DEFERRED" chip is honest but visually busy

Each of the 7 vertical cards in `/onboarding/pick-vertical` ends with `8 SKILLS · [yellow chip] DEMO MODE · INSTALL DEFERRED`. Honest, in keeping with `feedback_no_theater_in_demo`, but visually heavy — adds ~30% to each card's vertical height. On a 7-card grid the page becomes scroll-heavy.

**Suggested fix**: move the "demo mode · install deferred" chip out of every card into a single info-banner at the top of the page (it's the same message 7 times anyway).

### F10 [P2] — `/landing` carries "V1.0 SHIPPED · HIRING OUR FIRST 100 TEAMS" + "2,847 COMPANIES THIS MONTH" + fictitious testimonials

Per the function-vs-dressing rule, `/landing` marketing copy is dressing-side. These claims are fictitious but the camera doesn't drill in. Flagging because the language is louder than is appropriate for a POC investor demo:

- Header pill: "V1.0 SHIPPED · HIRING OUR FIRST 100 TEAMS" (you haven't, this is the POC demo)
- Footer: "2,847 COMPANIES THIS MONTH" (also on every onboarding step's footer)
- 5 testimonial customer logos (Northwind Commerce, Beacon Health Ops, Maple Finance, Apex Recruiting, Skyline Engineering) — all fictional
- 3 testimonials with named "VPs" and "Founders"

Operator-direction call: if the demo opens on `/landing`, soften the claims; if it opens on the dashboard, leave alone. Per the runbook (`docs/skycot/demos/v1-mission-mvp-runbook.md`), Scene 1 opens on `/workers` — so `/landing` isn't on the camera. Keeping at P2.

### F11 [P2] — `useTeam` returns sample data in demo mode but the hero claims "LIVE" pill

`/team` hero banner shows a green `· LIVE` indicator next to the "5 teammates, one team" headline. The data underneath is the SAMPLE_TEAM fixture, not live. The LIVE pill is theatre when the data is fixture.

**Suggested fix**: hide the LIVE pill in `isDemoMode()` OR (better, paired with F3 fix) actually show the real team.

### F12 [P2] — `WorkerLayoutShell` "Team" breadcrumb routes back to `/team` (sample fixture) after onboarding

After the wizard hands off to `/workers/<id>`, the breadcrumb says "Team > Nori" and clicking "Team" lands on the SAMPLE_TEAM page (5 fictional workers). The just-hired Maya isn't there. So the metaphor "you just hired a teammate, now meet your team" never resolves — clicking around the dashboard surfaces the sample roster, not the real one.

This is a consequence of F3 (team page shows fixture). Once F3 is fixed, this resolves automatically.

### F13 [P3] — Sub-copy "$0.001 test call confirms the key works" on connect-provider step is potentially confusing in demo mode

Step 3's eyebrow + sub-copy: "Connect Anthropic. A $0.001 test call confirms the key works — or skip and use a mock provider for now." In demo mode no real Anthropic call happens. The "$0.001" is a precise number that's mathematically a real charge against Anthropic credits — in demo mode users may worry their card will be billed.

**Suggested fix**: in demo mode, change the sub-copy to "Demo mode — no real Anthropic call. Use any sk-ant- key or skip." (Already partially handled in code at L266 of pick-vertical — same pattern would work here.)

---

## Resolved since prior audit (2026-05-25 11:00 AEST)

- The signup → demo-mint → onboarding handoff is clean. Cookie + principal mint round-trip works.
- `/onboarding/welcome` → `/workspace` → `/connect-provider` → `/pick-vertical` → `/first-hire` chain navigates without auth bounces or page reloads.
- F1 design lock holds across all 5 wizard pages.
- The `(setup)/onboarding/page.tsx` `redirect("/onboarding/welcome")` index page exists — bare `/onboarding` no longer 404s.

## Still unfixed from prior audit

- F5/F8 (charter shape mismatch crashing real worker overview) — my F2.
- Hero KPI hardcoding on `/team` — my F3 / variant of prior F4.
- WorkerPersonalityCard hardcoded by name — my F5 / variant of prior.
- Hire wizard fabricating sample worker UUIDs — my F1 (was less prominent in prior audit but is the load-bearing investor-narrative bug today).

---

## Orientation score: 64/100

- The wizard mechanics (navigation, form state, persisted draft, demo handoff) are clean: +20.
- The locked F1 design ships consistently: +15.
- The metaphor language ("Hire a teammate", "Onboard them like a human", "Describe the job") lands: +15.
- The function-side hire DOES work end-to-end against the gateway: +14.
- The wizard discards the real result and shows a fixture, breaking the investor reveal: -15.
- Real worker detail crashes the page: -10.
- Team roster shows fixture instead of the just-hired worker: -10.
- Personality data hardcoded by name → most real hires get default: -5.

Brought to ship-readiness with: (1) fix `useFirstHire` to return the real gateway worker_id in demo mode, (2) fix the `charter` contract (object vs string), (3) splice real team onto SAMPLE_TEAM so the new hire appears. Three files; ~150 LOC total estimated. After those: this becomes the strongest 5-minute investor moment in the product.
