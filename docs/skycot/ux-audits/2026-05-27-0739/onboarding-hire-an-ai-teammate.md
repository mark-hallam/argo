# UX audit — Onboarding hire-flow (the AI-teammate metaphor end to end)

Run: 2026-05-27, conducted by `e2e-ux-journey-test-agent`.
Scope: `/landing` → `/auth/signup` (demo-mint bypass) → 5-step onboarding wizard (`welcome` → `workspace` → `connect-provider` → `pick-vertical` → `first-hire`) → `/workers` → `/workers/{id}/overview`.
Mode: SPA started with `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1 NEXT_PUBLIC_ARGO_DEMO_MODE=1` (the recording-prep posture).
Method: live preview drove `/landing` → `/auth/signup` then preview wedged on the eval/click tools mid-walk; from there read-only source-trace of every step's submission path, gateway endpoint, and storage write to determine whether the "hire an AI teammate" metaphor is real or staged. No cargo, no edits.

## TL;DR

- 14 findings · **3 P0 · 4 P1 · 5 P2 · 2 P3**.
- The hire-flow in demo mode is **almost entirely theater the camera will see** (function-side fabrication). The user's typed worker description is silently discarded; they are redirected to one of four fixed canonical sample workers (Mariah/Priya/Jamal/Nori) based on the vertical they picked. None of the five wizard inputs (workspace name, company name, provider key, vertical, first-hire prompt) reach the gateway. The wizard then `reset()`s its draft, so even the local copy is gone.
- The non-demo (real auth) path looks correctly wired end to end (`/api/v1/onboarding/{provision,provider-key,install-vertical,first-hire}` are real endpoints; first-hire invokes `argo_genesis::plan` and persists a charter, duties, voice_profile). The investor-narrative payload is real *when the gateway is reached* — but the demo recording will not reach it on the current code path.
- Biggest blocker: F1. If the camera follows Scene 3 of the runbook (user types "A marketing intern named Maya …"), it will land on `/workers/<nori-uuid>/overview` showing **"How Nori works"**, not Maya. Function-side fabrication.

## Headline blockers

- **F1 [P0]** First-hire prompt is silently discarded in demo mode; user redirected to a fixed sample worker keyed off `verticalId`. The "Maya" example becomes Nori.
- **F2 [P0]** Worker the user "hired" does NOT appear on `/workers` (which redirects to `/team`). The roster is the same 5 sample workers — no append, no highlight, no "your new hire" badge.
- **F3 [P0]** Sample-worker `Nori` (the marketing-intern fallback) has **no entry** in `WorkerPersonalityCard.tsx`'s `PERSONALITIES` map. A demo user who picks Marketing Intern lands on a worker page that renders generic `DEFAULT_PERSONALITY` ("Reliable / Curious / Helpful") — sterile, off-brand, and the C2-personality design lock is broken specifically for the example prompt the wizard suggests.

## All findings, in order

### F1 [P0] — First-hire prompt is silently discarded in demo mode

The wizard tells the user "Plain English is fine. Give them a **name, a role, and what they should do this week**" with placeholder *"A marketing intern named Maya who drafts blog posts and runs weekly competitive scans."* When the user types and submits, `firstHire(req)` checks `isDemoMode()` and short-circuits to `buildDemoFirstHireResult(req.verticalId)` — a hardcoded `Record<verticalId, {id, name}>` that returns one of Mariah / Priya / Jamal / Nori. The user's `description` parameter is ignored. The user's intended worker name is never sent, never stored, never shown.

- Repro:
  - `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 pnpm --dir web dev`
  - Walk to `/onboarding/first-hire`, pick "Marketing Intern" upstream.
  - Type `A marketing intern named Maya who drafts blog posts`. Submit.
  - Land on `/workers/018f67d0-0000-7000-8000-000000000004/overview` — that's Nori's id.
- Code: `web/lib/onboarding/use-first-hire.ts:120-154` (`VERTICAL_TO_SAMPLE_WORKER` + `buildDemoFirstHireResult` + the `isDemoMode()` short-circuit at line 152).
- Suggested fix (do NOT implement here): append the synthesised hire to `appendDemoWorker()` (the same store `/hire` already uses — `web/lib/api/demo-roster-store.ts`) so the user's name + role + description survives onto `/team` and `/workers/<id>/overview`, mirroring the post-Genesis behaviour on the real path.

### F2 [P0] — Hired worker does not appear on `/workers`

`/workers` (the page the runbook lands on after hire) immediately `router.replace("/team")` whenever `workers.length > 0` (which is always — `SAMPLE_TEAM` has 5). `/team` reads `useTeam()` which in `auth-bypass` mode reads `SAMPLE_TEAM` + `listDemoWorkers()`. The onboarding wizard does NOT call `appendDemoWorker(...)`, so the hire is **never** in `listDemoWorkers()` and the roster is unchanged. Camera narrating "and there's your new teammate" will see the same 5 fixed cards as before onboarding.

- Repro: complete demo onboarding, then navigate to `/workers` — count of cards is 5 (matching `SAMPLE_TEAM`) regardless of what you typed.
- Code: `web/app/(app)/workers/page.tsx:25-27` (redirect); `web/lib/api/team.ts:60-71` (roster splice); `web/lib/onboarding/use-first-hire.ts` (no `appendDemoWorker` call).
- Suggested fix: in the demo short-circuit, before redirect, call `appendDemoWorker({worker, overview})` so the roster splice picks the new hire up.

### F3 [P0] — `Nori` has no personality entry; worker detail renders sterile default

`WorkerPersonalityCard.tsx` keys traits/great-at/escalate-when/signature by **worker name string** in `PERSONALITIES`. Entries exist for Mariah, Priya, Jamal — but **not Nori**, even though `SAMPLE_NORI` is the demo-hire target for the `marketing-intern` vertical (`use-first-hire.ts:124`). Result: the most natural demo path (user follows the wizard's "Marketing Intern" suggestion) → `/workers/018f67d0-…04/overview` → C2 personality card shows `DEFAULT_PERSONALITY` ("Reliable / Curious / Helpful" + 3 generic lines). That defeats the entire "named teammates, not tools" promise the landing page makes — the headline page of the hire flow renders the most generic of placeholders.

- Repro: in demo mode, complete onboarding with "Marketing Intern". Open `/workers/018f67d0-0000-7000-8000-000000000004/overview`. Personality card reads "HOW NORI WORKS · Reliable Curious Helpful · Working through the responsibilities you set …".
- Code: `web/components/skycot/worker/WorkerPersonalityCard.tsx:35-111`. Sam (the 5th `SAMPLE_TEAM` worker) also has no entry — same problem if a future vertical maps to Sam.
- Suggested fix: add Nori (and Sam) entries to `PERSONALITIES`, sourced from `web/lib/sample-personas.ts` (which DOES carry their `traits` array).

### F4 [P1] — `SAVE & EXIT` button is destructive, not saving

Top bar of every wizard step shows a button labeled **`SAVE & EXIT`**. Clicking opens an `AlertDialog` titled "Discard your in-progress teammate?" with body "Your draft will be cleared and you'll go back to the approvals inbox" and a `Discard and leave` confirm. The button name (SAVE) directly contradicts what it does (discard). For investor demo: a misclick at any point during the camera walk destroys the wizard state in front of the audience.

- Repro: open `/onboarding/workspace`, type something, click SAVE & EXIT. Dialog promises destruction.
- Code: `web/components/auth/WizardEditorialShell.tsx:357-360` (button label); `WizardEditorialShell.tsx:423-430` (dialog copy).
- Suggested fix: rename to `EXIT` (the wizard already persists everything to localStorage via `useOnboardingDraft`'s zustand persist middleware — exit is non-destructive *if* you don't `reset()`. F12 below).

### F5 [P1] — Onboarding/welcome vocab clashes with landing vocab ("AI workforce" vs "Hire a teammate")

`/landing` headline: "Hire a teammate." Then `/onboarding/welcome` headline: "Build your **AI workforce**." Plural framing vs. singular framing. The hire metaphor sells *one named colleague*; "workforce" snaps back to fleet/tool framing. Operator-locked design memory `project_onboarding_design_locked` describes the locked F1 design, but the headline copy here drifts from the landing promise that just got the user to click.

- Repro: compare `/landing` h1 to `/onboarding/welcome` h1.
- Code: `web/app/(setup)/onboarding/welcome/page.tsx:21-24` heroSegments.
- Suggested fix: change to `"Hire your "` `"first teammate."` (gradient on "first teammate") — keeps the singular framing through to step 5.

### F6 [P1] — Welcome pillar "Onboard them like a human" deep-links to `/genesis` mid-wizard

Pillar 2 in `/onboarding/welcome` is clickable (role=link, cursor: pointer, Enter/Space handlers) and pushes `/genesis`. Genesis is the "AI-teammate onboarding studio" memory (`feedback_genesis_is_ai_teammate_onboarding_studio`) — i.e. the *headline investor moment*. Surfacing it as a click-through *inside* the wizard splits the linear narrative. A demo viewer or buyer who clicks the pillar gets ejected from the 5-step flow and lost in `/genesis` — there's no breadcrumb back, the onboarding draft does not move forward, the user has to remember to `/onboarding/welcome` themselves.

- Repro: open `/onboarding/welcome`, click the "Onboard them like a human" pillar — you're on `/genesis` with no return path.
- Code: `web/app/(setup)/onboarding/welcome/page.tsx:43-62`.
- Suggested fix: either (a) drop the click handler and make this a passive marketing pillar, or (b) route to `/genesis` only via an explicit "Use Genesis instead" CTA below the Get Started button, with breadcrumb back to onboarding.

### F7 [P1] — Provider-key step is honest-mocked but step-5 (first-hire) is silently-mocked

Step 3 (`/onboarding/connect-provider`) handles demo mode the right way: green success banner reads **"Mock provider connected — workers run with simulated responses for the demo. Forwarding…"** Honest mocking; user knows. Step 5 (`/onboarding/first-hire`) does NOT do the same — there is no "your hire is staged — real provisioning needs a real workspace" copy. The user submits, sees a normal `Loader2 → /workers/<id>` transition, and reads it as "I just hired Maya." The asymmetry between the two demo-mode messages is the most operator-readable signal that the first-hire short-circuit is the bug.

- Repro: compare success copy in `use-provider-key.ts:56-59` (`mockProvider: true`) + `connect-provider/page.tsx:200-202` (honest banner) vs `use-first-hire.ts:129-143` (silent redirect) + `first-hire/page.tsx:46-57` (no mock banner).
- Suggested fix: in demo-mode, show an "Investor-demo · using sample worker `Nori`" badge on `/workers/<id>` overview after first-hire (or land on a `/onboarding/done` page that says so explicitly).

### F8 [P2] — All 5 step inputs are dropped on completion: `reset()` wipes the draft

`first-hire/page.tsx:52` calls `reset()` immediately after the mutation resolves, which clears `firstHirePrompt`, `workspaceName`, `companyName`, `verticalId` etc from localStorage. So even outside demo mode, the moment hire commits, the wizard's draft is gone — `/workers/<id>` cannot read back what the user typed (which would have been the cheapest way to wire F1 + F3). On the demo path with F1 + F2 already broken, this is the gravitational reinforcement that makes recovery hard.

- Code: `web/app/(setup)/onboarding/first-hire/page.tsx:46-57`.
- Suggested fix: hold the draft for 60s post-hire (or until `/workers/<id>` first-paints), then `reset()`. Lets the worker-overview page render "you described them as …" if desired.

### F9 [P2] — Wizard-footer trust strip claims "2,847 COMPANIES THIS MONTH"

Hardcoded dressing fixture in the trust-strip footer at the bottom of every step. Per `feedback_no_theater_in_demo`: dressing is OK if the camera doesn't drill in (it doesn't here) — but a number this specific in a wizard for a v1 product makes the eventual journalist/skeptic do a screenshot diff vs a year later. Better to either remove it for v1.0 (the founding-100 framing on the landing eyebrow is more honest) or use a fuzzy phrase like "early access · 100 founding teams".

- Code: `web/components/auth/WizardEditorialShell.tsx:418-420`.
- Suggested fix: replace with `<strong>Founding 100</strong>EARLY ACCESS COHORT` — matches landing-page eyebrow.

### F10 [P2] — `/onboarding/pick-vertical` lists 7 verticals but only 4 are mapped to sample workers in demo

`VERTICAL_TO_SAMPLE_WORKER` covers `marketplace-seller`, `recruiter`, `customer-support`, `marketing-intern`. The vertical grid renders 7 cards (also GTM Engineer, Sales SDR, Accounts Receivable). The 3 unmapped verticals fall through to `DEMO_FALLBACK_WORKER = Mariah`, so picking "Sales SDR" hires Mariah-the-marketplace-lister, who then renders Mariah's "lists and re-prices items for sale" charter on `/workers/<id>` — a worse-than-F3 vocabulary mismatch.

- Repro: in demo mode, pick "Sales SDR" → lands on Mariah's profile.
- Code: `web/lib/onboarding/use-first-hire.ts:120-127` + `web/app/(setup)/onboarding/pick-vertical/page.tsx:108-158`.
- Suggested fix: in demo mode, only render the 4 verticals that map cleanly. Or expand the map to 7 ids (Sam exists; +2 more demo workers needed, or temporary placeholder).

### F11 [P2] — Provider key field accepts any `sk-ant-` string in demo mode with no validation feedback delta

`use-provider-key.ts:56-59` returns `validated: true` after a 400 ms `setTimeout` for any input that begins `sk-ant-`. The user could paste `sk-ant-x` and pass. For investor demo: a CTO type viewer types a real (or what looks like real) key and watches it validate with no Anthropic round-trip, no visible "we did NOT touch your key" affordance. The hard front-end shape check at `connect-provider/page.tsx:102-108` is the only gate, and reading the code tells the story but the UI doesn't.

- Repro: in demo mode, enter `sk-ant-` followed by any char. Validates green.
- Suggested fix: in demo mode, mask the field with a "Mock provider · no real key required" placeholder + grey-out the input until the user clicks "Skip — use mock provider" (the Skip button already exists).

### F12 [P2] — Step-back navigation loses values in two of three transitions

Onboarding draft is persisted to localStorage on every `setStep/setWorkspace/setVertical/setFirstHirePrompt` call, so refresh-survival is good. **But** `connect-provider` doesn't persist the key (`apiKey` is local state only, not on the draft), and the `firstHirePrompt` gets `setFirstHirePromptInStore(description)` only inside `onSubmit`, not on `onChange`. So a user who navigates back from first-hire → pick-vertical → first-hire loses what they typed (only the *last submitted* prompt is restored). 

- Code: `web/app/(setup)/onboarding/first-hire/page.tsx:34-45`; `web/app/(setup)/onboarding/connect-provider/page.tsx:87` (key local-only).
- Suggested fix: persist `firstHirePrompt` on every keystroke (debounced) — the draft store already has the slot.

### F13 [P3] — Hire-success copy ("Hiring succeeded but your teammate isn't visible yet …") references "contact support" which has no resolution path

`FIRST_HIRE_VERIFY_FAILED_MESSAGE = "Hiring succeeded but your teammate isn't visible yet. Try refreshing or contact support."` But the SPA's help/support route is `/help` (a static doc) — there is no live support channel for a v1 buyer to *contact*. If this banner ever fires the user reads "contact support" and finds documentation. Small honesty gap; fires only on the real path, on a verify-race-loss.

- Code: `web/lib/onboarding/use-first-hire.ts:10-11`.
- Suggested fix: "Hiring succeeded — your teammate is provisioning. Refresh in a few seconds; if they don't appear, the `Help` link in the top right has next steps."

### F14 [P3] — Step pip strip + step-label show "5 of 5 · FIRST HIRE" but no celebratory step-6 / completion confirmation

The wizard caps at step 5. On submit, user redirects directly to `/workers/<id>/overview` — there is no `/onboarding/done` celebration moment, no "Maya is ready — meet your first teammate" handoff page. The hire commits, you land on what looks like a worker dashboard. For the investor demo the recording loses its emotional peak; for a real user the moment "I have hired a teammate" is undifferentiated from "I'm browsing a worker page."

- Code: `web/app/(setup)/onboarding/first-hire/page.tsx:55-57`.
- Suggested fix: route through `/onboarding/done?worker=<id>` with a 3-second celebration card ("Meet Maya. She'll Slack-message you in a few seconds with her first questions."), then redirect to the worker profile.

## Resolved since last audit

No prior onboarding-flow audit in `2026-05-27-0739/` to compare against. The 2026-05-25-1100 prior pass covered Scene 3 but pre-dates the Genesis pillar wiring, the Demo banner on signup, the demo-mode "Mock provider connected" honest-copy on connect-provider, and the new sample-worker UUID fallback in `use-first-hire.ts:120-143`. The fallback-to-sample-worker pattern was added per `feedback_no_theater_in_demo` to avoid a 404 — but the fix is itself a different theatre problem (F1).

## Orientation score

**52 / 100.**

- The design language is locked, consistent, and beautiful — F1 editorial-center, Audiowide, purple→pink gradient, warm radial-blob bg, Inter elsewhere. The design lock is implemented and matches the locked memory `project_onboarding_design_locked`. Trust-strip footer, eyebrow pills, step pips, hero typography all good.
- The non-demo path looks correctly wired: `routes_onboarding.rs:239-343` is a real handler that invokes `argo_genesis::plan`, synthesises a charter, derives duties, persists via `IdentityStore::create_worker_from_draft`, and emits the audit row. The investor narrative payload exists at the gateway.
- The demo-mode path is the entire problem. It was wired around the rough edges of running the recording without a real principal / SMTP / Anthropic — a reasonable engineering trade — but the trade landed *function-side* (the user's freeform input vanishes, the roster doesn't grow, the personality card defaults). Score is dragged down by three P0s that each lie about something the camera will see.
- Recoverable in ~2-3 focused briefs: (a) wire `appendDemoWorker()` into the demo first-hire fallback, (b) add Nori + Sam personality entries, (c) either restrict the vertical grid to the 4 mapped ids or expand the mapping. Doing (a) + (b) + (c) flips the demo from "theatre the camera sees" to "function-side real" for the recording-day take.
