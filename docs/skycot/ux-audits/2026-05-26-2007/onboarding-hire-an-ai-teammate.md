# UX audit — Onboarding: "hire an AI teammate" journey (2026-05-26 20:07 AEST)

**Scope**: `/landing` → `/auth/signup` (demo-mint) → 5-step wizard
(`welcome` → `workspace` → `connect-provider` → `pick-vertical` →
`first-hire`) → `/workers` → `/workers/{id}`.

**Mode**: read-only against running dev SPA (`pnpm dev` via Claude
Preview MCP) + curl probes against live gateway at
`http://127.0.0.1:7400` (schema v83, uptime ~5h45m at test time).

**Theatre rule applied**: this is the investor-pitch journey. The
operator narrates "describe a job in plain English, watch a real
teammate spawn." The drill-in test fires the moment the camera
clicks "Hire them" — what lands at `/workers/{newId}` MUST be real,
not a sample card. That fence is where most of the P0 weight lives.

---

## TL;DR

**5 P0**, **3 P1**, **4 P2**, **1 P3**.

1. The wizard's final step (the load-bearing moment of the demo) calls
   `POST /api/v1/onboarding/first-hire`. The gateway DOES create a
   real worker row, but **every read-back path 404s** because the
   tenant the hire writes to (per-email) is not the tenant the
   roster endpoint reads from (per-CWD). The wizard's own
   `verifyWorkerReadable` retry loop fails → operator sees the
   `FIRST_HIRE_VERIFY_FAILED_MESSAGE` banner and never lands on
   `/workers/{id}` (F1).
2. The **live gateway binary is stale by ~5.5h** — built before both
   PR #2400 (Genesis wire) and PR #9ea693db1 (workers collection
   alias). The "investor moment" backend exists in source but is
   not running, so a fresh demo cold-recording will hit the
   pre-Genesis templated charter path (F2, F3).
3. The welcome step promises a **Genesis Studio** ("describe the
   role in voice or text. Upload SOPs, wiki pages, and Slack
   threads; we verify they understand before they start"). Step 5
   delivers a single `<textarea>` and a "Hire them" button. No
   voice. No file upload. No comprehension check (F4).
4. The `/workers` and `/workers/{id}` pages render the locked C2
   trading-card + personality designs beautifully — but they read
   from `SAMPLE_TEAM` fixtures (Mariah/Peta/Priya/Jamal/Nori), not
   from the worker the operator just hired. The chip in the team
   page is even labelled `MOCK-OFFLINE`. The newly-minted Mariah's
   personality / great-at / signature behaviors all come from the
   fixture, not from Genesis (F5).
5. The `/onboarding/welcome` page's `2,847 companies this month`
   stat is dressing-side fine, but it sits next to the Genesis
   pillar — a viewer drilling for proof finds a hardcoded number
   and a hardcoded promise (F11).

Overall, the **front-end onboarding flow is the highest-craft
surface in the product** — designs, copy, register, brand
discipline are excellent. The **backend wire-through is broken at
the central read-back**, and the **demo-recording gateway is stale**.
Both are fixable in a single ops session.

**Orientation score: 56 / 100.** UX presentation: 90. Function: 35.
Demo-readiness: 30 (wizard never reaches its happy end).

---

## Headline blockers

| # | Sev | Finding | Demo scene blocked |
|---|---|---|---|
| F1 | P0 | First-hire → read-back returns 404 due to per-email vs per-CWD tenant resolver mismatch. Wizard shows verify-failed banner. | Scene 3 final step never completes |
| F2 | P0 | Live gateway binary is 5.5h older than PR #2400 — Genesis wire never fires. | "Genesis-synthesised charter" claim cannot be recorded |
| F3 | P0 | Live gateway binary is 5.5h older than PR #9ea693db1 — `GET /api/v1/workers` returns 404 even with correct tenant. | `/workers` page can never read live roster |
| F4 | P0 | Welcome pillar promises voice + SOP upload + comprehension verify; step 5 is a single textarea. | "Onboard them like a human" pillar is unsubstantiated by the page the camera lands on |
| F5 | P0 | `/workers` + `/workers/{id}` render `SAMPLE_TEAM` fixtures (Mariah is hardcoded fiction), not the just-hired worker. | If the camera clicks the just-hired worker, it would 404 even if F1 were fixed, because the page reads from sample data when gateway 404s |

---

## All findings, in order

### F1 [P0] — First-hire → read-back returns 404 (tenant resolver mismatch)

The hire-endpoint (`/api/v1/onboarding/first-hire`) resolves the
tenant from the principal's email
(`crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:254` —
`resolve_workspace(store, &email)`). The team-roster endpoint
(`/api/v1/team`, aliased to `/api/v1/workers`) resolves the tenant
from the gateway's current working directory
(`crates/argo-cli/src/handlers/gateway/routes_team_list.rs:234-243`
— `std::env::current_dir() → store.ensure_workspace(name, &root)`).

So the worker is written to tenant A (per email), but read from
tenant B (per CWD). Worker exists in DB, never visible.

The wizard's own verification loop
(`web/lib/onboarding/use-first-hire.ts:84-99`) calls
`GET /api/v1/workers/{id}/overview` 3 times with 500ms backoff,
then emits `wizard.first_hire_verify_failed` audit + shows the
operator `FIRST_HIRE_VERIFY_FAILED_MESSAGE`. The wizard never
redirects to `/workers/{id}`.

**Reproduction**:
- `PRINCIPAL=human:demo+e2e@skycot.test`
- `curl -X POST http://127.0.0.1:7400/api/v1/onboarding/provision -H "X-Argo-Principal: $PRINCIPAL" -H "Content-Type: application/json" -d '{"workspace_name":"X","company_name":"Y"}'` → returns `tenant_id=019e63ca-9beb-74c2-9b1f-6daa081ae67e`
- `curl -X POST http://127.0.0.1:7400/api/v1/onboarding/install-vertical -H "X-Argo-Principal: $PRINCIPAL" -H "Content-Type: application/json" -d '{"vertical_id":"gtm-engineer"}'` → 200
- `curl -X POST http://127.0.0.1:7400/api/v1/onboarding/first-hire -H "X-Argo-Principal: $PRINCIPAL" -H "Content-Type: application/json" -d '{"description":"Mariah, you research enterprise prospects","vertical_id":"gtm-engineer"}'` → returns `worker_id=019e63cb-b183-7d32-bafc-b5090417fc3f`
- `curl -H "X-Argo-Principal: $PRINCIPAL" http://127.0.0.1:7400/api/v1/workers/019e63cb-b183-7d32-bafc-b5090417fc3f/overview` → `{"error":{"code":"not_found","message":"worker not found"}}`

**Files**: `crates/argo-cli/src/handlers/gateway/routes_team_list.rs:234-243` (the CWD-based resolver), `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:254` (the email-based resolver), `web/lib/onboarding/use-first-hire.ts:84-99` (the failing verify loop).

**Suggested fix**: replace `resolve_workspace_tenant(store)` in
`routes_team_list.rs` with the per-principal resolver used in
`routes_onboarding.rs` (`resolve_workspace(store, &email)`). The
same fix needed on every singleton worker route
(`/api/v1/workers/{id}`, `/api/v1/workers/{id}/overview`,
`/api/v1/workers/{id}/activity`, etc.) — they almost certainly
share the same resolver.

---

### F2 [P0] — Live gateway binary predates PR #2400 (Genesis wire)

Gateway process is `/tmp/argo-shared-target/release/argo gateway
start --bind 127.0.0.1 --port 7400` (PID 7348, started
2026-05-26T04:34Z, ~5h45m uptime). Binary `mtime` is
`May 26 14:33` (AEST) = `2026-05-26T04:33Z`.

PR #2400 (`gateway(onboarding): invoke argo_genesis::plan on
first-hire`) was merged at `2026-05-26T20:01:06+10:00` =
`2026-05-26T10:01Z` — **5.5h AFTER the running binary was built**.

Live first-hire response confirms the pre-#2400 shape:
`{"name":"Mariah","vertical_id":"gtm-engineer","worker_id":"..."}` —
the new `genesis_invoked` + `genesis_fallback` fields from
`routes_onboarding.rs:312-322` are absent.

**Impact**: cold-recording the demo today produces a worker with
the pre-Genesis templated charter ("First hire for this workspace.
Operator brief: ...") + a single hardcoded `daily_check_in` duty
— NOT the Genesis archetype-based charter + per-archetype duty
stagger. The investor moment that distinguishes Skycot from
"ChatGPT with prompts" never lands.

**Files**: `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:263-322`, `crates/argo-cli/src/handlers/gateway/routes_onboarding_genesis.rs:75-95`.

**Suggested fix**: `./scripts/restart-live-gateway.sh` (per memory
`project_v1_strategic_decisions_2026_05_14` the canonical gateway
port is 7400; restart should be a no-op-ish rebuild). Verify with
`curl http://127.0.0.1:7400/health` showing fresh `uptime_seconds`
+ a hire response carrying `genesis_invoked: true`.

---

### F3 [P0] — `GET /api/v1/workers` returns 404 (no route handler in running binary)

PR #9ea693db1 (`gateway: alias workers collection route`) added the
dispatch entry at
`crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api_groups.rs:64-66`
on 2026-05-26T07:00Z. Running gateway binary is from 04:33Z, so
this route is NOT in the binary.

**Reproduction**:
- `curl -v http://127.0.0.1:7400/api/v1/workers -H "X-Argo-Principal: human:demo+e2e@skycot.test"` → `HTTP/1.1 404 Not Found` with body `{"error":"not found"}`
- `curl http://127.0.0.1:7400/api/v1/team -H "X-Argo-Principal: human:demo+e2e@skycot.test"` → 200 with sample worker (Maya) (proves the legacy route works; only the alias is missing)

**Files**: `crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api_groups.rs:64-66` (the route table entry not in the binary), `web/lib/api/team.ts:69-76` (the SPA call site that pins to `/api/v1/workers`).

**Suggested fix**: same restart as F2. Then the SPA's `fetchTeam`
will hit the live gateway instead of falling through to
`SAMPLE_TEAM`.

---

### F4 [P0] — Welcome promises Genesis Studio (voice + SOP + comprehension); step 5 is a single textarea

`/onboarding/welcome` pillar #2 reads: *"ONBOARD THEM LIKE A
HUMAN. Describe the role in voice or text. Upload SOPs, wiki
pages, and Slack threads; we verify they understand before they
start."*
(`web/app/(setup)/onboarding/welcome/page.tsx:43-49`).

The actual step-5 page (`web/app/(setup)/onboarding/first-hire/page.tsx:103-115`)
renders a single 5-row `<textarea>` with the placeholder
*"A marketing intern named Maya who drafts blog posts and runs
weekly competitive scans."* That's the only input. No microphone
button, no `<input type="file">`, no comprehension probe.

This is the central narrative mismatch. The investor is told the
agent is onboarded like a human; the operator types one sentence
and clicks Hire.

Per memory `feedback_genesis_is_ai_teammate_onboarding_studio`
(operator-clarified 2026-05-25T11:40Z): *"Phase 1 (text-only)
authored; Phase 2 (multi-modal onboarding) pending operator
alignment."* So this is a known gap, but the welcome copy promises
Phase 2 today. The pillar should be honest about Phase 1 OR the
step should have a stubbed "Upload SOP" tile so the gap is visible.

**Files**: `web/app/(setup)/onboarding/welcome/page.tsx:43-49` (the promise), `web/app/(setup)/onboarding/first-hire/page.tsx:101-116` (the delivery).

**Suggested fix**: either (a) soften pillar #2 to match the
text-only Phase 1 reality, or (b) add disabled "Upload SOP" /
"Voice intro" tiles to step 5 with "Coming in v1.1" badges, so the
investor demo can narrate the roadmap promise truthfully.

---

### F5 [P0] — `/workers` + `/workers/{id}` read from SAMPLE_TEAM fixtures, not the live tenant

`web/lib/api/team.ts:84-95` — when the gateway returns 404 (which
it always does today per F3), the SPA in demo mode (`isDemoMode()`)
silently falls through to `SAMPLE_TEAM` and tags the source
`mock-offline`. The team page renders the chip `MOCK-OFFLINE` next
to "Team" so this is visible to a careful viewer.

The C2 trading-card visuals are gorgeous — Mariah / Peta / Priya /
Jamal / Nori all have personalities, traits, level chips, "Tomorrow"
schedule lines. But none of it comes from the just-hired worker.
The just-hired Mariah's worker_id is never queryable.

For the worker-detail page (`web/lib/api/worker-overview.ts:103-115`),
the same fallback path means `findSampleWorkerOverview(workerId)`
runs against a hardcoded ID list. The new worker's UUID is not in
that list → `WorkerOverviewNotFoundError`. So even if the wizard
verify loop passed (F1), the redirect target would 404 in the
SPA's own fallback.

**Files**: `web/lib/api/team.ts:58-99`, `web/lib/api/worker-overview.ts:103-132`, `web/lib/sample-team.ts`.

**Suggested fix**: F1 + F2 + F3 unblock this. Once the gateway is
fresh and the tenant resolver is unified, the SPA's fallback path
won't fire. The `MOCK-OFFLINE` chip should also be hidden when the
data source is gateway, or relabelled (it'll be confusing in a
demo recording even when working).

---

### F6 [P1] — Genesis-synthesised memory schema + eval seeds are computed but never persisted to the worker

`crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:267-277`
builds the `HireDraft` from the Genesis plan but only pulls
`charter` + `duties` out. The full `plan.manifest` (which carries
`memory_schema`, `eval_seeds`, the action vocabulary) is dropped
on the floor. The worker's `memory_namespace` is created empty
(`crates/argo-identity/src/store_hire.rs:199` —
`MemoryNs::new(tenant_id, id, MemoryScope::WorkerSelf)`), and no
eval suite is registered for the worker.

The synthesised charter mentions `Genesis plan id: <id>` (per
`routes_onboarding_genesis.rs:140-143`), so the operator sees a
plan id in the charter, but clicking through to /workers/{id}/memory
or /workers/{id}/evals would show empty surfaces. **Theater the
camera might see** if the demo drills into a tab.

**Files**: `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:267-277` (the HireDraft assembly), `crates/argo-genesis/src/templates.rs:51-65` (the plan.manifest.eval_seeds + memory_schema fields), `crates/argo-identity/src/store_hire.rs:151-213` (the persistence path that drops them).

**Suggested fix**: extend `HireDraft` to carry the synthesised
memory schema + eval seeds, then persist them via
`assemble_worker_from_draft` and a new `register_worker_eval_seeds`
call. Or, simpler for v1.0: the Genesis plan_id is already in the
charter — surface it on the worker-detail Overview tab as a
"Plan provenance" chip so the camera has something concrete to
drill into.

---

### F7 [P1] — `connect-provider` step is Anthropic-only; SPA copy promises "link a provider" abstractly

`web/app/(setup)/onboarding/connect-provider/page.tsx:10` hard-codes
`ANTHROPIC_CONSOLE_URL`, the hero says "Connect Anthropic." and the
validator rejects keys that don't start with `sk-ant-`. The
landing page's hero promises "link a provider" — singular but
ambiguous. The platform itself supports multiple providers per
manifests.

For the v1.0 demo this is fine (operator uses Anthropic), but it
will look thin in an investor Q&A if they ask "what about OpenAI?"

**Files**: `web/app/(setup)/onboarding/connect-provider/page.tsx:10,30,60`.

**Suggested fix**: post-MVP — add a provider picker (Anthropic /
OpenAI / Bedrock) with a "Recommended: Anthropic" pre-selection.
For v1.0 demo, no change needed.

---

### F8 [P1] — Wizard back-button on step 2 goes to `/onboarding/welcome`, which is a different mental model than "go back to my workspace"

`web/app/(setup)/onboarding/workspace/page.tsx:110` —
`onClick={() => router.push("/onboarding/welcome")}`. Workspace
provisioning is already complete by this point (the operator's
input has been persisted), so going back to a marketing-style
welcome screen is jarring. Most multi-step wizards either
disable back on step 1 of the substantive form OR step back to
the previous in-form state.

**Files**: `web/app/(setup)/onboarding/workspace/page.tsx:106-114`.

**Suggested fix**: remove the back button on step 2 (workspace
name is required) or change it to a "Restart" with a confirm.

---

### F9 [P2] — Mobile viewport (375×812) clips the wizard top bar

Captured on `/onboarding/welcome` at mobile preset: the top bar
shows "SKYCOT · STEP 01 OF 05 · [dots]" and then the step label
"W..." gets cropped + the "Help" + "SAVE & EXIT" buttons are
off-screen entirely. The page body (eyebrow + hero + pillars + CTA)
flows fine in a single column.

Not a demo blocker — the demo records on desktop — but a mobile
visitor hitting the wizard on a phone hits a clipped chrome.

**Files**: `web/app/(setup)/onboarding/_OnboardingShell.tsx` (the
top-bar grid/flex), the wizard `.css` (not inspected).

**Suggested fix**: on `< 640px`, collapse the dot-progress to a
single "1 / 5" pill, drop the "STEP 01 OF" text, hide the "Help"
link behind the existing help-floating-button, and keep "SAVE &
EXIT" as a hamburger entry.

---

### F10 [P2] — `Save & exit` button has no `aria-haspopup`, no confirmation, and no visible target route

Top-bar `SAVE & EXIT` button (visible on all 5 wizard pages). I
didn't click it to test (would dirty state) but the button is
declared as a plain `<button>` without `aria-haspopup` or
`aria-controls`. Operator clicking it during a demo recording
could blow away the run if the side-effect is unclear.

**Suggested fix**: change to a `<Link>` to `/workers` (or wherever
"exit the wizard" is meant to land) and label it "Exit setup" so
the destination is clear.

---

### F11 [P2] — Hardcoded "2,847 companies this month" stat sits in the wizard footer

`web/app/(setup)/onboarding/welcome/page.tsx` footer renders
`NO CREDIT CARD · SOC 2-READY AUDIT TRAIL · GDPR ARTICLE 30 ·
2,847 COMPANIES THIS MONTH`. The number is dressing-side per the
function-vs-dressing rule, so it's not a demo blocker. But it
sits next to a pillar promising "audit-first" and "enterprise-
grade" — a viewer drilling for proof finds a hardcoded number in
the same visual frame as a load-bearing trust claim.

**Suggested fix**: either gate the stat behind a real Plausible /
PostHog read OR move it to the public marketing landing (where
dressing-side fixtures are clearly framed as marketing copy).

---

### F12 [P2] — First-hire prompt placeholder seeds "Maya / marketing intern" but Step 4 pre-selected GTM Engineer (or whatever the user picked)

`web/app/(setup)/onboarding/first-hire/page.tsx:14` —
`EXAMPLE_PROMPT = "A marketing intern named Maya who drafts blog
posts and runs weekly competitive scans."` — but the operator has
already picked a vertical in step 4 (e.g. GTM Engineer). The
placeholder example contradicts the picked template, and Genesis
will dispatch on the prompt's archetype words ("marketing intern")
rather than the picked vertical.

Could lead to the operator typing the placeholder example and
getting a Marketing Intern Genesis plan despite picking "GTM
Engineer" on the previous step.

**Files**: `web/app/(setup)/onboarding/first-hire/page.tsx:14-15`, `crates/argo-cli/src/handlers/gateway/routes_onboarding_genesis.rs:75-95` (where Genesis ignores `vertical_id` and dispatches on the prompt text).

**Suggested fix**: parameterise `EXAMPLE_PROMPT` by the
draft.verticalId. For `gtm-engineer`: *"Mariah, you research
enterprise prospects and book demo calls."* For
`marketing-intern`: keep current. Etc.

---

### F13 [P3] — Welcome accessibility tree splits "OF 0 5" into two static-text nodes

Caught in `preview_snapshot` (a11y tree) of `/onboarding/welcome`:
the step indicator renders text "STEP 01 OF 05" visually correctly
in the screenshot, but the a11y tree shows
`StaticText: "0"` followed by `StaticText: "5"` — likely from a
span split in the source. Screen-reader users may hear "step one,
of, zero, five" instead of "step one of five."

Not visible to sighted users; not a demo blocker.

**Suggested fix**: collapse the span split in the top-bar progress
component to a single `aria-label="Step 1 of 5"` text node.

---

## Resolved since last audit

This is the first audit specifically scoped to this journey at
this directory path. The prior, broader audit at
`docs/skycot/ux-audits/2026-05-25-1100-onboarding-hire-an-ai-
teammate.md` flagged different failure modes (pre-Wave-11);
no overlap to call out.

---

## Orientation score: 56 / 100

- UX presentation: 90 (locked F1 editorial, brand-consistent,
  clear progress, good copy)
- Function: 35 (tenant resolver mismatch + stale gateway = wizard
  never reaches its happy end; Genesis is wired in source but not
  in running binary)
- Demo-readiness: 30 (cannot record the headline scene as written
  without first restarting gateway + fixing F1)

The fix order is short and concrete:

1. **Restart the gateway** (F2 + F3 vanish in one ops command —
   `./scripts/restart-live-gateway.sh`). Re-verify `/health` shows
   fresh uptime + a hire response with `genesis_invoked: true`.
2. **Unify the tenant resolver** (F1 + F5 vanish) — replace
   `routes_team_list.rs:234-243` `resolve_workspace_tenant`'s
   CWD-based path with the principal-email resolver in
   `routes_onboarding.rs`. Apply the same fix to all singleton
   `/api/v1/workers/{id}/*` routes.
3. **Either soften the welcome pillar or add stubbed upload tiles
   on step 5** (F4) — operator decision: which message do we want
   on camera?

Items 1 + 2 are agent-doable; item 3 is operator-decisional. The
remaining P1/P2/P3 findings are polish for the next iteration.
