# Onboarding hire flow — "Hire an AI teammate" UX audit

**Run:** 2026-05-27 ~01:19 AEST (15:19Z 2026-05-26)
**Agent:** e2e-ux-journey-test-agent
**Scope:** `/landing` → `/auth/signup` → 5-step wizard (`welcome → workspace → connect-provider → pick-vertical → first-hire`) → `/workers` → `/workers/{id}`
**Mode:** read-only. No code changes, no commits, no cargo.

## TL;DR

The hire flow *renders* — every step returns 200, the F1 editorial-center shell is consistent, copy is on-brand, demo mode walks cleanly. But the **honest (non-demo) path is broken end-to-end at the data layer**, and the "onboard them like a human" promise from the welcome screen is **not visible in the artefact** the wizard produces.

- **2 P0 demo blockers** (function-side bugs the camera literally lands on)
- **5 P1** (visible during a hire walk; would jar a buyer or investor)
- **5 P2** (polish; not a recording blocker)
- **2 P3** (nits)

**Biggest blocker — the hired worker is invisible to the rest of the SPA in honest mode.** `POST /api/v1/onboarding/first-hire` creates the worker in the per-principal provisioned tenant, but every read endpoint (`/workers/<id>/overview`, `/api/v1/workers`, `/api/v1/team`) resolves tenant from the gateway process `cwd()` and queries the workspace-default tenant. The SPA's `verifyWorkerReadable` (3 × 500ms retries) will always 404. The user finishes the wizard, gets redirected to `/workers/<new-id>`, sees "Worker not found." This was reproduced live.

**Second-biggest — the example placeholder produces a worker named "A".** `name_from_description` picks the first capitalised token; the wizard's example prompt `"A marketing intern named Maya who drafts blog posts and runs weekly competitive scans."` → worker name = `"A"`. Genesis didn't run for me (returned `genesis_invoked` field is unset on the response shape). This is the only naming logic that fires.

## Headline blockers

### F1 [P0] — Honest-mode hire creates a worker that the SPA can never read back

The gateway's hire endpoint provisions a per-user tenant on first signup (`/api/v1/onboarding/provision`) and writes the new worker into that tenant. But `routes_worker_overview.rs:330-339`, `routes_team_list.rs:234-242`, and (presumably) the `/api/v1/workers` list collection ALL derive their tenant via `resolve_workspace_tenant()` — which calls `store.ensure_workspace(cwd_name, cwd_path)` and returns the **default workspace tenant for the gateway's cwd**. Two different tenants for the same user; the post-hire readback always 404s.

**Camera test:** Scene 3 of the runbook is `/onboarding/welcome → … → first-hire → click "Hire them" → land on /workers/<id>`. The destination renders the **`Worker not found` shell** instead of the personality card. This is the literal investor-narrative climax.

### F2 [P0] — `name_from_description` makes the wizard's example prompt produce a worker named "A"

`crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs:232-252` picks the **first capitalised token** as the worker's display name, falling back to `"Pioneer"`. The wizard's example prompt is hard-coded at `web/app/(setup)/onboarding/first-hire/page.tsx:16`:
`"A marketing intern named Maya who drafts blog posts and runs weekly competitive scans."`
→ first capitalised token = `"A"` → persona row has `name="A"`. Reproduced live:

```
curl -X POST .../api/v1/onboarding/first-hire -d '{"description":"A marketing intern named Maya ...","vertical_id":"marketing-intern"}'
→ {"name":"A","worker_id":"019e64e6-4f39-..."}
```

If the operator types exactly what the placeholder suggests, they get a worker called "A". Both the placeholder copy and the worker-name parsing need to agree.

## All findings, in order

### Wizard chrome (shared)

#### F3 [P2] — Welcome step's eyebrow breaks the "STEP X · LABEL" pattern

`web/app/(setup)/onboarding/welcome/page.tsx:20` uses `eyebrow="WELCOME TO SKYCOT"`, but every later step uses `eyebrow="STEP TWO · WORKSPACE"`, `eyebrow="STEP THREE · CONNECT AI PROVIDER"`, etc. Welcome is the *first* step, so the pattern wants `"STEP ONE · WELCOME"`. The step-strip already shows `STEP 01 OF 05`, so the inconsistent eyebrow reads as a typo, not a deliberate choice.

**Repro:** open `/onboarding/welcome` — the eyebrow says "WELCOME TO SKYCOT"; click → `/onboarding/workspace` says "STEP TWO · WORKSPACE". 5-second double-take, no functional impact.

**Suggested fix:** change to `"STEP ONE · WELCOME"`.

### Step 1 — `/onboarding/welcome`

#### F4 [P2] — Three feature pillars overlap with `/landing` pillars verbatim

`web/app/(setup)/onboarding/welcome/page.tsx:36-56` lists "Audit-first" / "Onboard them like a human" / "Enterprise-grade". Same shape as the `/landing` value-prop strip the user just came from. Not a function bug; it's a missed opportunity — the welcome page should *bridge* into doing, not re-pitch. A buyer who chose to sign up doesn't need a second sales beat. Suggest replacing with a 3-bullet "here's what we'll do next: name your workspace → connect AI provider → pick a template → hire."

### Step 2 — `/onboarding/workspace`

#### F5 [P2] — Demo-mode provision builder returns no `created` flag, silently drops the celebration toast

`web/lib/onboarding/use-provision.ts:56-62` (`buildDemoProvisionResult`) returns `{ tenantId, workspaceId }` only — the typed return is `ProvisionResult` which has `created: boolean`. The workspace page (`workspace/page.tsx:47-52`) only fires `toast.success("Workspace created — welcome to Skycot.")` + the 1200ms `CELEBRATION_HOLD_MS` when `res.created === true`. In demo mode that's `undefined`, so demo-mode users get **no celebration, no hold** — they jump straight to step 3, breaking the human-rhythm pacing the celebration was designed to create.

Honest mode hits the real gateway and gets `created: true` on first call, so the demo recording is the path that misses the toast.

**Suggested fix:** `buildDemoProvisionResult` should return `created: true` once per session (e.g. keyed off `sessionStorage` presence of the demo tenant id).

### Step 3 — `/onboarding/connect-provider`

#### F6 [P3] — "$0.001 test call" claim has no live validation behind it

The sub-copy promises "A $0.001 test call confirms the key works" but `routes_onboarding_helpers.rs:118` says: *"v1 ships without the live one-token validation call; the wire shape mirrors the eventual real-validation contract so a follow-up brief can swap the body without changing the response."* So validation is shape-only (`sk-ant-` prefix check). The "$0.001 test call" line is a function-side promise made by the dressing.

**Camera test:** dressing-side; the camera doesn't probe whether real bytes went to Anthropic. Probably fine to leave for v1.0 demo. Flag for v1.1 polish.

### Step 4 — `/onboarding/pick-vertical`

#### F7 [P1] — 32 verticals on the gateway, but the card grid hard-decorates only 7

`web/app/(setup)/onboarding/pick-vertical/page.tsx:37-100` has `VERTICAL_DECORATION` for exactly 7 vertical ids (`gtm-engineer`, `marketing-intern`, `marketplace-seller`, `recruiter`, `sales-sdr`, `customer-support`, `finance-ar`). The gateway reports **32 verticals loaded** (`curl /verticals` returns 32 ids including `bookkeeping-assistant`, `executive-assistant`, `product-manager`, `qa-engineer`, `pr-checker`, etc.). The other 25 fall through to `cardFromRow`'s synthetic-from-tags branch, which produces tag-shaped chips like "Customer Onboarding", "Csm", "Post Sale", "Playbook", "Churn", "Demo" — not actual duties.

**Camera test:** the camera lands here in Scene 3 of the runbook. Three "Demo" chips on a card is visible from 1m back. Investor will notice.

**Suggested fix:** either author the 25 missing decorations or hide non-canon verticals from the wizard (operator can curate via a `[demo]` tag filter — `/verticals` already exposes tags).

#### F8 [P1] — Wizard says it installs "skills, memory schema, and approval gates" but only 2/3 actually fire

Sub-copy: *"We install the **skills, memory schema, and approval gates**. You can swap or extend later."* The `install-vertical` response (`{"installed":true,"skills_added":5,"vertical_id":"marketing-intern","workflows_added":0}`) only confirms skills added. There's no `memory_schema_added` or `approval_gates_added` count in the response. The promise is bigger than the receipt.

Function-side (the actual install handler may do all 3); but the user-visible receipt doesn't reassure them all 3 happened.

**Suggested fix:** expand the install response to include `memory_schema_attached: true/false` + `approval_policy_attached: <name>` so the SPA can either show "All 3 installed" or correct the copy.

### Step 5 — `/onboarding/first-hire`

#### F9 [P0 — see F1, F2] — see F1 (gateway-tenant readback bug) + F2 (worker named "A")

These are the load-bearing demo blockers; called out as headline blockers above.

#### F10 [P1] — Error copy says "Hiring succeeded but your teammate isn't visible yet" — burying the systemic 404

`web/lib/onboarding/use-first-hire.ts:10` defines `FIRST_HIRE_VERIFY_FAILED_MESSAGE = "Hiring succeeded but your teammate isn't visible yet. Try refreshing or contact support."` This is what the user sees when F1 fires. The phrasing implies a transient eventual-consistency hiccup that "refresh" will solve. In honest mode it's deterministic — every fresh signup will see this. "Try refreshing" is a dead end; the worker will NEVER appear from the SPA's perspective until the tenant-resolution bug is fixed.

#### F11 [P2] — `Maya` example prompt and the locked-design personality card don't share a name dictionary

`web/components/skycot/worker/WorkerPersonalityCard.tsx:35-96` defines personality presets for **only 3 names**: Mariah, Priya, Jamal. Everyone else (including the wizard's example "Maya", the seeded "Nori"/"Peta"/"Patrick", and the fallback "Pioneer") gets `DEFAULT_PERSONALITY` = *"Reliable / Curious / Helpful"* with three generic "Great at" lines. So even if F1 + F2 are fixed and the operator's worker appears, the personality card promised by `/landing`'s "onboard them like a human" pitch falls back to the same beige content for every newly-hired teammate.

**Camera test:** Scene 1 of the runbook drills into Mariah's personality card. The hire-flow Scene 3 lands on **a different** worker (newly created), and the card looks generic. The contrast between the seeded Mariah and a freshly-hired Maya is visible side-by-side.

**Suggested fix:** either (a) wire the personality card to read live `worker.persona.bio` / charter from the gateway (the schema exists, just no data flowing), or (b) the demo Scene 3 should drill back into Mariah, not a fresh hire.

### `/workers` (post-hire roster surface)

#### F12 [P1] — `/workers` redirects to `/team` when the roster is non-empty, but the locked C2 trading-card design only exists at `/team`

`web/app/(app)/workers/page.tsx:25-27`: *"When workers exist, redirect to `/team`, the canonical roster surface."* The redirect happens after hydration, so the user sees the `/workers` URL flash, then `/team` in the address bar. Two URLs, one design, undefined which is the "real" path. Scene 1 of the runbook references `/workers` but the deck design lives at `/team`. This will read as messy in the demo.

**Suggested fix:** decide which URL is canonical (the operator memo specifies `/workers`) and either redirect `/team → /workers` permanently or move the deck UI into `/workers/page.tsx`.

#### F13 [P2] — Empty-state copy "No teammates yet. Pick a vertical and they'll be in your Slack in four minutes."

`/team` SSR body literally renders "No teammates yet. Pick a vertical and they'll be in your Slack in four minutes." This is a Slack-shaped promise the v1.0 demo doesn't deliver (no Slack integration runs in the recorded demo). Either remove "in your Slack" or condition on whether the user actually connected Slack.

### `/workers/{id}` — worker profile after hire

#### F14 [P1] — Three primary actions on the freshly-hired worker profile are inert stubs

`web/components/skycot/worker/WorkerOverviewContent.tsx:75-92`:

- `Add responsibility` → toast "Add responsibility dialog ships in E4-h"
- `Edit schedule` → toast "Edit schedule ships in E4-h"
- `Save charter` → console.warn + toast "Charter changes persist in E4-g"

So the operator finishes hiring, lands on the worker profile, and the three buttons that say "make this teammate yours" all flash a placeholder toast. The wizard pitch is "you can swap or extend later" — the destination says "the swap/extend hooks ship in a future brief."

**Camera test:** if the operator hovers/clicks these in the recording the toast appears. Avoid hovering; or hide the buttons in v1 until they wire.

#### F15 [P3] — Persona row's `bio` and `voice_preamble` fields are persisted but ~empty for hire-flow workers

DB inspection: `SELECT bio, voice_preamble FROM persona WHERE worker_id IN (hired-via-wizard)` — `bio` is empty/NULL, `voice_preamble` is ~40 chars (boilerplate "I'm a marketing-intern style worker named X"). The "describe what they should do this week" prompt from step 5 ends up only in the worker's charter, not in the persona's bio. So the worker profile's "About" section (if it renders persona.bio) is blank.

### Sidebar / persistent chrome

#### F16 [P1] — Sidebar shows "Sam Harlow · Acmeco · Owner" even with `NEXT_PUBLIC_SKYCOT_DEMO_MODE` off in `.env.local`

The dev server inherited `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` from the shell that launched it, even though `.env.local` doesn't set it. Result: my "honest mode" walk actually ran in demo mode end-to-end (the `useFirstHire` and `useProvision` short-circuits both fired, producing synthetic ids). The `.env.local` is not authoritative; shell env wins.

For the runbook, this means: if the operator opens a fresh terminal and runs `npm run dev` without re-exporting `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`, demo mode is off and Scene 3 hits F1 immediately. There is **no SPA-side affordance** to know which mode you're in (the demo banner only shows when `isDemoMode()` is true; honest mode shows nothing).

**Suggested fix:** add a tiny "Demo mode" / "Live mode" stamp to the bottom of the sidebar so the operator can tell at a glance which path they're on before recording.

## Resolved since last audit

No prior onboarding-hire audit exists at `docs/skycot/ux-audits/`. First run.

## Orientation score

**52 / 100** — *demo-recordable in demo mode only, broken in honest mode.*

The shell + copy + design language are strong (F1 editorial-center is locked and consistent; brand voice is correct; step strip + back/save-exit work). What scores are tanking:

- The hire wizard's terminal step (`/onboarding/first-hire`) produces a **404-able worker in honest mode** — the single most-load-bearing path through the product breaks on the freshest user
- The "onboard them like a human" promise (the welcome page's central pillar + Mark's investor pitch) produces a worker with a generic 3-word personality card + empty bio + 3 inert action buttons
- The worker's display name is parsed from the description's first capitalised token — the wizard's own example placeholder produces a worker named "A"

If demo mode is the only intended path for the v1.0 MP4 recording (and the runbook says it is — `ARGO_DEMO_MODE=1` everywhere), then F1/F2 don't block the recording, only the post-recording trial path. The investor watching the MP4 will see a clean flow; the investor opening the SPA themselves after will see the 404. Decide deliberately which experience matters more before recording.

## Function-vs-dressing scorecard (per `v1-demo-positioning-poc-vs-public.md`)

| Finding | Function or Dressing | Camera sees? | POC blocker? |
|---|---|---|---|
| F1 — tenant-resolution 404 on readback | **Function** | Yes — last beat of Scene 3 | **YES in honest mode, no in demo mode** |
| F2 — worker named "A" | **Function** | Yes — visible in URL + page title | **YES** (placeholder demo would record this name) |
| F11 — generic personality card | **Function** (the experience the agent gives) | Yes — Scene 1/3 contrast | **YES if Scene 3 lands on the new worker** |
| F14 — inert buttons | **Function** | Only if hovered | NO if operator avoids hover |
| F7 — undecorated verticals | **Dressing** | Yes — Scene 3 card grid | YES (visible) |
| F8 — install-vertical receipt | Dressing | No (silent install) | NO |
| F3 — eyebrow inconsistency | Dressing | Yes — top of step 1 | NO |
| F12 — `/workers` ↔ `/team` redirect | Dressing | URL bar | NO |

Camera blockers in priority: **F1 → F2 → F11 → F7**.
