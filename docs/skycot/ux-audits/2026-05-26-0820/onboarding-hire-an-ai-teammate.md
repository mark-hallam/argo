# SKYCOT e2e UX audit — Onboarding "hire an AI teammate" journey

**Audit date**: 2026-05-26 08:20 AEST (Melbourne)
**Auditor**: Claude (e2e UX journey agent, headless — curl + Read; no browser MCP)
**Scope**: `/landing → /auth/signup → /onboarding/{welcome,workspace,connect-provider,pick-vertical,first-hire} → /workers → /workers/{id}` — the operator's primary investor narrative
**Setup**: dev server `http://localhost:3000` UP, gateway `http://127.0.0.1:7400` UP (schema v81 FRESH). `.env.local` has `ARGO_GATEWAY_AUTH_BYPASS=1` but **NOT** `NEXT_PUBLIC_ARGO_DEMO_MODE=1` (operator-relevant — see F1).
**Prior audit**: `2026-05-25-1100-onboarding-hire-an-ai-teammate.md` (16 findings, 4 P0). Several P0s from there re-tested; status noted in §Resolved.

---

## TL;DR

**17 findings**: 5 P0, 7 P1, 4 P2, 1 P3.

The pages all render (200s across all 7 routes), and the gateway exposes 3 of the 4 onboarding endpoints with real auth and real DB writes — but **two load-bearing bugs invalidate the entire narrative**:

1. **`/api/v1/team` reads from a CWD-derived tenant**, but `/api/v1/onboarding/first-hire` writes to a **per-principal tenant**. The freshly-hired worker is invisible everywhere in the SPA. The whole point of the investor demo — "I hired Maya and there she is on my team" — does not actually work end-to-end. (F1 / P0, function-side, root-cause)
2. **The worker-name parser grabs the first capitalised token**, so the wizard's own example prompt ("A marketing intern named Maya...") produces a worker called **"A"** at the gateway. The example the UI primes the user with creates a broken row. (F2 / P0, function-side)

On top of that, the destination `/workers/{newhire-id}` page never resolves: gateway 404 on `/api/v1/workers/<id>` → sample-data fallback misses → React Query throws `WorkerNotFoundError` → no error boundary, so the page sits on "Loading" with no end-state. (F3 / P0)

The wizard pages themselves are beautifully built — F1 editorial-center is locked, the Audiowide hero + step pip strip + SAVE & EXIT chrome render correctly, validation gates each Continue button, zustand persists draft state across reload, and the brand-tone is genuinely warm ("Build your AI workforce", "you'll have hired your first AI teammate"). The metaphor works. The plumbing past the wizard does not.

---

## Headline blockers

| # | Sev | Finding | Investor-demo impact |
|---|-----|---------|----------------------|
| F1 | P0 | `first-hire` writes to per-email tenant; `/api/v1/team` reads from CWD tenant → new hire invisible everywhere | "Show me the team" is empty. The narrative breaks the moment the operator leaves the wizard. |
| F2 | P0 | `name_from_description` produces "A" from the wizard's own example prompt | First hire is called "A". The team page header reads "A". The audit row reads "A". |
| F3 | P0 | `/workers/{newhire-id}` never resolves — sits on "Loading" forever | Wizard's redirect target shows no content. Operator stares at a spinner after pressing "Hire them". |
| F4 | P0 | Magic-link signup requires SMTP; `NEXT_PUBLIC_ARGO_DEMO_MODE=1` is not set in `.env.local` | Operator cannot get past `/auth/signup` headlessly. Demo can't be recorded without env-flip + restart. |
| F5 | P0 | `/onboarding/connect-provider` returns HTTP 500 (not a graceful error) when `SKYCOT_WORKSPACE_ROOT_KEY_BASE64` is unset and any plausible key is submitted | A user typing "sk-ant-blahblahblah1234567" gets a generic gateway failure instead of "Anthropic rejected the key". |

---

## All findings

### Pre-wizard

#### F1 [P0] — Hired worker invisible on /team because endpoints use different tenant-resolution strategies (ROOT CAUSE, function-side, demo-narrative-breaking)

`POST /api/v1/onboarding/first-hire` calls `resolve_workspace(store, &email)` at `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:251` — per-principal tenant. `GET /api/v1/team` calls `resolve_workspace_tenant(store)` at `crates/argo-cli/src/handlers/gateway/routes_team_list.rs:213` — **the gateway's `current_dir()` filename**, ignoring the principal entirely. Same pattern at `crates/argo-cli/src/handlers/gateway/billing.rs:191` and `crates/argo-cli/src/handlers/gateway/routes_cohorts.rs:207`. So when the wizard completes and writes a worker under `human:demoer@skycot.com`'s tenant, the SPA's roster page reads from the CWD-derived workspace tenant (currently has only one pre-existing Maya from prior seeding) and the newly-hired worker is absent.

Reproduction:
- `curl -X POST http://127.0.0.1:7400/api/v1/onboarding/first-hire -H 'X-Argo-Principal: human:smoke@skycot.com' -H 'content-type: application/json' -d '{"description":"Bob the SDR","vertical_id":"sales-sdr"}'` → returns `{"worker_id":"019e613c-abb5-7733-a945-6d8ae9584ffc","name":"Bob",...}`
- `curl http://127.0.0.1:7400/api/v1/team -H 'X-Argo-Principal: human:smoke@skycot.com'` → returns only `[{"id":"019e5dbb...","name":"Maya"}]`. Bob is missing.
- Cross-checked: requesting `?tenant_id=019e613b-9ca2-...` (the provision response) returns 403 `cross-tenant query not permitted`.

Suggested fix: `routes_team_list::resolve_workspace_tenant` must take `&email` like `routes_onboarding::resolve_workspace` does, or both should funnel through a single principal→tenant resolver. CWD-derived tenant is a dev convenience that has leaked into product behavior.

#### F2 [P0] — `name_from_description` derives "A" from the wizard's own example prompt (function-side, theater the camera sees)

Gateway helper `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs:232` reads the first capitalised token after whitespace/comma/period. The first-hire page placeholder + hint copy `web/app/(setup)/onboarding/first-hire/page.tsx:11` literally says **"A marketing intern named Maya who drafts blog posts and runs weekly competitive scans."** — first capitalised token is **A**, so the new worker is named "A". Operators following the suggested prompt verbatim get this.

Reproduction:
- `curl -X POST .../first-hire -d '{"description":"A marketing intern named Maya...","vertical_id":"marketing-intern"}'` → `{"name":"A",...}`
- `'{"description":"Maya is a marketing intern"}'` → `{"name":"Maya"}` (works only because Maya is first capitalised word)
- `'{"description":"draft marketing posts"}'` → `{"name":"Pioneer"}` (the silent fallback)

Suggested fix: either (a) make the parser look for `named X` / `called X` / `My X is named Y` patterns and prefer those; or (b) add a separate "what's their name?" field to the first-hire form. The current "plain English" promise + "parse anything" backend is set up for failure.

#### F3 [P0] — `/workers/{newhire-id}` shows only "Loading" forever (function-side, demo destination broken)

`web/app/(setup)/onboarding/first-hire/page.tsx:51` redirects to `/workers/${res.workerId}` after a successful hire. That page mounts `WorkerOverviewContent` (`/(app)/workers/[workerId]/overview/page.tsx:26`), which calls `useWorker(workerId)` (hits `/api/v1/workers/<id>` — returns **404**) and `useWorkerOverview(workerId)`. Both fall back to sample lookups (`findSampleWorker` at `web/lib/api/workers.ts:140`, `findSampleWorkerOverview` at `web/lib/sample-worker-overview.ts:208`) keyed on hardcoded UUIDs of 6 fictional personas (`018f67d0-0000-7000-8000-00000000000{1..6}`). A real UUID like `019e613c-abb5-7733-a945-6d8ae9584ffc` doesn't match → both throw NotFound. Headless curl of the destination shows the page renders the sidebar + a single `data-testid="workers-loading"` div with no error boundary, no `Worker not found` headline, no link back to `/team`.

Reproduction:
- Hire any worker (per F1 setup), get a worker_id back.
- `curl -L http://localhost:3000/workers/<that-id>/overview` → page renders app shell + "Loading" only. No worker name in the DOM. No retry affordance.

Suggested fix: at minimum, surface `WorkerNotFoundError` in an error.tsx on the worker-detail route, with a link back to `/team`. Underlying fix is shared with F1 — wire `GET /api/v1/workers/<id>` to read from the same per-principal tenant as the writer.

#### F4 [P0] — Magic-link signup requires SMTP and there is no demo-bypass enabled in `.env.local`

`web/app/auth/signup/page.tsx:26` gates the demo-mint button behind `NEXT_PUBLIC_ARGO_DEMO_MODE === "1"`. `.env.local` sets `ARGO_GATEWAY_AUTH_BYPASS=1` but **does not** set `NEXT_PUBLIC_ARGO_DEMO_MODE`. So when the operator clicks "Send magic link" they hit `useSignup` → POST to NextAuth → SMTP send → check-email page; without a real SMTP server the magic link never arrives. The gateway *does* expose `POST /api/v1/demo/mint-principal` (returns a JWT directly — verified working: `curl ... → {"ok":true,"session_token":"eyJ..."}`), and the web proxy `/api/auth/demo-mint` also works (returns `needs_onboarding:true`) — but the UI doesn't surface either path without the env flag.

Reproduction:
- `curl -X POST http://127.0.0.1:7400/api/v1/demo/mint-principal -d '{"email":"x@y.com"}'` → 200, returns a JWT (works)
- `curl http://localhost:3000/auth/signup` → renders "Send magic link" button, no demo branch
- Operator cannot get past this without setting `NEXT_PUBLIC_ARGO_DEMO_MODE=1` and restarting the dev server

Suggested fix for demo-recording: document the demo-mint env-flip prominently in `docs/skycot/demos/v1-mission-mvp-runbook.md` (or default to `DEMO_MODE=1` in `.env.local` since the gateway already supports the endpoint).

#### F5 [P0] — Provider-key validation 500s with a raw internal error when workspace root key is unset (function-side, error UX)

`POST /api/v1/onboarding/provider-key` with any plausibly-shaped key (e.g. `sk-ant-blah-1234567890123`) returns HTTP **500** with body `{"error":{"code":"internal_error","message":"wrap key: schema validation failed: no workspace root key: set SKYCOT_WORKSPACE_ROOT_KEY_BASE64 (base64 32 bytes) or configure ARGO_CREDENTIALS_MASTER_KEY for back-compat derivation"}}`. The wizard's catch surfaces `e.message` as-is (`web/app/(setup)/onboarding/connect-provider/page.tsx:41`) — so a recording-day operator sees the raw infrastructure error in the form.

Reproduction:
- `curl -X POST .../provider-key -H 'X-Argo-Principal: human:smoke@skycot.com' -d '{"api_key":"sk-ant-totally-fake-key-xxxxxx"}'` → 500 with the raw error
- (The shape check ≥16 chars returns 400 gracefully — only the post-shape encryption path 500s.)

Suggested fix: either (a) set the workspace root key in the env that runs the gateway, or (b) catch this specific error in the handler and return 503 with a friendly "provider validation is not configured in this workspace — Skip for now is fine" message.

---

### `/landing`

#### F6 [P1] — Landing CTA wires to `/auth/signup` not `/onboarding/welcome` (correct, but worth confirming with demo runbook)

`web/app/landing/page.tsx:145, :414` both link to `/auth/signup`. That's correct for prod, but for a recording session where the operator wants to skip auth, there's no `?demo=1` shortcut or visible "skip to demo" affordance. Two CTAs to the same page is fine; just noting that demo prep needs DEMO_MODE+SMTP-skip wired before landing→signup→onboarding can be walked end-to-end.

Suggested fix: when DEMO_MODE is on, swap the landing CTA href to `/onboarding/welcome` (bypassing the signup step entirely), since the gateway demo-mint can hydrate the session lazily on first onboarding write.

---

### `/auth/signup`

#### F7 [P1] — Signup page renders "Send magic link" label even when SMTP isn't configured

The "Send magic link" CTA always renders unless `NEXT_PUBLIC_ARGO_DEMO_MODE === "1"`. There's no client-side probe that detects whether magic-link delivery actually works. A self-host evaluator without SMTP gets an irrecoverable email-pending state with no fallback.

Suggested fix: surface a small inline note ("Magic link requires SMTP — see /docs/install for self-host setup") below the form when running locally, OR provide an unconditional "Try a demo workspace instead" link to `/onboarding/welcome` that calls the demo-mint proxy.

---

### `/onboarding/welcome`

#### F8 [P2] — Wizard step strip reads "STEP 01 OF 05" but the visible label below the pips just says "WELCOME" (not "WELCOME · STEP 1 OF 5")

Step strip at `web/components/auth/WizardEditorialShell.tsx:348` renders the digits separately from the textual step label. Not broken — operator can count the pips — but the eyebrow on the *body* (`STEP TWO · WORKSPACE`, `STEP THREE · CONNECT AI PROVIDER`) doesn't match the eyebrow on welcome (`WELCOME TO SKYCOT` — no step indicator). Minor inconsistency the camera will catch on a wide shot.

Suggested fix: change welcome's `eyebrow` from `"WELCOME TO SKYCOT"` to `"STEP ONE · WELCOME"` for parity with steps 2–5.

---

### `/onboarding/workspace`

#### F9 [P1] — Provision endpoint is fully wired and idempotent (POSITIVE — keep)

`POST /api/v1/onboarding/provision` with a valid principal returns `{"tenant_id":"...","workspace_id":"...","next_step":"/onboarding/connect-provider","idempotent":false}`. A second POST with same principal returns `idempotent:true` — solid. The wizard correctly stashes both IDs in zustand. No finding — calling out so the repro is in the audit history.

#### F10 [P2] — SAVE & EXIT button discards in-progress draft and redirects to `/` with no confirmation prompt unless `unsavedHire=true`

`web/components/auth/WizardEditorialShell.tsx:310` only triggers the confirm dialog when `unsavedHire` is true — but `_OnboardingShell.tsx` never passes that prop. So on every onboarding step, clicking SAVE & EXIT (or the SKYCOT brand wordmark) takes the user straight to `/` without warning. The zustand persist keeps their draft, so they can resume — but the UX feels like data loss.

Reproduction: fill workspace name → click SAVE & EXIT → land on `/` (Approvals page). Browser-back returns to wizard with draft intact, but the user doesn't know that.

Suggested fix: wire `unsavedHire={Boolean(workspaceName || verticalId || firstHirePrompt)}` from `useOnboardingDraft` into `OnboardingShell`, OR show a toast on exit "Draft saved — you can resume from /onboarding/welcome."

---

### `/onboarding/connect-provider`

#### F11 [P1] — "Skip for now" silently makes the workspace mock-provider-only, with the warning sized too small to read at projector distance

The warning copy at `connect-provider/page.tsx:148` ("Skipping uses a mock provider. Workers will run, but responses are simulated until you add a real key in Settings.") uses the `ob-form-note` class (small, muted) and sits below the form. For an investor demo where the operator deliberately skips to avoid leaking a real API key on camera, the camera will likely not see this caveat — and the recorded video could imply the worker will work with real LLM calls when in fact it'll return simulated responses. Mild theatre risk if a viewer later wonders "did that actually call Claude?"

Suggested fix: not changing for the demo (skipping IS the demo path), but worth adding a one-line on-screen banner during the demo recording: "Mock provider used for recording — real Anthropic keys validated identically."

---

### `/onboarding/pick-vertical`

#### F12 [P1] — Vertical list is hardcoded in JSX, not fetched from the gateway's `/verticals` endpoint

`web/app/(setup)/onboarding/pick-vertical/page.tsx:24` defines `VERTICALS` as a 7-entry inline constant. The gateway exposes `/verticals` (returns 200 — verified with curl), but this page doesn't call it. Skill-count and sample-duty fields are also hardcoded; for the demo, if `install-vertical` reports `{"skills_added":5,"workflows_added":0}` but the card says "5 skills · ready to install", there's risk of drift between the card and the actual install.

Reproduction:
- `curl http://127.0.0.1:7400/verticals` → 200 (real endpoint exists)
- `grep VERTICALS web/app/(setup)/onboarding/pick-vertical/page.tsx` → hardcoded array

Suggested fix: post-MVP, fetch the vertical list + skill counts from the gateway so the labels can't drift from the install behavior.

#### F13 [P2] — Install-vertical works and is idempotent (POSITIVE)

`POST /api/v1/onboarding/install-vertical` returns `{"installed":true,"vertical_id":"marketing-intern","skills_added":5,"workflows_added":0}`. Subsequent calls with same vertical likely no-op (not tested). No finding; recording confirmation.

---

### `/onboarding/first-hire`

#### F14 [P1] — Pressing "Hire them" with a successful gateway response shows no acknowledgment before the redirect, and a failed response leaves the user on the form with a stale draft

`first-hire/page.tsx:42-64` awaits the mutation, calls `reset()` (clears the draft!), then `router.push(/workers/<id>)`. On success there's no "Hiring Maya..." toast, no "✓ Hired" confirmation — the page just navigates. Worse, on **failure** the draft is NOT reset (good) but the error copy says "Try again, or come back once the gateway is up" — yet there's no Back button to /pick-vertical and no Skip-to-team link. The operator is stuck.

Suggested fix: add a brief loading-state hero swap ("Onboarding Maya...") + on success a 600ms toast before redirect; on failure surface a "Skip to your team" CTA so the operator isn't trapped.

#### F15 [P1] — No "what their name is" field — the wizard infers from prose, with no preview

Per F2, the gateway parses a name out of the description, but the form never shows the user what name it'll pick. A simple "Their name (we'll guess from the description if you skip)" optional field with a live preview would prevent the "A" outcome entirely.

Suggested fix: add a `<input name="hireName" optional />` above the textarea + show the parsed-or-typed name in the submit button label ("Hire **Maya**" rather than "Hire them").

---

### `/workers`

#### F16 [P2] — `/workers` redirects to `/team` if the tenant has any workers, but the sidebar shows BOTH "Team" and "Workers" as separate nav items pointing at the two URLs

Sidebar wiring puts "Team" → `/team` and "Workers" → `/workers` (`web/app/(app)/workers/page.tsx:11-24` documents the redirect-to-`/team`-on-has-workers behavior). Two nav items, one of which silently redirects to the other, is a confusing IA. The "Workers" empty-state page is real (`<HireStarterPrompts />`), but the moment a worker exists, the URL silently changes under the user.

Suggested fix: pick one canonical surface. Either delete the Workers nav entry (Team is canonical) or delete the Team nav entry (Workers is canonical). The redirect logic should not be load-bearing.

---

### `/workers/{id}`

#### F17 [P3] — Worker personality data is keyed by `worker.name` against a hardcoded `PERSONALITIES` map of 5 fictional names (Mariah / Priya / Jamal / Mariah / Sam — sample)

`web/components/skycot/worker/WorkerPersonalityCard.tsx:35` defines `PERSONALITIES` as a `Record<string, PersonalityTraits>` with only 5 keys. Any worker not named exactly "Mariah"/"Priya"/"Jamal"/"Sam"/"Mariah" falls back to `DEFAULT_PERSONALITY` — generic and clearly templated ("Reliable", "Curious", "Helpful"). Acceptable in V1 (operator-locked, per the C2-personality design memory), but means a real customer's new hire never gets real personality data, only the demo personas do. Mark as P3 because the prior audit already covered it; flagging for continuity.

Suggested fix: long-term — the gateway must surface a `personality` field on `/api/v1/workers/<id>` derived from the charter + vertical + first-hire prompt. Genesis (per the memory note) is the place for this.

---

## Resolved since 2026-05-25-1100 audit

Quickly cross-checked the 4 P0s from the prior audit:

- **Prior F5** (worker-detail crash on charter-object render): could not reproduce in this audit because the upstream call never resolves at all (F3 here — different failure mode). Either fixed or masked by the new sample-fallback throwing instead of rendering. Worth re-verifying with a real existing worker (Mariah) on a configured tenant.
- **Prior F12** (`/audit/action/<id>/replay` fabrication): out of scope for this run.
- **Prior F4** (team-hero KPIs lie): out of scope; not on the onboarding path the camera walks.
- **Prior F2** (landing copy theatre): unchanged. Dressing-side per function-vs-dressing rule; not a POC blocker.

---

## Function-vs-dressing call (per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`)

- **Function-side P0s the camera DOES see**: F1 (new hire invisible on team), F2 (worker named "A"), F3 (loading-forever destination), F5 (raw 500 on provider key). All four break the recording.
- **Function-side P0s the camera might NOT see (depends on demo path)**: F4 (signup → only blocks if the recording starts before signup; mitigation = pre-mint principal off-camera).
- **Dressing-side findings (NOT POC blockers)**: F11 (mock-provider caveat copy), F17 (hardcoded personality map for 5 demo personas — acceptable per locked design).

---

## Orientation score

**41 / 100** for the onboarding journey.

The wizard *itself* is genuinely good — locked F1 editorial design renders, brand wordmark + step pips + SAVE & EXIT all wired, validation gates Continue, zustand persists state. The metaphor is right — the wizard does feel like onboarding a teammate. But the **plumbing past the wizard is broken in a load-bearing way** (F1 root cause + F2 name parser + F3 destination spin). The investor narrative cannot be recorded end-to-end as scripted without the gateway tenant-resolution fix landing first. Score reflects: pages-render = good, gateway-endpoints-exist = good, but new-worker-not-visible-anywhere + suggested-prompt-creates-broken-row + destination-page-never-loads = three independent showstoppers, each P0 on its own.

---

## Suggested fix-order

1. **F1** (tenant resolver) — single biggest unblock; one PR could fix `routes_team_list` + `billing` + `routes_cohorts` + any other CWD-tenant readers to take `&email` like `routes_onboarding` does.
2. **F3** (destination loading) — same root cause as F1; fixing F1 makes the worker visible at `/api/v1/workers/<id>` and the page resolves. Add an `error.tsx` for `WorkerNotFoundError` as belt-and-braces.
3. **F2** (name parser) — add a name field to the first-hire form (cheap) OR rewrite the placeholder to start with the name ("Maya, a marketing intern who...") as a one-character workaround.
4. **F4 + F5** (demo path) — flip `NEXT_PUBLIC_ARGO_DEMO_MODE=1` in `.env.local` + set `SKYCOT_WORKSPACE_ROOT_KEY_BASE64` on the gateway. Both are environment changes, no code.
5. Everything else is P1/P2/P3 and survives a recording.
