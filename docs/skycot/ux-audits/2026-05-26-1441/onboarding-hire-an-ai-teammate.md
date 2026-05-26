# UX audit — onboarding hire-an-ai-teammate journey

**Date**: 2026-05-26 14:41 AEST
**Journey scope**: `/landing` → `/auth/signup` (demo-mint bypass) → 5-step onboarding wizard (`welcome` → `workspace` → `connect-provider` → `pick-vertical` → `first-hire`) → `/workers/{id}` (C2-personality detail)
**Method**: code archeology + curl traces against the live gateway (`:7400`) + dev SPA (`:3000` / `:62927`). The Claude Preview Chrome renderer hung mid-walk; conclusions below are still high-confidence because they reproduce against the gateway directly with the same demo session the SPA mints, and they're rooted in the route source not in transient screen state.
**Demo-positioning lens**: POC + investor demo (function-vs-dressing rule applied — `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`).

---

## TL;DR

Five P0s, three P1s, four P2s. The Scene-3 investor moment ("plain-English in → real teammate out") is broken end-to-end in three independent ways: (1) the in-page example prompt and the runbook talk-track both start with "A marketing intern named Maya" — the gateway's name-extractor returns the first capitalised token so the worker is literally named **"A"**; (2) `POST /api/v1/onboarding/first-hire` writes the new worker into the principal's tenant via `resolve_workspace(principal_email)` while `GET /api/v1/workers/{id}` and `GET /api/v1/team` resolve a *different* tenant via `std::env::current_dir()`, so the worker is invisible to every read path that follows; (3) no Genesis call happens — the runbook's "Skycot turns that sentence into a vertical — system prompt, action vocabulary, memory schema, eval seeds" is fiction at this code path (the handler just stamps the vertical id and copies the prompt into a charter string). The provider-key step also 500s ("no workspace root key") so any operator who pastes a real key sees a server error before the "Skip" workaround is taken.

The wizard frame itself (F1 editorial-center, 5 pips, eyebrow + hero + body + trust footer + SAVE & EXIT) is implemented cleanly and matches the locked design. The breakage is in the wiring underneath, not in the chrome.

## Headline blockers

1. **F1** [P0] — first-hire name parser returns "A" when the operator types the example/runbook prompt. The whole Scene-3 "hire Maya" beat shows a worker named "A" on the camera.
2. **F2** [P0] — the newly-hired worker 404s on `/workers/{id}`, every detail tab, and the team listing. The wizard's final beat (redirect to `/workers/{id}`) lands on the `WorkerNotFound` shell.
3. **F3** [P0] — `current_dir()`-based tenant resolution on read paths (`/api/v1/team`, `/api/v1/workers/{id}`, `/api/v1/skills`) versus `principal_email`-based tenant resolution on the write path (`/onboarding/*`). Two systems, two tenants, no overlap. Mission-claim-6 cross-tenant guarantees are also at risk here.
4. **F4** [P0] — `POST /api/v1/onboarding/provider-key` returns HTTP 500 with `no workspace root key: set SKYCOT_WORKSPACE_ROOT_KEY_BASE64 …` on a fresh demo workspace. Step 3's primary path is dead in default demo env; the "Skip for now" rescue still works but the talk-track "$0.001 test call confirms the key works" can't be demonstrated.
5. **F5** [P0] — Genesis is not invoked on first-hire. The runbook narration ("Skycot turns that sentence into a real vertical — system prompt, action vocabulary, memory schema, eval seeds") doesn't match the implementation: `routes_onboarding::first_hire_response` stamps the operator-picked vertical id, copies the prompt into a charter string, and inserts one hardcoded `"0 9 * * 1-5"` duty. No prompt synthesis, no eval seeding, no memory-schema generation.

---

## All findings

### F1 [P0] — first-hire name extractor returns "A" on the example prompt

`POST /api/v1/onboarding/first-hire` derives the worker name by finding the **first capitalised token** in the description (`name_from_description` in `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs:232`). The placeholder + example caption on the form (`web/app/(setup)/onboarding/first-hire/page.tsx:11`) is `"A marketing intern named Maya who drafts blog posts and runs weekly competitive scans."`. The runbook talk track (`docs/skycot/demos/v1-mission-mvp-runbook.md`) uses the same opener: *"A marketing intern named Maya…"*. So the operator following either source verbatim hires a worker named "**A**" (token-1 of the prompt). Confirmed against the live gateway: `description: "A marketing intern named Maya who…"` → `{"name":"A","worker_id":"…"}`. Same with `"I want a marketing intern."` → `name: "I"`. Only works when the operator capitalises the worker's actual name as the first word ("Sam SDR …" → "Sam"; "Maya is a marketing intern …" → "Maya").

Reproduction:
- `argo demo-mint` to get a session token
- `curl -X POST :7400/api/v1/onboarding/first-hire -H "X-Argo-Principal: human:..." -d '{"description":"A marketing intern named Maya …","vertical_id":"marketing-intern"}'`
- Response: `{"name":"A", …}`

File: `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs:232-252` + `web/app/(setup)/onboarding/first-hire/page.tsx:10-11`.

Suggested fix: either swap to a proper NER pass (Genesis already needs to run anyway — see F5) or, for a same-day stopgap, change `name_from_description` to look for the token following `"named"` / `"called"` / `"name is"` before falling back to first-capitalised; and update the example/runbook prompts to lead with the name ("Maya is a marketing intern who…").

### F2 [P0] — newly-hired worker 404s on the detail page

The wizard's last step redirects to `/workers/${workerId}` (`web/app/(setup)/onboarding/first-hire/page.tsx:51`). That route's layout (`web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx:50-60`) calls `useWorker(workerId)` → `fetchWorker` → `GET /api/v1/workers/{id}`. Against a real new hire the gateway returns 404, the client throws `WorkerNotFoundError`, and the layout renders the `<WorkerNotFound>` component. The dev-only sample-data fallback (`web/lib/api/workers.ts:139-145`) doesn't rescue the case because the UUID isn't in `SAMPLE_WORKERS_DETAIL` (sample IDs are `018f67d0-...`, real hires are `019e...`).

Reproduction (curl-direct):
- Mint session, provision workspace, install-vertical marketing-intern, first-hire ("Maya is a marketing intern…")
- `GET :7400/api/v1/workers/<the-id-just-returned>` with the same bearer + `X-Argo-Principal`
- Response: `{"error":{"code":"not_found","message":"worker not found"}}`

File: `crates/argo-cli/src/handlers/gateway/routes_worker_detail.rs:401-410` (root cause is F3); `web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx:50-60` (renders the NotFound shell).

Suggested fix: tie the worker-detail tenant resolver to `principal_email` the same way `first_hire_response` does (call `resolve_workspace(store, &email)`).

### F3 [P0] — tenant resolver inconsistency between write and read paths

`first_hire_response`, `install_vertical_response`, and the rest of `/onboarding/*` resolve the workspace tenant from the authenticated principal's email (`routes_onboarding.rs:251`). But `routes_team_list::resolve_workspace_tenant` (`routes_team_list.rs:213-222`) and `routes_worker_detail::resolve_workspace_tenant` (`routes_worker_detail.rs:401-410`) both resolve tenant from **`std::env::current_dir()`** — the gateway-process working directory. So in a multi-user dev/demo run, every reader sees the gateway's CWD-derived tenant while writers populate their own principal-derived tenant. The two never meet.

Observable: `/api/v1/team` returns the same `019e5dbb-...` "Maya" row to every distinct demo email I tested (`ux-step3@`, `ux-name-test@`, `ux-charter@`, `distinctly-different-user@`). Newly hired workers (real ones, written under the demo email's tenant) never appear in the team list and 404 on direct fetch. Skills installed via `/onboarding/install-vertical` ("skills_added: 5") aren't queryable via `/api/v1/skills` (returns 0 in the principal tenant; whatever is in the CWD tenant is invisible to the operator's session).

This also means **claim-6 (multi-tenant)** is undermined for these specific reader endpoints — the reader contract reduces to "everyone reads the gateway's home tenant."

File: `crates/argo-cli/src/handlers/gateway/routes_team_list.rs:213-222`, `routes_worker_detail.rs:401-410` (likely repeated in the other `routes_worker_*` files — grep confirms `routes_worker_overview.rs`, `routes_worker_memory.rs`, etc. cite the same pattern).

Suggested fix: replace `current_dir()`-based resolution with `resolve_workspace(store, &principal_email)` everywhere, mirroring the onboarding write path.

### F4 [P0] — `/api/v1/onboarding/provider-key` 500s on fresh workspace

Step 3 of the wizard calls `POST /api/v1/onboarding/provider-key` with `{"api_key":"sk-ant-..."}`. Against a freshly provisioned workspace the gateway returns HTTP 500 with body:

```
{"error":{"code":"internal_error","message":"wrap key: schema validation failed: no workspace root key: set SKYCOT_WORKSPACE_ROOT_KEY_BASE64 (base64 32 bytes) or configure ARGO_CREDENTIALS_MASTER_KEY for back-compat derivation"}}
```

The operator sees their wizard error block render *"…wrap key: schema validation failed: no workspace root key: …"* — verbatim gateway 500 string. The "Skip for now" CTA still works (sets `useMockProvider: true`), so the journey is completable, but the talk-track ("a $0.001 test call confirms the key works") cannot be demonstrated, and pasting a real key looks broken in front of the camera.

Reproduction:
- Demo-mint, `/provision`, then `POST /onboarding/provider-key` with any `sk-ant-...` body.

File: handler at `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:55-130` calls `upsert_provider_key` which needs `SKYCOT_WORKSPACE_ROOT_KEY_BASE64` or `ARGO_CREDENTIALS_MASTER_KEY` in env. Demo launch (per `.claude/launch.json:9`) doesn't set either.

Suggested fix: either pre-set a deterministic demo root key in the gateway dev/demo env (mirroring the existing `NEXT_PUBLIC_ARGO_DEMO_MODE` pattern), or have the onboarding handler trap the missing-root-key error and fall back to a session-scoped in-memory wrap for demo mode with a clear "demo mode — no real key persisted" banner.

### F5 [P0] — Genesis is not invoked; the "build a vertical from one sentence" promise is unimplemented

The Scene-3 talk track promises: *"Skycot turns that sentence into a real vertical — system prompt, action vocabulary, memory schema, eval seeds."* The runbook also calls it the live-test source `crates/argo-workflow/tests/claim_3_live_genesis_to_run.rs`. But `first_hire_response` in `routes_onboarding.rs:226-307` never calls `argo_genesis::plan()`. It just:
- looks up the already-installed vertical id via `load_vertical(&vertical_id)` (vertical comes from the picker step)
- extracts a one-word name from the prompt (F1)
- builds a `HireDraft` with `charter: format!("First hire for this workspace. Operator brief:\n\n{description}")`, a hardcoded duty `"0 9 * * 1-5" daily_check_in`, no voice, no tagline
- calls `IdentityStore::create_worker_from_draft`

No prompt synthesis, no action-vocabulary generation, no memory-schema derivation, no eval seeding. The vertical was already installed (or hardcoded) by step 4; nothing about the operator's plain-English brief shapes the worker beyond its display name + a charter string.

This means **Scene 3 narrates a capability that doesn't exist at this code path**. (The capability may exist in `argo_genesis::plan()` invoked through a different surface — e.g. /hire's Genesis step — but the /onboarding/first-hire endpoint the runbook explicitly names doesn't go through it.)

File: `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:226-307`.

Suggested fix: in /onboarding/first-hire's happy path, call `argo_genesis::plan(description, vertical_id)` and persist the returned vertical artefacts + use the synthesised name/charter/duties on the `HireDraft`. Make the existing string-derived fallback a `genesis_fallback: true` audit-tagged path (matching the brief comment in `web/lib/api/workers.ts:182-196`).

### F6 [P1] — `/team` listing shows the *same* Maya row to every authenticated demo user

Side effect of F3 — the cwd-resolved tenant for the gateway's working directory is whichever one has the seed Maya in it. Three distinct demo mints all returned the identical `019e5dbb-c03d-79a0-a9a4-ee60bb3c8218` row to `/api/v1/team`. For Scene 1 (which the demo records before Scene 3), this is dressing-side OK since the rendered roster name/avatar matches what Scene 1 narrates (Mariah/Peta/etc). But it means the "I just hired Maya" beat at the end of Scene 3 has no carry-over to subsequent scenes — Maya doesn't appear in `/team` post-hire.

Reproduction: same as F3.

Suggested fix: see F3.

### F7 [P1] — `/landing` "Hire your first teammate" CTA bypasses the demo-mint button

The /landing CTA (`href="/auth/signup"`, 3 instances of the same text) drops the operator into the signup page where they have to (a) click the SSO row, OR (b) enter an email and choose between "Send magic link" or the dashed-border "Or try a demo workspace · no email required" button. For investor demos the operator wants one click to demo, not three. The dashed border + secondary-button styling makes it look like an escape-hatch (which the code comment confirms: *"so the demo path is discoverable in local + preview environments without the operator having to remember the env-var dance"*).

When the gateway is running with `ARGO_DEMO_MODE=1` (which it is in the demo path), the primary submit button DOES become "Enter demo workspace" and uses the same mint endpoint — but it still requires an email-keystroke. Demo-mode signup HTML I curled didn't show the env was wired through (DEMO_MODE banner attr was present but the primary button still required `email.length > 0` before enabling).

Reproduction:
- Open `/landing` in demo-mode env → click "Hire your first teammate" → land on `/auth/signup` → primary CTA disabled until email field has content.

File: `web/app/auth/signup/page.tsx:200-216` (CTA disabled gate), `web/app/landing/page.tsx` (CTA target).

Suggested fix: when `NEXT_PUBLIC_ARGO_DEMO_MODE === "1"` the landing CTA could deep-link to `/auth/signup?demo=1&email=investor%40skycot.com` (auto-fill + auto-submit) — single tap to demo. Or change the primary `disabled` gate in DEMO_MODE so the button is always enabled and synthesises a `demo@<workspace>.local` email on submit.

### F8 [P1] — Step 3 success state is invisible because of the immediate router.push

`connect-provider/page.tsx:34-39` sets `setSuccess(true)` then immediately `router.push("/onboarding/pick-vertical")`. The "Key validated. Forwarding…" banner (line 109) never has a chance to render before the navigation kicks. The talk-track *"…confirms the key works"* relies on the green check-mark being visible to camera. A 600-900ms delay before push would give the operator (and camera) time to see the validation succeed.

Note: this is moot until F4 is fixed (the endpoint currently 500s so success state is unreachable).

File: `web/app/(setup)/onboarding/connect-provider/page.tsx:34-39`.

Suggested fix: `await new Promise(r => setTimeout(r, 700))` between `setSuccess(true)` and `router.push(...)`; or replace with a CSS-animated state transition that holds for ~1s.

### F9 [P2] — Charter content is a templated string, not a worker-shaped persona

`first_hire_response` builds the charter as: `format!("First hire for this workspace. Operator brief:\n\n{description}")` (line 262). For Scene 3's "Skycot writes the system prompt from your sentence" beat, the charter that lands on the worker is literally the operator's prompt with a 35-char preamble. If the camera drills into the worker's charter card on `/workers/{id}` (e.g. via the C2-personality tab) it would see this string — that's function-side fabrication (camera-visible). Caveat: F2 means the operator can't drill in at all currently.

File: `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:262`.

Suggested fix: see F5 — Genesis should synthesise a real charter.

### F10 [P2] — Duty is hardcoded; vertical's `sampleDuties` chip labels don't carry through

`pick-vertical/page.tsx:30,32,37,...` lists each vertical's `sampleDuties` (e.g. marketing-intern: "Draft blog posts", "Schedule social posts", "Weekly competitive scan"). These show on the role cards as `<span class="ob-skill-chip">`. After hire, the worker has a single hardcoded duty `"0 9 * * 1-5"` `"daily_check_in"` — the chip-labelled duties don't materialise as the worker's actual scheduled work. If the camera then visits the worker's Schedules tab, it sees one cron-shaped daily-check-in unrelated to the sampleDuties shown a screen ago.

File: `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:267-273`.

Suggested fix: derive duties from the picked vertical's manifest (1 row per vertical's `sampleDuties` entry, with a placeholder cron the operator can edit on the worker-detail Schedules tab).

### F11 [P2] — F1 wizard hero hardcoded "2,847 COMPANIES THIS MONTH"

Visible on every onboarding step (`wiz-foot` row). Dressing-side fixture under the function-vs-dressing rule, so **not a POC blocker** — the camera shouldn't drill into it. Flagging for completeness because the operator explicitly called this out as part of an earlier 87-finding audit and the `feedback_no_theater_in_demo` clarification subsequently re-allowed it. Recommend leaving as-is for the POC; revisit before public launch.

File: `web/components/auth/WizardEditorialShell.tsx` (the `wiz-foot` row).

### F12 [P2] — Signup-page secondary demo button is hidden in production, but its dashed-border styling looks broken in dev

`signup-demo-mint` button (lines 217-243) is a `<button>` with `border: "1px dashed rgba(30, 27, 75, 0.35)"` — looks like a placeholder/unstyled element next to the polished CTA above. This is the primary entry path for investor demos. Suggest promoting it to first-class look (gradient outline or filled-secondary) so demo recordings don't accidentally show a "looks-broken" button.

File: `web/app/auth/signup/page.tsx:217-243`.

Suggested fix: use the same brand gradient as the primary CTA but with `background: transparent`, or just style it like an ob-cta-ghost equivalent.

### F13 [P3] — `Tip: add an environment suffix like "· production"` is internal-language

Workspace step's hint (line 84) suggests a dev/SRE pattern that an enterprise buyer or investor wouldn't recognise. Not a blocker for the POC (camera is fast on this step). Consider rephrasing as "If you have multiple workspaces (e.g. Acme Co · sales, Acme Co · ops), add a suffix here."

File: `web/app/(setup)/onboarding/workspace/page.tsx:84-86`.

### F14 [P3] — Browser sees lots of `net::ERR_FAILED` against `:7400` while iterating pages

In the preview Chrome session I drove, many in-app routes background-poll the gateway with CORS-preflight + `GET` that come back as failures (`/api/v1/skills`, `/api/v1/approvals`, `/api/v1/team`, `/api/v1/notifications`, `/api/v1/meta/error-reporting`). The OPTIONS preflights all returned 204 successfully — the failures are in the *follow-up* GETs. The same URLs work via `curl` from the same machine with the same bearer, so this is likely a per-tab cookie/CSP edge or a Chrome connection-coalescing quirk under prefetch load, not a true gateway bug. Worth a quick smoke test on the demo machine before recording (Network tab on `/workers` during a cold load) since a wall of red 4xx/5xx in DevTools-open is camera-visible.

## Resolved since last audit

(No prior audit in this directory.)

## Orientation score

**52 / 100.**

The chrome is locked-design-perfect: the F1 editorial center renders cleanly, the 5-pip progress bar tracks correctly, the SAVE & EXIT affordance works, the SSO row + magic-link form + dashed demo-mint button all hang together visually, and the marketing landing has a clean hand-off into `/auth/signup`. That's the +50 floor.

But the **function the camera will narrate** doesn't match the **function the code implements** at three structural points (F1 name extraction, F3 tenant inconsistency, F5 no-Genesis-call) and one hard 500 (F4). All four are function-side per the POC/investor-demo rule, all four are camera-visible during Scene 3, and three of them are independent — fixing any one doesn't unblock the other two. Until those are addressed, the wizard demo can't be recorded as the runbook is currently written — operator would need to either (a) shoot Scene 3 with a pre-seeded worker via `argo demo seed` and a "ceremonial" wizard walk-through that ends on a different page, or (b) ship targeted fixes to the three structural items plus a same-day fix to the example-prompt opener.

The +2 above 50 is for the breadth of locked design that's already wired (every step has the right shell, the right copy, the right testids; the demo-mint endpoint actually works at the SPA layer; the SsoButtons row is real; the WorkerNotFound shell at least *exists* rather than crashing the app).
