# Onboarding hire-flow UX audit — "Hire an AI teammate" investor narrative

**Date**: 2026-05-28 04:38 (Melbourne local)
**Auditor**: e2e-ux-journey-test-agent
**Scope**: `/landing` → `/auth/signup` → `/auth/verify` → `/auth/signin` → 5-step onboarding wizard → `/workers` → `/workers/{id}`
**Mode**: read-only against running SPA on `http://localhost:3000` + gateway on `http://127.0.0.1:7400` (schema v83); preview ran on auto-port 60628 with `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`, `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`, `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1`.

## TL;DR

The hire-flow narrative is the strongest single asset Skycot has shipped — `/landing` lands a *visceral* "you're meeting Mariah, Peta, Priya, Jamal and Nori" promise in the first viewport, the wizard's "Onboard them like a human" pillar nails the differentiator, and the demo-mint path skips the SMTP round-trip without faking the auth (a real gateway JWT comes back). **Function-side, every URL in the journey returns a real 200 page and the demo-mint endpoint mints a real principal**. But there are 3 P1s and a cluster of P2s where the *promise* of the narrative outruns what the wizard does behind the scenes — in particular the first-hire prompt is captured-then-discarded in demo mode, the description text never reaches the gateway, and the post-hire detail page can only render its "C2 personality" card for the 5 hardcoded names. The journey is recordable for the POC/investor demo as long as the narrator uses one of the 5 canon names (Mariah etc.) at Scene 3.

- **0 P0 demo-blockers** — the 6-scene runbook is recordable as-scripted.
- **3 P1** — visible if the camera deviates from canonical names or zooms into the wizard's `/team` redirect quirk.
- **8 P2** — polish items the next iteration should pick up.
- **4 P3** — nits worth flagging once but not gating anything.

**Function-side reality** (real, no theater): gateway `/api/v1/demo/mint-principal` mints a real `human:<email>` JWT, gateway `/api/v1/workers` returns 5 seeded workers (Mariah/Peta/Priya/Jamal/Nori with real Marketplace/Project-Ops/Recruiting/Support/Marketing taglines), gateway `/api/v1/onboarding/first-hire` is wired and rejects unauthorised callers correctly. **Dressing-side acceptable** (fixtures, fine per `feedback_no_theater_in_demo`): personality traits ("Patient · Detail-oriented · Picture-perfect listings · Polite"), trait counts, the 7-vertical fallback list, customer logos and the landing chat-bubbles.

## Headline blockers

None. The journey is shippable as a POC/investor demo.

The closest thing to a blocker is **F-03** (demo-mode discards the first-hire prompt text and routes to a canonical fixture worker by vertical) — but that's load-bearing in *real* signed-in mode, not demo mode, and the operator can sidestep it by describing one of the 5 canon names ("a marketplace lister named Mariah…") at Scene 3.

## Findings

### F-01 [P1] — Two parallel worker rosters with non-overlapping UUIDs

**What I saw**: gateway `/api/v1/workers` returns 5 workers with UUIDs of the form `019e6785-928*-*-*-*` (Mariah, Peta, Priya, Jamal, Nori). The SPA's demo-mode wizard hard-codes a separate set of UUIDs `018f67d0-0000-7000-8000-00000000000{1..4,7}` in `web/lib/onboarding/use-first-hire.ts:109-128` and `web/lib/sample-workers-detail.ts:32-93`. When `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` is on (preview environment), `fetchTeam()` short-circuits the gateway and serves only the SAMPLE_TEAM fixtures — the gateway-real workers are never shown. When bypass is off (production-shaped path) and the user has a verified principal, the gateway roster IS shown and the SAMPLE fixtures aren't.

This is fine for POC because the v1 demo MP4 will run against ONE configuration, but it means: (a) screenshots and developer testing in bypass mode show a different roster than a real signed-in user, (b) anyone clicking a fixture-UUID link won't find that worker in the gateway, (c) the gateway's seeded Mariah and the SPA's fixture Mariah are technically different entities with different IDs.

- **Repro**: with bypass on, `curl http://localhost:60628/api/v1/workers` (proxied) returns fixtures; without bypass, `curl -H 'Authorization: Bearer <demo-mint-jwt>' http://127.0.0.1:7400/api/v1/workers` returns the gateway 5.
- **Files**: `~/code/argo/web/lib/api/team.ts:85-135`, `~/code/argo/web/lib/sample-workers-detail.ts:30`, `~/code/argo/web/lib/onboarding/use-first-hire.ts:109-128`.
- **Suggested fix**: align the two UUID spaces — either change the SPA fixtures to use the gateway-seeded UUIDs (post-`argo demo seed`), or have the bypass path *call the gateway with a synthetic principal* instead of returning fixtures.

### F-02 [P1] — `/workers` empty-state redirects to `/team`, splitting the surface in half

**What I saw**: `/workers` (nav-bar entry) renders the `HireStarterPrompts` empty-state if the tenant has zero workers, otherwise *redirects* to `/team` (`web/app/(app)/workers/page.tsx:24-27`). So the runbook's Scene 1 narration "click /workers" actually lands on `/team` in the URL bar — which is fine if the demo doesn't pan the address field, but the `WorkersPage` source comment itself flags this as a stop-gap: *"A dedicated workers list with column-level filters is a separate Track-E brief; this page only owns the empty-state experience until then."* This is a known intent debt acknowledged in code.

For the investor narrative, having `/workers` and `/team` be two URLs for one experience adds avoidable URL confusion (and the Sign-In returnTo path could land a buyer on the wrong one).

- **Repro**: `curl -L http://localhost:3000/workers` (with session) → redirects to `/team`; `curl http://localhost:3000/team` is the canonical 5-card grid.
- **Files**: `~/code/argo/web/app/(app)/workers/page.tsx:9-46`.
- **Suggested fix**: pick one URL (`/team` is the dominant one per source comment) and make the other a server-side `redirect()` rather than a useEffect bounce.

### F-03 [P1] — In demo mode the first-hire prompt is captured then discarded; user is routed to a fixture worker keyed by vertical, not by the text they typed

**What I saw**: `~/code/argo/web/lib/onboarding/use-first-hire.ts:130-144` defines `buildDemoFirstHireResult()` which, in demo mode, returns one of `{SAMPLE_MARIAH, SAMPLE_PRIYA, SAMPLE_JAMAL, SAMPLE_NORI}` *based on the chosen vertical*, ignoring the operator's typed prompt entirely. The default placeholder text on the page is `"A marketing intern named Maya who drafts blog posts and runs weekly competitive scans."` — if the operator types that, they don't end up at a worker named Maya, they end up at `/workers/018f67d0-0000-7000-8000-000000000004` (Nori). The detail page then shows Nori, NOT Maya.

In real (non-demo, signed-in) mode the code does call `POST /api/v1/onboarding/first-hire` with `description` and `vertical_id` and verifies the returned worker_id is readable. So the wire is there; demo mode just bypasses it.

This is the single most narrative-breaking moment in the journey: the operator believes they're "hiring Maya" and the next page shows "Nori". For the POC/investor demo this is recoverable by having the narrator type *"a marketplace lister named Mariah…"* (selects marketplace-seller vertical → routes to SAMPLE_MARIAH → page shows Mariah). For any non-canon name the illusion breaks.

- **Repro**: with bypass on, walk welcome → workspace → connect-provider → pick-vertical (choose "marketing-intern") → first-hire (type `"a content lead named Bex"`) → submit. Lands on `/workers/018f67d0-0000-7000-8000-000000000004` which renders as Nori.
- **Files**: `~/code/argo/web/lib/onboarding/use-first-hire.ts:103-144`.
- **Suggested fix**: the demo-mode short-circuit comment says it exists because the gateway needs a *verified principal AND real tenant id*. Both ARE available when the user came through `/api/auth/demo-mint` (verified `human:<email>` JWT + a real gateway tenant). Drop the demo short-circuit and let the wizard hit the gateway endpoint — the post-hire readback already has a fallback that resolves by name (`findWorkerByName`) so even a "ghost worker" race recovers.

### F-04 [P2] — Personality card falls back to generic copy for any non-canon name

**What I saw**: `WorkerPersonalityCard` (`~/code/argo/web/components/skycot/worker/WorkerPersonalityCard.tsx:35-111`) hard-codes traits / great-at / escalate-when / signature for *exactly* {Mariah, Peta, Priya, Jamal, Nori}. Any other worker name (e.g. "Maya", "Alex", "Sam" — the kind of name a real customer would type) renders `DEFAULT_PERSONALITY`: "Working through the responsibilities you set / Asking when something is ambiguous / Surfacing patterns across recent runs / Always confirms before a public or irreversible action." Same boilerplate for every worker.

The source comment at line 23 is honest: *"V1 seeds traits from a static personality map keyed by worker name. The character/charter system on the gateway will surface these in a follow-up brief; the data shape here matches the future API contract."* So this is acknowledged tech debt — it ships now to keep the C2 design viable for 5 canon names.

- **Files**: `~/code/argo/web/components/skycot/worker/WorkerPersonalityCard.tsx:35-111`.
- **Suggested fix**: as the source comment says — pull personality from a gateway endpoint (`/api/v1/workers/{id}/personality` currently 404s) keyed off the worker's charter, not a string-match on name. P2 because it's invisible during the v1.0 demo MP4 (camera only meets canon names).

### F-05 [P2] — Onboarding wizard's vertical-catalogue fallback (7 items) doesn't match the gateway's catalogue (1 item)

**What I saw**: gateway `/api/v1/verticals` returns `{verticals: [{id: "marketplace", name: "Marketplace", workflow_count: 1}]}` — exactly ONE installed vertical. The SPA's wizard at `pick-vertical` defines `FALLBACK_VERTICALS` with 7 cards (`gtm-engineer`, `marketing-intern`, `marketplace-seller`, `recruiter`, `sales-sdr`, `customer-support`, `finance-ar`). When `useVerticals()` returns an empty array, the wizard renders the fallback grid (`web/app/(setup)/onboarding/pick-vertical/page.tsx:210-215`). When gateway returns >= 1 row, it switches to gateway-only.

In demo mode the wizard fetches the catalogue but short-circuits the `install-vertical` POST per lines 228-233 ("Demo-mode short-circuit: install-vertical requires a verified principal. Skip the gateway and advance straight to /first-hire") — so the choice between gateway and fallback is purely visual.

For the demo MP4, the runbook chooses Marketplace, which IS in the gateway catalogue — so the buyer sees a real catalogue source label. But if the camera shows 7 cards (fallback rendered) instead of 1 (gateway rendered), the narrator should not say "the gateway returned these 7 templates."

- **Files**: `~/code/argo/web/app/(setup)/onboarding/pick-vertical/page.tsx:108-158, 210-215`.
- **Suggested fix**: either (a) seed the gateway with the other 6 vertical packs so the catalogue is real, or (b) drop the 6 non-`marketplace` fallback entries (or label them "available in v1.1") so the visible card count matches the gateway truth.

### F-06 [P2] — Demo-mode banner on `pick-vertical` says "install deferred" while the wizard subtitle still claims to "install the real skills, memory schema, and approval gates"

**What I saw**: the wizard subtitle (`web/app/(setup)/onboarding/pick-vertical/page.tsx:266-279`) reads `"Demo mode — pick a starting template. We'll install the real skills, memory schema, and approval gates once you create a real workspace."` while the inline banner immediately below says `"Demo mode — install deferred. Picking a vertical loads its workflows and skills as samples; nothing is provisioned to the gateway until you connect a real provider in Settings."`

Both are true but they contradict each other. "We install" vs "nothing is provisioned to the gateway" puts the buyer in two minds about whether this *is* their workspace or *will be later*.

- **Files**: `~/code/argo/web/app/(setup)/onboarding/pick-vertical/page.tsx:266-322`.
- **Suggested fix**: pick one framing. The most honest is "this is a tour — your real workspace comes after sign-in." The most aspirational is "we'll install for real once you connect a provider."

### F-07 [P2] — `/onboarding/welcome` Genesis aside ("Have a stack of role docs to upload?") points to `/genesis` which isn't part of the locked F1 wizard

**What I saw**: `web/app/(setup)/onboarding/welcome/page.tsx:77-88` has a one-line aside under the Get-started CTA: *"Have a stack of role docs to upload? Try Genesis studio →"* linking to `/genesis?return=/onboarding/welcome`. Per memory `feedback_genesis_is_ai_teammate_onboarding_studio` (2026-05-25T11:40Z), Genesis is the *real* future onboarding studio (voice/text role + SOP uploads + access-controls + comprehension verification), and Phase 2 multi-modal onboarding is pending operator alignment.

For the v1.0 demo MP4, the camera lands on welcome → describes pillars → clicks Get started. If the camera pans to the Genesis aside the narrator has to explain what Genesis IS and isn't, which adds 30-60 seconds of explanation for a path the demo doesn't take.

- **Files**: `~/code/argo/web/app/(setup)/onboarding/welcome/page.tsx:76-88`.
- **Suggested fix**: keep the aside but de-emphasise it visually for the demo recording (smaller type, fainter colour) so the camera flows past it; or temporarily hide behind `NEXT_PUBLIC_SHOW_GENESIS_TEASER=1` during recording sessions.

### F-08 [P2] — Onboarding step 2 (workspace) celebrates with a toast then auto-pushes to step 3 1.2 seconds later

**What I saw**: `web/app/(setup)/onboarding/workspace/page.tsx:16-49` defines `CELEBRATION_HOLD_MS = 1200` and uses `setTimeout` to push to `/onboarding/connect-provider` after the success toast. The toast says "Workspace created — welcome to Skycot." If the user is hovering the Continue button when the toast fires, the page navigates underneath them — which mid-click can trigger a wrong-page action on the destination, or feel jarring on a slow connection.

This is the kind of micro-pacing thing that's invisible at investor-demo cadence (1 step every 8 seconds) but feels off in real onboarding. The 1.2s celebration hold is also a magic number with no escape — no "skip" affordance for returning users.

- **Files**: `~/code/argo/web/app/(setup)/onboarding/workspace/page.tsx:16, 47-49`.
- **Suggested fix**: shorten to 600 ms OR keep the toast and let the user advance manually via the Continue button (the toast itself doesn't need to gate navigation).

### F-09 [P2] — `connect-provider` lets you skip with a "mock provider" CTA that has the same visual weight as "Validate & continue" once you've typed a key

**What I saw**: `web/app/(setup)/onboarding/connect-provider/page.tsx:218-249` renders both "Skip for now" and "Validate & continue" as same-weight pills, with their classes swapping (`ob-cta` vs `ob-cta-ghost`) based on whether the last validation error preferred skipping. So an operator who typed a wrong key gets a *more prominent* Skip button than Continue. That's defensible UX (let them escape) but during the demo it could read as Skycot pushing customers to fake mode.

- **Files**: `~/code/argo/web/app/(setup)/onboarding/connect-provider/page.tsx:218-249`.
- **Suggested fix**: keep the swap but make the difference subtler — outline-only Skip vs filled Continue, regardless of error state. The mock-provider warning callout is already prominent enough to signal what Skip does.

### F-10 [P2] — `first-hire` example prompt has a name ("Maya") that the system cannot honour

**What I saw**: `web/app/(setup)/onboarding/first-hire/page.tsx:15-16` shows `EXAMPLE_PROMPT = "A marketing intern named Maya who drafts blog posts and runs weekly competitive scans."` as both the placeholder and the hint. But per F-03, demo mode discards the name. So the example is teaching the user a behaviour the demo can't fulfil. In real (non-demo) mode the gateway endpoint accepts the description but the post-hire personality card (per F-04) won't render Maya's traits — it'll show DEFAULT_PERSONALITY.

For the investor demo, the narrator typing "Maya" → landing on Nori is a credibility scratch.

- **Files**: `~/code/argo/web/app/(setup)/onboarding/first-hire/page.tsx:15-16`.
- **Suggested fix**: change the example to use a canon name in demo mode — e.g. "A marketplace lister named Mariah who lists items and re-prices weekly." This way the placeholder, the system response, and the camera all agree.

### F-11 [P2] — `/landing` Hire CTA in the preview overlapped by the Next.js dev-tools floating button

**What I saw**: in the desktop screenshot, a black circular "N" badge (Next.js dev-tools toggle, position fixed bottom-left) is visually overlapping the lower-left corner of the "Hire your first teammate" gradient pill CTA. This is a dev-only artifact (production builds don't ship the toggle), but if the demo is being recorded against `next dev --turbopack` rather than `next build && next start`, the toggle WILL appear.

- **Repro**: visit `/landing` on port 60628 (turbopack dev), inspect bottom-left.
- **Files**: not a code issue — recording-environment checklist item.
- **Suggested fix**: record demo against `next start` (production build) OR add a documented step to disable dev-tools before recording.

### F-12 [P3] — Demo-mode banner on `/auth/signup` is hidden because `NEXT_PUBLIC_ARGO_DEMO_MODE !== "1"` in the launch.json profile

**What I saw**: `launch.json` sets `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`, `NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:7400`, `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`, `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1` — but NOT `NEXT_PUBLIC_ARGO_DEMO_MODE=1` (note the slightly different env var). The signup page checks `process.env.NEXT_PUBLIC_ARGO_DEMO_MODE === "1"` (`web/app/auth/signup/page.tsx:32-33`) for the full demo-mode UI, and falls through to the secondary "Or try a demo workspace · no email required" dashed-outline button via `shouldShowDemoMintButton()` (which keys off `NEXT_PUBLIC_ENV !== "production"`).

So the preview surfaces the secondary CTA but not the primary one. That's fine — the secondary path mints a real principal — but the env-var naming inconsistency (`SKYCOT_DEMO_MODE` vs `ARGO_DEMO_MODE`) is a footgun for someone trying to record the "primary" demo path.

- **Files**: `~/code/argo/.claude/launch.json`, `~/code/argo/web/app/auth/signup/page.tsx:32-33`.
- **Suggested fix**: unify on `NEXT_PUBLIC_SKYCOT_DEMO_MODE` (the new name) and treat the old `NEXT_PUBLIC_ARGO_DEMO_MODE` as an alias during the rename window.

### F-13 [P3] — `/auth/verify` only triggers on a `?token=` query param; arriving at the URL bare lands you on the "missing or malformed magic-link token" error

**What I saw**: `web/app/auth/verify/page.tsx` reads the token from `useSearchParams()`. If the user navigates to `/auth/verify` with no token (e.g. typing it manually or after a stale cached redirect), the page renders the "missing or malformed" error state. The error state is actually nicely designed (eyebrow + hero "That link / didn't work" + "Get a new link" retry CTA), so this is a soft P3, but it's a slightly user-hostile dead-end for anyone exploring URLs.

- **Files**: `~/code/argo/web/app/auth/verify/page.tsx:44-46, 105-128`.
- **Suggested fix**: detect "no token at all" vs "token present but invalid" and show distinct copy (e.g. "Looking for your sign-in link? Check your inbox or request a new one.").

### F-14 [P3] — Gateway has a 5-worker seeded roster that includes Peta, but the SPA's `VERTICAL_TO_SAMPLE_WORKER` map doesn't route to Peta from any vertical

**What I saw**: gateway returns Mariah / Peta / Priya / Jamal / Nori for verticals marketplace / project-coordination / recruiting / support / marketing. The SPA's demo-mode first-hire mapping (`web/lib/onboarding/use-first-hire.ts:121-126`) covers Mariah / Priya / Jamal / Nori but NOT Peta — so if the operator picks a "project coordination" vertical (gateway has it; the SPA fallback list does not), the demo short-circuit falls through to DEFAULT_FALLBACK = Mariah. The first-hire experience can never land on Peta in demo mode.

- **Files**: `~/code/argo/web/lib/onboarding/use-first-hire.ts:121-128`.
- **Suggested fix**: add `"project-coordination": { id: SAMPLE_PETA, name: "Peta" }` to the map (and surface a constant for SAMPLE_PETA — currently the file only knows 4 sample UUIDs).

### F-15 [P3] — Preview-environment theme stickiness can leave the page rendered in `skycot-dark` even after `colorScheme=light`

**What I saw**: when the preview opened `/customers` for the first time, the document HTML had `class="... skycot-dark"` and `data-theme` mismatch even though the colour-scheme media feature was set to `light`. After explicit `preview_resize colorScheme=light`, subsequent pages rendered correctly in light mode. This is preview-environment-specific (the dev tools store a theme preference in localStorage) and not a user-visible bug, but it's a noise source for anyone scripting screenshots.

- **Suggested fix**: documented checklist item for demo recording — "open `/auth/signin` first, system-theme matches OS, no manual override".

## What's working — strong assets to keep

- **Landing hero**: "Don't buy another tool. Hire a teammate. (Seriously.)" with the Audiowide-gradient "teammate" is the strongest single brand moment on the site. Pair it with the chat-bubble preview of Mariah/Peta/Priya/Jamal/Nori on the right and you have the entire investor narrative in one viewport.
- **`/landing` chat bubbles match real worker fixtures**: "Mariah: 3 listings live, 2 buyers waiting on you" matches `sample-workers-detail.ts:41` exactly. The detail page Scene 1 lands on will say the same numbers — no cross-surface contradiction.
- **demo-mint flow is real**: `/api/auth/demo-mint` → gateway `/api/v1/demo/mint-principal` → real `human:<email>` JWT → `__Host-session` cookie that downstream gateway endpoints actually verify. No theater.
- **Wizard step labels + microcopy**: "Onboard them like a human", "Workers live in workspaces. You can rename them later", "Plain English is fine. Give them a name, a role, and what they should do this week" — all on-voice and warm without being twee.
- **Error states are designed**: `/auth/verify` error, `connect-provider` mock-provider warning, `first-hire` skip-fallback messaging — all show real care. Compare to the "something went wrong" black-hole patterns most SaaS apps ship with.
- **Worker detail (C2 personality) for canon names is genuinely distinctive**: the "Patient · Detail-oriented · Picture-perfect listings · Polite" trait chips for Mariah read like a LinkedIn profile, not a tool config screen. This will land with investors.

## Answers to the original critical questions

- **Does the hired worker actually exist in the gateway after the flow completes?** YES in real signed-in mode (gateway `/api/v1/onboarding/first-hire` is wired). NO in demo mode (short-circuit returns one of 4 fixture UUIDs, none of which exist in the gateway). The preview environment with bypass=1 falls into the second bucket.
- **Does the worker get a real charter / persona / memory schema, or is it a fixture/mock?** Real in signed-in mode (per `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs`). Fixture in demo mode — the personality card is keyed by name against a hardcoded map of 5 canon characters.
- **Does the flow FEEL like onboarding a human (warm, narrative, distinctive) or like filling out a form?** Warm and narrative — the microcopy across all 5 steps maintains the "teammate not tool" register. The C2 personality card for canon names lands the *human* metaphor.
- **Does the verify magic-link work? Does signin work? Any dead-ends or 500s?** Verify route accepts POST, classifies error codes properly, renders editorial error state. Signin discovers SSO and falls through to magic-link. No 500s observed in the journey. The middleware redirect for unauth users to `/auth/signin?from=…` works correctly. Only dead-end: `/auth/verify` with no token (P3 finding F-13).
- **Does the new worker appear in /workers? Is the detail page populated with real personality data?** YES if it's one of the 5 canon names. The detail page is populated with the *hardcoded* personality fixture, not gateway-sourced character data. The gateway endpoints for `/personality` and `/schedule` currently 404.

## Orientation score: 78/100

The hire-flow narrative is the closest the SPA gets to "the investor demo writes itself" — top-of-funnel through detail-page is recordable as a single cohesive story, the function-side reality is honest (demo-mint mints real, gateway has real workers, audit chain is real), and the wizard's 5 steps each land their pillar without padding. The 22-point deduction is mostly **F-03** (the wizard's signature moment — "describe your first hire in plain English" — discards the name in the path the operator will actually use) plus **F-01/F-02** (the two-rosters / two-URLs split that will eventually bite once a real customer notices). Fix F-03 alone and the score goes to ~88; the others are post-MVP polish.

## Methodology notes

- Preview MCP renderer hung after ~6 navigations (consistent with fast-refresh storm on first compile of each route); fell back to source-read + curl probes for the back half of the journey. Findings F-04 through F-15 derive from source-read rather than visual capture.
- `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` in the launch.json profile means the preview server never hits the gateway for `/api/v1/workers`, `/api/v1/team`, or `/api/v1/onboarding/*` — fixtures are served instead. Findings about the gateway are from direct `curl http://127.0.0.1:7400/api/v1/*` probes.
- No code edits, no cargo runs, no git mutations.
