# UX audit — Onboarding "hire an AI teammate" journey (2026-05-26 22:14 AEST)

**Scope**: `/landing` → `/auth/signup` (demo-mint) → 5-step wizard
(`welcome` → `workspace` → `connect-provider` → `pick-vertical` →
`first-hire`) → `/workers/{id}` (C2 personality detail).

**Mode**: read-only against the running SPA. Walked the flow via the
Claude Preview MCP (mcp__Claude_Preview__*) + curl probes + source
reads. Gateway at `http://127.0.0.1:7400`, schema v83, uptime ~7h41m
(binary mtime `May 26 14:33 AEST` = `2026-05-26T04:33Z` — same stale
binary as the 22:02 audit). SPA dev launched via
`.claude/launch.json` `web` config with
`NEXT_PUBLIC_ARGO_DEMO_MODE=1` + `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`.

**Prior audits**:
- [`2026-05-26-2152/onboarding-hire-an-ai-teammate.md`](../2026-05-26-2152/onboarding-hire-an-ai-teammate.md) (22:02, 16 findings, score 48)
- [`2026-05-26-2007/onboarding-hire-an-ai-teammate.md`](../2026-05-26-2007/onboarding-hire-an-ai-teammate.md) (20:07, 13 findings, score 56)

This run **confirms the 22:02 P0 cluster is unchanged** (code paths
identical; gateway binary still 7.7h stale) and adds **3 new findings
this audit surfaced first**:
- **F-NEW1** [P0]: A zombie worker `pnpm dev` from a previous worker
  bash loop is squatting `localhost:3000` and serving a stale build
  from `/private/tmp/fix-playwright-hire-flow-regression-cluster/web`
  without the proper env vars — so any operator (or audit agent)
  who naively curls or browses `http://localhost:3000` is testing the
  WRONG server and gets MissingSecret + 500 on `/api/auth/session` +
  502 on `/api/auth/demo-mint`. Demo-recording footgun.
- **F-NEW2** [P1]: Gateway `/api/v1/verticals` returns **zero**
  verticals. Step 4 `pick-vertical` silently falls back to a
  client-side hardcoded list of 7 cards (`FALLBACK_VERTICALS` at
  `pick-vertical/page.tsx:108-158`). The page's footer line "v1
  catalogue (offline · 7 verticals)" hints at it, but the operator
  recording the demo will narrate "pick from your installed
  templates" — none of them are actually installed.
- **F-NEW3** [P1]: Gateway `/api/v1/team` returns **just 1 worker**
  ("Maya" - marketing-intern). The runbook expects the 5-persona
  trading-card deck (Mariah / Peta / Priya / Jamal / Nori). So even
  if the demo-mode short-circuit were fixed and the wizard landed
  the user on `/workers` (which it doesn't — see F1 carry-forward),
  the page wouldn't render the Scene 1 deck.

The function-vs-dressing rule (`docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`)
classifies POST-HIRE-CLICK theatre as P0: the camera follows the
operator's click and sees what lands. Today the hire click in demo
mode lands on a "Worker not found" empty-state shell (F1 carry-forward).
That's the demo-killer.

---

## TL;DR

**3 P0 (all carry-forward from 22:02)**, **3 P1 (2 NEW)**, **3 P2**,
**1 P3**. The wizard chrome is gorgeous. The function side ends in
a 404. The operator running the demo on their own machine is also
fighting a stale port-3000 worker process they don't know about.

**Orientation score: 46 / 100.** UX presentation 92 (locked F1
designs land cleanly, brand discipline, copy register). Function 20
(every step 2-5 is short-circuited in demo mode, ending at "Worker
not found"). Demo-readiness 25 (operator environment has a zombie
worker squatting port 3000; gateway has only 1 of the 5 expected
personas + 0 verticals; one short-circuit chain ends in an error
shell).

The fix order has not changed since 22:02:
1. Operator decides demo policy: real-tenant + seeded principal, or
   extend the demo short-circuits to seed a canonical sample worker.
2. Restart the live gateway so PR #2400 (Genesis wire) +
   workers-collection-alias load.
3. Run `argo demo seed --tenant <demo>` so `/api/v1/team` returns
   the 5 personas + `/api/v1/verticals` returns the catalog.
4. Kill the zombie worker process on `localhost:3000` before
   recording.

---

## Headline blockers (top 4)

| # | Sev | Finding | Demo scene blocked |
|---|---|---|---|
| F1 | P0 | (Carry-forward) Demo-mode hire short-circuit lands on `Worker not found` | Scene 3 final beat lands on an error |
| F2 | P0 | (Carry-forward) Provider-key validation is fake in demo mode (theatre the camera sees) | Scene 3 step 3 — "$0.001 test call confirms the key works" is a lie |
| F3 | P0 | (Carry-forward) Live gateway binary 7.7h stale (Genesis + workers-alias unloaded) | Switching off demo mode doesn't recover; principal-bearing path still 404s |
| F-NEW1 | P0 | Zombie worker `pnpm dev` squatting `localhost:3000` from a stale `/private/tmp/...` worktree without env vars | Operator hitting the canonical `http://localhost:3000` during recording gets MissingSecret + 500 + 502 |

---

## All findings, in order

### F1 [P0] — (carry-forward from 22:02) Demo-mode hire ends on "Worker not found"

The wizard runs in `NEXT_PUBLIC_ARGO_DEMO_MODE=1` per
`.claude/launch.json`. On step 5 submit, `firstHire()` at
`web/lib/onboarding/use-first-hire.ts:120-122` returns a synthetic
`demo-worker-${verticalId}-${Date.now().toString(36)}` id without
any gateway round-trip, then `router.push("/workers/" + synthId)`
(`first-hire/page.tsx:57`).

The destination layout at
`web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx:50-60`
treats `useWorker(synthId)` → `WorkerNotFoundError` as canonical
404 and renders `<WorkerNotFound />`
(`web/components/skycot/WorkerNotFound.tsx`):

> **Worker not found** — The worker
> 'demo-worker-marketing-intern-mc2j8x4z' doesn't exist or you
> don't have access.
> [Back to team]

The investor-pitch journey climaxes on a not-found empty state.

**Verified at 22:14**: Code paths unchanged since the 22:02 audit;
`use-first-hire.ts:102-112` `buildDemoFirstHireResult()` still
synthesizes a per-session-unique id; sample fallback at
`web/lib/sample-worker-overview.ts:183-187` still keys on the
canonical-persona ids only.

**Reproduction**:
- visit `http://localhost:<preview-port>/onboarding/welcome`
- walk welcome → workspace ("Acme Co" / "Acme Inc.") →
  connect-provider (type any `sk-ant-foo`) → pick-vertical (pick
  any card) → first-hire (any prompt) → "Hire them"
- observe URL becomes `/workers/demo-worker-{vertical}-{ts}/overview`
- observe "Worker not found" shell

**Suggested fix** (see 22:02 audit F1 for full menu; preferred):
extend `buildDemoFirstHireResult` to return one of the canonical
sample-worker ids (e.g. `018f67d0-0000-7000-8000-000000000001` =
Mariah) so the redirect lands on a fully-rendered C2 personality
card. The synthetic per-session-unique id design is the bug —
the sample fallback cannot keep up with distinct ids by
construction.

---

### F2 [P0] — (carry-forward from 22:02) Provider-key validation is fake in demo mode

Re-verified at 22:14: `web/lib/onboarding/use-provider-key.ts:40-42`
unchanged. In demo mode, ANY string starting with `sk-ant-` returns
`{validated: true, testCost: 0.001}` with no network call. The page
copy says *"A $0.001 test call confirms the key works"* and on
submit renders *"Key validated. Forwarding…"*.

Per the function-vs-dressing rule this is the camera DRILLING INTO
the validation result — a hard claim shown live on screen — so it
must be real. Today the operator typing `sk-ant-anything-fake` sees
fake validation succeed. **Theatre the camera sees = P0.**

**Suggested fix**: in demo mode replace the "Validate & continue"
CTA with "Use mock provider for demo" so the copy matches reality;
OR route through a real mock-validation endpoint that returns
explicit `mock_provider: true` and the page surfaces "Mock provider
— workers run with simulated responses." Either way, do not
silently lie about a $0.001 test call that never happened.

---

### F3 [P0] — (carry-forward from 22:02) Live gateway binary still 7.7h stale

Gateway `/health` reports `uptime_seconds: 27697` (~7.7h) at
`2026-05-26T22:14 AEST`. Binary mtime `May 26 14:33 AEST` =
`2026-05-26T04:33Z`. Predates PR #2400 (Genesis wire) merged
2026-05-26T10:01Z and the workers-collection-alias PR merged
2026-05-26T07:00Z. The 22:02 audit flagged the same staleness; no
restart between 22:02 and 22:14.

**Suggested fix**: `./scripts/restart-live-gateway.sh`
(operator-runnable in ~2 minutes; release build cached). Verify
`/api/v1/workers` returns 200 (not 404) and `POST
/api/v1/onboarding/first-hire` response carries `genesis_invoked:
true`.

---

### F-NEW1 [P0] — Zombie worker `pnpm dev` squatting `localhost:3000` from a stale worktree

Confirmed via `lsof -p $(lsof -nP -iTCP:3000 -sTCP:LISTEN -t)`:

```
node 35584 operator cwd DIR 1,14 1024 308879234
  /private/tmp/fix-playwright-hire-flow-regression-cluster/web
```

That worktree has **no** `.env.local` or `.env` (verified — only
`.env.example`), so the running Next.js server runs without
`AUTH_SECRET`, without `NEXT_PUBLIC_GATEWAY_URL`, and without
`NEXT_PUBLIC_ARGO_DEMO_MODE`. Operational symptoms an operator
trying to record the demo against `http://localhost:3000` will see:

- `/api/auth/session` returns 500 with auth.js `MissingSecret:
  Please define a 'secret'` server-error (observed in
  `preview_console_logs`).
- `/api/auth/demo-mint` returns 502 with
  `{"error":{"code":"gateway_unreachable", …}}` even though the
  gateway is alive at `127.0.0.1:7400` (the worker server's process
  env doesn't have `NEXT_PUBLIC_GATEWAY_URL` set so the demo-mint
  route uses the wrong fallback; possibly a different IPv6/IPv4
  issue, but the symptom is the same: demo-mint fails on the wrong
  server).
- `next-auth` retries `/api/auth/session` infinitely from the SPA
  shell — every page navigation triggers a flood of
  `ClientFetchError` in the browser console. This is what wedged
  the Claude Preview chromium tab during my walk (the page
  rendered, but the renderer choked on the retry storm).

The operator does NOT know this is happening. The
`scripts/start-dev-server.sh` / `pnpm dev` shorthand both assume
nothing else is on `:3000`.

**Demo impact**: if the operator triggers `pnpm --dir web dev` for
recording and port 3000 is already taken, Next.js will autoport to
3001 / 3002 / etc. — but the operator may have a Chrome tab open
on `localhost:3000` from a previous session that's now serving the
zombie, and they'll record from the wrong server. The MissingSecret
500s and the "We couldn't reach SKYCOT" demo-mint error will be
camera-visible.

**Reproduction**:
- `lsof -nP -iTCP:3000 -sTCP:LISTEN` shows the squatting node PID
- `lsof -a -p <PID> -d cwd` shows the worktree path
- `ls /private/tmp/fix-playwright-hire-flow-regression-cluster/web/.env*`
  shows only `.env.example` (no real env files)

**Files**: not a code fix — operational. Add a pre-record kill
step to the demo runbook:

```bash
# Pre-record: nuke any stray dev servers
lsof -nP -iTCP:3000-3010 -sTCP:LISTEN -t | xargs -r kill -9
lsof -nP -iTCP:7400 -sTCP:LISTEN -t  # confirm gateway alive
```

Also recommend the supervisor's worker-loop cleanup hook (the bash
script that exits a brief's session) kill any `pnpm dev` it
started — this looks like a worker that crashed mid-brief and
never reaped its `pnpm dev` child.

---

### F4 [P0] — (carry-forward from 22:02) Welcome promises Genesis Studio; step 5 is a single textarea

`/onboarding/welcome` pillar 2 promises:

> **Onboard them like a human** — Describe the role in voice or
> text. Upload SOPs, wiki pages, and Slack threads; we verify they
> understand before they start.

Step 5 `/onboarding/first-hire` (verified at 22:14, file unchanged)
renders a single 5-row `<textarea>` and a "Hire them" submit
button. No voice. No SOP upload. No comprehension check.

Per `memory:feedback_genesis_is_ai_teammate_onboarding_studio`
(operator-locked 2026-05-25T11:40Z): the Genesis Studio is the
investor-defining moment. Text-only Phase-1 is authored; Phase-2
(multi-modal onboarding) is pending operator alignment. Until
Phase-2 ships, the welcome pillar copy is writing a check that
the page the camera lands on cannot cash.

**Suggested fix**: rewrite the welcome pillar copy to match what
step 5 actually does. Today's reality is closer to:

> **Plain-English onboarding** — Describe the role in plain
> English. We compile a charter, install the skills, and gate the
> first decisions for approval.

The voice + SOP-upload + comprehension promise can return as a
"Coming next: Voice & SOP-aware Genesis" line below the pillar.

---

### F-NEW2 [P1] — Gateway `/api/v1/verticals` returns zero rows; wizard silently shows hardcoded fallback

Verified at 22:14 via `curl`:

```
GET /api/v1/verticals → {"verticals":[]}
```

Step 4 `pick-vertical` source at
`web/app/(setup)/onboarding/pick-vertical/page.tsx:207-212`:

```ts
const cards = useMemo(() => {
  if (catalogRows.length > 0) return catalogRows.map(cardFromRow);
  return FALLBACK_VERTICALS;   // 7 hardcoded cards
}, [catalogRows]);
```

The footer line at `:319-329` does say
`"v1 catalogue (offline · 7 verticals)"` when the fallback is used
— but it's quiet, single-line, easy for the camera to miss. The
operator narrating "pick a starting template from your installed
catalog" will be wrong: nothing is installed; the cards are
client-side decoration.

In demo mode the chain compounds with F1: pick-vertical
short-circuits at `:226-230` (skip the install-vertical gateway
call); first-hire short-circuits at use-first-hire.ts:120 (skip
the gateway hire); the destination 404s. So the entire wizard
"installs skills + memory schema + approval gates" promise from
step 4's sub-hero ("We install the skills, memory schema, and
approval gates. You can swap or extend later.") is **never
exercised** in the demo path.

**Suggested fix**: bundle this with the pre-record gateway seed
(`argo demo seed --tenant <demo>` or equivalent), then verify
`/api/v1/verticals` returns 7+ rows before recording. If the demo
must run in `ARGO_DEMO_MODE=1`, additionally drop the misleading
sub-hero copy ("we install the skills") so the camera doesn't see
a promise the demo can't keep.

---

### F-NEW3 [P1] — Gateway `/api/v1/team` returns only 1 worker (Maya), not the 5-persona Scene-1 deck

Verified at 22:14:

```
GET /api/v1/team → {"total_count":1,"workers":[
  {"name":"Maya","role":{"vertical":"marketing-intern"}, …}
]}
```

The runbook's Scene 1 expects the 5-persona trading-card deck
(Mariah / Peta / Priya / Jamal / Nori — per
`memory:project_workers_page_design_locked`). The post-hire
journey in the demo doesn't visit `/workers` (it goes straight to
`/workers/{synthId}` per F1), but if the operator navigates to
`/workers` to demonstrate Scene 1 after the hire, the page redirects
to `/team` (see `web/app/(app)/workers/page.tsx:25-27`) which will
render a 1-card deck, not 5.

**Suggested fix**: same as F-NEW2 — run `argo demo seed` (or
equivalent) so the gateway carries the canonical 5 personas. The
seed brief was rewritten 2026-05-24 (per
`memory:feedback_no_theater_in_demo`) from Option A (mock-bypass)
to Option B (real backend seed CLI) precisely for this.

---

### F5 [P1] — (carry-forward from 22:02) Signup "no email required" copy lies about the gate

`web/app/auth/signup/page.tsx:221` — the secondary demo-mint
button is disabled when `email.trim().length === 0`, yet its
visible label is *"Or try a demo workspace · no email required"*
(line 238-239). The primary "Enter demo workspace" CTA at :203
ALSO requires email. The copy contradicts the gate.

**Verified at 22:14**: unchanged.

**Suggested fix**: either (a) drop the email-required guard for
demo mode (the demo-mint route at
`web/app/api/auth/demo-mint/route.ts:128-135` rejects empty
email upstream anyway, but a placeholder like `demo@skycot.dev`
could be sent automatically), or (b) change the copy to *"Or
try a demo workspace · just enter any email"*.

---

### F6 [P2] — (carry-forward from 22:02) `buildDemoProvisionResult()` missing `created` field — TS2741

`web/lib/onboarding/use-provision.ts:56-63` returns
`{tenantId, workspaceId}` but `ProvisionResult` (line 12-20)
requires `created: boolean`. `tsc --noEmit` emits TS2741 on main.
Already flagged in
`docs/build-plan/streams/fix-pr-2404-use-provision-builddemoprovisionresult-set-created-false.md`
(brief authored against `ea161df3a`). Runtime is correct (demo
re-runs shouldn't celebrate); only the build is type-broken.

**Verified at 22:14**: brief authored, not yet shipped. Will close
on merge.

---

### F7 [P2] — Onboarding wizard footer carries hardcoded "2,847 COMPANIES THIS MONTH"

`web/components/auth/WizardEditorialShell.tsx:419` —
`<strong>2,847</strong>COMPANIES THIS MONTH`. The matching
landing-page version was removed via PR #2305; the wizard footer
was missed. On a recorded demo this is camera-visible across all
5 onboarding steps and reads as fabricated social-proof theater.

Per operator memory `feedback_no_theater_in_demo`: no fake
numbers. Already authored as in-flight brief
`fix-onboarding-wizard-footer-hardcoded-companies-stat.md`
(`ea161df3a`). Will close on merge.

**Verified at 22:14**: file unchanged.

---

### F8 [P2] — `/workers` redirects to `/team` when any workers exist; lock-design C2 deck never shown on this path

`web/app/(app)/workers/page.tsx:25-27` — when the tenant has any
workers, `/workers` `router.replace("/team")`. The
operator-locked C2 holographic-showcase trading-card deck design
(`memory:project_workers_page_design_locked` 2026-05-18) was
authored for `/workers`. Today `/workers` only renders the empty-
state (`HireStarterPrompts`) when the tenant has zero workers;
otherwise it bounces you to `/team`, which has its own rendering.

This is a design-vs-implementation gap, not a demo blocker by
itself, but it bears on the audit because the prior audit's "C2
deck Scene 1" expectation can't ever fire on `/workers` while the
redirect is live.

**Suggested fix**: either (a) port the C2 trading-card deck design
to `/team` (which is the canonical roster), or (b) drop the
redirect and let `/workers` own the locked C2 design + show the
brand-loud deck even when workers exist.

---

### F9 [P3] — Landing trust strip mixes "NO CREDIT CARD" with "EARLY ACCESS · APPLY"

`web/app/landing/page.tsx:155-172` — the hero trust strip lists
five chips: NO CREDIT CARD / SOC 2-READY AUDIT TRAIL / ISO 27001
CONTROLS SUPPORTED / GDPR · HIPAA WORKFLOWS / EARLY ACCESS ·
APPLY. The CTA at :74-77 says "Hire your first teammate" (open
self-serve framing), but the rightmost chip says "EARLY ACCESS ·
APPLY" (closed waitlist framing). The hero badge "V1.0 SHIPPED ·
HIRING OUR FIRST 100 TEAMS" leans early-access too. Mixed
signals: the operator should pick a story (open self-serve or
waitlist) and commit to it across the strip + CTA.

Not a demo blocker — the demo doesn't follow the EARLY ACCESS
chip. Filed for the marketing-coherence pass.

---

## Resolved since 2026-05-26-2152 audit (12 minutes ago)

None. The interval was too short for any of the 22:02 P0s to be
fixed:

- F1 (demo hire short-circuit) — code paths unchanged at
  `use-first-hire.ts:120`
- F2 (fake provider validation) — code paths unchanged at
  `use-provider-key.ts:40`
- F3 (stale gateway binary) — gateway uptime grew from ~7h20m to
  ~7h41m; no restart between the two audits
- F4 (Welcome promises Genesis) — pillar copy unchanged
- F5 (signup demo button copy lies) — page unchanged

---

## What this run adds that prior audits did not

1. **F-NEW1**: The zombie `pnpm dev` on port 3000 from
   `/private/tmp/fix-playwright-hire-flow-regression-cluster/web` —
   this is the operator's local-machine state, observable only by
   inspecting the live process list. Worth fixing on the demo
   machine BEFORE recording.

2. **F-NEW2**: Gateway verticals catalog is empty — a precondition
   the demo runbook needs to satisfy (via `argo demo seed`) and
   isn't.

3. **F-NEW3**: Gateway team list has 1 worker not 5 — same seeding
   precondition.

4. Concrete reproduction artifacts from the curl walk:
   - `POST /api/auth/demo-mint` on the proper preview port returns
     `{ok:true, principal:"human:investor@demo.skycot",
     session_token:"eyJ…", needs_onboarding:true}` AND only sets
     the `__Host-csrf` cookie — `__Host-session` is NOT set by the
     proxy at `web/app/api/auth/demo-mint/route.ts:118-200`
     (browser must complete the `signIn("gateway-bearer", …)` step
     before the session is real). This explains why the curl-only
     walk hits `/api/auth/session` returning `null`. Not itself a
     bug, but worth documenting because it means the demo cannot
     be smoke-tested with curl alone — a real browser session is
     required.

---

## Orientation score: 46 / 100

- **UX presentation**: 92 — F1 editorial-center designs land cleanly,
  brand discipline holds, copy register is enterprise-warm,
  Audiowide + violet→pink gradient applied consistently.
- **Function**: 20 — every wizard step beyond welcome is
  short-circuited in demo mode; the chain ends at a "Worker not
  found" empty state. The principal-bearing alternative is blocked
  by the stale gateway binary (F3) + zero verticals + 1-of-5
  workers (F-NEW2 + F-NEW3).
- **Demo-readiness**: 25 — the operator's machine has a zombie
  worker squatting port 3000 (F-NEW1); the gateway needs a restart
  + a seed run; the wizard's terminal beat ends in an error shell
  that the camera will absolutely catch.

This is unchanged from the 22:02 score (48) within noise — the
function side has not moved; my run adds operational context
(F-NEW1 zombie, F-NEW2 + F-NEW3 seed state) but does not promote
or demote any of the prior P0s.
