# Onboarding hire-an-AI-teammate flow — UX audit

**Run:** 2026-05-27 ~04:15 AEST (18:15Z 2026-05-26)
**Agent:** e2e-ux-journey-test-agent
**Scope:** `/landing` → `/auth/signup` → 5-step wizard (`welcome → workspace → connect-provider → pick-vertical → first-hire`) → `/workers/<id>` (post-hire profile)
**Mode:** read-only. No code changes, no commits, no cargo.
**Demo env:** `.claude/launch.json` exports `NEXT_PUBLIC_ARGO_DEMO_MODE=1 NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` — every walk this session was demo-mode.

## TL;DR

The post-#2424 in-product CTA rewire is wired correctly: every "Hire" affordance inside the app now points at `/onboarding/first-hire`, and the public landing CTAs route to `/auth/signup` then forward into the wizard. The five wizard step routes all return 200. But the wizard's terminal click — "Hire them" → `/workers/<id>` — **fails to land on a real worker in BOTH demo mode and honest mode**, for two independent root causes. And the welcome page's central pillar ("Onboard them like a human… upload SOPs, wiki pages, Slack threads; we verify they understand") is promised to the buyer but **NOT delivered** — step 5 is a single textarea, no upload, no voice, no comprehension check. This is the operator's investor-narrative pillar (per memory `feedback_genesis_is_ai_teammate_onboarding_studio`) and is the biggest narrative gap.

- **3 P0** (would break the investor demo MP4 if Scene 3 narrates the wizard or drills into the post-hire worker)
- **5 P1** (visible during the hire walk; a buyer or investor would notice)
- **5 P2** (polish, not a recording blocker)
- **2 P3** (nits)

## Headline blockers

### F1 [P0] — Demo-mode wizard hands back a fake worker id; `/workers/<fake-id>` renders "Worker not found"

`web/lib/onboarding/use-first-hire.ts:102-112` — in demo mode `firstHire()` returns `workerId: "demo-worker-${verticalId}-${Date.now().toString(36)}"`. The wizard's submit handler at `web/app/(setup)/onboarding/first-hire/page.tsx:57` then does `router.push("/workers/${res.workerId}")`. The destination `/workers/demo-worker-marketing-intern-mevb9k/overview` runs `useWorker(workerId)` → `apiGet("/api/v1/workers/<fake-id>")` → 404 → `fetchWorker` falls back to `findSampleWorker(workerId)` (`web/lib/api/workers.ts:139-148`) — but `findSampleWorker` only matches the 5 seeded uuids (`018f67d0-0000-7000-8000-00000000000{1,2,3,4,7}`). The synthetic id doesn't match → `WorkerNotFoundError` → `WorkerLayoutShell` renders the `<WorkerNotFound>` shell (`web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx:50-60`). Page title literally reads `demo-worker-marketing-intern-mevb9k · Overview`.

**Camera test:** Scene 3 of the runbook ends with the wizard's "Hire them" click landing on the new worker's personality card. If demo mode is on (it is, in launch.json), the camera lands on a 404 shell. **Investor-narrative climax broken.**

**Reproduction:**
- `curl -s -o /tmp/wid.html http://127.0.0.1:3000/workers/demo-worker-marketing-intern-xyz/overview` → 200 (page hydrates client-side)
- Inspect: `<title>` contains the literal synthetic id; body renders the not-found shell after hydration
- Source: `web/lib/onboarding/use-first-hire.ts:102-112` + `web/lib/sample-worker-overview.ts:176-180`

**Suggested fix:** in demo mode, either (a) redirect to one of the 5 seeded sample workers (`/workers/018f67d0-0000-7000-8000-000000000001/overview` for Mariah) rather than fabricating a fake id, OR (b) register the synthetic id at hire time into the in-memory sample-roster map so the readback resolves. (a) is the smaller change.

### F2 [P0] — Welcome page promises "voice + SOPs + wiki + Slack uploads + comprehension verification" — step 5 delivers a single textarea

`web/app/(setup)/onboarding/welcome/page.tsx:43-49` (the centre pillar, `data-testid="onboarding-pillar-genesis-studio"`):

> **Onboard them like a human** — Describe the role in voice or text. Upload SOPs, wiki pages, and Slack threads; we verify they understand before they start.

`web/app/(setup)/onboarding/first-hire/page.tsx:96-148` is **a single textarea + Hire-them button**. No voice input. No file upload. No URL paste. No Slack-thread import. No comprehension verification step (no quiz, no "the worker understood the following 5 points" recap). The honest delivery is "type a sentence; we'll parse the first capitalised word as a name."

Per operator-locked memory `feedback_genesis_is_ai_teammate_onboarding_studio` (2026-05-25T11:40Z): *"Genesis takes voice/text role + SOP-PDF/wiki/Slack uploads + access-controls + comprehension verification. The investor moment that distinguishes Skycot from 'ChatGPT with prompts'."* Phase 1 (text-only) was scoped, but the welcome copy advertises Phase 2.

**Camera test:** the welcome page IS shown in Scene 3 (per the runbook); the investor reads the pillar copy and then watches step 5 deliver a textarea. The gap will read on camera as overpromise/underdeliver — the exact "ChatGPT with prompts" framing the operator wants to escape.

**Suggested fix:** either (a) shrink the welcome pillar's copy to match what step 5 delivers ("Describe the role in plain English; we generate the charter, duties, and approval gates"), OR (b) ship the Phase-2 upload + comprehension UI before the demo records. The narrative is the bigger lift; (a) is the same-day fix.

### F3 [P0] — Example placeholder produces a worker named "A" (honest mode); `name_from_description` still picks first capitalised token

`crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs:232-252` — `name_from_description` picks the first token whose first char `is_alphabetic() && is_uppercase()`. The wizard's placeholder + example hint at `web/app/(setup)/onboarding/first-hire/page.tsx:15-16` + `:110` + `:116`:

```
const EXAMPLE_PROMPT =
  "A marketing intern named Maya who drafts blog posts and runs weekly competitive scans.";
```

→ first capitalised token = `"A"` → persona row created with `name="A"`. This is the same finding as the 2026-05-27-0119 prior audit's F2. Confirmed still active in source tree (no fix shipped between then and now). In demo mode the F1 short-circuit hides this; in honest mode the worker is named "A".

**Camera test:** if the operator records in honest mode (no demo env vars), the worker the wizard creates is literally named "A" — the Slack-style introduction "Hi, I'm A" is unrecoverable on camera. The example prompt the placeholder suggests is the worst possible input.

**Suggested fix:** either rewrite the placeholder to lead with the name (`"Maya — a marketing intern who drafts blog posts and runs weekly competitive scans."`) OR teach `name_from_description` to skip indefinite articles ("a", "an", "the") + auxiliary words. The placeholder rewrite is the safer one-line change.

## All findings, in order

### Wizard chrome (shared)

#### F4 [P2] — Welcome step's eyebrow is "WELCOME TO SKYCOT", not "STEP ONE · WELCOME"

`web/app/(setup)/onboarding/welcome/page.tsx:20` uses `eyebrow="WELCOME TO SKYCOT"`, but every subsequent step uses `STEP TWO · WORKSPACE`, `STEP THREE · CONNECT AI PROVIDER`, `STEP FOUR · …`, `STEP FIVE · FINAL`. Same finding as prior-audit F3; still present.

**Suggested fix:** change to `eyebrow="STEP ONE · WELCOME"`.

#### F5 [P3] — Page title on the post-hire destination reads `demo-worker-marketing-intern-xyz · Overview`

When F1 fires, the `<title>` set by the worker-detail layout includes the raw synthetic id (e.g. `demo-worker-marketing-intern-mevb9k · Overview`). Even before the page hydrates and falls back to the not-found shell, the browser tab/window title leaks the implementation-detail id. Tied to F1 fix.

### Step 1 — `/onboarding/welcome`

#### F6 [P1] — "Build your AI workforce" hero is generic enterprise-speak; loses the wizard's plain-English voice

`web/app/(setup)/onboarding/welcome/page.tsx:21-30` — hero is "Build your AI workforce" + sub "In four minutes you'll onboard your first AI teammate." Compare to the brand voice on `/landing` ("Don't buy another tool. Hire a teammate. (Seriously.)") — the welcome page lands flatter than the marketing pitch the user just came from. The visitor took two clicks (Hire-CTA → Sign up → Welcome) and the energy dropped. Brand voice consistency between marketing and onboarding is load-bearing for the investor-narrative pitch.

**Suggested fix:** swap to a continuation-of-pitch hero: "Let's hire your first teammate." + sub aligned to step 5's reality ("Four minutes. Voice or text — your call.").

### Step 2 — `/onboarding/workspace`

#### F7 [P2] — Demo-mode provision still returns no `created` flag (prior-audit F5 unfixed)

`web/lib/onboarding/use-provision.ts:56-62` (`buildDemoProvisionResult`) returns only `{ tenantId, workspaceId }` — the typed `ProvisionResult` has an optional `created: boolean` field. The workspace page only fires the celebration toast + 1200ms hold when `res.created === true`. Demo mode → `undefined` → no toast, no hold, the wizard jumps to step 3 with no acknowledgment of the workspace being "created." Same finding as prior-audit F5; still present.

**Suggested fix:** return `created: true` once per session (keyed off sessionStorage presence of the demo tenant id) so first-time demo users get the celebration.

### Step 3 — `/onboarding/connect-provider`

#### F8 [P3] — "$0.001 test call" still claimed; validation still shape-only

Wizard sub-copy promises "A $0.001 test call confirms the key works" but `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs:107-114` is shape-only validation (`sk-ant-` prefix). Prior audit's F6, still present, still dressing-side, still POC-acceptable.

### Step 4 — `/onboarding/pick-vertical`

#### F9 [P1] — Vertical-detail API path inconsistency: list works, detail 404s

`web/lib/api/verticals.ts:231` calls `apiGet("/verticals")` (no `/api/v1` prefix) — works (200, 32 verticals returned). But `web/lib/api/verticals.ts:323` calls `apiGet("/api/v1/verticals/<id>")` — **404s** because the api-prefixed singular detail route isn't registered:

```
curl /api/v1/verticals/marketing-intern → 404
curl /verticals/marketing-intern         → 200
```

Wizard step 4 will appear to load the list fine, but any feature that hits the detail endpoint (the "Show me what's in this template" expand affordance, if added; the marketplace envelope; any hover-preview) silently fails. Demo mode hides this because the SPA's local seed fallback fires; honest mode hits the 404.

**Suggested fix:** either alias `/api/v1/verticals/<id>` → existing handler, or rewrite the SPA `fetchVertical` to call `/verticals/<id>`. The aliasing is the cheaper change (one new dispatch arm) and matches what `9ea693db1` did for the workers collection.

#### F10 [P1] — 32 verticals on the gateway, 7 decorated in the SPA (prior-audit F7 unfixed)

`web/app/(setup)/onboarding/pick-vertical/page.tsx:37-100` decorates exactly 7 vertical ids; the gateway's `/verticals` returns 32. The other 25 fall through to tag-shaped chips ("Csm", "Post Sale", "Demo", "Playbook"). Same finding as prior-audit F7; still present. Camera will see undecorated cards if Scene 3 lingers on the grid.

**Suggested fix:** curate the wizard to only show decorated verticals via a `vertical.tags.includes("demo")` filter, OR author the 25 missing decorations.

### Step 5 — `/onboarding/first-hire`

#### F11 [P0 — see F1, F2, F3]

The terminal-step issues are headline-blockers; called out above.

#### F12 [P1] — Error copy still says "Try refreshing or contact support" — but in honest mode the readback failure is deterministic (prior-audit F10 unfixed)

`web/lib/onboarding/use-first-hire.ts:10-11` — `FIRST_HIRE_VERIFY_FAILED_MESSAGE = "Hiring succeeded but your teammate isn't visible yet. Try refreshing or contact support."` The phrasing implies transient eventual-consistency; in honest mode the underlying tenant-mismatch (workspace tenant vs principal-provisioned tenant) is deterministic — refreshing won't help. Same finding as prior-audit F10; still present.

### `/workers` (post-hire roster surface)

#### F13 [P1] — `/workers` still redirects to `/team` (prior-audit F12 unfixed)

`web/app/(app)/workers/page.tsx:25-27` — useEffect redirect to `/team` when workers exist. The wizard pushes to `/workers/<id>` directly (per-worker route, not collection), so this doesn't break the journey, but the nav-bar "Workers" link will URL-flash. Same finding as prior-audit F12; still present.

### `/workers/{id}` — worker profile after hire

#### F14 [P1] — `WorkerPersonalityCard` is hard-coded to 3 names (prior-audit F11 unfixed)

`web/components/skycot/worker/WorkerPersonalityCard.tsx:35-96` defines personality data for **only Mariah, Priya, Jamal**. Every other worker (Maya, Peta, Nori, Pioneer, any freshly-hired worker) renders `DEFAULT_PERSONALITY` = `{ traits: ["Reliable", "Curious", "Helpful"], greatAt: [3 generic lines], escalateWhen: [3 generic lines] }`. The newly-hired worker — the climactic destination of the entire wizard — looks beige. Compare to the existing Mariah card which is rich, specific, personal. The "onboard them like a human" promise lands as "and then they have the same 3-word personality as everyone else."

**Camera test:** if Scene 3 lands on a fresh worker (per F1), this also fires; the contrast against Scene 1's Mariah is visible side-by-side.

**Suggested fix:** wire the personality card to read live `worker.persona.bio` + `worker.persona.traits` from the gateway worker-overview payload (the schema exists in `argo-store`; the data path isn't wired). Until then, do NOT land Scene 3 on a fresh worker; have the camera drill back into Mariah.

#### F15 [P1] — Three primary action buttons on the worker profile are still inert stubs (prior-audit F14 unfixed)

`web/components/skycot/worker/WorkerOverviewContent.tsx:75-92` — Add responsibility / Edit schedule / Save charter all show "ships in E4-g/E4-h" toasts. Same finding as prior-audit F14; still present.

### Routing / chrome

#### F16 [P1] — Browser-side runaway fetch loop on the `meta/error-reporting` endpoint wedges the renderer

While driving the SPA via the Claude Preview MCP, every page mount triggered:
- `GET /api/v1/meta/error-reporting` from `web/lib/error-reporting/init.ts:53` (preflight succeeds, fetch fails with `net::ERR_FAILED` because the gateway response has no `Access-Control-Allow-Origin` header)
- The fetch's failure triggers the global `window.error` handler (re-attached on every page mount because `useEffect`'s dep is `[source]` which is stable but the effect body re-runs on each navigation)
- The error handler tries to POST a Sentry envelope → another failed fetch → another error event

Network log shows the SPA cycling through /landing → /skills → /workers → /team → /approvals back-to-back in <100ms intervals, each mount firing 4-6 gateway fetches that all fail. After ~30s the preview renderer became unresponsive (`preview_eval`, `preview_screenshot` all timing out for the rest of the session).

This is the same shape as the earlier "perpetual loading" bug class. CORS misconfig on the gateway side + a SPA error-handler that itself fires fetches is a feedback loop. Not directly a hire-flow issue, but it breaks any flow you try to drive in a browser session that runs longer than a few minutes.

**Suggested fix:** (a) gateway returns `Access-Control-Allow-Origin: http://127.0.0.1:3000` on `/api/v1/meta/error-reporting` (and arguably all `/api/v1` GETs that the SPA hits with credentials-omit); (b) `lib/error-reporting/init.ts` should never throw on its own fetch failures into the configured backend — wrap `reportBrowserError` in a guard that returns early when the previous report failed within the last N seconds.

#### F17 [P2] — Running gateway binary is ~2.5 hours stale vs current code (`/api/v1/workers` 404 because the alias commit hasn't rebuilt)

`9ea693db1 gateway: alias workers collection route` shipped 2026-05-26 17:00; running gateway binary booted 2026-05-26 14:34 (uptime 49972s). The SPA's `useTeam` hook successfully uses `/api/v1/team` (legacy path, 200); but any client that adopts the new canonical `/api/v1/workers` per the commit's docstring will 404 against this gateway until it's restarted. Environmental, not a code defect — but the gateway-restart cadence is itself a demo-prep step.

### Sidebar / persistent chrome

#### F18 [P1] — No "Demo mode" / "Live mode" affordance, so the operator can't tell which path they're on (prior-audit F16 unfixed)

Demo mode is the path the operator will record in (launch.json forces it on), but the only visible signal is the demo-mode banner at the top of in-app pages. The onboarding wizard pages do NOT show that banner (verified by inspecting `web/app/(setup)/onboarding/_OnboardingShell.tsx`'s render tree — the banner mounts in `(app)/layout.tsx`, not in `(setup)/layout.tsx`). So a wizard-time recording has no on-screen indicator of which mode it's in. Same finding as prior-audit F16; still present.

**Suggested fix:** add a small "DEMO MODE" pill in the OnboardingShell's footer when `isDemoMode()` returns true, so the recording-prep checklist can be "DEMO pill visible" rather than "did I export the env var."

## Resolved since last audit (2026-05-27 ~01:19)

- **#2424 (in-product hire CTAs)** is wired correctly. Verified via DOM scrape on the landing-served `/approvals` view: the "Hire" link points at `/onboarding/first-hire`. The 7 callsites called out in the commit body (`web/app/(app)/page.tsx` Approvals top-bar, `(app)/team/page.tsx` Team header, `(app)/team/error.tsx`, `CommandPalette.tsx`, `TeamEmptyState.tsx`, `WorkflowEdgeStates.tsx`) all route to the canonical wizard entry. Prior-audit F7 closed.
- **`/landing` CTA → `/auth/signup` rewrite** is live. Three `Hire your first teammate` links on landing all `href="/auth/signup"`. Prior 2309-F6 closed.

## Function-vs-dressing scorecard (per `v1-demo-positioning-poc-vs-public.md`)

| Finding | Function or Dressing | Camera sees? | POC blocker? |
|---|---|---|---|
| F1 — demo-mode synthetic worker id 404s on readback | **Function** | Yes — last beat of Scene 3 | **YES in demo mode** (the recording mode) |
| F2 — welcome pillar overpromises vs delivered step 5 | **Function/narrative** | Yes — Scene 3 welcome page | **YES** — narrative is the investor moment |
| F3 — worker named "A" from example placeholder | **Function** | Yes (honest mode) | **YES in honest mode** |
| F14 — generic personality card on fresh worker | **Function** | Yes — Scene 3 climax | **YES if Scene 3 lands on a fresh worker** |
| F9 — vertical-detail 404 | Function (silent) | No (silent failure) | NO |
| F15 — inert action buttons | Function | Only if hovered | NO (avoid hover) |
| F13 — `/workers` → `/team` redirect | Dressing | URL bar flash | NO |
| F16 — error-reporting feedback loop | Function (chrome) | No (silent until renderer dies) | NO (unless recording takes >5min) |
| F4 — eyebrow inconsistency | Dressing | Yes | NO |
| F6 — flat welcome hero | Dressing | Yes | NO |
| F7 — no celebration on demo workspace | Dressing | Possibly | NO |
| F8 — $0.001 test-call claim | Dressing | Yes | NO |
| F10 — undecorated verticals | Dressing | Yes — Scene 3 card grid | NO (curate) |
| F12 — misleading error copy | Function | Only if hit | NO (won't hit on demo) |
| F18 — no demo/live indicator | Function (operator tool) | No | NO (operator-side) |
| F5 — synthetic id leaks into page title | Dressing | Yes (tab title) | NO (tied to F1 fix) |
| F17 — stale gateway binary | Environmental | No | NO (restart fix) |

Camera blockers in priority: **F1 → F2 → F3 → F14**.

## Orientation score

**44 / 100** — *cosmetically polished surface; the function-side experience the investor demo needs to record is broken in both demo mode and honest mode for different reasons.*

What's working:
- The five-step wizard chrome is consistent + on-brand (F1 editorial-center locked design)
- The post-#2424 CTA rewire is correct everywhere
- Every step returns 200; no 500s observed
- The gateway endpoint actually exists and creates real workers (the existing test worker "Maya" id `019e5dbb-c03d-79a0-a9a4-ee60bb3c8218` proves the pipeline works once you bypass auth + tenant-resolution friction)
- Brand voice continues from landing through auth to wizard

What's tanking the score:
- **F1 + F3**: the wizard's single most-load-bearing click ("Hire them" → land on the new worker) breaks in BOTH modes — demo mode lands on a 404, honest mode creates a worker named "A"
- **F2**: the welcome page sells an "AI teammate onboarding studio" (voice + uploads + comprehension verification) that step 5 doesn't deliver. This is the operator's investor-distinguishing pitch and the gap reads on camera
- **F14**: even if the post-hire worker DID render, the personality card falls back to "Reliable / Curious / Helpful" because the fresh worker isn't in the hard-coded 3-name dictionary

For the v1.0 demo MP4 specifically, this means: **do not record Scene 3 with a fresh wizard hire.** Either pre-seed the demo with a worker whose id matches one of the 5 sample uuids (so the post-hire redirect lands on a rich personality card), or fix F1 + F14 before recording. The narrative-gap from F2 is a longer fix; the simplest mitigation is to soften the welcome pillar copy until Phase 2 ships.

If the demo is recorded in demo mode (the default per `launch.json`), F3 doesn't fire — but F1 still does, and the synthetic-id leak into the page title is visible from the URL bar + tab title.
