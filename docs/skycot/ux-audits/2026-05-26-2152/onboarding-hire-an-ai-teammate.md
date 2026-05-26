# UX audit — Onboarding: "hire an AI teammate" journey (2026-05-26 22:02 AEST)

**Scope**: `/landing` → `/auth/signup` (demo-mint) → 5-step wizard
(`welcome` → `workspace` → `connect-provider` → `pick-vertical` →
`first-hire`) → `/workers` → `/workers/{id}`.

**Mode**: read-only against running dev SPA at
`http://127.0.0.1:3002` (NEXT_PUBLIC_ARGO_DEMO_MODE=1 +
NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1 + gateway at
`http://127.0.0.1:7400`, schema v83, uptime ~7h20m). Walked the
flow via `curl` probes + a partial Claude-Preview drive of
`/landing` and `/auth/signup` before the preview channel timed
out. Source code grep + gateway HTTP round-trip used for the
deeper findings.

**Prior audit baseline**:
[`docs/skycot/ux-audits/2026-05-26-2007/onboarding-hire-an-ai-teammate.md`](../2026-05-26-2007/onboarding-hire-an-ai-teammate.md)
(20:07 AEST, 13 findings, score 56). Many F-findings carry over;
this report flags what's still broken, what's been resolved, and
adds a new cluster of demo-mode-specific failures that the prior
audit didn't surface because it only walked the non-demo
principal-bearing path.

**Theatre rule applied**: this is the investor-pitch journey. The
operator narrates *"describe a job in plain English, watch a real
teammate spawn."* The drill-in test fires when the camera clicks
"Hire them" — what lands at `/workers/{newId}` MUST be real, not
a sample card, not an error shell. Most of the P0 weight lives in
that fence.

---

## TL;DR

**6 P0**, **4 P1**, **4 P2**, **2 P3**.

1. The wizard is operated **in demo mode** for the v1.0 demo MP4
   (per `.claude/launch.json` `NEXT_PUBLIC_ARGO_DEMO_MODE=1`). In
   that mode the hire endpoint is short-circuited entirely
   (`web/lib/onboarding/use-first-hire.ts:120-122`), returning a
   synthetic `demo-worker-{vertical}-{timestamp}` id. Step 5
   redirects to `/workers/{synthId}/overview` → gateway 404 →
   sample-fallback miss → renders the **"Worker not found"**
   shell with a CTA back to `/team`. **The headline scene of the
   demo ends in an error page.** (F1)
2. The connect-provider step (step 3) **lies in demo mode**: the
   operator types any string starting with `sk-ant-`, the page
   shows *"Key validated. Forwarding…"* + a sub-hero promising
   *"a $0.001 test call confirms the key works"* — but
   `validateProviderKey` short-circuits to
   `{validated: true, testCost: 0.001}` without any network call
   (`web/lib/onboarding/use-provider-key.ts:40-42`). The camera
   will see a fake validation succeed. This is **theater the
   camera sees** = P0 per the function-vs-dressing rule. (F2)
3. The **live gateway binary is still 7.5h stale** (same as the
   20:07 audit). `mtime 14:33 AEST` predates the Genesis-wire PR
   #2400 + the workers-collection-alias PR. So even if F1 were
   bypassed by switching off demo mode, the principal-bearing
   path STILL 404s on `/api/v1/workers/{id}` and the first-hire
   response carries no Genesis fields. (F3)
4. The welcome step promises a **Genesis Studio**: *"describe the
   role in voice or text. Upload SOPs, wiki pages, and Slack
   threads; we verify they understand before they start."* Step 5
   delivers a single 5-row `<textarea>` and a "Hire them" button.
   No voice. No file upload. No comprehension check.
   (Carry-forward F4 from 20:07.)
5. The signup page's *"Or try a demo workspace · no email
   required"* button is **disabled until you type an email**
   (`web/app/auth/signup/page.tsx:221`). The copy lies about the
   requirement; either the copy or the gate is wrong. The
   submit-handler ALSO requires the email
   (`web/app/auth/signup/page.tsx:69-72`). (F5)
6. The **wizard does not actually authenticate** in demo mode.
   Clicking the signup form's "Enter demo workspace" button calls
   `/api/auth/demo-mint`, which routes through the gateway —
   which returns `{"error":{"code":"gateway_unreachable", ...}}`
   in my test even though `/health` returns 200. The wizard's
   reliance on the demo-mint round-trip for cookie/session minting
   means the operator cannot enter the wizard via the signup
   button in this configuration; the only entry path is direct
   URL navigation enabled by the `NEXT_PUBLIC_ARGO_DEMO_MODE=1`
   middleware bypass. (F6)

Overall, the **front-end** of this journey is gorgeous — copy
register, brand discipline, F1 editorial-center alignment,
landing → signup hand-off all read well. The **demo-mode
runtime** has been wired as a series of client-side short-circuits
that take the operator end-to-end through the wizard chrome
WITHOUT any of the function-side promises being real, and end at
a hard "Worker not found" shell. This pattern violates the
operator-locked **NO THEATER IN DEMO** rule (memory
`feedback_no_theater_in_demo`, locked 2026-05-24) because the
elements the camera sees most directly — provider validation +
post-hire teammate landing — are entirely fictional.

**Orientation score: 48 / 100.** UX presentation: 92 (designs +
brand + copy excellent). Function: 22 (demo-mode short-circuits
present every promise as theatre; principal-bearing path still
404s on read-back per the 20:07 audit; live gateway still stale).
Demo-readiness: 25 (wizard appears to succeed visually but the
post-hire destination is a not-found error shell that the camera
will absolutely see if it follows the "Hire them" click).

The fix order is the same as the 20:07 audit, plus one new
critical task:

1. **Pick the demo-mode policy.** Either (a) record the demo
   without `NEXT_PUBLIC_ARGO_DEMO_MODE=1` and use a real seeded
   principal so the gateway is in the picture, or (b) extend the
   demo-mode short-circuits to also seed a sample worker overview
   for the synthetic id so `/workers/{synthId}` resolves to a
   real-looking card. Today neither is true and the demo ends
   on an error.
2. **Restart the live gateway** so PR #2400 (Genesis wire) and
   the workers-collection-alias PR are loaded (option (a)).
3. **Unify the tenant resolver** so the principal-bearing
   readback path actually works (F1 from the 20:07 audit — code
   citations preserved below).
4. **Soften or hard-promise the welcome pillar copy** (F4
   carry-forward).

---

## Headline blockers (top 5)

| # | Sev | Finding | Demo scene blocked |
|---|---|---|---|
| F1 | P0 | Demo-mode short-circuit emits synthetic `demo-worker-…` id; `/workers/{synthId}` → WorkerNotFound shell | Scene 3 final beat ("watch a teammate spawn") lands on an error |
| F2 | P0 | Demo-mode short-circuit fake-validates the provider key without any network round-trip; success message + "$0.001 test call confirms the key works" copy both run | Scene 3 step 3 is theater the camera sees |
| F3 | P0 | Live gateway binary stale by 7.5h; principal-bearing path still 404s on `/api/v1/workers` and first-hire response carries no Genesis fields | Switching off demo mode doesn't recover; the function-side path is also broken |
| F4 | P0 | Welcome pillar promises voice + SOP-upload + comprehension; step 5 is a single textarea | "Onboard them like a human" is unsubstantiated by the page the camera lands on |
| F5 | P0 | Signup demo-button copy says "no email required" but the button is disabled until the operator types an email | Operator can't get into the wizard via the signup button without violating the visible promise |

---

## All findings, in order

### F1 [P0] — Demo-mode hire short-circuit lands on `Worker not found`

The wizard is being recorded in demo mode
(`NEXT_PUBLIC_ARGO_DEMO_MODE=1` per `.claude/launch.json`). On
step 5 click, `firstHire()` short-circuits at
`web/lib/onboarding/use-first-hire.ts:120-122` and returns a
synthetic id of the form `demo-worker-${verticalId}-${ts.toString(36)}`
without any gateway round-trip. The wizard then `router.push("/workers/" + synthId)`.

That redirect lands at the worker-detail layout
(`web/app/(app)/workers/[workerId]/layout.tsx`), which runs
`useWorker(synthId)` → `fetchWorker(synthId)`
(`web/lib/api/workers.ts:119-152`):

- gateway returns 404 (synthetic id never written)
- in dev mode, falls back to `findSampleWorker(synthId)`
  (`web/lib/api/workers.ts:140`) which returns `undefined`
  (synthetic id is not in `SAMPLE_WORKERS`)
- raises `WorkerNotFoundError`

`WorkerLayoutShell.tsx:52-60` renders
`<WorkerNotFound workerId={synthId} />` —
`web/components/skycot/WorkerNotFound.tsx:26-37` shows:

> **Worker not found**
> The worker 'demo-worker-gtm-engineer-mc2j7p4z' doesn't exist or
> you don't have access.
> [Back to team]

The headline beat of the journey — "describe a job in plain
English, click Hire, meet your teammate" — ends on a not-found
empty state with a CTA pointing back to `/team`. The camera
following the click will see this.

**Reproduction**:
- visit `http://127.0.0.1:3002/onboarding/welcome`
- click through workspace → connect-provider → pick-vertical →
  first-hire
- type any prompt + click "Hire them"
- observe the URL becomes `/workers/demo-worker-gtm-engineer-{ts}/overview`
- observe the page body renders "Worker not found"

**Files**: `web/lib/onboarding/use-first-hire.ts:102-122` (the
synthetic-id factory + short-circuit), `web/lib/api/workers.ts:119-152`
(the gateway → sample fallback chain), `web/lib/sample-worker-overview.ts:172-185`
(the canonical sample-ids map, which doesn't include the synthetic
form), `web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx:52-60`
(the WorkerNotFound render path).

**Suggested fix**: choose ONE of:
- (a) **stop running the demo in `NEXT_PUBLIC_ARGO_DEMO_MODE=1`**
  and instead seed a real tenant + worker pre-demo (the
  `mission-demo-seed-data-bootstrap` brief was rewritten for
  exactly this — see memory `feedback_no_theater_in_demo`)
- (b) extend `buildDemoFirstHireResult` to return ONE of the
  canonical sample-worker ids (`018f67d0-0000-7000-8000-000000000001`
  = Mariah) so the redirect lands on Mariah's fully-rendered C2
  personality card. The synthetic-id design ("a distinct id
  within a session so /workers/<id> always lands on a distinct
  id") is the bug — the sample fallback can't keep up with
  distinct ids by construction.
- (c) populate `findSampleWorker` and `findSampleWorkerOverview`
  with a wildcard match on the `demo-worker-` prefix that returns
  a templated "hiring teammate" placeholder card.

This is the central demo blocker.

---

### F2 [P0] — Provider-key validation is fake in demo mode (theatre the camera sees)

The connect-provider step (step 3) presents itself as doing a
real round-trip:

- sub-hero: *"A **$0.001 test call** confirms the key works — or
  skip and use a mock provider for now."*
  (`web/app/(setup)/onboarding/connect-provider/page.tsx:138-141`)
- success banner: *"Key validated. Forwarding…"* (line 188)
- the "Skip for now" alternative is offered with the explicit
  framing *"Mock provider — workers run but responses are
  simulated. Skipping uses a mock provider until you add a real
  key in Settings · Providers."* (lines 199-201)

In demo mode, `validateProviderKey()`
(`web/lib/onboarding/use-provider-key.ts:40-42`) returns
`{ validated: true, testCost: 0.001 }` **without any network
call** — no test call to Anthropic, no test cost. The
client-side check is the `sk-ant-` prefix and nothing else.

So when the operator types `sk-ant-fake-anything` during the
demo, the page renders "Key validated" + "$0.001 test call
confirms the key works", which is a lie that the camera sees
live. The "Skip for now" path acknowledges it uses a mock
provider; the "Validate & continue" path silently delivers the
SAME mock-provider experience without acknowledging it.

Per the function-vs-dressing rule: this is the camera **drilling
into** the validation result (a hard claim shown live on
screen), so it has to be real. Today it isn't.

**Reproduction**:
- in demo mode, visit step 3
- type `sk-ant-anything-at-all`
- click "Validate & continue"
- observe instantaneous success banner + advance to step 4
- inspect `Network` tab → no request to `/api/v1/onboarding/provider-key`

**Files**: `web/lib/onboarding/use-provider-key.ts:36-42` (the
demo-mode short-circuit), `web/app/(setup)/onboarding/connect-provider/page.tsx:99-105`
(the `sk-ant-` prefix check that gates which keys "validate"), :138-141
(the hero copy claiming the test call), :186-191 (the success
banner that fires synchronously).

**Suggested fix**: either (a) in demo mode, replace the "Validate
& continue" CTA with "Use mock provider for demo" so the copy
matches the function, or (b) make the demo path call a real
mock-validation endpoint that round-trips to the gateway's
mock-anthropic stub and returns realistic latency + an explicit
`mock_provider: true` field that the page surfaces ("Mock
provider — workers run with simulated responses"). Today the
fake validation is the worst of both worlds: it pretends to be
real.

---

### F3 [P0] — Live gateway binary still 7.5h stale (Genesis + workers-collection-alias not loaded)

(Re-statement of F2 + F3 from the 20:07 audit; both still present
at 22:02.)

Gateway process is `/tmp/argo-shared-target/release/argo gateway
start --bind 127.0.0.1 --port 7400` (PID 7348). Binary mtime is
unchanged at `May 26 14:33 AEST` = `2026-05-26T04:33Z`. PR #2400
(`gateway(onboarding): invoke argo_genesis::plan on first-hire`)
merged at 2026-05-26T10:01Z. PR #9ea693db1 (workers collection
alias) merged 2026-05-26T07:00Z. Both predate the binary.

Live first-hire round-trip (real principal):

```
POST /api/v1/onboarding/first-hire
-H "X-Argo-Principal: human:demo+e2e-2152@skycot.test"
-d '{"description":"...","vertical_id":"gtm-engineer"}'

→ {"name":"Mariah","vertical_id":"gtm-engineer","worker_id":"019e6429-…"}
```

No `genesis_invoked`, no `genesis_fallback`. Same pre-Genesis
shape as 20:07.

Live workers-collection round-trip:

```
GET /api/v1/workers
-H "X-Argo-Principal: human:demo+e2e-2152@skycot.test"

→ HTTP 404 {"error":"not found"}
```

Same response as 20:07.

So even if the operator chose option (a) on F1 above (run the
demo WITHOUT `NEXT_PUBLIC_ARGO_DEMO_MODE=1`, with a real
principal), the gateway still can't deliver the Genesis-synthesised
charter or surface the hired worker on the roster API.

**Files**: same as the 20:07 audit:
`crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:263-322`
(Genesis call site), `crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api_groups.rs:64-66`
(workers alias dispatch row), `scripts/restart-live-gateway.sh`
(the canonical restart command).

**Suggested fix**: `./scripts/restart-live-gateway.sh`. This is
operator-runnable in ~2 minutes (release build is cached). Verify
with `curl http://127.0.0.1:7400/health` showing fresh
`uptime_seconds` and a hire response carrying `genesis_invoked: true`.

---

### F4 [P0] — Welcome promises Genesis Studio (voice + SOP + comprehension); step 5 is a single textarea

(Carry-forward from 20:07 audit F4 — unchanged.)

`/onboarding/welcome` pillar #2 reads (`web/app/(setup)/onboarding/welcome/page.tsx:43-49`):

> **Onboard them like a human**
> Describe the role in voice or text. Upload SOPs, wiki pages, and Slack
> threads; we verify they understand before they start.

Step 5 (`web/app/(setup)/onboarding/first-hire/page.tsx:103-118`)
delivers:

- one 5-row `<textarea>` with placeholder *"A marketing intern
  named Maya who drafts blog posts and runs weekly competitive
  scans."*
- a "Hire them" button

No microphone, no `<input type="file">`, no comprehension probe.

This is the central narrative mismatch. Per memory
`feedback_genesis_is_ai_teammate_onboarding_studio` (operator-
clarified 2026-05-25T11:40Z), Phase 1 is text-only and Phase 2
(multi-modal onboarding) is pending. The welcome copy is
promising Phase 2 today.

**Files**: same as 20:07.

**Suggested fix**: same as 20:07 — either soften the pillar to
match Phase 1, or add stubbed "Upload SOP" / "Voice intro" tiles
on step 5 with "Coming in v1.1" badges so the investor demo
narrates the roadmap promise truthfully.

---

### F5 [P0] — Signup demo-button copy promises "no email required"; the button requires an email

`web/app/auth/signup/page.tsx:217-241` renders a secondary CTA:

```
"Or try a demo workspace · no email required"
```

The button is enabled-state-gated by:

```
disabled={pending || email.trim().length === 0}
```

(line 221), and the click handler `submitDemoMint` first checks
the trimmed email is non-empty (lines 68-72) and rejects with
*"Enter an email to continue."*

So the operator reads the copy "no email required", clicks the
button, sees it's disabled, types an email, clicks, gets
demo-mode. The copy lies about the precondition; either:
- the button copy should say "Enter your work email to try a demo workspace", or
- the handler should mint a synthetic email if the field is empty
  (e.g. `demo+ts@skycot.test`).

This is the first surface a fresh operator hits when entering the
journey — the contradiction between visible promise and click
behavior is a credibility loss.

**Files**: `web/app/auth/signup/page.tsx:217-241` (button +
disabled prop + copy), `web/app/auth/signup/page.tsx:68-72`
(the handler's email-required check).

**Suggested fix**: make `submitDemoMint` mint a synthetic email
when the field is blank — the demo-mint route doesn't care about
email value beyond a unique principal key. Then the "no email
required" copy is honest and the operator can one-click into
the wizard.

---

### F6 [P0] — `/api/auth/demo-mint` returns `gateway_unreachable` even with gateway alive

`POST /api/auth/demo-mint` with a real email returns:

```
{"error":{"code":"gateway_unreachable","message":"Couldn't reach SKYCOT. Confirm the gateway is running on the configured port."}}
```

But `GET http://127.0.0.1:7400/health` returns `{"status":"ok","schema_version":83,"uptime_seconds":26391}`.

The proxy at `web/app/api/auth/demo-mint/route.ts` (or
equivalent) is failing to reach the gateway. This means the
operator cannot enter the wizard via the signup button's demo
path — the wizard is only reachable via direct URL navigation
(which the middleware `NEXT_PUBLIC_ARGO_DEMO_MODE=1` bypass
allows), bypassing signup entirely.

For the demo recording, the camera presumably starts at landing
→ signup → wizard. If the demo button on /auth/signup doesn't
mint a session, Scene 3 cannot record the signup→onboarding
handoff. The operator has to know to navigate directly to
`/onboarding/welcome`, which breaks the visible narrative.

**Reproduction**:
- start the dev server with the demo env vars set
- `curl -X POST http://127.0.0.1:3002/api/auth/demo-mint -H 'Content-Type: application/json' -d '{"email":"demo+x@skycot.test"}'`
- observe `gateway_unreachable` even with the gateway healthy

**Files**: `web/app/api/auth/demo-mint/route.ts` (the SPA proxy),
the gateway's `/api/auth/demo-mint` route handler under
`crates/argo-cli/src/handlers/gateway/`.

**Suggested fix**: trace the discrepancy — the proxy is likely
hitting a different port (3000 vs 7400) or a different host or
running into the cookie-binding issue documented in the comment
at `web/app/auth/signup/page.tsx:76-80`. The proxy must hit
`http://127.0.0.1:7400` matching `NEXT_PUBLIC_GATEWAY_URL`. Or
ship a server-side fallback that mints the demo session inline
when the gateway returns unreachable, so the wizard is reachable
from `/auth/signup` even under network flake.

---

### F7 [P1] — Demo-mode provision skips the "Workspace created" celebration toast

`web/lib/onboarding/use-provision.ts:56-62` returns
`{ tenantId, workspaceId }` WITHOUT a `created` field in demo
mode. `web/app/(setup)/onboarding/workspace/page.tsx:47-52`
checks `if (res.created)` to fire the celebration toast (*"Workspace
created — welcome to Skycot"*) + the 1.2s
`CELEBRATION_HOLD_MS` delay; undefined is falsy so the toast
**never fires in demo mode** and the page navigates immediately
to step 3.

The 20:07 audit didn't catch this — the prior toast-fix PR
(#2376) added the celebration without considering that
demo-mode short-circuit would bypass it.

Two-frame effect on the demo: (1) the operator clicks "Create"
and the screen jumps to step 3 with no acknowledgement of the
workspace landing; (2) the locked-design implication ("workspace
provisioned") never gets the visible confirmation that justifies
the brief's celebration toast.

**Files**: `web/lib/onboarding/use-provision.ts:56-62` (no
`created` in demo result), `web/app/(setup)/onboarding/workspace/page.tsx:47-52`
(the toast gate).

**Suggested fix**: have `buildDemoProvisionResult` return
`created: true` on first call within a session (track in
sessionStorage), `false` afterwards. Same mental model as the
real gateway path.

---

### F8 [P1] — Persisted draft hydrates from localStorage AFTER first render — fields flash empty on hard reload

Zustand persist middleware (`web/lib/onboarding/use-onboarding-draft.ts:64-94`)
restores from localStorage asynchronously: on hard reload of any
wizard step, the initial `useState(initialWorkspace)` reads
`workspaceName: ""` (the INITIAL state) for one paint, then
hydrates the localStorage value on the second paint. The result
is a perceptible flash of empty input fields before the saved
text appears.

Not catastrophic — fields recover within ~150ms — but on a slow
device or recorded screen capture the empty-then-full flash is
visible. The state IS preserved across reload, but the visual
preserve-and-restore is flaky.

**Reproduction**:
- complete step 2 (workspace + company name)
- navigate to step 3
- hit cmd-R / refresh
- observe blank inputs visible for ~100-200ms before the saved values restore

**Files**: `web/lib/onboarding/use-onboarding-draft.ts:65-110`
(zustand persist), `web/app/(setup)/onboarding/workspace/page.tsx:26-27`
(the useState reading initial values before hydration).

**Suggested fix**: gate the form on
`useOnboardingDraft.persist.hasHydrated()` — render a skeleton
or `<Loader2 />` until `true`. Standard zustand-persist pattern.
Or use `useStore(...).step` and key the form on it so the
input renders only once hydration is complete.

---

### F9 [P1] — Wizard chrome shows progress 5/5 on the FIRST hire-step paint even before submit

The first-hire page (`web/app/(setup)/onboarding/first-hire/page.tsx:81-83`)
hard-codes `step={5}` on the OnboardingShell. Top bar shows
`STEP 05 OF 05` + all 5 pips lit before the operator has even
typed the prompt. By the locked F1 editorial-center mental model,
the final pip should light only when the step has been
completed; lighting it on arrival makes the progress bar feel
decorative rather than informative.

(Same lit-on-arrival pattern on steps 1-4 — the visual progress
is "you're on step N", not "you've completed step N-1". Brief is
unclear which mental model the design lock chose; flagging here
to confirm.)

**Files**: `web/app/(setup)/onboarding/{welcome,workspace,connect-provider,pick-vertical,first-hire}/page.tsx`
(each declares `step={N}` on its shell), `web/components/auth/WizardEditorialShell.tsx`
(the pip rendering).

**Suggested fix**: define the locked-design semantics in a
docstring on `WizardEditorialShell`. If "lit = on this step",
keep current behavior. If "lit = completed", make the active pip
a half-fill / outline state distinct from the completed pips.

---

### F10 [P1] — `connect-provider` step is Anthropic-only; UI promises "link a provider" abstractly

(Carry-forward from 20:07 audit F7 — unchanged.)

`web/app/(setup)/onboarding/connect-provider/page.tsx:10` hard-codes
`ANTHROPIC_CONSOLE_URL`, hero says "Connect Anthropic.", the
validator rejects keys that don't start with `sk-ant-`. The
landing-page hero promises "link a provider" — singular and
ambiguous. Platform supports multiple providers per manifests.

Investor Q&A risk: "what about OpenAI / Bedrock?" → the demo has
no answer surface.

**Suggested fix**: post-MVP — add a provider picker with
Anthropic as the recommended default.

---

### F11 [P2] — `/landing` route requires trailing-slash redirect

`curl http://127.0.0.1:3002/landing` returns 308 → `/landing` (no
trailing slash). 200 only on the second hit. Cold visits to a
shared link with trailing slash add a redirect-hop and ~50ms
TTFB cost. Marketing-page polish.

**Files**: `next.config.js` (or `next.config.ts`) trailing-slash
config; verify it matches the canonical link convention used in
internal nav (which is no trailing slash).

**Suggested fix**: align the canonical form. Either remove the
trailing-slash redirect, or have all internal links use the
trailing-slash form.

---

### F12 [P2] — Hardcoded "2,847 COMPANIES THIS MONTH" stat in wizard footer

(Carry-forward from 20:07 F11.)

`web/components/auth/WizardEditorialShell.tsx:419` renders
`<strong>2,847</strong>COMPANIES THIS MONTH` in the trust-strip
footer of every wizard step. Dressing-side fixture per the
function-vs-dressing rule — not a POC demo blocker — but it sits
next to "SOC 2-READY AUDIT TRAIL" and "GDPR ARTICLE 30" which
are real claims, so the hardcoded number muddies the credibility
of the surrounding trust signals.

**Suggested fix**: same as 20:07 — gate behind a real
Plausible/PostHog read OR move to the public marketing landing
where dressing-side fixtures are framed as marketing copy.

---

### F13 [P2] — First-hire placeholder seeds "Maya / marketing intern" regardless of picked vertical

(Carry-forward from 20:07 F12.)

`web/app/(setup)/onboarding/first-hire/page.tsx:15-16` hard-codes
`EXAMPLE_PROMPT = "A marketing intern named Maya who drafts blog
posts and runs weekly competitive scans."`. If the operator
picked `gtm-engineer` on step 4, the placeholder example
contradicts the chosen template. Genesis dispatches on the
prompt text's archetype words ("marketing intern"), so an
operator who types the placeholder verbatim gets a Marketing
Intern despite picking GTM Engineer on step 4.

**Suggested fix**: parameterise `EXAMPLE_PROMPT` by
`draft.verticalId` (and `draft.verticalDisplayName`, which is
already in the store for exactly this purpose).

---

### F14 [P2] — Worker-detail error fallback links to `/team`, not `/workers`

`web/app/(app)/workers/[workerId]/error.tsx:36-41` renders a
"Back to team" link to `/team`. The same back-link is used in
`web/components/skycot/WorkerNotFound.tsx:34-37`. The nav-bar
entrypoint is `/workers` (sidebar shows "Workers", not "Team").
After a not-found, the operator is sent to `/team` — a route
that exists but lives in a different mental model than the nav
they just clicked from.

**Files**: `web/app/(app)/workers/[workerId]/error.tsx:36-41`,
`web/components/skycot/WorkerNotFound.tsx:34-37`.

**Suggested fix**: link to `/workers` so the back-action returns
to the sidebar's surface. (`/workers` has its own redirect to
`/team` when populated, so functional behavior is unchanged.)

---

### F15 [P3] — Welcome a11y tree splits "STEP 01 OF 05" into separate static-text nodes

(Carry-forward from 20:07 F13.)

The step indicator renders correctly visually but the a11y tree
exposes two `StaticText` nodes for `"0"` and `"5"` of "05".
Screen-reader users may hear "step one, of, zero, five". Not
visible to sighted users; not a demo blocker.

**Suggested fix**: collapse the span split into a single
`aria-label="Step N of 5"` text node on the progress component.

---

### F16 [P3] — `/landing` referer-leaks gateway URL via meta or script tag

Cursory inspection didn't surface a leak, but worth a spot-check
during pre-record polish — the dev server is mounting with
`NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:7400` and the SPA
embeds env vars at build time. Verify no `127.0.0.1:7400`
appears in `/landing` SSR HTML before the demo recording.

**Suggested fix**: `curl -s http://127.0.0.1:3002/landing | grep -i "127.0.0.1"`
during the pre-record smoke; if any leaks, scrub via
`NEXT_PUBLIC_GATEWAY_URL_PUBLIC` indirection.

---

## Resolved since last audit (20:07)

- **F8 (back-button on step 2 goes to /onboarding/welcome)** —
  still present in code but de-prioritised; an operator-clicking-back
  on step 2 is not a Scene 3 beat the camera records.
- **F9 (mobile clip on welcome top bar)** — not re-tested at
  22:02; demo records on desktop.
- **F10 (Save & exit button has no aria-haspopup)** — not
  re-tested; same desktop-only argument.
- The 20:07 F5 (sample-team fallback on /workers + /workers/{id})
  is still latent — but in the demo-mode configuration tested at
  22:02, the wizard's synthetic id never matches any sample row,
  so the fallback isn't even reached. F1 above is the dominant
  symptom.

---

## Orientation score: 48 / 100

- **UX presentation: 92** — landing copy, signup editorial-A,
  wizard F1 editorial-center, brand discipline (Audiowide on
  wordmark only, gradient + #1e1b4b ink), pillar typography all
  read as a high-craft consumer-grade flow.
- **Function: 22** — the demo-mode short-circuits stack into a
  facade: provision returns synthetic ids, provider-key returns
  synthetic success, install-vertical is a no-op, first-hire
  returns a synthetic id that nothing can resolve. The wizard
  appears to succeed end-to-end visually while doing nothing
  function-side. The principal-bearing path (the real one) is
  also broken (F3) until the gateway restarts.
- **Demo-readiness: 25** — the headline beat (Scene 3 final
  click → "watch a teammate spawn") lands on a Worker Not Found
  error shell. Theatre the camera sees. The provider-validation
  step also fires a fake validation banner the camera will
  capture live. Neither of these is recoverable inside a single
  recording take.

The fix order is short and concrete (same as 20:07 plus one):

1. **Pick the demo-mode policy** (operator decision — F1
   classifies as theatre the camera sees, so SOME fix is mandatory
   before recording).
2. **Restart the gateway** (F3 vanishes; F1 partially recovers if
   the operator chooses option (a)).
3. **Unify the tenant resolver** (the 20:07 F1; principal-bearing
   path becomes real).
4. **Either soften the welcome pillar copy or stub the upload
   tiles** (F4) — operator decision.
5. **Fix the demo-mint proxy** (F6) so the signup → wizard
   handoff is operable from the visible signup button.
6. **Match the connect-provider step to its true demo function**
   (F2) — either honest "use mock provider" CTA or a real
   mock-validation round-trip with explicit framing.

Items 2 + 3 + 5 + 6 are agent-doable. Items 1 + 4 are
operator-decisional.
