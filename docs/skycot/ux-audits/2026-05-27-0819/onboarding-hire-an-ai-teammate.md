# Onboarding hire-flow — "hire an AI teammate" investor narrative audit

**Run** 2026-05-27 08:19 AEST
**Scope** `/landing` → `/auth/signup` (demo-mint bypass) → 5-step wizard
(`welcome` → `workspace` → `connect-provider` → `pick-vertical` →
`first-hire`) → `/workers/{id}` (C2-personality detail).
**Mode** Read-only. Dev server + gateway are up; preview Chrome was wedged
for most of the run, so the walk used `curl` + the running gateway on
`127.0.0.1:7400` to exercise each backend hop the wizard actually fires,
plus static-HTML inspection of every step. Static-route walking is enough
to expose the function-side breakages; visual polish (animations, focus
rings) wasn't fully testable this run.

## TL;DR

The narrative "describe an AI teammate in plain English, we hire them"
is **broken on the function side** in two load-bearing ways:

1. **Demo-mode bait-and-switch (F1, P0)**: when the SPA is launched via
   `.claude/launch.json` (which the operator uses for live previews and
   for recording the v1.0 demo), `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` is on.
   The wizard's "Hire them" button silently throws the user's prompt
   away and redirects to one of four hardcoded sample-persona UUIDs
   (Mariah / Priya / Jamal / Nori). The investor sees themselves type
   "Maya the marketing intern" and land on a worker called
   **Nori** with a charter they never wrote. The bug is acknowledged
   inline (`buildDemoFirstHireResult` comment) but the wizard copy
   never tells the user.
2. **Gateway first-hire only persists the first call per tenant (F2,
   P0)**: when demo mode is OFF, `POST /api/v1/onboarding/first-hire`
   returns `201 { worker_id: ... }` for the *second* hire, but the
   worker is never persisted: `GET /api/v1/workers/{id}/overview`
   returns 404 forever and the worker doesn't appear in
   `/api/v1/team`. The SPA's `verifyWorkerReadable` retries 3 × 500 ms
   and then surfaces `"Hiring succeeded but your teammate isn't visible
   yet"`. So the very first signed-up user can hire one teammate; the
   second hire from the same tenant is a permanent ghost.

Other findings: the SPA's `/team` page calls `/api/v1/workers` which
the gateway doesn't have (404 → empty roster + empty state), the
`first-hire` name-extraction is a "first capitalized word" heuristic
(input "A marketing intern named Maya" → name "A"), the help button
in the wizard shell points at `/help` which 404s, the `SAVE & EXIT`
button is destructive (pushes to marketing landing, discards draft),
and every onboarding page shares one generic `<title>` so browser
tabs read identically across the 5 steps. F3 is the orphan endpoint;
F2 is the showstopper.

If the demo records via `pnpm web dev` from the launch.json
configuration (most likely path) the demo is recordable — but the
"function — what the agents DO" pillar is being violated for the
single most narrated beat. **Score the function-vs-dressing rule
against this**: the camera literally drills into "Hire them" and
lands on a destination the user did not author. That's theatre the
camera sees.

## Headline blockers

| # | sev | finding | scene/journey impact |
|---|---|---|---|
| F1 | P0 | Demo-mode `first-hire` discards prompt + redirects to a hardcoded sample persona | Investor narrative breaks: "we hired Maya" → lands on Nori |
| F2 | P0 | Gateway `first-hire` second call per tenant returns 201 but never persists | Any non-demo signup that hires a second teammate hits a permanent "isn't visible yet" error |
| F3 | P0 | SPA `/team` calls `/api/v1/workers` (404 on gateway) → roster always empty in real mode | Post-hire `/workers` is the brand-loud trading-card deck moment; renders as empty state instead |
| F4 | P1 | First-hire name = first capitalized word of the prompt | "A marketing intern named Maya" → worker name "A" |
| F5 | P1 | `SAVE & EXIT` pushes to `/` (→ marketing landing) and saves nothing | Step-3/4/5 user clicks it expecting a draft save → loses 4 minutes of input |
| F6 | P1 | Help button → `/help` 404s | Wizard top-bar Help is non-functional |

## All findings

### F1 [P0] — Demo-mode `first-hire` silently discards the user's prompt and routes to a hardcoded sample persona

The hero beat of the investor narrative — the operator types a
description in step 5, clicks "Hire them", lands on a profile that
matches who they typed — is theatre. When
`NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` (which `.claude/launch.json` sets for
the `web` config the operator uses to launch the preview), the
`firstHire()` mutation short-circuits the gateway entirely and returns
a hardcoded sample-worker id keyed by `verticalId`:

```ts
// web/lib/onboarding/use-first-hire.ts:108-143
const SAMPLE_MARIAH = "018f67d0-0000-7000-8000-000000000001";
// ... maps marketing-intern → Nori, recruiter → Priya, etc.
function buildDemoFirstHireResult(verticalId) {
  const chosen = VERTICAL_TO_SAMPLE_WORKER[verticalId] ?? DEMO_FALLBACK_WORKER;
  return { workerId: chosen.id, workerName: chosen.name, pending: false };
}
```

The submitted description (`"Maya the marketing intern who drafts blog
posts..."`) is written to the Zustand draft store, then never read.
The `/workers/{id}` destination renders one of four pre-baked sample
personas. Per `feedback_no_theater_in_demo` (2026-05-24), the v1.0
demo must be REAL not mocks — this is exactly the "theatre the camera
sees" failure mode that memory was written to block.

The bug is acknowledged inline as a deliberate fallback because
real-mode hire isn't wired end-to-end (see F2/F3). The fix is to
either honour the user's prompt in demo mode (synthesize a real
worker row + overview) or change the copy in step 5 to tell the user
"we'll match you to a demo teammate — describing them helps us pick
the closest one".

- Repro: launch `pnpm --dir web dev` with `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`
  (or use `.claude/launch.json` config `web`). Visit
  `/onboarding/first-hire`. Type "Maya the marketing intern".
  Click "Hire them". Land on `/workers/018f67d0-0000-7000-8000-000000000004`
  which renders as "Nori" (the marketing-intern sample persona).
- File: `web/lib/onboarding/use-first-hire.ts:108-143` (mapping),
  `web/lib/onboarding/use-first-hire.ts:152-154` (the short-circuit),
  `.claude/launch.json:7` (env var that turns demo mode on).
- Suggested fix: in demo mode, route through the gateway anyway with a
  scoped demo tenant — the gateway already mints workers (`name` extracted
  from prompt, see F4). The current path was authored when the real-mode
  hire wasn't ready; F2/F3 are the now-or-never blocker for switching.

### F2 [P0] — Gateway `first-hire` after the first hire per tenant returns 201 but never persists; UI shows "isn't visible yet"

Tested directly against `http://127.0.0.1:7400/api/v1/onboarding/first-hire`
with a fresh `demo-mint` JWT:

```
$ # FIRST call, vertical=marketing-intern
$ POST /api/v1/onboarding/first-hire {"description":"Sara...",vertical_id:"marketing-intern"}
   201 {"name":"Sara","worker_id":"019e6674-ecb0-...","vertical_id":"marketing-intern"}
$ GET /api/v1/workers/019e6674-ecb0-.../overview
   404 {"error":{"code":"not_found"}}
$ GET /api/v1/team
   200 { total_count:1, workers:[{ id:"019e5dbb-c03d-...", name:"Maya" }]}  # the FIRST-EVER hire
```

The endpoint returns a UUID that never appears in storage. The SPA's
`verifyWorkerReadable` retries 3 attempts × 500 ms then surfaces
`FIRST_HIRE_VERIFY_FAILED_MESSAGE = "Hiring succeeded but your
teammate isn't visible yet. Try refreshing or contact support."` This
is the wizard's final step in real-mode; the user has just typed
their teammate description, clicks "Hire them", waits ~1.5 seconds,
sees a contradiction message. There is no support route to contact
(see F6).

Tested against marketing-intern + sales-sdr verticals with two
different descriptions — same outcome. Only the very first hire ever
for a tenant persists; subsequent hires return fresh worker_ids that
don't land in the database.

- Repro: with auth-bypass on, `curl -X POST` the first-hire endpoint
  twice with two different descriptions. Second one returns 201 but
  the worker is unreachable + absent from `/api/v1/team`.
- File: gateway-side, not in `web/`. Suspect
  `crates/argo-gateway/src/onboarding/first_hire.rs` or equivalent
  (one-hire-per-tenant guard somewhere); SPA-side this would manifest
  as F1 even if F1 were fixed.
- Suggested fix: gateway needs to either (a) actually persist every
  hire or (b) return 409 idempotent with the prior worker_id so the
  SPA can route to the canonical row instead of a fresh ghost id.

### F3 [P0] — SPA `/team` calls `/api/v1/workers` which the gateway doesn't expose; roster always empty in real mode

`web/lib/api/team.ts:80` calls `apiGet("/api/v1/workers")`. The
gateway returns `404 {"error":"not found"}` for `GET /api/v1/workers`
(only `GET /api/v1/workers/{id}*` exists). `fetchTeam` catches the
404 in `isMissingOrUnreachable` and returns
`{ workers: [], source: "unreachable" }`. The `/workers` page then
sees an empty workers array → renders `HireStarterPrompts` (the empty
state). The `/team` page renders the same empty state.

This means: a real authenticated user who has hired one teammate
successfully (F2's "first hire ever" path) lands on `/workers` or
`/team` and sees **zero workers** plus a "Hire your first AI
teammate" prompt — even though they just did. The "teammate, not a
tool" metaphor depends on the roster persisting visibly between
hires. It doesn't.

The legacy `/api/v1/team` endpoint **does** exist on the gateway and
returns the worker. The SPA was migrated to `/api/v1/workers` (per
the file's own header comment "canonical collection endpoint") but
the gateway never landed the collection route alias.

- Repro: with demo mode off, `curl http://127.0.0.1:7400/api/v1/workers
  -H "Authorization: Bearer ..."` → 404. `curl ... /api/v1/team` → 200 with
  the roster.
- File: `web/lib/api/team.ts:80` (SPA caller) + the gateway-side
  collection alias is missing. Companion to
  `streams/fix-gateway-workers-collection-route-alias-404-add-canonical.md`
  which is already authored.
- Suggested fix: either land the gateway alias `GET /api/v1/workers →
  GET /api/v1/team` or temporarily flip the SPA back to `/api/v1/team`
  until the alias lands.

### F4 [P1] — First-hire name extraction is "first capitalized word", not a real parse

Gateway behaviour:
- prompt `"A marketing intern named Maya who drafts blog posts"` → name `"A"`
- prompt `"Maya, marketing intern"` → name `"Maya"`
- prompt `"a marketing intern named maya"` → likely name `null` or `"a"`

The user is implicitly told their prompt is parsed ("Plain English is
fine. Give them a name, a role, and what they should do this week") —
but the gateway just slices off the first capitalized token. The
worker-overview charter copy ("Drafts blog posts based on briefs and
asks for approval before publishing.") looks great but is the
**vertical's** default charter, not synthesized from the user's
prompt. So we promise plain-English understanding and deliver
substring-extraction.

- Repro: see F2 first call (`"A marketing intern named Maya..."` →
  `name:"A"`).
- File: gateway-side hire endpoint, likely
  `crates/argo-gateway/src/onboarding/first_hire.rs`. SPA-side, the
  prompt-vs-name mismatch isn't visible because demo mode shortcuts
  the call (F1).
- Suggested fix: either (a) use the prompt's first sentence as the
  worker name placeholder until Genesis returns a properly-parsed
  name, or (b) ship the wizard's name field as a separate input so
  the user owns it (current copy says "Give them a name" — it's
  pretending we parse names we don't actually parse).

### F5 [P1] — `SAVE & EXIT` pushes to `/` and saves nothing

Top-bar `SAVE & EXIT` button (`WizardEditorialShell.tsx:355-357`)
calls `attemptLeave()` → `router.push("/")`. `/` 307-redirects to
`/landing`. The "save" is a lie — there's no draft-persist call,
and the operator's company name, API key, vertical pick, and
description are gone. The shell does support a discard-confirm
dialog (`confirmDiscard`) but only fires when `unsavedHire` is true;
the onboarding pages don't pass it.

This is at minimum copy-deceptive: button reads SAVE & EXIT, behaves
as EXIT WITHOUT SAVING. Worse, the destination is the public
marketing landing page, not the app. A user mid-onboarding clicking
"save & exit" expecting "come back later" instead lands on the
public marketing site.

- Repro: visit `/onboarding/workspace`, fill in workspace name +
  company, click `SAVE & EXIT`. Land on `/landing`. Back-button
  doesn't restore the form; the Zustand draft persists in
  `localStorage` but the operator has no UI to resume from it
  ("Resume onboarding" CTA missing on landing for signed-in users).
- File: `web/components/auth/WizardEditorialShell.tsx:310-321` +
  `web/app/(setup)/onboarding/workspace/page.tsx:122-130`. Existing
  brief: `streams/fix-onboarding-save-and-exit-button-destructive-rename.md`.
- Suggested fix: rename button to `EXIT` and add a small destination
  affordance ("Your draft is saved on this device — pick up where you
  left off."). Or wire a real /api/v1/onboarding/draft persist + a
  "Resume" entry point on the post-login dashboard.

### F6 [P1] — Wizard top-bar Help button points at `/help`, which 404s

The Help button in the wizard top bar
(`WizardEditorialShell.tsx:350`) is a `<Link href="/help">`. `/help`
returns 307 → `<meta name="next-error" content="not-found">`. So the
wizard advertises a Help affordance that is broken. Existing brief:
`streams/fix-onboarding-hire-success-copy-help-route-not-support.md`
implies the correct destination is probably the Intercom help-widget
or `/docs`.

- Repro: visit any `/onboarding/*` step → click "Help" in the top
  bar → see Next.js 404.
- File: `web/components/auth/WizardEditorialShell.tsx:350`.
- Suggested fix: change href to `/docs` (exists) or trigger the
  `data-testid="help-widget-trigger"` floating widget instead.

### F7 [P1] — Every onboarding page shares one generic `<title>` so browser tabs read identically

Curling each step:
- `/onboarding/welcome` → `<title>SKYCOT — Hire your first AI teammate</title>`
- `/onboarding/workspace` → same
- `/onboarding/connect-provider` → same
- `/onboarding/pick-vertical` → same
- `/onboarding/first-hire` → same

A user with the browser-tab-row open during recording can't tell
which step they're on; the title doesn't include the step number or
label even though the in-page step-strip carries `01 02 03 04 05`.
This is also a small but visible accessibility regression — the
window title for screen readers is the only signal of progress for
keyboard users moving between tabs.

- Repro: `curl -s http://localhost:3000/onboarding/<step> | grep -oE
  '<title>[^<]+</title>'` — same string across all 5 steps.
- File: `web/app/(setup)/onboarding/*/page.tsx` — none of them export
  `metadata = { title: ... }`. Companion to
  `streams/fix-app-shell-default-metadata-title-leaks-approvals-across-9-pages.md`
  (which catches the same pattern on `/genesis`, `/workers/[id]` etc.
  surfacing "Approvals · SKYCOT" as the title).
- Suggested fix: each step page exports `metadata = { title: 'Step N
  of 5 · <Label> · SKYCOT' }`.

### F8 [P1] — Worker-detail browser title is "Worker · Overview" — no name in title even after a successful hire

Visiting `/workers/{id}/overview` returns `<title>Worker · Overview</title>`.
The Mariah/Maya/Sara name doesn't surface in the browser title even
though the page hero shows it. The investor demo storyboard hinges on
"meet your new teammate" — the browser-tab signal undercuts that.

- Repro: `curl http://localhost:3000/workers/<id>/overview | grep title`.
- File: `web/app/(app)/workers/[workerId]/overview/page.tsx` (or
  layout) — should pull worker name into the page metadata.
- Suggested fix: server-side `generateMetadata` that fetches the
  worker name and emits `${name} · Overview · SKYCOT`.

### F9 [P1] — Welcome-step copy contradicts itself on singular-teammate vs workforce

`/onboarding/welcome` hero reads:
- eyebrow: WELCOME TO SKYCOT
- h1: "Build your **AI workforce**."
- sub: "In **four minutes** you'll onboard your **first AI teammate**..."

Workforce (collective) vs first teammate (singular) — these don't
land the same emotional beat. The metaphor across the whole flow is
"hire one teammate" (signup CTA: "Hire your first teammate"; signup
hero: "Hire your first AI teammate"; step-5 CTA: "Hire them"). The
welcome step pivots to "workforce" without warning. The brief
`streams/fix-onboarding-welcome-headline-singular-teammate-vs-workforce.md`
flagged this already.

- Repro: `/onboarding/welcome` rendered text.
- File: `web/app/(setup)/onboarding/welcome/page.tsx:21-30`.
- Suggested fix: change h1 to `"Hire your first **AI teammate**."`
  or `"Onboard your first **AI teammate**."` so the entire flow uses
  the same metaphor.

### F10 [P1] — Connect-provider step calls itself "Demo mode" without any visible flag — and the user doesn't know what changed

When `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` (live preview default), the
connect-provider step renders:
> "Demo mode — workers run with a mock provider (simulated
> responses). Type any sk-ant- key to continue; no Anthropic call is
> made."

But there's no top-level demo-mode banner on the wizard top bar (the
signup page has a yellow `DemoModeBanner`; the wizard doesn't carry
it through). A user who started at `/auth/signup` and saw the banner
there might forget it; a user who entered the wizard via a deep link
(or `Skip` from elsewhere) never sees the banner at all. The "Demo
mode" copy on step 3 is the only signal — and the user might not
recognize "Demo mode" as the SPA-wide flag that's also responsible
for sending them to Nori-not-Maya at step 5 (F1).

- Repro: `curl /onboarding/connect-provider` text → "Demo mode — workers
  run with a mock provider".
- File: `web/app/(setup)/onboarding/connect-provider/page.tsx:141-153`
  + the missing wizard-level demo banner.
- Suggested fix: surface the same `DemoModeBanner` at the top of
  `WizardEditorialShell` whenever `isDemoMode()` is true, so the user
  knows the entire flow is simulated.

### F11 [P2] — Vertical catalogue is always empty from the gateway; the wizard always falls back to its hardcoded 7-card list

`GET /api/v1/verticals` returns `{"verticals":[]}` (gateway-side). The
wizard's `pick-vertical` step calls this, gets empty, falls back to
`FALLBACK_VERTICALS` (7 hardcoded). The hint text below the grid
reads `v1 catalogue (offline · 7 verticals)` — so the user is told
the catalogue is "offline" even though it's just empty.

The skill counts on the fallback cards (`gtm-engineer: 8`,
`marketing-intern: 5`, etc.) match `skills_added` from the actual
gateway install — so picking any card and clicking it does land
real skills. But the catalogue list itself is a placeholder.

- Repro: `curl http://127.0.0.1:7400/api/v1/verticals` → `{"verticals":[]}`.
- File: `web/app/(setup)/onboarding/pick-vertical/page.tsx:210-215`
  uses the fallback. The gateway needs to surface installed verticals
  via `/api/v1/verticals` from its catalog.
- Suggested fix: make gateway populate `/api/v1/verticals` from the
  on-disk packs in `crates/argo-gateway/src/verticals/`. Until that
  ships, change copy from `(offline · 7 verticals)` to `(7
  starting templates)` so the user doesn't think something's broken.

### F12 [P2] — `pick-vertical` grid restricts to 7 cards even though the system supports more (and the operator might want to author one)

Per memory `feedback_genesis_is_ai_teammate_onboarding_studio`, the
Genesis vision is "voice/text role + SOP uploads → bespoke vertical".
The pick-vertical step renders 7 hardcoded cards with no "+ Create
custom" affordance and no Genesis link. The welcome step's "Onboard
them like a human" pillar links to `/genesis` — but pick-vertical
doesn't carry that thread. The user picks one of 7 templates or
nothing.

This is a P2 because the function works for the 7 templates, but the
investor narrative says "describe in plain English" and the
plain-English description doesn't land until step 5 after they pick
a constrained template.

- File: `web/app/(setup)/onboarding/pick-vertical/page.tsx`.
- Suggested fix: add an 8th card "+ Author a new role" or "Describe a
  custom teammate" that routes to `/genesis` — already designed.

### F13 [P2] — First-hire form lacks an explicit "name" field; the prompt is the only input

Step 5 says `"Plain English is fine. Give them a **name, a role, and
what they should do this week**."` — but the only input is a
multi-line textarea (`first-hire-prompt`). The user has to embed the
name in prose. Given F4 (gateway just takes the first capitalized
word), this is the load-bearing failure mode.

- Repro: visit `/onboarding/first-hire`, type "She's called Maya and
  she runs the blog" → worker named "She's" (or similar).
- File: `web/app/(setup)/onboarding/first-hire/page.tsx:96-118`.
- Suggested fix: separate fields for name + description (or accept
  the messy prose and ship server-side LLM-parse for the name, but
  that's a bigger swing).

### F14 [P2] — Worker-overview returns all `permissions = false`; operator can't edit charter, pause, view audit on their own newly hired worker

Gateway `/api/v1/workers/{id}/overview` returns:
```json
"permissions": {
  "can_archive": false, "can_delete": false,
  "can_edit_charter": false, "can_edit_duties": false,
  "can_pause_resume": false, "can_view_audit": false,
  "can_view_settings": false
}
```

For the **owner** who just hired the worker. The teammate metaphor
collapses if you can't pause your teammate or read what they've
done. This may be intentional capability-RBAC defaults that need a
post-hire grant — but for a fresh demo-mint principal who just minted
the tenant, all-false is wrong.

- Repro: hire via gateway, fetch overview, observe permissions block.
- File: gateway-side capability seeding; suspect
  `crates/argo-gateway/src/workers/overview.rs` or the RBAC default
  ladder. SPA-side, the worker-detail page presumably hides edit
  buttons based on these flags → renders as read-only-everything for
  the operator.
- Suggested fix: when a principal owns the tenant they minted, the
  capability default should be "owner-everything", not
  "everything-false".

### F15 [P3] — "Pause or adjust **her** anytime" — gendered language in empty-state copy

`TeamEmptyState.tsx:27` reads `"Pause or adjust **her** anytime —
she's your teammate, not your tool."` Hardcoded `her` in the generic
empty-state. The roster mixes Mariah/Peta/Priya (she), Patrick/Jamal
(he), and Nori (ambiguous). Either go gender-neutral ("them") or
randomize.

- File: `web/components/skycot/TeamEmptyState.tsx:26-28`.
- Suggested fix: `"Pause or adjust them anytime — they're your
  teammate, not your tool."`

### F16 [P3] — `install-vertical` reports `workflows_added: 0` for every vertical even though "schedules" is part of the investor metaphor

`POST /api/v1/onboarding/install-vertical` returns
`{"installed":true,"skills_added":5,"workflows_added":0}` for both
`marketing-intern` and `sales-sdr`. The operator is told skills are
ready but no schedules ("duties") are installed. The Scene 2 demo
beat (per the runbook) is "click Run now on a duty → audit row lands
in 5s" — for a freshly hired worker, there are no duties to run.

- Repro: install any vertical, fetch worker overview, see
  `duties: []`.
- File: gateway-side; vertical-pack manifest is missing the
  `workflows[]` block, or install-vertical isn't propagating them.
- Suggested fix: ensure each shipped vertical pack ships ≥1 duty so
  the hire→run flow is recordable end-to-end without a separate
  manual step.

### F17 [P3] — `/workers` page has its `<h1>` hidden (`sr-only`) — no visible page heading on the empty state

The `/workers` page wraps `<HireStarterPrompts />` inside an `<h1
className="sr-only">Workers</h1>` — visually invisible. The
`TeamEmptyState` heading "Hire your first AI teammate" is rendered as
`<h2>`, not as the page's `<h1>`. This is fine for screen-readers
(the sr-only h1 exists) but loses an editorial-typography moment on
the empty state — the operator who just landed here from onboarding
sees no "Workers" page-title, just the empty card.

- File: `web/app/(app)/workers/page.tsx:39`.
- Suggested fix: remove the `sr-only` and elevate the empty-state
  heading to h1, or render a real visible "Your team" / "Workers"
  page header above the empty state.

### F18 [P3] — Genesis pillar on welcome step is a `role="link"` div clickable via Enter/Space but doesn't visually look clickable

`/onboarding/welcome` makes the middle pillar ("Onboard them like a
human") a clickable div that routes to `/genesis`. The other two
pillars are static. There's no visual cursor cue beyond
`style={{ cursor: "pointer" }}`, no hover state, and the screen
reader gets `role="link"` but a sighted user has no signal this is a
hotspot.

- File: `web/app/(setup)/onboarding/welcome/page.tsx:43-62`.
- Suggested fix: either add a visible arrow / "Try Genesis →" affordance
  or make it a proper `<Link>` button styled as a card.

## Resolved since last audit

(2026-05-26 audits in `~/code/argo/docs/skycot/ux-audits/2026-05-26*/`)

- The post-hire 404 on `/team/<id>` was resolved when the redirect
  flipped to `/workers/<id>` (visible in `use-first-hire.ts:57`).
  Still has F2 (verify-404 ghost) but the destination route exists.
- The `NEXT_PUBLIC_` env-var rename to `NEXT_PUBLIC_SKYCOT_DEMO_MODE`
  consolidates the demo flag, but launch.json still also sets
  `NEXT_PUBLIC_ARGO_DEMO_MODE=1` — both flags are checked in
  different places.

## Orientation score

**52 / 100.**

The shell, the editorial design (F1-locked across 5 steps), the brand
voice, the SAVE/EXIT/HELP affordances, the step-progress strip, and
the empty-state grids are all polished and recognizable. The
"audit-first / onboard like a human / enterprise-grade" pillars
land emotionally. The signup → demo-mint → wizard path works in
~four minutes as promised. Dressing-side is mostly good.

The deduction is heavy because the function side — the actual
"hire a teammate" verb — fails for the load-bearing investor demo
beat. Demo-mode is bait-and-switch (F1). Real mode persists only
the first ever hire per tenant (F2). The roster page is empty in
real mode because the SPA queries an endpoint the gateway doesn't
expose (F3). The name-extraction is brittle (F4). Three of four
key affordances in the top bar are broken or destructive (F5, F6).

For the **POC investor demo MP4**, this still records if the camera
stays on the first-ever-hire happy path under demo mode and accepts
that the destination worker is whichever sample persona matches the
chosen vertical. The narration "we hired Maya, let's see what she's
doing" is recordable IF the operator types a description matching
the persona name they know they'll land on (e.g. type "Nori" for
marketing-intern). That's narrow but workable.

For a real customer trial, this is not shippable — the second hire
per tenant doesn't exist; the roster is empty after success.
