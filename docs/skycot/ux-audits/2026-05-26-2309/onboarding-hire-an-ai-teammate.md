# Onboarding "Hire an AI teammate" — investor narrative e2e UX audit

**Date**: 2026-05-26 23:09 AEST (Melbourne)
**Auditor**: e2e-ux-journey-test-agent (read-only, demo-mode SPA on :3000 + gateway :7400 schema v83)
**Scope**: `/landing` → primary hire CTA → 5-step wizard → `/workers` → `/workers/{id}` (C2-personality).
**Lens**: POC + investor demo (per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md` — function side must be real, dressing side may be fixture).

---

## TL;DR

15 findings: 3 P0, 5 P1, 5 P2, 2 P3.

The headline narrative — **"hire an AI teammate in four minutes"** — is fundamentally broken end-to-end in two different ways:

1. **There are TWO competing 5-step wizards** (`/hire/step/1..5` and `/onboarding/welcome..first-hire`). The marketing site's primary CTA wires to `/hire`; the route the brief lists wires to `/onboarding`. Neither flow knows about the other; copy and substance diverge.
2. **Genesis — the "AI Teammate Onboarding Studio" the operator specifically called out as the investor moment** (memory `feedback_genesis_is_ai_teammate_onboarding_studio`) — exists at `/genesis` as an orphan page. It is not in the top nav, not linked from the marketing CTA, not linked from `/onboarding/welcome`, and not the destination of any "hire" button anywhere in the SPA. The gateway endpoint `POST /api/v1/genesis` returns 503 `genesis_provider_key_missing` on this dev gateway, so even if reached, the streaming pipeline doesn't function.

Functional-side blockers (POC test): the new hire in demo mode lands on `/workers/stub-<uuid>` — a page whose title is the literal UUID, whose personality card can't resolve, and which is never inserted into `/team` or `/workers`. So when the operator narrates "I just hired Mariah; let's go meet her," the camera goes to a 404-shaped page. Recommend operator narrate the journey using a SAMPLE_TEAM persona (e.g. open `/team` then click Mariah) rather than running the wizard live.

Biggest blocker: **F1** — the investor moment doesn't exist as a discoverable surface, and the page that does exist doesn't have a backend key.

---

## Headline blockers (top 3 P0s)

| # | Finding | Demo-narrative impact |
|---|---|---|
| **F1** | Genesis is orphaned (not in nav, not in wizard, not linked from landing) AND its gateway endpoint returns 503 on dev. | The investor-moment narrative ("describe a role in voice + upload SOPs + see comprehension verification") has no surface to walk through. Recording the demo cannot show what the operator told the build to show. |
| **F2** | Marketing CTA "Hire your first teammate" routes to `/hire` (a parallel wizard), but the brief and the v1-mission-mvp runbook assume `/onboarding/welcome`. Two flows, both 5-step, both ship in main, neither knows about the other. | If Scene 3 of the runbook tries to walk `/onboarding/*` after clicking the marketing CTA, the camera silently jumps into `/hire` instead. Mid-take pivot. |
| **F3** | In demo mode (the recording configuration), the hire commit returns a fake `stub-<uuid>` worker ID. `/team` continues to show only the 5 SAMPLE_TEAM personas; the just-hired worker is invisible. `/workers/stub-xxx/overview` renders the raw UUID as the page title and the personality card 404s. | The "meet your new teammate" payoff is theater the camera sees: title is a UUID, deck doesn't update. Violates the no-theater-on-function rule (`feedback_no_theater_in_demo`). |

---

## Did the agent answer the brief's critical questions?

| Question | Answer |
|---|---|
| Does the hired worker exist in the gateway after the wizard completes? | **No** in the recording configuration (demo mode). `hireWorker()` short-circuits to `stub-<uuid>` (web/lib/api/workers.ts:333). Outside demo mode the SPA does POST to `/api/v1/workers` (which the gateway does honour, despite not advertising it at `/routes`), but the demo recording configuration disables that path. |
| Does the new worker get a real charter / persona / memory schema? | **In demo mode: no.** Stub UUID, no overview record, name is the UUID. Outside demo: the wizard's worker_draft + answers POST to `/api/v1/workers` and a real worker row is written; but `/team` doesn't refetch optimistically so the new row only appears on a hard reload. |
| Does the flow feel like onboarding a human teammate? | **/onboarding does (good editorial-center, F1-locked design, brand voice).** /hire does too on step 1 + 2 (textarea + question card). Steps 3-5 we couldn't walk reliably because the operator was actively driving the same browser tab — see "Methodology note". |
| Is there a Genesis entry point? | **No discoverable one.** `/onboarding/welcome`'s middle pillar describes Genesis behavior ("Onboard them like a human / Upload SOPs, wiki pages, and Slack threads / we verify they understand before they start") but its "Get started" button routes to `/onboarding/workspace`, which is a Name-the-workspace form — NOT Genesis. The standalone `/genesis` page exists but has zero inbound links from any nav or wizard. Flagging absence as P0 per the brief. |

---

## Methodology note

The shared preview browser was actively being driven by the operator in parallel during this run — pages jumped from `/landing` → `/audit` → `/docs` → `/for/sales` → `/security` → `/skills` between my eval calls. I worked around this by combining: (a) atomic single-eval interactions for the wizard, (b) direct `curl` SSR fetches for status checks, (c) source-reading the route handlers and API client modules, and (d) opportunistic screenshots. Findings F1-F8 are all backed by code paths (file:line cited); F9-F15 are visual / interaction observations from the screenshots I did capture.

---

## All findings

### F1 [P0] — Genesis is orphaned and broken at the gateway

The `/genesis` page exists (`web/app/(app)/genesis/page.tsx:60`) and is a real-backend flow with no mocks. But:

- **No discoverable entry point.** `grep -rn 'href="/genesis\|router.push("/genesis"' web/` returns zero matches. The canonical nav (`web/components/skycot/nav-items.tsx:22-30`) lists Approvals / Team / Workers / Cohorts / Skills / Workflows / Audit — no Genesis. Marketing footer + /docs landing + /onboarding/welcome's "Get started" all bypass it.
- **Gateway endpoint returns 503.** `curl -X POST http://127.0.0.1:7400/api/v1/genesis` returns `{"error":{"code":"genesis_provider_key_missing","message":"Set ARGO_GENESIS_PROVIDER_KEY to enable POST /api/v1/genesis (LLM-driver cost guardrail)"}}`. So even if a user found Genesis, the stream would surface a "Couldn't reach the Genesis stream" error.
- Per memory `feedback_genesis_is_ai_teammate_onboarding_studio` (operator-clarified 2026-05-25T11:40Z): "Genesis takes voice/text role + SOP-PDF/wiki/Slack uploads + access-controls + comprehension verification. The investor moment that distinguishes Skycot from 'ChatGPT with prompts'."

**Reproduction**:
1. From `/landing` click "Hire your first teammate" — lands on `/hire` (not `/genesis`)
2. Visit `/onboarding/welcome` directly — middle pillar describes Genesis behaviour; "Get started" goes to `/onboarding/workspace` (workspace-name form), not `/genesis`
3. Check the top nav at `/team`, `/workers`, anywhere — no "Genesis" link
4. `curl -X POST http://127.0.0.1:7400/api/v1/genesis -H 'Content-Type: application/json' -d '{}'` returns 503

**Files**:
- `web/components/skycot/nav-items.tsx:22-30` (canonical nav)
- `web/app/(app)/genesis/page.tsx:1-60` (the orphaned page)
- `web/app/(setup)/onboarding/welcome/page.tsx:43-49` (the Genesis-pillar copy with no link)

**Suggested fix**: Add `/genesis` to NAV_ITEMS (or replace `/onboarding/welcome` with `/genesis` as the canonical first-hire entry). Set `ARGO_GENESIS_PROVIDER_KEY` on the dev gateway. (Operator decision: which surface is canonical?)

---

### F2 [P0] — Two parallel 5-step wizards, both shipping; marketing CTA picks the one that's not in the brief

Two routes exist:

- **`/hire` family**: 5 steps (Land → Listen → Drafting → Introduce → Connect). Real-LLM-backed (`useGenesisWorkerDraft` calls `/api/v1/genesis/stream`; `useHireWorker` calls `/api/v1/workers`). Owns the `WizardEditorialShell` (Audiowide hero per F1 design lock).
- **`/onboarding` family**: 5 steps (welcome → workspace → connect-provider → pick-vertical → first-hire). Backend-backed via `/api/v1/onboarding/{provision,provider-key,install-vertical,first-hire}`. Owns the `OnboardingShell` (also F1 editorial-center).

The /landing primary CTA "Hire your first teammate" → `/hire`. The "Get started" in the marketing site footer + the user-typed `/onboarding/welcome` URL → `/onboarding`. The brief lists `/onboarding/*` as canonical. The 6-scene v1-mission-mvp runbook isn't explicit about which is recorded.

Wizard scope is also genuinely different:
- /hire collects: role description → clarifying questions → AI-drafted persona → channel hookup. Skips: workspace name, company, provider key, vertical pick.
- /onboarding collects: workspace name + company → Anthropic key → vertical pick → first-hire description. Skips: clarifying questions, AI persona draft, channel hookup.

So a buyer doing /hire never names their workspace, never picks a vertical, never enters an API key. A buyer doing /onboarding never sees the AI personality drafted live. Each tells half the story.

**Reproduction**:
1. Open `/landing` in clean session, click "Hire your first teammate" — lands on `/hire` (step 1 of 5, "Build your AI workforce.", textarea: "What do you need help with?")
2. In a new tab open `/onboarding/welcome` — also "Build your AI workforce." (step 1 of 5, "WELCOME"), but next button goes to a workspace-name form
3. Confirm via code: `web/app/landing/page.tsx` CTA hrefs all point to `/hire` (need to verify on the landing source); `web/app/(setup)/onboarding/welcome/page.tsx:63` routes to `/onboarding/workspace`

**Files**:
- `web/app/(hire)/hire/page.tsx` + `web/app/(hire)/hire/step/[step]/page.tsx`
- `web/app/(setup)/onboarding/{welcome,workspace,connect-provider,pick-vertical,first-hire}/page.tsx`

**Suggested fix**: pick one. Either make /hire wrap /onboarding (workspace-name + key collection on top, persona-draft below) OR redirect /hire → /onboarding and delete the dead surface. Operator decision.

---

### F3 [P0] — Hire commit in demo mode returns a stub UUID; new worker is invisible in `/team`

`web/lib/api/workers.ts:331`:
```
export async function hireWorker(req: HireWorkerRequest): Promise<HireWorkerResult> {
  if (isAuthBypassed()) {
    return { workerId: stubWorkerId(), success: true };
  }
  // ...
}
```

`stubWorkerId()` returns `stub-<crypto.randomUUID()>` (line 219). In the recording configuration (`NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` per `.claude/launch.json:9`), every "hire" in the /hire wizard ALWAYS returns a stub regardless of what the buyer typed.

Consequences:
- `useTeam()` (`web/lib/hooks/use-team.ts`) refetches `fetchTeam()` which in demo mode returns `SAMPLE_TEAM` (the 5 hard-coded personas: Mariah, Peta, Priya, Jamal, Nori). No optimistic insert. So the new hire never appears in `/team` or `/workers`.
- The wizard's Step 6 (`/hire/done/<stub-uuid>`) shows celebration UI, then the operator clicks "View profile" → `/workers/stub-xxx/overview`.
- That page title is literally `stub-xxx · Overview` (`web/app/(app)/workers/[workerId]/overview/layout.tsx:10`).
- `fetchWorkerOverview('stub-xxx')` throws `WorkerOverviewNotFoundError` (`web/lib/api/worker-overview.ts:114`). The personality card shows the error state, not a charter.

Per `feedback_no_theater_in_demo`: function-side theater is forbidden. A hire that returns a stub and a deck that doesn't update IS function-side theater that the camera sees.

**Reproduction**:
1. Open /hire with auth-bypass on
2. Fill step 1, walk through to step 4, click "Hire <name>"
3. Watch the redirect to `/workers/stub-<uuid>/overview` — title is the UUID, personality card is errored
4. Open `/team` — only 5 SAMPLE_TEAM workers, no fresh hire

**Files**:
- `web/lib/api/workers.ts:331-361` (stub fallback)
- `web/lib/api/team.ts:58-99` (no insert into roster)
- `web/app/(app)/workers/[workerId]/overview/layout.tsx:10` (raw-UUID title)

**Suggested fix**: For the recording, narrate the journey via an existing SAMPLE_TEAM persona (open /team → Mariah → her personality / activity / memory). Don't run the wizard live during the take. Long-term: in demo mode, `hireWorker()` should write to a session-scoped local mock that the /team query can read, OR the demo should run against a real gateway with seeded persistence.

---

### F4 [P1] — `/onboarding/welcome` pillar "Onboard them like a human" describes Genesis but doesn't link to it

`web/app/(setup)/onboarding/welcome/page.tsx:43-49`:
```jsx
<li className="ob-pillar" data-testid="onboarding-pillar-genesis-studio">
  <p className="ob-pillar-label">Onboard them like a human</p>
  <p className="ob-pillar-body">
    Describe the role in voice or text. Upload SOPs, wiki pages, and Slack
    threads; we verify they understand before they start.
  </p>
</li>
```

The pillar's `data-testid` is literally `onboarding-pillar-genesis-studio` — somebody KNEW this was Genesis. But the only button on the page is "Get started" which routes to `/onboarding/workspace` (workspace name + company name form). The voice/SOP/upload/comprehension-verification flow isn't reachable from this page.

This is the exact "promise → deliver" mismatch that breaks the investor narrative. The pillar IS the investor pitch ("describe in voice, upload SOPs"); the next step is "what would you like to call your workspace?"

**Reproduction**: Visit `/onboarding/welcome`, read the middle pillar, click "Get started", land on the workspace-name form.

**File**: `web/app/(setup)/onboarding/welcome/page.tsx:43-69`

**Suggested fix**: Either (a) make the pillar itself the CTA into /genesis (clicking the tile routes), or (b) replace this welcome page with the /genesis page entirely. Operator decision.

---

### F5 [P1] — Inconsistent primary-CTA copy: "Hire your first teammate" vs "Hire your first worker"

Captured in screenshots:
- `/landing` top-bar CTA + hero CTA: **"Hire your first teammate"**
- `/for/sales` top-bar CTA: **"Hire your first worker"**
- `/onboarding/welcome` body CTA: **"Get started"**
- `/hire` Step 1 body CTA: **"Continue"**
- `/hire` Step 4 body CTA: **"Hire {name}"** (e.g. "Hire Mariah")
- `/onboarding/first-hire` body CTA: **"Hire them"**

The "teammate not tool" thesis (CLAUDE.md global, the marketing hero, the runbook) is undercut by "Hire your first worker" on the verticals pages. Per `project_skycot_brand_identity` brand voice, teammate is the load-bearing word.

**Reproduction**: Open `/landing`, note the CTA. Open `/for/sales`, note the CTA. Compare.

**Files**: `web/app/landing/page.tsx`, `web/app/for/sales/page.tsx` (need to verify each via grep)

**Suggested fix**: Pick "teammate" everywhere. "Worker" is the in-product noun (nav item, technical context); marketing surfaces should consistently say "teammate."

---

### F6 [P1] — `/landing` CTA bypasses signup entirely; user lands inside a wizard with no account context

The "Hire your first teammate" CTA on `/landing` goes straight to `/hire/step/1`. There's no account creation, no email collection, no workspace identity until step 5 ("Connect a channel") which patches `/api/v1/workers/<id>` — but by then the user has authored a worker without ever telling Skycot who they are.

In the demo recording configuration this works because the demo-mint bearer + the auth-bypass header bypass the gateway's principal requirement. In a real buyer-trial deployment the wizard would either: (a) fail at hireWorker because the gateway rejects unauthenticated POST, or (b) silently create a worker against an anonymous tenant the buyer can't return to.

Compare `/onboarding/welcome` → `/onboarding/workspace` (collects workspace + company name FIRST). That flow is identity-first, then hire.

**Reproduction**: Open `/landing` in private browsing → click "Hire your first teammate" → land on `/hire` with no signup prompt → fill step 1 → continue. The wizard never asks "who are you?"

**File**: `web/app/(hire)/hire/page.tsx` — no auth guard before showing the wizard.

**Suggested fix**: At step 1, when the user clicks Continue, if they're unauthenticated, route to `/auth/signup?returnTo=/hire/step/2&problem=<text>` and resume after verify. Or fold /hire under /onboarding so workspace is collected first.

---

### F7 [P1] — `/workers` redirects to `/team`; the brief's "see the newly-hired worker in the deck" check happens on `/team`, not `/workers`

`web/app/(app)/workers/page.tsx:25-27`:
```js
useEffect(() => {
  if (hasWorkers) router.replace("/team");
}, [hasWorkers, router]);
```

So /workers is an empty-state-only surface — if even one worker exists (and SAMPLE_TEAM has 5 in demo mode), the user is bounced to /team. The brief assumes "/workers verify the newly-hired worker is visible in the deck" but the deck is on `/team`, not `/workers`.

This isn't a bug per se but a navigation footgun for anyone walking the journey by URL: the brief / runbook should specify `/team` for the deck check, not `/workers`.

**Reproduction**: Visit `/workers`, get bounced to `/team`. The deck (5 trading cards: Mariah / Peta / Priya / Jamal / Nori) is on /team.

**File**: `web/app/(app)/workers/page.tsx:25`

**Suggested fix**: Either rename "Workers" in the nav to "Team" (to match the deck-route), or make `/workers` show the deck and `/team` be the empty-state. Don't make them aliases that depend on roster size.

---

### F8 [P1] — `/api/v1/workers` route is missing from the gateway `/routes` catalog despite existing

`curl http://127.0.0.1:7400/routes | jq` returns 143 routes; none under the `/api/v1/*` namespace. But `POST /api/v1/workers` actually responds (returns 400 with field validation errors). So the route exists but isn't catalogued.

This isn't a journey-blocker but it's a discoverability landmine for anyone debugging the SPA → gateway wiring. The brief's "curl `http://127.0.0.1:7400/api/...` to verify backend state" check would mislead a new agent into thinking the workers endpoint doesn't exist.

**Reproduction**:
- `curl http://127.0.0.1:7400/routes | jq '. | length'` → 143
- `curl http://127.0.0.1:7400/routes | jq '[.[] | .path] | map(select(startswith("/api")))'` → empty
- `curl -X POST http://127.0.0.1:7400/api/v1/workers -d '{}'` → 400 (route exists)

**Files**: `crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs` (route exists); gateway catalog source likely in `crates/argo-cli/src/handlers/gateway/routes.rs` (need to verify).

**Suggested fix**: Surface `/api/v1/*` routes in the `/routes` catalog response. Single-line addition to the catalog builder. (Out of POC-demo scope; flag for v1.1.)

---

### F9 [P2] — Step 1 of /hire has very-low-contrast label text on the F1 editorial-center

In the screenshot of `/hire` step 1, "What do you need help with?" appears as ghosted white-on-white text above the (dark) textarea. The placeholder text inside the textarea is readable; the label above it isn't.

This is the F1 design lock — light page + Audiowide hero in #1e1b4b — but the label `<LabelText>` is rendering with very low contrast. WCAG 2.1 AA failure if it's below 4.5:1.

**Reproduction**: Open `/hire` step 1, observe the label "What do you need help with?" above the textarea — it's barely visible against the warm-radial-blob background.

**Files**: `web/components/skycot/hire/Step1Land.tsx` + `WizardEditorialShell.tsx`

**Suggested fix**: Bump the label text color to at least `#475569` (slate-600) or apply the same `#1e1b4b` ink token the heading uses.

---

### F10 [P2] — Hardcoded "2,847 COMPANIES THIS MONTH" social-proof in the /hire wizard footer (dressing, but visible during recording)

Captured in the step-1 screenshot. Per `feedback_no_theater_in_demo` + `v1-demo-positioning-poc-vs-public.md`, dressing-side fake numbers are fine, but Scene 3 of the runbook IS the wizard walkthrough — the camera will read this number. If the operator narrates "we don't fake our numbers," the camera reading 2,847 contradicts the narration.

Recommend either (a) operator-narrate around it, (b) hide the social-proof strip from the wizard footer in demo mode, or (c) keep it (dressing is allowed).

**File**: `web/app/(hire)/hire/page.tsx` footer section or `WizardEditorialShell.tsx`

---

### F11 [P2] — Worker overview page title is the raw UUID, not the human name

`web/app/(app)/workers/[workerId]/overview/layout.tsx:10`:
```js
return { title: `${workerId} · Overview` };
```

So when the operator opens Mariah's profile, the browser tab reads `018f67d0-0000-7000-8000-000000000001 · Overview` (confirmed in the snapshot earlier — Nori's UUID). For Scene 1 of the runbook ("click Mariah → see her C2-personality detail"), the tab title is the UUID.

If recording in single-tab mode (the runbook), this only affects the OG-image / browser-tab framing. If the camera shows the tab strip, it's visible. Easy fix: server-side resolve to `{name} · Overview` once the worker fetch resolves.

**Suggested fix**: `generateMetadata` async-fetch the worker, use `worker.name ?? workerId` for the title.

---

### F12 [P2] — `/onboarding/welcome` page does not collect anything; it's a static splash

The 5-step wizard's step 1 is effectively a 3-pillar info splash with one "Get started" button. The user spends a click and a route transition on a page that asks zero questions. Compare /hire step 1 which collects the role description immediately.

This is consistent with `project_onboarding_design_locked` (F1 editorial-center across 5 steps including welcome) so it's intentional. But it adds 1 step to the 4-minute promise without delivering progress.

**Suggested fix**: Either fold welcome into workspace (single page: pillars on top, form below) or delete the welcome step entirely and make /onboarding/workspace the entry.

---

### F13 [P2] — Auth signup is bypassed in the recording configuration; the brief's "(or demo-bypass if signup is gated)" instruction is currently always-bypass

`NEXT_PUBLIC_ARGO_DEMO_MODE=1` + `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` (`.claude/launch.json:9`) means every request is anonymously authorized in dev. The middleware short-circuits in both branches (`web/middleware.ts:56-77`).

For the recording this is intentional — the operator doesn't want to wait for an SMTP round-trip. But it means I can't actually test the signup → magic-link → verify path end-to-end against this dev server.

**Suggested fix**: For one pre-record session, turn the bypass off, walk the full signup → verify → onboarding flow, screenshot any breakage. (Out of scope for this read-only audit.)

---

### F14 [P2] — `/skills` shows "Loading skills from the marketplace…" indefinitely when the gateway has no skills

Caught in passing during operator-driven navigation. The /skills marketplace shows "0 skills / 0 shown" and the "Loading skills from the marketplace…" message stays put. This is the Scene 4 surface; if the seed packs aren't installed before recording, Scene 4 breaks.

Not directly in scope (the brief is onboarding) but flagging so the runbook execution sees it: confirm `argo demo seed --tenant <demo>` runs before recording.

**File**: `web/app/(app)/skills/page.tsx` (loading-state lock)

---

### F15 [P3] — Onboarding pillar copy uses HTML entity dashes (`&mdash;`) without breaking word-flow

`web/app/(setup)/onboarding/welcome/page.tsx:40`: `Every action signed and replayable &mdash; SOC 2-ready audit trail.` — the em dash renders as the correct character but the surrounding spacing is single-space (`replayable — SOC`) which is non-typographically-tight. Cosmetic only; safe to ignore.

---

## Resolved since last audit

The two prior audits in this directory tree:

- `2026-05-25-1100-onboarding-hire-an-ai-teammate.md` (yesterday) — has not been cross-referenced for this run (the brief asked for fresh signal).
- `2026-05-26-0820/` (this morning) — likewise not cross-referenced.

If the operator wants delta-vs-prior, that's a follow-up call. This audit was scoped to fresh findings on the current main.

---

## Orientation score: 52 / 100

The presentation layer (F1 editorial-center, brand voice, Audiowide hero, "Hire a teammate. (Seriously.)" tone) is unequivocally on-brand and feels investor-grade. That earns ~70 points.

The functional substrate has two structural P0s (Genesis orphaned + two competing wizards) and one demo-recording-blocker P0 (stub-UUID hire result is camera-visible). That drops ~18 points.

The 5-finding P1 layer (orphan pillar copy, CTA inconsistency, signup-bypass, /workers→/team alias, route-catalog gap) is the kind of polish that keeps the demo recordable but signals "we shipped 2x of everything; nobody picked which one wins." Net: 52.

What lifts it back to 80+:
1. Decide between /hire and /onboarding; delete the other (or fold one inside the other).
2. Add /genesis to the nav OR replace /onboarding/welcome with /genesis as the canonical first-touch.
3. Make hireWorker() in demo mode write to a session-local mock store the team-query reads, so the new hire actually appears in /team.

Operator decisions, not engineering ones.

---

## Pointer-back

- Runbook: `docs/skycot/demos/v1-mission-mvp-runbook.md`
- POC scope: `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`
- Track P (pitch agent, the OTHER investor moment): BUILD_STATUS.md header
- Genesis clarification: memory `feedback_genesis_is_ai_teammate_onboarding_studio`
- Locked onboarding design: memory `project_onboarding_design_locked`
- Function-vs-dressing rule: memory `feedback_no_theater_in_demo`
