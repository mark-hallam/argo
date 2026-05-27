# Onboarding hire-flow — "hire an AI teammate" investor narrative audit

**Run** 2026-05-28 00:51 AEST (00:00 UTC tick after midnight)
**Scope** `/landing` → `/auth/signup` (demo-mint bypass) → 5-step wizard
(`welcome` → `workspace` → `connect-provider` → `pick-vertical` →
`first-hire`) → `/workers/{id}` (C2-personality detail).
**Mode** Read-only. Dev server (port 3000) + gateway (port 7400,
schema v83) up. Walked with Claude Preview MCP for live DOM + cross-
checked against curl + source inspection. Followed the function-vs-
dressing rule per
[`docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`](../../demos/v1-demo-positioning-poc-vs-public.md).
Companion to the 2026-05-27-0819 audit; this run revalidates that
audit's findings against current `origin/main` and surfaces what
changed.

## TL;DR

The narrative "describe an AI teammate in plain English, we hire
them" remains **broken on the function side** for the demo-mode path
the investor narrative actually traverses. Three load-bearing
breakages survive from the prior audit:

1. **Demo-mode `firstHire` discards the user's prompt and routes to
   one of four hardcoded sample-persona UUIDs (F1, P0).** Investor
   types "Maya the marketing intern" → clicks `Hire them` → lands on
   `/workers/018f67d0-…-000000000004/overview` titled "Nori ·
   Overview". The wizard's hero copy ("Plain English is fine. Give
   them a name, a role, and what they should do this week.") is
   theater the camera sees the moment it drills into the destination.
2. **Two parallel "hire a teammate" wizards co-exist; the marketing
   funnel points at the stubbed one (F2, P0).** `/landing` →
   `/auth/signup` → `/onboarding/*` is the funnel the investor will
   actually walk. A separate `/hire/step/<n>` wizard exists, mounts
   `WizardEditorialShell` with the same chrome, and actually
   persists session-local hires through `appendDemoWorker()` (the
   roster grows, `/workers/<id>` resolves to the just-typed name).
   The OAuth callback routes new users to `/hire`; the magic-link +
   demo-mint paths route to `/onboarding/welcome`. Two answers to the
   same question.
3. **The "Acmeco · Owner" identity is hardcoded for every demo-mode
   visitor (F3, P0).** `lib/skycot/use-session.ts:39` returns the
   constant `DEMO_SESSION` whenever `isDemoMode()` is true. The
   investor types "Acme Growth / Acme Inc." in step 2, accepts the
   "Workspace created — welcome to Skycot" toast, and then sees
   "Sam Harlow · Acmeco · Owner" baked into the sidebar identity
   pill for the rest of the session. The wizard's step-2 inputs are
   discarded the moment the user leaves the wizard.

Other findings: a stale env-var name leaves the signup page's primary
CTA reading "Send magic link" instead of "Enter demo workspace" when
launched via `.claude/launch.json`; only 3 of 5 demo personas (Mariah,
Priya, Jamal) have personality-card data, so Nori + Peta land on
DEFAULT_PERSONALITY ("Reliable / Curious / Helpful") — the personality
card is brittle for the persona the marketing-intern vertical
redirects to; the wizard top-bar Help + SAVE & EXIT buttons sit
off-screen in a 375px viewport; in dark color-scheme, the wizard's
light gradient bg ends and exposes a near-black band below it on tall
pages; `SAVE & EXIT` still pushes to `/landing` and discards every
field the user typed; the worker-detail browser tab title reads
"Worker · Overview" for any worker not in the bundled persona map
(every gateway-real worker hits this); the personality-card title
"How {name} works" + body "What she does" are gendered around the
sample-persona roster but the personality card itself is generic for
half of them.

**Function-vs-dressing classification**: F1, F2, F3, and F4 are all
**function-side** (the destination the camera lands on does not match
what the user typed). F5, F6, F7 are dressing-side polish that the
camera may or may not see.

For the **POC investor demo MP4**, the recordable workaround the
prior audit identified — "operator types a description matching the
sample-persona name they know they'll land on (e.g. type 'Nori' for
marketing-intern)" — still holds. But the investor narrative
"we describe in plain English and SKYCOT compiles a worker" is
camera-visible-broken if the operator deviates even slightly.

## Headline blockers

| # | sev | finding | scene/journey impact |
|---|---|---|---|
| F1 | P0 | Demo-mode `firstHire` returns one of 4 hardcoded sample UUIDs; user's prompt is discarded | Investor types "Maya the marketing intern" → lands on "Nori · Overview" |
| F2 | P0 | Two parallel hire wizards (`/onboarding/*` stub-path vs `/hire/step/*` real-path); marketing funnel points at the stub | "Hire your first teammate" CTA on /landing routes to the stubbed wizard, not the working one |
| F3 | P0 | `useSkycotSession` hard-returns "Sam Harlow · Acmeco · Owner" for every demo-mode visitor | Wizard step-2 inputs ("Acme Growth / Acme Inc.") are visually discarded post-wizard |
| F4 | P1 | Stale env-var name: signup page reads `NEXT_PUBLIC_ARGO_DEMO_MODE`, launch.json sets only `NEXT_PUBLIC_SKYCOT_DEMO_MODE` | Signup CTA reads "Send magic link" + no demo banner; the demo path is a secondary dashed-border button |

## All findings

### F1 [P0] — Demo-mode `firstHire` silently discards the user's prompt and routes to a hardcoded sample persona

**Function-side, drill-in test fails.** This is the prior audit's F1
re-verified against current `origin/main` — the bug is unchanged.

The hero beat of the investor narrative — operator types a teammate
description in step 5, clicks `Hire them`, lands on a profile that
matches who they typed — is theater. When
`NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` (which `.claude/launch.json` sets for
the `web` config the operator uses to launch the preview), the
`firstHire()` mutation short-circuits the gateway entirely and returns
a hardcoded sample-worker id keyed by `verticalId`:

```ts
// web/lib/onboarding/use-first-hire.ts:108-143
const SAMPLE_MARIAH = "018f67d0-0000-7000-8000-000000000001";
const SAMPLE_PRIYA  = "018f67d0-0000-7000-8000-000000000002";
const SAMPLE_JAMAL  = "018f67d0-0000-7000-8000-000000000003";
const SAMPLE_NORI   = "018f67d0-0000-7000-8000-000000000004";

const VERTICAL_TO_SAMPLE_WORKER = {
  "marketplace-seller": { id: SAMPLE_MARIAH, name: "Mariah" },
  recruiter:            { id: SAMPLE_PRIYA,  name: "Priya"  },
  "customer-support":   { id: SAMPLE_JAMAL,  name: "Jamal"  },
  "marketing-intern":   { id: SAMPLE_NORI,   name: "Nori"   },
};
const DEMO_FALLBACK_WORKER = { id: SAMPLE_MARIAH, name: "Mariah" };
```

The submitted description (e.g. `"A marketing intern named Maya who
drafts blog posts and runs weekly competitive scans"`) is stashed in
the Zustand draft store and never read again. The destination page
`/workers/<NORI_UUID>/overview` renders Nori's pre-baked C2 card.
Screenshot of the destination is in this run's preview trace (browser
tab title literally reads "Nori · Overview").

This violates the operator-locked
`feedback_no_theater_in_demo` rule (2026-05-24) which says the camera
must not see fabricated function-side results. The fact that an
inline code comment acknowledges the bug
(`buildDemoFirstHireResult` calls it "the post-hire redirect lands on a
real-rendered C2 card matching the operator's chosen template")
doesn't make it not-theater — the operator's chosen template ≠ the
operator's typed name.

Recordable workaround: operator types "Nori the marketing intern who
drafts blog posts" in step 5 (matching the persona that the
marketing-intern vertical redirects to). The destination title +
roster card then match. Narrow workaround; the wizard's example
prompt copy ("A marketing intern named Maya who drafts blog posts and
runs weekly competitive scans") actively pushes the operator away
from this path.

- Repro: launch `pnpm --dir web dev` with the launch.json `web`
  config. Visit `/onboarding/first-hire`. Type "Maya the marketing
  intern". Click `Hire them`. Land on
  `/workers/018f67d0-0000-7000-8000-000000000004/overview` —
  page title "Nori · Overview".
- File: `web/lib/onboarding/use-first-hire.ts:108-144` (mapping),
  `web/lib/onboarding/use-first-hire.ts:153-155` (the short-circuit).
- Suggested fix: in demo mode, append the user's typed prompt as a
  new entry in `web/lib/api/demo-roster-store.ts` (which already
  supports session-local hires via `appendDemoWorker()` — used by
  the `/hire/step/*` parallel wizard). Mint a real UUIDv7, persist
  to sessionStorage, redirect to that. The session-local roster
  store machinery already exists; the onboarding wizard just
  doesn't call it.

### F2 [P0] — Two parallel "hire a teammate" wizards co-exist; the marketing funnel points at the stubbed one

**Architecture finding.** During inspection I found:

- `/onboarding/welcome → workspace → connect-provider → pick-vertical
  → first-hire` (the route the investor walks after signup)
- `/hire → /hire/step/2 → 3 → 4 → 5 → /hire/done/{workerId}` (a
  separate 5-step flow under `app/(hire)/hire/`)

The two wizards both use `WizardEditorialShell`, both have a step-1
"land" beat, both promise plain-English hire. But:

- The onboarding flow's `firstHire()` mutation (F1) discards the
  prompt and hardcodes the destination.
- The `/hire` flow's `hireWorker()` mutation
  (`web/lib/api/workers.ts:416-430`) mints a real UUIDv7, persists
  the worker via `appendDemoWorker({ worker, overview })`, the
  `/team` roster grows by one, and `/workers/<id>/overview`
  resolves to a real card with the typed name.

`/auth/signup` redirects new email-flow users to `/onboarding/welcome`
(see `web/app/auth/signup/page.tsx:125`,
`web/app/auth/verify/page.tsx:56`). The OAuth callback redirects new
users to `/hire` (see
`web/app/auth/oauth/[provider]/callback/page.tsx:66`). So the
"answer" to "where does a new user land" depends on whether they
signed up via SSO (real wizard) or magic-link (stub wizard). The
demo-mode path forces magic-link / demo-mint → onboarding stub
wizard → the F1 dead-end.

The likely history (from in-code comments + brief filenames): the
`/onboarding/*` flow was authored when the gateway's first-hire
endpoint wasn't ready; the demo short-circuit was meant as a
stopgap; the gateway endpoint subsequently landed
(`POST /api/v1/onboarding/first-hire` now returns 401 not 404, so
the route exists) but the SPA's stop-gap never got removed.

- Repro: from `/landing` click "Hire your first teammate" → lands
  on `/auth/signup` → click "Enter demo workspace" → lands on
  `/onboarding/welcome` (the stub wizard). Compare with directly
  visiting `/hire` (after demo-mint) — works end-to-end and the
  hired worker appears on `/team`.
- File: `web/app/auth/signup/page.tsx:125` (magic-link →
  `/onboarding/welcome`), `web/app/auth/oauth/[provider]/callback/page.tsx:66`
  (OAuth → `/hire`), `web/app/(setup)/onboarding/first-hire/page.tsx`
  (stub wizard step 5), `web/app/(hire)/hire/step/[step]/page.tsx`
  (real wizard step 2-5), `web/lib/onboarding/use-first-hire.ts`
  (stub mutation), `web/lib/api/workers.ts:416` (real mutation).
- Suggested fix: pick ONE wizard. The simplest move is to redirect
  `/onboarding/first-hire` to `/hire` at the start of step 5 (carrying
  any selected vertical via `?example=<id>`). Longer term, delete
  the `/onboarding/*` stub wizard; the workspace + provider + vertical
  steps that the `/onboarding/*` flow uniquely owns can move to a
  pre-flight before `/hire` or live as a separate "settings → first
  setup" surface.

### F3 [P0] — Demo-mode hardcodes the workspace identity to "Sam Harlow · Acmeco · Owner"; the wizard's typed inputs are discarded the moment the user leaves the wizard

**Function-side, camera-visible.** Step 2 of the wizard asks for
workspace + company name (placeholder: "Acme Growth", "Acme Inc.")
and on submit fires a celebration toast — "Workspace created — welcome
to Skycot." Visually the operator just "named their company." Two
seconds later, when the wizard finishes and the operator lands on
any in-app surface (e.g. `/workers/<id>/overview`), the sidebar
identity pill reads "Sam Harlow · Acmeco · Owner". The operator never
typed those words.

The cause is `web/lib/skycot/use-session.ts:39-58`:

```ts
export const DEMO_SESSION: SkycotSession = {
  status: "authenticated",
  isDemoMode: true,
  email: "sam@acmeco.com",
  displayName: "Sam Harlow",
  tenantId: "tenant_acmeco",
  workspaceLabel: "Acmeco · Owner",
  role: "owner",
};

export function deriveSession(data, status): SkycotSession {
  if (isDemoMode()) return DEMO_SESSION;
  // ...
}
```

In demo mode the hook ignores the actual session entirely. So:

- Sidebar identity = "Sam Harlow · Acmeco · Owner"
- Email = `sam@acmeco.com`
- TenantId = `tenant_acmeco`

…regardless of whatever the operator typed during signup or step 2.
Per the function-vs-dressing rule, a hardcoded "Acmeco" workspace
label is dressing if the wizard ALSO didn't ask the operator to name
their workspace. But the wizard DOES ask. Then it discards the
answer. That's a camera-visible contradiction: "Workspace created"
(toast) → "Acmeco · Owner" (sidebar 30 seconds later).

The right approach for a POC demo is one of: (a) hide the workspace +
company inputs in demo mode (the wizard already short-circuits
`provision()` to a synthetic id; just don't ask for a name we won't
honour), (b) honour the typed name by updating `DEMO_SESSION` from
the zustand draft post-step-2, or (c) replace step 2 with a "Welcome
to your Skycot workspace — we're using a sample workspace for the
demo" page.

- Repro: type "Acme Growth / Acme Inc." in step 2, accept the
  toast, complete onboarding, look at the bottom-left sidebar pill on
  any in-app page — "Sam Harlow · Acmeco · Owner".
- File: `web/lib/skycot/use-session.ts:39-58` (hardcoded constant),
  `web/lib/onboarding/use-provision.ts:56-63` (demo provision discards
  the inputs).
- Suggested fix: per (a) above — make step 2 disabled / hidden in
  demo mode, or rewrite the copy ("Step 2 of 5 — your workspace" →
  "Step 2 of 5 — sample workspace") with no inputs.

### F4 [P1] — Stale env-var name leaves the signup page's primary CTA reading "Send magic link" in launch.json mode; demo path is a secondary dashed-border button

**Function-side, friction.** `.claude/launch.json` (the operator's
preview config used to launch the dev server) sets
`NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`. The signup page reads the OLD env
var name, `NEXT_PUBLIC_ARGO_DEMO_MODE`:

```ts
// web/app/auth/signup/page.tsx:32-33
const DEMO_MODE =
  typeof process !== "undefined" && process.env.NEXT_PUBLIC_ARGO_DEMO_MODE === "1";
```

Every other file in the SPA reads
`NEXT_PUBLIC_SKYCOT_DEMO_MODE` (canonical, declared in
`web/lib/skycot/demo-mode.ts:17`). The rename swept the project but
missed the signup page.

Result: when the operator opens the preview from `.claude/launch.json`
to walk the demo, the signup page renders in "real mode" — no yellow
DemoModeBanner, primary CTA reads "Send magic link", below-form copy
reads "We'll email you a one-tap sign-in link." The demo path is only
discoverable via the small dashed-border secondary button at the
bottom ("Or try a demo workspace · no email required").

If the operator clicks the primary button by reflex, they get the
real magic-link flow, which then fails because demo-mint sets a
session JWT with `demo: true` claim and the magic-link verify
expects a fresh token. Two minutes of investor time wasted.

- Repro: run preview via launch.json, visit `/auth/signup`, observe:
  no `data-testid="demo-mode-banner"` is present, primary CTA reads
  "Send magic link", below-form copy reads about email. Confirmed
  via `curl http://localhost:3000/auth/signup | grep -oE 'Send magic
  link|Enter demo workspace'` returns "Send magic link".
- File: `web/app/auth/signup/page.tsx:33`. Companion canonical declaration
  at `web/lib/skycot/demo-mode.ts:17` for the right env var name.
- Suggested fix: replace `process.env.NEXT_PUBLIC_ARGO_DEMO_MODE`
  with `isDemoMode()` from `@/lib/skycot/demo-mode`. Same single-line
  fix as `web/components/skycot/AuthProvider.tsx:21` (which also still
  reads the stale name but doesn't break because the bypass-mode
  fallback at line 22 saves it).

### F5 [P1] — Only 3 of 5 demo personas have personality-card data; Nori and Peta get the generic fallback

**Function-side, camera reads it.** The locked C2-personality design
memory (`project_worker_detail_design_locked`) commits the worker
detail page to "trait chips + great-at + escalate-when + signature
behaviors" as the moment that sells "your AI hires are colleagues, not
tools." `WorkerPersonalityCard` hardcodes those traits per-name:

```ts
// web/components/skycot/worker/WorkerPersonalityCard.tsx:35-96
const PERSONALITIES: Record<string, PersonalityTraits> = {
  Mariah: { /* full Mariah traits */ },
  Priya:  { /* full Priya traits */ },
  Jamal:  { /* full Jamal traits */ },
};

const DEFAULT_PERSONALITY = {
  traits: ["Reliable", "Curious", "Helpful"],
  greatAt: ["Working through the responsibilities you set", ...],
  // ...
};
```

Nori is in `SAMPLE_PERSONAS` (with traits `["On-voice", "Considered",
"Schedule-respecting", "Reach-tracking"]`) but NOT in `PERSONALITIES`.
Peta is also missing. So when the F1 redirect lands the camera on
Nori (the marketing-intern vertical's destination), the personality
card reads "Reliable / Curious / Helpful" not "On-voice /
Considered / Schedule-respecting / Reach-tracking" — and the
"great at" + "escalate when" + "signature behaviors" all read the
generic placeholder text "Working through the responsibilities you
set / Asking when something is ambiguous / Surfacing patterns across
recent runs."

The investor sees a teammate detail page with brand-specific charter
copy ("Nori helps the Brand team keep customer-facing comms on-voice
and on-schedule…") sitting RIGHT NEXT TO generic personality copy
("Asking when something is ambiguous"). The disconnect is visible
in screenshot.

The single source of truth for traits already lives in
`web/lib/sample-personas.ts:55-119` (each persona has its 4 traits).
The personality card should read from there instead of maintaining a
parallel map.

- Repro: visit `/workers/018f67d0-0000-7000-8000-000000000004/overview`
  (Nori). Card "HOW NORI WORKS" renders chips "Reliable / Curious /
  Helpful" instead of Nori's actual traits.
- File: `web/components/skycot/worker/WorkerPersonalityCard.tsx:35-96`
  (only 3 of 5 personas defined), `web/lib/sample-personas.ts:55-150`
  (canonical traits exist here for all 6 personas including Patrick).
- Suggested fix: replace `PERSONALITIES` with a lookup against
  `findSamplePersona(workerId).traits` from `sample-personas.ts`. The
  `greatAt` / `escalateWhen` / `signature` arrays can grow on the
  persona record (or stay in a separate file but include all 6
  personas).

### F6 [P1] — Wizard top-bar Help + SAVE & EXIT clipped off-screen at mobile width (375px)

**Camera-visible if the recording rig uses mobile preview.** At
375px viewport (`preview_resize preset=mobile`) the wizard's top bar
overflows. `data-testid="wiz-shell-exit"` (SAVE & EXIT button) sits at
`x=505px width=34px` — the right edge of the button is at 540, well
past the 375px viewport. The Help button is also clipped. Only
the SKYCOT brand wordmark + truncated step strip are visible.

The wizard top bar's `display: grid` layout doesn't switch to a
hamburger or hide the right-side affordances on narrow viewports.
The investor demo will be recorded on desktop, but any reviewer
following along on a phone (e.g. a remote investor watching the
shared link before a call) sees a broken top bar.

- Repro: in preview, `preview_resize preset=mobile`, navigate to
  `/onboarding/welcome`. SAVE & EXIT button is past the right edge.
  Confirm with `preview_inspect [data-testid="wiz-shell-exit"]` —
  boundingBox x=505 width=34, viewport width 375.
- File: `web/components/auth/WizardEditorialShell.tsx:336-362`
  (header layout). The shell currently uses a fixed `grid-template-
  columns: auto 1fr auto`; needs a `@media (max-width: 640px)` rule
  that either hides the right-side affordances or collapses them
  into an overflow menu.
- Suggested fix: at narrow viewports, hide SAVE & EXIT (keep brand +
  step strip), or replace SAVE & EXIT + Help with a single
  three-dot overflow button.

### F7 [P1] — In dark color-scheme, the wizard's light gradient ends and exposes a near-black band below the wizard on tall pages

**Camera-visible.** The wizard shell paints a soft cream/lavender
gradient as its background. The shell's height is determined by its
content (eyebrow + hero + body + CTA). On a 900px-tall viewport with
the welcome step (1062px content height), the wizard scrolls slightly
— and the area below the wizard's gradient is the document body
background, which goes to near-black in dark color-scheme (rgb(2,7,11)
in the preview test).

Confirmed via `preview_resize colorScheme=dark` + screenshot: the
gradient ends at content-height, the body bg is dark below. Visually
this reads as a recorded-video glitch ("why does the bottom half of
the page go black?") rather than a designed transition.

The investor demo will most likely be recorded in light mode, but
this regresses on systems where the operator's preference is dark
and they forget to toggle to light. The `useSkycotSession()` /
`isDemoMode()` paths don't override system color-scheme.

- Repro: `preview_resize colorScheme=dark`, visit
  `/onboarding/welcome`, scroll to bottom of viewport — gradient cuts
  off, body shows dark stripe.
- File: `web/components/auth/WizardEditorialShell.tsx` — the
  shell's `.wiz-editorial` should either set its bg on
  `html.dark` to match the gradient OR enforce `min-height: 100vh`
  on the bg layer.
- Suggested fix: in `WIZARD_STYLES`, add `min-height: 100vh` to
  `.wiz-editorial` (it already exists for medium viewports per
  some media queries; just make it unconditional).

### F8 [P1] — `SAVE & EXIT` still pushes to `/landing` and discards every wizard input — unchanged from prior audit

The top-bar SAVE & EXIT button in
`WizardEditorialShell.tsx:355-361` calls `attemptLeave()` which
pushes to `/` — `/` 307-redirects to `/landing`. There is no draft
persist call to the gateway, the operator's company name + API key +
vertical pick + description are all lost. The shell has a
`confirmDiscard` modal path but onboarding pages don't pass the
`unsavedHire` prop, so the confirmation never fires from this flow.

This is at minimum copy-deceptive — the button reads "SAVE & EXIT"
and behaves as "EXIT WITHOUT SAVING, AND DROP THE OPERATOR ON THE
PUBLIC MARKETING LANDING." For a demo recording this is high-risk: if
the operator accidentally clicks SAVE & EXIT mid-wizard, the demo
restart goes back to `/landing` and the workspace name they typed is
gone.

- Repro: visit `/onboarding/workspace`, fill in the fields, click
  SAVE & EXIT. Land on `/landing`. Back-button doesn't restore.
- File: `web/components/auth/WizardEditorialShell.tsx:310-321` +
  `web/app/(setup)/onboarding/workspace/page.tsx:122-130`.
- Suggested fix: rename to "EXIT" and target `/team` (the post-login
  shell) instead of `/landing`; OR keep the SAVE branding and wire a
  real /api/v1/onboarding/draft persist + a "Resume" entry point on
  the post-login dashboard. For demo recording, the simplest fix is
  to disable SAVE & EXIT in demo mode entirely (the operator
  doesn't need it during a take and the click is one keystroke from
  derailing the recording).

### F9 [P1] — Worker-detail browser tab title reads "Worker · Overview" for any gateway-real worker; only the 4 bundled sample-persona UUIDs get a name-aware title

**Camera-visible if the browser tab bar is open.** The Overview tab's
`generateMetadata` reads:

```ts
// web/app/(app)/workers/[workerId]/overview/layout.tsx:13-24
export async function generateMetadata({ params }) {
  const { workerId } = await params;
  const persona = findSamplePersona(workerId);
  if (persona) {
    return { title: `${persona.name} · Overview` };
  }
  return { title: "Worker · Overview" };
}
```

`findSamplePersona` only resolves the 4 hardcoded sample UUIDs
(`000000000001` through `000000000004` + Peta at `000000000007`). The
gateway-seeded "Nori" worker has UUID `019e6785-928f-7281-bf8d-
e062bc277478` — different from the sample Nori UUID — so its
overview page browser title reads "Worker · Overview" instead of
"Nori · Overview." This means the camera-visible browser tab title
is correct ONLY when the F1 redirect drops the operator on a
hardcoded sample UUID. Any "real" demo recording that hits a
gateway-real worker (e.g. clicking a card on `/team` for the
seeded roster) gets the generic title.

- Repro: visit `/team`, click any roster card, observe browser tab —
  "Worker · Overview", not "{name} · Overview." Visit
  `/workers/018f67d0-0000-7000-8000-000000000004/overview` directly
  — "Nori · Overview" (sample UUID, works).
- File: `web/app/(app)/workers/[workerId]/overview/layout.tsx:13-24`.
- Suggested fix: fetch the worker name server-side (the gateway has
  the endpoint; auth-bypass mode resolves in <100ms) and surface in
  metadata. Until then, fall back to the gateway team roster name
  via a server-side lookup keyed by id.

### F10 [P1] — `/api/v1/workers/<sample-uuid>` returns 404; SPA falls back to bundled sample but the network panel logs a 404 the camera might catch

Network capture during a `/workers/<sample-id>/overview` visit shows:

```
GET /api/v1/workers/018f67d0-0000-7000-8000-000000000001 → 404 Not Found
GET /api/v1/workers/018f67d0-0000-7000-8000-000000000001/retrain/recent → 404 Not Found
```

The SPA's `fetchWorker` catches the 404 and falls back to the
bundled `findSampleWorker` (per
`web/lib/api/workers.ts:152-159`). Page renders correctly. But the
investor's Chrome dev-tools panel — if it happens to be open — shows
404s on the network tab. Demos with the inspector open will surface
these to a technical reviewer.

The 4 hardcoded sample-persona UUIDs in
`web/lib/onboarding/use-first-hire.ts:109-112` should match the
gateway-seeded UUIDs OR the SPA should not hit the gateway for known
sample UUIDs at all (short-circuit before the request fires).

- Repro: open `/workers/018f67d0-0000-7000-8000-000000000001/overview`
  with dev-tools network panel open → see 404s.
- File: `web/lib/api/workers.ts:129-166` (fetchWorker), `web/lib/api/worker-overview.ts:110-138` (fetchWorkerOverview).
- Suggested fix: short-circuit `apiGet` when the worker id is one of
  the bundled sample ids (`SAMPLE_PERSONA_IDS` already exists at
  `web/lib/sample-personas.ts`). Hits the sample renderer with zero
  network noise.

### F11 [P2] — Onboarding "What she does" card has a gendered card title across all worker genders

`WhatSheDoesCard.tsx:103` reads:

```tsx
<CardTitle>What she does</CardTitle>
```

Hardcoded "she" — but the demo roster includes Patrick (he), Jamal
(he), Sam (ambiguous). The same component renders on every worker's
overview. On Jamal's profile the card title still reads "What she
does."

Minor finding; the camera may not linger long enough to read the
card title vs the body copy, but a side-by-side comparison of Jamal +
Nori shows it.

- Repro: visit `/workers/018f67d0-0000-7000-8000-000000000003/overview`
  (Jamal — "he") — top card reads "What she does."
- File: `web/components/skycot/worker/WhatSheDoesCard.tsx:103`.
- Suggested fix: rename to "What they do" or "How they work" —
  parallel to the personality card's "HOW {NAME} WORKS" pattern.

### F12 [P2] — Welcome step's "Hire your first teammate" hero ≠ Signup page's "Hire your first AI teammate" hero

The signup page renders `<h1>Hire your first AI teammate.</h1>`
(line 158 of `web/app/auth/signup/page.tsx`). The welcome page
renders `<h1>Hire your first teammate.</h1>` (per the
`heroSegments` prop in `web/app/(setup)/onboarding/welcome/page.tsx:22-25`).
The "AI" inclusion / omission is inconsistent.

Both are correct in isolation; the inconsistency is the friction
("did SKYCOT just stop using 'AI' on purpose?"). The cleanest
unification is to drop "AI" from the signup hero — the wizard's job
is to onboard a teammate, the camera already saw "AI" 18 times on
`/landing`.

- Repro: navigate `/landing` (h1 includes "AI teammate") → `/auth/signup`
  (h1 reads "Hire your first AI teammate.") → `/onboarding/welcome`
  (h1 reads "Hire your first teammate.").
- File: `web/app/auth/signup/page.tsx:158-162` (signup hero).
- Suggested fix: harmonize on "teammate" (drop the redundant "AI"
  on the signup page, since `/landing` has done the framing already).

### F13 [P2] — Pick-vertical wizard renders 5 cards from `SEED_VERTICALS` (auth-bypass), 7 cards from `FALLBACK_VERTICALS` (gateway-offline), or N cards from gateway — three parallel sources of truth

`fetchVerticals()` in `web/lib/api/verticals.ts:226-243`:

- If `isAuthBypassed()`: returns 5 hardcoded `SEED_VERTICALS`
- If gateway returns empty: returns 5 hardcoded `SEED_VERTICALS`
  (`mock-offline` source)
- If gateway returns rows: uses gateway

But the pick-vertical page (`web/app/(setup)/onboarding/pick-vertical/page.tsx:108-158`)
has its OWN `FALLBACK_VERTICALS` list with 7 entries. The page only
uses its fallback if `catalogRows.length === 0` AFTER the hook
returns, which never happens because `useVerticals()` short-circuits
to `SEED_VERTICALS` (5 entries) in auth-bypass.

So the wizard renders 5 cards (finance-ar, gtm-engineer,
customer-support, marketing-intern, sales-sdr) — and the page's
own 7-entry FALLBACK is dead code. Marketplace-seller and recruiter
are in the page's fallback but not in SEED_VERTICALS, so they don't
appear in demo mode — meaning Mariah's `marketplace-seller` vertical
+ Priya's `recruiter` vertical are unreachable from the wizard.

This collides with F1's `VERTICAL_TO_SAMPLE_WORKER` mapping (which
includes marketplace-seller + recruiter). Operator can never pick a
vertical that redirects to Mariah or Priya; they're stuck with
Nori (marketing-intern), Jamal (customer-support), or the
DEMO_FALLBACK to Mariah for any unmatched pick (i.e. gtm-engineer,
sales-sdr, finance-ar — all of which fall through to
DEMO_FALLBACK_WORKER = Mariah).

So the camera's reachable demo destinations are: Nori (if user picks
marketing-intern), Jamal (if user picks customer-support), Mariah
(everything else). Priya is unreachable via the onboarding flow.

- Repro: visit `/onboarding/pick-vertical` in demo mode → 5 cards
  rendered (none are marketplace-seller or recruiter). Click any
  of finance-ar / gtm-engineer / sales-sdr → step 5 → land on Mariah
  regardless.
- File: `web/lib/api/verticals.ts:124-197` (SEED_VERTICALS),
  `web/app/(setup)/onboarding/pick-vertical/page.tsx:108-158`
  (FALLBACK_VERTICALS — dead code in demo mode),
  `web/lib/onboarding/use-first-hire.ts:121-127`
  (VERTICAL_TO_SAMPLE_WORKER mapping — includes 4 verticals, only 2
  of which appear in the wizard).
- Suggested fix: reconcile to one list. Either expand SEED_VERTICALS
  to include marketplace-seller + recruiter (so all 4 mapped
  verticals are pickable), or trim VERTICAL_TO_SAMPLE_WORKER to
  match SEED_VERTICALS (so the user can't pick a vertical that
  redirects to "Mariah" without the card being marketplace-seller).

### F14 [P2] — Wizard never visibly indicates "demo mode" except via the inline copy on step 3

The `DemoBanner` component (the lavender top-of-shell strip reading
"Demo mode — showing sample data") is mounted only in `AppShell`
(`web/components/skycot/AppShell.tsx:42`). The wizard pages use
`WizardEditorialShell` not `AppShell`, so the demo banner is not
shown anywhere in the 5-step flow. Only step 3 (connect-provider)
mentions demo mode inline ("Demo mode — workers run with a mock
provider. Type any sk-ant- key to continue; no Anthropic call is
made.").

A user who lands on `/onboarding/welcome` via the secondary
"Or try a demo workspace" CTA never gets a clear "this is demo
mode" indicator until step 3. If they made it past F4 (the stale
env var) by clicking the secondary dashed button, they have only the
button's label ("…no email required") as the cue.

- Repro: visit `/onboarding/welcome` via the demo-mint path → no
  `data-testid="demo-banner"` mounted, no other indicator.
- File: `web/app/(setup)/onboarding/_OnboardingShell.tsx`
  (wraps WizardEditorialShell — should mount DemoBanner above it).
- Suggested fix: surface `<DemoBanner />` at the top of every
  `OnboardingShell` page so the demo state is visible across the
  flow, matching the in-app shell.

### F15 [P2] — Pick-vertical "demo banner" + step-3 inline copy + future wizard-level banner = three competing demo-mode indicators

Step 4 (`/onboarding/pick-vertical`) renders its own demo banner
("Demo mode — install deferred. Picking a vertical loads its
workflows and skills as samples; nothing is provisioned to the
gateway until you connect a real provider in Settings.") just below
the back/back-CTA row. Step 3 (`/onboarding/connect-provider`) renders
"Demo mode — workers run with a mock provider…" inside the
subtitle prop. If F14 lands (`DemoBanner` mounted globally on the
wizard), there'll be 3 demo-mode indicators on step 4: the global
banner, the inline subtitle, and the step-4-specific banner.

Pick one. The global `DemoBanner` is the right level (matches the
in-app shell).

- Repro: visit `/onboarding/pick-vertical` in demo mode — step has
  its own banner.
- File: `web/app/(setup)/onboarding/pick-vertical/page.tsx:295-322`
  (step-specific banner), `web/app/(setup)/onboarding/connect-provider/page.tsx:142-153`
  (subtitle copy).
- Suggested fix: once F14 lands the global banner, delete the
  per-step ones.

### F16 [P2] — Wizard's per-step page titles all read "SKYCOT — Hire your first AI teammate" — re-confirmed from prior audit

Every wizard step returns the same `<title>`:

- `/onboarding/welcome`         → "SKYCOT — Hire your first AI teammate"
- `/onboarding/workspace`       → "SKYCOT — Hire your first AI teammate"
- `/onboarding/connect-provider`→ "SKYCOT — Hire your first AI teammate"
- `/onboarding/pick-vertical`   → "SKYCOT — Hire your first AI teammate"
- `/onboarding/first-hire`      → "SKYCOT — Hire your first AI teammate"

A user with the browser-tab row open during the recording can't tell
which step they're on; the title doesn't include the step number or
label even though the in-page step strip carries `01/02/03/04/05`.
Also an accessibility miss — the screen-reader tab announcement is
the only progress signal for users moving between tabs.

- Repro: `for p in welcome workspace connect-provider pick-vertical
  first-hire; do curl -s -H "Cookie: ..." http://localhost:3000/onboarding/$p
  | grep -oE '<title>[^<]+</title>'; done` — same title, 5 times.
- File: none of the `web/app/(setup)/onboarding/*/page.tsx` files
  export `metadata = { title: ... }`.
- Suggested fix: each step page exports
  `metadata = { title: 'Step N of 5 · <Label> · SKYCOT' }`.

### F17 [P3] — Genesis pillar on welcome step still reads "Voice and SOP uploads coming next" — operator memory says Genesis IS the AI teammate onboarding studio

Per memory `feedback_genesis_is_ai_teammate_onboarding_studio`
(2026-05-25), Genesis is the multi-modal onboarding studio (voice +
SOP + Slack + comprehension). The welcome step's middle pillar
reads "Describe the role in plain English; Skycot synthesises charter,
action vocabulary, memory schema, and eval suites. Voice and SOP
uploads coming next." The phrasing buries the lede — "coming next"
implies a stub, when in fact Phase 1 is text-only-but-real.

For the investor narrative, this pillar should read like the
flagship (and the "Try Genesis studio →" link below it should be a
proper CTA, not a tiny underline).

- File: `web/app/(setup)/onboarding/welcome/page.tsx:46-54`.
- Suggested fix: lead with what Genesis DOES today, defer "Voice +
  SOP uploads" to the link target.

### F18 [P3] — Welcome step's `Have a stack of role docs to upload? Try Genesis studio →` is a thin underline link, not a card/CTA

The "Try Genesis studio →" affordance is rendered as a single
underlined-on-hover link below the Get Started CTA. The text is small
and easy to miss. If Genesis is the investor-narrative flagship per
memory, this should be a tertiary button or at least a more
prominent CTA chip.

- File: `web/app/(setup)/onboarding/welcome/page.tsx:76-88`.
- Suggested fix: promote to a `<button class="ob-cta-ghost">` styled
  affordance OR remove the discoverability — but don't leave it as
  a small underline.

## Resolved since prior audit (2026-05-27-0819)

- **F7 (per-step page titles all identical)** — still present in
  this run; carry-over (now my F16).
- **F6 (`/help` → 404)** — RESOLVED. `/help` now 307s to `/docs`.
- **F9 (welcome headline contradicts singular-teammate vs
  workforce)** — RESOLVED. Welcome reads "Hire your first
  teammate." (singular, consistent with the metaphor) on current
  main.
- **F14 (worker-overview permissions all false)** — RESOLVED.
  Gateway returns `can_archive: true, can_delete: true,
  can_edit_charter: true, …` for the demo session, all-true. F14
  from prior audit is gone.
- **F3 (SPA `/team` calls `/api/v1/workers` which gateway didn't
  expose)** — RESOLVED. Gateway now responds 200 on
  `/api/v1/workers`. `/team` resolves correctly.
- **F18 (Genesis pillar `role="link"` div without visual cue)** —
  RESOLVED. The welcome step's middle pillar is now a static `<li>`,
  and Genesis is reachable via a proper `<Link>` below the CTA row.
- **F2 (gateway first-hire second-call returns 201 but doesn't
  persist)** — NOT VERIFIED. Demo-mint principals can't satisfy the
  gateway's `X-Argo-Principal: human:<email>` requirement, so I
  couldn't reproduce. Believed still present but unverifiable from
  this run without a magic-link verified principal.

## Orientation score

**56 / 100.**

The editorial design (F1 locked across 5 steps), the brand voice, the
demo-banner on the app shell, the worker-detail C2-personality card,
and the post-hire trajectory are all polished and recognizable. F14
(permissions all-false) shipped, F9 (workforce-vs-teammate copy)
shipped, F6 (`/help` 404) shipped — three of the prior audit's P1s
landed. The post-hire `/workers/<id>/overview` now has correct
RBAC for the owning principal AND a workable charter copy for at
least the 5 demo personas.

The deduction is heavy because the function side — the actual "hire
a teammate" verb that the marketing funnel walks through — fails
for the load-bearing investor demo beat. F1 (prompt → wrong worker)
is the same dead-end as the prior audit and is the headline lie of
the funnel. F2 (two parallel hire wizards, marketing routes at the
stubbed one) is an architecture finding that explains why F1 hasn't
been fixed — the working hire path exists at `/hire/*`, just not on
the marketing funnel. F3 (Acmeco · Owner hardcoded for every demo
user) is a third-tier camera-visible contradiction.

For the **POC investor demo MP4**, the recordable workaround is to
type a description matching the persona name the chosen vertical
will redirect to (e.g. for marketing-intern, type "Nori the
marketing intern…"). The narration "we hire Nori, let's see what
she's doing" then matches. Narrow workaround; the wizard's example
prompt text actively pushes the operator off this path.

For a public-release demo (out of scope per the function-vs-dressing
rule but worth mentioning): the two-parallel-wizards architecture
(F2) is the deeper fix. The correct move for v1.1 is probably to
delete `/onboarding/*` (the stub flow) and route every signup CTA at
`/hire`, optionally with `?workspace=&provider=&vertical=` query
params if the post-signup workspace + provider + vertical setup
matters enough to keep as a pre-flight.
