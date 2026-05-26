# UX audit — onboarding "hire an AI teammate" flow

- **Date**: 2026-05-26
- **Scope**: `/landing` → `/auth/signup` → 5-step wizard (welcome → workspace → connect-provider → pick-vertical → first-hire) → `/workers/{id}` C2 detail page
- **Why this audit**: this is THE investor narrative — the demo opens on landing, signup, then four-minute wizard. If the wizard climaxes on a 500 page, the investor pitch dies.
- **Method**: read-only static analysis of `web/app/(setup)/onboarding/**`, `web/app/auth/signup/**`, and the gateway handlers in `crates/argo-cli/src/handlers/gateway/routes_onboarding*.rs`, paired with live curl probes against the running gateway on `:7400`. Preview MCP captured landing rendering, then hung after the signup navigation — remaining steps were verified via source + direct gateway HTTP. The browser-side journey was previously walked via the prior 2026-05-25 audit; this audit re-runs the **function-side** verifications and finds two new P0 blockers since.

---

## TL;DR

The 5-step wizard renders beautifully (F1 editorial-center is well-implemented, brand-consistent, copy is on-voice). But **the function under the dressing is broken**: the wizard's final "Hire them" button posts to the gateway, gets HTTP 201 with a worker_id back, then redirects the user to `/workers/{that-id}` — **which returns 404 because the create-worker handler and the read-worker handler use different tenant-resolution paths**. The newly hired worker is invisible to every other UI surface (workers list, team list, audit). The user lands on a 500-styled "We couldn't load this worker" page at the moment the investor needs to see the teammate they just hired.

Tally: **14 findings — 4×P0, 4×P1, 5×P2, 1×P3**.

The headline blocker: a brand-new user finishing the wizard cannot see the worker they just hired. The runbook calls this the investor moment ("show me my teammate"); right now the demo would have to skip onboarding and start on the pre-seeded `/workers` deck.

The Genesis "AI Teammate Onboarding Studio" framing — voice/text role + SOP-PDF/wiki/Slack uploads + access-controls + comprehension verification, per memory `feedback_genesis_is_ai_teammate_onboarding_studio` — is **NOT present** in this 5-step wizard. The rich Genesis-driven flow lives at `/hire` (an entirely separate route group, `(hire)/hire`) with multi-step structured drafting, examples, redraft, and `useGenesisWorkerDraft`. The post-signup `/onboarding/first-hire` is the simple text-area path that calls a different, non-Genesis endpoint. This is the single biggest narrative gap in the investor demo: signup → onboarding lands users on the "ChatGPT-with-prompts-shaped" path, not the "AI Teammate Onboarding Studio" path that distinguishes Skycot. See F2 + F12.

---

## Headline blockers (top P0s)

| # | Severity | Finding | Scene impact |
|---|---|---|---|
| F1 | P0 | First-hire creates worker in email-resolved tenant; `/workers/{id}/overview` reads from cwd-resolved tenant → 404 right after hire. | Scene 3 (onboarding) ends on a 500-styled error page. |
| F2 | P0 | Genesis "AI Teammate Onboarding Studio" UX exists at `/hire` but is **not reachable from signup → onboarding**. The post-signup wizard uses a stripped-down text-area path that doesn't deliver the investor differentiator. | Whole-deck differentiator absent from the recorded narrative. |
| F3 | P0 | Worker name extraction parses "first capitalised word" — the wizard's own example placeholder "A marketing intern named Maya…" produces a worker named **"A"**, not "Maya". Same for every operator description that opens with a capital article ("A sales SDR named…"). | Camera sees worker named "A" on the (broken) detail page. |
| F4 | P0 | Signup page in current `.env.local` has **no demo-mode flag** (`NEXT_PUBLIC_ARGO_DEMO_MODE` is unset). The form sends a real magic-link email that requires SMTP delivery. There is no demo-bypass button for the operator. | Recording flow demands env-var tweak the runbook does not mention. |

---

## All findings, in order of journey

### F1 [P0] — Tenant-mismatch makes new hires invisible 30s after creation

**File**: `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:251` (first-hire) + `crates/argo-cli/src/handlers/gateway/routes_duties_helpers.rs:107-115` (resolve_workspace_tenant used by overview).

**What I saw**: A POST to `/api/v1/onboarding/first-hire` returns HTTP 201 with `worker_id`. An immediate GET to `/api/v1/workers/{worker_id}` and `/api/v1/workers/{worker_id}/overview` both return HTTP 404 "worker not found".

**Why**: first-hire's `resolve_workspace(store, &email)` looks up the principal's provisioned tenant by email (`routes_onboarding_helpers.rs:38`). The detail/overview endpoints use `resolve_workspace_tenant(store)` which derives tenant from `std::env::current_dir()` (`routes_duties_helpers.rs:108-114`). These resolve to different tenants in any setup where the gateway was started in a non-onboarded working directory. Confirmed live: created worker `019e61b8-da3b-78f0-be11-7df951b8f3d5`, immediately 404'd on detail GET.

**SPA consequence**: `web/app/(setup)/onboarding/first-hire/page.tsx:51` calls `router.push(\`/workers/${res.workerId}\`)`. The detail layout calls `useWorker(id)` → `fetchWorker(id)` (`web/lib/api/workers.ts:147-149`) which throws `WorkerNotFoundError` on 404, which trips `web/app/(app)/workers/[workerId]/error.tsx:23` ("We couldn't load this worker. … 500" error shell).

**Reproduction**:
- Boot the gateway from any working directory other than the one whose name matches the operator's onboarded workspace.
- `curl -X POST http://127.0.0.1:7400/api/v1/onboarding/first-hire -H 'Content-Type: application/json' -H 'X-Argo-Principal: human:demo@skycot.com' -d '{"description":"A sales SDR named Sam","vertical_id":"sales-sdr"}'` → 201 + worker_id.
- `curl http://127.0.0.1:7400/api/v1/workers/<worker_id>/overview -H 'X-Argo-Principal: human:demo@skycot.com'` → 404.

**Suggested fix**: make `routes_worker_detail::show_response` and `routes_worker_overview::show_response` resolve tenant from the principal (same path as first-hire), not from cwd. Eliminate `resolve_workspace_tenant(store: &SessionStore)` cwd-fallback variants in all read paths. Five files duplicate this cwd-fallback (`routes_duties_helpers.rs`, `routes_worker_memory_helpers.rs`, `routes_worker_trust_grade.rs`, `routes_notifications.rs`, `routes_retrain.rs`). This is a systemic data-isolation hole.

---

### F2 [P0] — Genesis "AI Teammate Onboarding Studio" is NOT in the signup → onboarding path

**Files**: `web/app/(setup)/onboarding/first-hire/page.tsx` (simple text area) vs `web/app/(hire)/hire/step/[step]/...` + `web/components/skycot/hire/Step1Land.tsx`...`Step5Connect.tsx` (Genesis-driven multi-step).

**What I saw**: Signup CTA → `/auth/signup` → wizard → `/onboarding/first-hire` is a plain textarea + "Hire them" button. The Genesis flow with examples popover, structured drafting (`Step3Drafting.tsx:60` `useGenesisWorkerDraft`), GenesisFallbackBanner, redraft, and StructuredAdjustDrawer lives at `/hire` — but **nothing in the signup or onboarding flow links to `/hire`** (verified: `grep -rn "/hire\b" web/app/(setup)/onboarding/` returns only one comment reference).

**Why this is the headline gap**: per memory `feedback_genesis_is_ai_teammate_onboarding_studio`, Genesis is "the investor moment that distinguishes Skycot from 'ChatGPT with prompts'". The post-signup wizard is precisely the ChatGPT-with-prompts shape — a textarea that sends a string and gets back a worker. There's no voice/text role definition, no SOP-PDF/wiki/Slack upload, no access-control gates, no comprehension verification — the four pieces the operator-locked memory says distinguish the product.

**Demo impact**: Scene 3 of the runbook walks the 5-step onboarding. The viewer sees a generic enterprise wizard, not "Skycot is different from every other AI tool because it onboards a teammate like a human." The headline pitch isn't on camera.

**Reproduction**:
- Walk signup → wizard → step 5. Confirm step 5 is a plain `<textarea>` with placeholder "A marketing intern named Maya who drafts blog posts…" — no upload widget, no voice option, no question card, no skill comprehension check.
- Visit `/hire` directly — confirm rich Step1Land + Step3Drafting + examples popover + Genesis call exist.

**Suggested fix**: this is a strategic call for the operator, not a bug fix. Either (a) replace `/onboarding/first-hire` with the `/hire` Genesis flow (and remove `/api/v1/onboarding/first-hire`), or (b) keep the simple path as a "skip Genesis" lane but add a primary CTA on step 4 (vertical pick) routing to the Genesis-driven `/hire?example={vertical_id}` flow. Either way, the post-signup default should be the Genesis path so the investor demo's onboarding scene IS the distinguishing moment.

---

### F3 [P0] — Worker name parsed as "first capitalised word" → "A marketing intern named Maya" becomes worker "A"

**File**: `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs:232-252` `name_from_description`.

**What I saw**: The function picks the first whitespace-/punctuation-delimited token whose first character is alphabetic + uppercase. The wizard's own example placeholder is `"A marketing intern named Maya who drafts blog posts and runs weekly competitive scans."` (`web/app/(setup)/onboarding/first-hire/page.tsx:10-11`). Live test: POSTing that exact string → worker named `"A"`. Same outcome for any description starting with "A", "An", "The", or any proper-noun verb the operator likely starts with ("Build", "Create", "Hire", "I").

**Demo impact**: investor reads the placeholder, types something natural like "An SDR named Sam who books inbound demos" — gets a worker named "An". If F1 is fixed and the user lands on the detail page, they see the C2 personality card render "An" as the teammate's name. Camera-visible disaster.

**Reproduction**: see F1 reproduction; the response payload includes `"name":"A"`.

**Suggested fix**: the parser should look for the pattern `named <Name>` first, fall back to the last capitalised proper-noun token, and as a final fallback use "Pioneer". Better still: make this a required form field in the wizard rather than fragile NLP — "What's their name?" as the first input, "What should they do this week?" as the second. The current text-area approach trades a 10-second UX win for a 100% chance of demo-blocking name garbage.

---

### F4 [P0] — `/auth/signup` is a real magic-link form in current `.env.local`; no demo-bypass for recording

**Files**: `web/app/auth/signup/page.tsx:27` (DEMO_MODE = `NEXT_PUBLIC_ARGO_DEMO_MODE === "1"`), `web/.env.local` (neither `NEXT_PUBLIC_ARGO_DEMO_MODE` nor `NEXT_PUBLIC_SKYCOT_DEMO_MODE` is set).

**What I saw**: With current env, the "Hire your first teammate" CTA on `/landing` routes to `/auth/signup`. The signup form's submit button reads "Send magic link" and calls `useSignup().submit({email})` (not the demo-mint lane). That triggers `/api/auth/signup` → gateway → SMTP send. There's no SMTP server attached in dev, so the operator never receives the email and cannot complete signup without setting the env var. The runbook (`docs/skycot/demos/v1-mission-mvp-runbook.md`) doesn't mention this prerequisite.

**Demo impact**: the operator who follows the runbook step-for-step gets stuck on `/auth/check-email?email=...` with no email. Recording aborts in Scene 3.

**Reproduction**:
- Confirm `web/.env.local` has neither demo flag set.
- Navigate to `/auth/signup` and click submit → form shows "Sending magic link…" then redirects to `/auth/check-email`.
- Check the mail server — no message because there's no SMTP configured.

**Suggested fix**: either (a) set `NEXT_PUBLIC_ARGO_DEMO_MODE=1` + `ARGO_DEMO_MODE=1` (gateway) in `.env.local` and mention it in the runbook, or (b) add a prominent "Try the demo" button on `/auth/signup` that calls `/api/auth/demo-mint` directly without requiring an env-var flip. Right now the demo-mint code path exists but is gated on a flag the operator has to set manually.

---

### F5 [P1] — Skill counts on vertical cards are hardcoded and DON'T match the gateway

**File**: `web/app/(setup)/onboarding/pick-vertical/page.tsx:24-74`.

**What I saw**: Each of the 7 vertical cards declares `skillCount: 5|6` in a hardcoded array. The gateway's `/api/v1/onboarding/install-vertical` response returns the *actual* count. Live probe:

| Vertical | Card says | Gateway installs |
|---|---|---|
| gtm-engineer | 5 skills | 8 |
| marketing-intern | 5 | 5 |
| marketplace-seller | 6 | **2** |
| recruiter | 6 | 5 |
| sales-sdr | 5 | 5 |
| customer-support | 5 | 5 |
| finance-ar | 4 | 5 |

**Demo impact**: medium — only marketplace-seller is dramatically off (claims 6, installs 2). If the operator clicks "marketplace-seller" on Scene 3, the next screen could expose the mismatch (worker shows 2 skills active, not 6 as promised). The other 6 verticals are close enough to be invisible to the camera.

**Suggested fix**: drive `skillCount` from the gateway. Either fetch `/api/v1/verticals` and merge into the card data on mount, or change the card copy to "5 skills typical · ready to install" so it's no longer a precise promise.

---

### F6 [P1] — `/api/v1/verticals` returns `{"verticals":[]}` — wizard pick-vertical is decoupled from gateway truth

**Files**: gateway `/api/v1/verticals` handler (returns empty); `web/app/(setup)/onboarding/pick-vertical/page.tsx:24-74` (hardcoded 7-vertical list).

**What I saw**: The pick-vertical step is hardcoded in client code. The gateway's verticals catalog endpoint returns an empty list. If the operator runs the wizard in a fresh tenant, the gateway can't tell the user what's available — the SPA is the source of truth.

**Demo impact**: low for the recorded path (operator picks one of the 7). High for the "skycot is enterprise" pitch (Scene 3 narration says "we install the skills, memory schema, and approval gates" — but the catalog is client-side static, which is the opposite of "the platform delivers verticals").

**Suggested fix**: populate `/api/v1/verticals` from the registered vertical packs and render `pick-vertical` from that response. Until then, add a banner like "v1 catalog · 7 verticals · pack-loaded verticals coming Q3" to align with the enterprise narrative.

---

### F7 [P1] — Hardcoded duty `daily_check_in` is a string with no handler anywhere

**File**: `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:267-273`.

**What I saw**: every first-hire creates a single duty with `entry_action: "daily_check_in"`, cron `"0 9 * * 1-5"`. A grep for `daily_check_in` across the entire `crates/` tree returns only this one line. No worker, skill, or workflow registers that action name.

**Demo impact**: medium — Scene 2 of the runbook clicks "Run now" on a duty. If the operator drills into the freshly hired worker (post-F1 fix) and tries to fire its duty, nothing executes; the audit trail will have no row from a 5-second-window run. Camera reads "duty fired but no result."

**Suggested fix**: the first-hire path should pick a real `entry_action` from the installed vertical's skill set. The simplest fix is to grab the first skill the vertical declared as "daily" and use its action name; alternatively, drop the auto-duty and require the operator to add the first duty manually in the worker detail page (which is the real "manage your teammate" loop).

---

### F8 [P1] — `useFirstHire` hook returns optimistic `worker_id` even when the worker doesn't exist downstream

**File**: `web/lib/onboarding/use-first-hire.ts:34-53`.

**What I saw**: The hook treats any HTTP 201 with a `worker_id` field as success and resolves the mutation. There's no second probe to verify the worker is actually readable. Combined with F1 (tenant mismatch), this means the wizard happily redirects to a 404'ing detail page.

**Suggested fix**: after `apiPost(/first-hire)` returns, immediately probe `apiGet(/workers/{id}/overview)`. If it 404s, surface a wizard-side error ("Hiring succeeded but the worker isn't visible yet — try refreshing in 5 seconds") instead of pushing the user to a broken page. This is a belt-and-braces fix for F1 — even after F1 is fixed, the verification is cheap and protects against future tenant-resolution regressions.

---

### F9 [P2] — "Skip for now" on connect-provider routes to a mock-provider-only experience the user isn't warned about

**File**: `web/app/(setup)/onboarding/connect-provider/page.tsx:45-49, 148-151`.

**What I saw**: the Skip button calls `setProvider(false, true)` and pushes to `/onboarding/pick-vertical`. A small note at the bottom reads "Skipping uses a mock provider. Workers will run, but responses are simulated until you add a real key in Settings." But the note is in the same visual register as the rest of the body copy — easy to miss in a recording.

**Demo impact**: low — the demo wants the operator to skip (real `sk-ant-...` keys live in 1Password). But the warning copy is buried, so a future viewer who sees the recorded demo and tries the wizard themselves may not realise they're getting mock responses.

**Suggested fix**: move the warning into a small yellow callout above the Skip button itself, with copy like "Mock provider — workers will run but responses are simulated. Add a key later in Settings." Camera reads "honest about the limitation," which is on-brand for the audit-first positioning.

---

### F10 [P2] — Provision endpoint logs `idempotent:true` even on first call — user can't tell whether a fresh workspace was created

**File**: `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs:38-66`.

**What I saw**: `resolve_workspace` calls `provision_for_user` which always returns `outcome.created` — but the workspace step's API response just says `idempotent:true` regardless. There's no signal in the response or the UI distinguishing "I just provisioned your fresh workspace" from "you already had one."

**Demo impact**: low — the operator already knows. But the wizard wastes a chance to celebrate ("Workspace created · welcome to Skycot") with no concrete visual at the step transition.

**Suggested fix**: surface `created:true|false` in the provision response payload and have the wizard show a one-line confirmation toast on the workspace → connect-provider transition.

---

### F11 [P2] — `vertical_id` in pick-vertical is sent as the `id` field but persisted via `setVertical(verticalId)` without a name

**File**: `web/app/(setup)/onboarding/pick-vertical/page.tsx:89-103`, `web/lib/onboarding/use-onboarding-draft.ts` (vertical store).

**What I saw**: the draft store retains `verticalId: string` only. The vertical's display name and skill count never propagate to step 5. Step 5's hero copy is generic ("Describe your first worker") instead of contextualised ("Your sales-SDR template is ready — describe your first SDR"). The investor narrative would benefit from referencing the chosen vertical.

**Suggested fix**: persist `verticalId` + `verticalDisplayName` + `verticalSkillCount` together. Use them in step 5's hero ("Your **Sales SDR** template is installed — now meet your first hire").

---

### F12 [P2] — Genesis Studio framing missing from welcome step copy

**File**: `web/app/(setup)/onboarding/welcome/page.tsx:36-55`.

**What I saw**: the welcome step's three pillars are "Audit-first", "Single-binary", "Enterprise-grade". These are correct-but-bland infra claims. None mentions the AI Teammate Onboarding Studio thesis (voice/text role, SOP uploads, comprehension verification — the framing locked in memory `feedback_genesis_is_ai_teammate_onboarding_studio`).

**Demo impact**: medium — the welcome page is the investor's first deep impression after signup. Right now it reads "we're a backend platform" rather than "we onboard AI teammates like humans."

**Suggested fix**: replace at least one pillar with a Genesis-Studio claim ("Onboard them like a human · voice or text · upload your SOPs · verify understanding") even before the rest of F2 is addressed. The copy lift is 30 minutes.

---

### F13 [P2] — Workflows are mentioned in install-vertical responses but invisible in the UI

**File**: `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:188-218` returns `workflows_added`, but the wizard only consumes the boolean `installed`.

**What I saw**: gateway returns e.g. `{"installed":true,"skills_added":8,"workflows_added":1}` for gtm-engineer. The wizard's `InstallVerticalResponse` type (page.tsx:9-14) reads all 3 fields but only acts on `installed`. The user is never told that 1 workflow was just provisioned for them. Scene 5 of the runbook walks `/workflows` — that scene gets stronger if step 4 had previously confirmed "we just installed the customer-research workflow for you."

**Suggested fix**: surface the install summary as a toast or step-4 → step-5 transition card ("Installed: 8 skills · 1 workflow · 4 approval gates").

---

### F14 [P3] — Genesis fallback banner exists in /hire but no analog in /onboarding/first-hire

**File**: `web/components/skycot/hire/GenesisFallbackBanner.tsx` exists; nothing analogous in `web/components/skycot/onboarding/`.

**Suggested fix**: out of scope unless F2 is resolved by porting the /hire flow into onboarding.

---

## Resolved since the 2026-05-25 audit

The prior audit at `docs/skycot/ux-audits/2026-05-25-1100-onboarding-hire-an-ai-teammate.md` is 1 day old and was scoped to a similar walk. Spot-check of the prior P0s:

- Prior P0 "first-hire route 404s" — still broken (F1 here). NOT resolved. This is the load-bearing investor break and it has survived a full conductor tick since.
- Prior P1 "vertical skill counts hardcoded" — still broken (F5 here).
- Prior P2 "welcome pillars don't pitch Genesis" — still copy as before (F12 here).

In other words: no onboarding-flow fixes have landed since 2026-05-25.

---

## Orientation score

**Visual / brand**: 87/100. F1 editorial-center is excellent; Audiowide + purple→pink gradient renders correctly on landing; warm radial bg consistent across all 5 steps; trust footer and SAVE & EXIT affordances work.

**Function under the dressing**: 31/100. The wizard is a beautiful funnel into a broken commit step. Two of the four headline blockers (F1 tenant mismatch, F3 name parsing) make the demo's climax unrecordable as scripted. F4 (no demo bypass in current env) blocks the *entry* to the flow. F2 (no Genesis Studio in onboarding) collapses the strategic narrative.

**Overall (weighted: function 2× dressing)**: **49/100** — visually-ready, functionally-not. Three of the four P0s are 1-4 hour fixes (F1 = consolidate tenant-resolution to email-based; F3 = upgrade name parser or add a name field; F4 = set env vars + document in runbook). F2 is a multi-day strategic decision but the operator memory already locks the answer — the simple text-area path needs to be replaced or routed-around-by-default.

**Recommendation**: **DO NOT** record the v1.0 demo MP4's onboarding scene until F1 + F3 + F4 land. They are blocking, well-isolated, and small. F2 is the longer-arc bet but it's the one that makes the demo a *Skycot* demo rather than a *yet-another-AI-tool* demo.
