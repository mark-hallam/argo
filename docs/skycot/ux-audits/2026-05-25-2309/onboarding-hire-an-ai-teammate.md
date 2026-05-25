# UX e2e audit — onboarding "hire an AI teammate" journey

**Audit window**: 2026-05-25 ~23:09 AEST
**Auditor**: e2e-ux-journey-test-agent (Claude Opus 4.7)
**Mode**: read-only against running dev server + gateway
**Scope**: `/landing` → `/auth/signup` (demo-mint) → 5-step onboarding wizard → `/workers` → `/workers/<id>`
**Gateway**: `http://127.0.0.1:7400` (schema v81), healthy
**SPA**: preview server on port 60894 with `NEXT_PUBLIC_ARGO_DEMO_MODE=1` (operator's pre-running port 3000 has no demo bypass and would have hard-blocked the audit at signup)

---

## TL;DR

The "hire an AI teammate" metaphor is **brilliantly written** at the marketing layer and the wizard chrome is on-brand — but the central investor narrative collapses the moment a real browser tries to use it. Two **function-side P0s** kill the demo for any non-staged take:

1. **Gateway returns no CORS headers** → every SPA → gateway fetch fails in the browser. Workers list, audit list, approvals, skills, error-reporting — all permanently stuck in skeleton/empty state, even though `curl` against the same endpoints returns 200 with real data.
2. **Worker persona is hardcoded frontend dressing**, not gateway-backed. A newly hired worker gets a one-of-five-name → static-traits lookup; anyone with a name outside that map (including Maya, the only worker actually in the gateway) lands at the C2-personality detail page with the placeholder traits "Reliable / Curious / Helpful" and the generic escalation rules. This is theatre the camera would read aloud at the worker-detail drill-in.

Beyond those two, the wizard chrome itself is good: F1 editorial-center is implemented, copy is honest ("Skipping uses a mock provider"), the 5 steps render and route forward as designed. **Findings**: 4 P0, 5 P1, 4 P2, 2 P3.

If the operator wants to record the demo MP4 from the current build, the recording will have to (a) pre-stage gateway data + browser cookies so the SPA never refetches, or (b) ship a same-origin proxy in front of the gateway that strips the CORS problem. Without one of those, the camera will see "No teammates yet" on `/workers` while the gateway has a working worker called Maya sitting in the database.

---

## Headline blockers

| # | Severity | One-line finding | Scene impact |
|---|---|---|---|
| F01 | P0 | Gateway sends no `Access-Control-Allow-Origin` header — every browser fetch to `:7400` fails | All scenes that fetch gateway data: Scene 1 (workers grid), Scene 2 (schedules + audit), Scene 4 (skills marketplace), Scene 5 (workflows), Scene 6 (approvals + audit). |
| F02 | P0 | Worker personality (traits, great-at, escalate-when, signature) is a static frontend map keyed by worker name — newly hired workers get generic placeholder traits | C2-personality detail page (Scene 1 drill-in) — theatre the camera reads. |
| F03 | P0 | `/workers` empty-state never resolves because team fetch fails; UI shows "Hire your first AI teammate" template grid even though the gateway has 1 worker (Maya) | Scene 1 — the trading-card-deck is never shown. |
| F04 | P0 | `/workers` empty-state template grid uses **different role names** ("Marketplace seller", "Tax-prep document intake") than the `/onboarding/pick-vertical` grid ("GTM Engineer", "Marketing Intern", "Sales SDR", "Customer Support", "Accounts Receivable") — same surface, contradictory taxonomy | Investor walks the funnel; sees two different "what we offer" lists. |

---

## All findings

### F01 [P0] — Gateway returns no CORS headers; every browser-side SPA fetch fails

The gateway at `http://127.0.0.1:7400` does not set `Access-Control-Allow-Origin` on either preflight (OPTIONS) or actual responses. The SPA at `http://127.0.0.1:60894` (or `:3000`) is a **different origin** and uses `fetch` against the gateway base URL — every request is blocked by the browser before the response is read.

Confirmed via:
- `preview_network` showed 7 consecutive `ERR_FAILED` requests against `/api/v1/meta/error-reporting`, `/api/v1/approvals`, `/api/v1/approvals/bundles`, `/api/v1/notifications`, `/api/v1/team`.
- Curl with `Origin: http://127.0.0.1:60894` header returns the body but with **zero** `Access-Control-*` response headers.
- OPTIONS preflight returns `204 No Content` with no CORS headers.

Effect: the workers grid, audit list, approvals queue, skills marketplace, and "error-reporting" config all silently fail. The SPA falls back to skeleton loaders or empty states. Operator did not see this earlier because direct curl works.

**Reproduction**:
1. Open `http://127.0.0.1:60894/team` (any in-app page that fetches gateway data)
2. Open DevTools → Network
3. Observe `:7400/api/v1/team` request fails with CORS error
4. Page shows skeleton placeholders or empty-state forever

**Source pointer**: gateway CORS middleware — search `crates/argo-cli/src/handlers/gateway/` for `cors`, `Access-Control`, `allow_origin`. The SPA caller at `web/lib/api/client.ts:180` does not include `credentials: 'include'`; even if it did, the gateway would still need the `Allow-Origin` header.

**Suggested fix**: ship a Next.js API-route proxy (`web/app/api/v1/[...path]/route.ts`) that forwards to `:7400` same-origin, OR add a permissive-on-localhost CORS middleware to the gateway when `NODE_ENV=development` / `ARGO_DEMO_MODE=1`. The proxy route is the safer pick because it also lets the SPA attach the `__Host-session` cookie without same-site headaches.

---

### F02 [P0] — Worker persona is hardcoded frontend dressing, keyed by worker name

`/workers/<id>/overview` renders a `WorkerPersonalityCard` whose traits / great-at / escalate-when / signature-behaviors are **not** fetched from the gateway. They come from a static map in `web/components/skycot/worker/WorkerPersonalityCard.tsx:35-100` keyed by `worker.name`. Five named workers (Mariah, Priya, Jamal, Nori, Patrick?) are seeded; anyone else falls through to `DEFAULT_PERSONALITY` at line 99:

```
traits: ["Reliable", "Curious", "Helpful"]
greatAt: ["Working through the responsibilities you set", "Asking when something is ambiguous", ...]
escalateWhen: ["Confidence in the next action is below 80%", ...]
signature: ["Always confirms before a public or irreversible action"]
```

Maya — the only worker actually in the gateway right now — is NOT in the map. Newly hired workers via `/onboarding/first-hire` (worker name generated server-side, see `web/lib/onboarding/use-first-hire.ts`) will also miss the map.

**Per the function-vs-dressing rule**: persona text is FUNCTION when the camera drills into the worker. The runbook Scene 1 explicitly drills into a worker's C2-personality detail page. Static placeholder traits = theatre. The operator-locked memory `feedback_no_theater_in_demo` calls this out as a P0 demo-blocker.

**Reproduction**:
1. `curl http://127.0.0.1:7400/api/v1/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218` — returns Maya with `role.tagline` but no traits/great-at/escalate-when fields.
2. Navigate `/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218/overview` (if CORS fixed) — would render `DEFAULT_PERSONALITY` placeholders.
3. Camera reads "Reliable. Curious. Helpful." as Maya's personality — generic.

**Source pointer**: `web/components/skycot/worker/WorkerPersonalityCard.tsx:35-117` (the `PERSONALITIES` lookup + fallback).

**Suggested fix**: stand up a `GET /api/v1/workers/<id>/persona` endpoint backed by a persona JSON column or skill-pack metadata; have `WorkerPersonalityCard` read from it. For the demo recording, pre-seed Maya (or whoever the on-screen worker is) into the static map AND author the persona endpoint in the same brief so the trick isn't camera-visible.

---

### F03 [P0] — `/workers` empty state lingers forever because team fetch fails; the trading-card-deck never appears

`/workers/page.tsx:25` only redirects to `/team` (where the C2 holographic trading-card-deck lives) when `teamQuery.data?.workers.length > 0`. The team fetch fails via CORS (F01), so `hasWorkers === false`, so the page stays on the empty-state "Hire your first AI teammate" template grid. The investor never reaches the locked C2 design.

**Reproduction**:
1. `curl http://127.0.0.1:7400/api/v1/team` returns 1 worker (Maya)
2. Browser-navigate `/workers` — see "Hire your first AI teammate" empty state with 5 template tiles
3. Inspect `useTeam()` query — `isError: true` because CORS blocked the fetch

**Source pointer**: `web/app/(app)/workers/page.tsx:23-36` + `web/lib/hooks/use-team.ts`.

**Suggested fix**: F01 is the root cause; fix that first. As a defensive change, render an "Unable to load team — gateway unreachable" diagnostic above the empty state when `teamQuery.isError`, so the operator notices during dev instead of assuming "no workers."

---

### F04 [P0] — Onboarding's 5 vertical templates and `/workers` empty-state's 5 starter prompts list DIFFERENT roles

Within ~30 seconds of each other in the same session the investor sees TWO contradictory "what kind of teammate can I hire?" lists:

**`/onboarding/pick-vertical`** (step 4 of wizard, light editorial F1):
- GTM Engineer · Marketing Intern · Sales SDR · Customer Support · Accounts Receivable

**`/workers` empty state** (after wizard, dark in-app shell):
- Marketplace seller · Recruiter (candidate screening) · Customer support — Tier 1 · Sales SDR — outbound · Tax-prep document intake

Only 2 of the 5 roles overlap (Sales SDR, Customer Support). "Marketplace seller" and "Tax-prep document intake" don't appear in the wizard at all. The wizard's "GTM Engineer", "Marketing Intern", "Accounts Receivable" don't appear on `/workers`. The narrative thread snaps.

**Source pointers**:
- Wizard list hardcoded at `web/app/(setup)/onboarding/pick-vertical/page.tsx:24-58`
- Empty-state list at `web/components/empty-state/HireStarterPrompts.tsx` (search the repo)

**Suggested fix**: single source of truth — move the 5 starter prompts to `web/lib/verticals/starters.ts` or pull from the gateway's `/api/v1/verticals` endpoint (which currently returns `{verticals:[]}` — another finding, F05). The two surfaces should display the same 5 roles in the same order.

---

### F05 [P1] — `GET /api/v1/verticals` returns `{verticals:[]}` despite the wizard claiming 5 templates exist

The wizard's pick-vertical step would normally fetch from `/api/v1/verticals` to drive the cards (and to validate the chosen `vertical_id` against a real catalog). Today the cards are hardcoded in the SPA and the gateway endpoint returns an empty array. The `install-vertical` endpoint still accepts the hardcoded IDs without error, which masks the gap.

Risk: any future "show recommended verticals based on workspace size" or "search verticals" feature has nothing to read from. The pick-vertical UI is also lying — there are no verticals server-side to install in any meaningful sense.

**Reproduction**: `curl http://127.0.0.1:7400/api/v1/verticals` → `{verticals:[]}`

**Suggested fix**: seed the gateway with the 5 canonical vertical pack manifests at boot when `ARGO_DEMO_MODE=1`, or strip the empty endpoint until it has data. Don't leave a hollow shell.

---

### F06 [P1] — `/api/v1/skills`, `/api/v1/skills/marketplace`, `/api/v1/workflows` all 404 — Scene 4 + Scene 5 cannot run

For the wider 6-scene runbook, Scene 4 (skill marketplace) and Scene 5 (workflows) depend on these endpoints. Today:
- `GET /api/v1/skills` → 404
- `GET /api/v1/skills/marketplace` → 404
- `GET /api/v1/skill-library/list` → 404
- `GET /api/v1/workflows` → 404
- `GET /api/v1/workflows/list` → 404

The SPA `/skills` page is hardcoded "competitor pricing scrape" placeholder text and renders "6 skills · 0 shown / No skills match" because there is no list to filter.

Out of strict scope for this audit (hire flow), flagged here because the same backend gap will block the wider demo recording.

**Suggested fix**: a single demo-seed brief that puts marketplace skills + workflows behind real endpoints.

---

### F07 [P1] — `/api/v1/audit` rows all have `worker.id=""` and `worker.name="unknown"`, so any drill-in shows "unknown" worker

The audit list endpoint works (200 OK, real data, hash-chained), but every one of the ~15 rows currently in the database is a `hipaa_compliance_check` action from May 22–25 with **no worker attribution**:

```
"worker":{"id":"","name":"unknown"}
```

The runbook Scene 6 says "click a row → real provenance (NOT fabricated mock report)". With every worker reading "unknown", the camera lands on a detail panel that says "unknown ran hipaa_compliance_check at 2026-05-25T06:03:11" — function-side, but tells nothing about teammates.

**Reproduction**: `curl http://127.0.0.1:7400/api/v1/audit | python3 -m json.tool | grep -A1 worker` — every entry has empty id + "unknown" name.

**Suggested fix**: when a worker takes an action, the audit event must include the worker's id + canonical display name. This is a fan-out gap, not a presentation gap. Until then, the audit page will look like dead inventory.

---

### F08 [P1] — The signup → `/team` shortcut bypasses the 5-step wizard entirely; the runbook's Scene 3 narrative cannot be recorded by hitting "Enter demo workspace"

`web/app/auth/signup/page.tsx:106` routes the demo-mint flow to `/onboarding/welcome` AFTER calling `signIn("gateway-bearer", ...)`. In the screenshot session, submitting the signup form (with the prefilled `demo@skycot.com` email) instead landed on `/team` directly — i.e. the operator-set demo-mint code path is good, but the next-router push raced with NextAuth's session refresh and the wizard was never shown.

If the investor narrative is "hire a teammate is a 4-minute wizard", and the demo bypass deposits the operator into `/team` past the wizard, the recording either:
- shows the wizard manually (operator types `/onboarding/welcome`) — fine but breaks the natural funnel narrative
- or never reaches the wizard at all

**Reproduction**: navigate `/auth/signup` on port 60894 (demo-mode on), submit form with the prefilled email, observe the URL lands at `/team` rather than `/onboarding/welcome`. (Hard to reproduce 100% because the preview MCP also jumps between tabs — but the destination was repeatedly `/team` in this audit, not `/onboarding/welcome`.)

**Source pointer**: `web/app/auth/signup/page.tsx:106` and the `signIn("gateway-bearer", { redirect: false })` race. The session callback may also be pushing to `/team` first.

**Suggested fix**: `router.replace("/onboarding/welcome")` only AFTER `signInRes.status === "success"` and the session JWT has been confirmed via a single round-trip. Or always route to `/onboarding/welcome` from demo-mint regardless of session state — it's the demo lane, the destination is fixed.

---

### F09 [P1] — `/auth/signup` is dark; `/onboarding/*` is light. Brand register switches under the user mid-flow

The signup page renders with `data-theme="skycot-dark"`, dark navy background (`lab(2.73 -0.64 -3.71)`) — purple/cyan/pink Audiowide hero on dark. The wizard immediately after is light editorial-center per the F1 lock (warm-paper bg, `#1e1b4b` ink).

The operator-locked memories say:
- Auth pages: editorial-A "brand-medium register, seamless marketing → auth → onboarding feel" (locked 2026-05-19)
- Onboarding: F1 editorial-center, light page primary

The current state — dark auth → light onboarding — is a visible break. Either auth needs to be light editorial-A like the wizard (matching `project_auth_pages_design_locked` claim "matches Onboarding F1"), or the wizard needs a dark variant. The memory says they should match.

Function-vs-dressing: dressing-side. P1 not P0 because no scene narrates the auth page, but a recording cuts between them is jarring.

**Source pointer**: `web/app/auth/layout.tsx` or `web/components/auth/AuthEditorialShell.tsx`. Theme provider may default-to-dark for the auth group.

**Suggested fix**: set the auth group to `data-theme="skycot-light"` to match the F1 wizard recipe.

---

### F10 [P2] — "Connect Anthropic" is provider-monolithic; landing claims "provider routing"

`/onboarding/connect-provider` only offers Anthropic ("sk-ant-..."). The hero on `/landing` claims "provider routing" as a core capability — implying multi-provider choice. The wizard locks the operator into one provider. Honest fallback ("Skip uses a mock provider") preserves correctness but the marketing → product gap is visible.

P2 because the v1.0 demo isn't really about multi-provider yet (operator-locked decisions say model=latest, single-provider OK). Worth surfacing for v1.1.

**Suggested fix**: either soften the landing copy to "Anthropic Claude — multi-provider routing in v2" or accept a smaller chooser in the wizard.

---

### F11 [P2] — Welcome step does NOT auto-focus the "Get started" button; keyboard users must tab from the SAVE&EXIT to find it

The wizard advertises "↵" enter-to-continue affordances (see the `<span className="ob-cta-kbd">↵</span>` decoration on every CTA). On the welcome step, hitting Enter on page load does nothing because no element is focused. Tabbing lands on "Return home" link first, then SAVE & EXIT, then Help, then finally "Get started." Five tabs to reach the primary CTA.

Workspace step is better — its first input has `autoFocus`. Pick-vertical and first-hire also have it. Welcome is the odd one out.

**Source pointer**: `web/app/(setup)/onboarding/welcome/page.tsx:54-67` — the `<button>` has no `autoFocus`.

**Suggested fix**: add `autoFocus` to the "Get started" button on the welcome step.

---

### F12 [P2] — Pick-vertical step shows 5 cards but no "Back" CTA above the fold at 1280×800; user thinks they're stuck

At desktop preset 1280×800, the 5 vertical cards (2×2 grid + 1 alone in the third row) push the "← Back" CTA below the fold. The viewport-bottom radio-blob hides it visually. Users assume they can't go back. Tab key still works but the affordance is missing.

**Source pointer**: `web/app/(setup)/onboarding/pick-vertical/page.tsx:160-170` (the back row is at the very bottom). Same `.ob-cta-back` lives at the top in workspace.

**Suggested fix**: float a sticky "Back · Step 3 of 5" bar at the wizard top so the navigation is always visible, OR reduce the row gap on the vertical cards so the Back row fits.

---

### F13 [P2] — `2,847 COMPANIES THIS MONTH` shown in the wizard footer is the same stat as `2,847 TEAMS THIS MONTH` on `/landing` — same number, different noun

Dressing-side stat, OK per function-vs-dressing rule. Flagging because someone in the funnel will see "TEAMS" then "COMPANIES" with the exact same digit and notice; it's a copy slip, not a data issue. P2.

**Source pointer**: `web/app/(setup)/onboarding/_OnboardingShell.tsx` footer + `web/app/landing/page.tsx` hero strip.

**Suggested fix**: pick one noun (probably "TEAMS" since the SKYCOT proposition is teammates) and use it consistently.

---

### F14 [P3] — Multiple `next-server` processes (PIDs 18719, 38412, 37945, 15062) suggest leaked dev servers from prior worktree sessions

`ps aux | grep next-server` returned 4 processes. The preview MCP started a 5th. Likely benign for the audit but in a low-RAM situation can contribute to jetsam pressure (per CLAUDE.md memory-pressure section). Operator may want to kill the orphans before recording.

**Suggested fix**: operator runs `pkill -f next-server` then `pnpm --dir web dev` cleanly before the recording session.

---

### F15 [P3] — First-hire's example prompt names the worker "Maya" — the same name as the existing seeded worker

The placeholder + Example text on `/onboarding/first-hire` says "A marketing intern named Maya who drafts blog posts...". The gateway already has a worker named Maya (the seeded marketing-intern in the demo tenant). If the operator (or investor demo) uses the example as-is, they create a duplicate Maya in the same tenant. Name-collision behaviour was not tested (could be 409, could be silently allowed with two Mayas).

P3 because the recording would type something else. But unguarded duplicates is a sharp edge.

**Source pointer**: `web/app/(setup)/onboarding/first-hire/page.tsx:11` — `EXAMPLE_PROMPT` constant.

**Suggested fix**: change the example name to "Marcus" or have the gateway return a "name already taken" error envelope and the wizard show it inline.

---

## Cross-cutting observations (not findings, for orientation)

- **F1 editorial-center wizard chrome is real and on-brand.** Audiowide hero typography, weight-200 sub-copy, gradient accents, warm radial-blob background, "STEP NN OF 05" sectioned progress bar, JetBrains Mono eyebrow + footer trust badges — the locked design exists, end-to-end, across all 5 steps. This is the highest-leverage piece of investor-narrative scenery in the entire app.
- **Copy is honest where it matters.** "Skipping uses a mock provider. Workers will run, but responses are simulated until you add a real key in Settings." That's the kind of plain-speak that builds trust. Preserve it.
- **Onboarding API endpoints exist and work.** `POST /api/v1/onboarding/provision` returns `{tenant_id, workspace_id, vertical_id, next_step}` (201 Created). `POST /api/v1/onboarding/first-hire` returns `{worker_id, vertical_id, name}` (201 Created). The plumbing is real — the surface lights are off because of CORS.
- **The signup → demo-mint → wizard handoff was clearly engineered.** The code comments at `web/app/auth/signup/page.tsx:80-95` show the author understood the JWT + cookie + NextAuth race carefully. It's not a hacky shortcut. F08's "lands on `/team` not `/onboarding/welcome`" issue may be a router-prefetch race, not a logic bug.

## Resolved since last audit

First run for this dir. Prior 2026-05-25-1100-onboarding audit (in the same `ux-audits/` tree) used a different scope (Track-P recording prep) and is not directly comparable.

## Orientation score

**62 / 100**.

Up from where it would be a month ago — the F1 wizard chrome and honest copy genuinely impress. Down hard from where it needs to be to record investor footage tomorrow because:
- CORS is a hard production-blocker for ANY browser-driven demo
- Static persona = theatre the runbook explicitly forbids
- /workers vs /onboarding template mismatch breaks the funnel narrative inside 30 seconds

Five focused fixes — (a) gateway CORS / Next.js API-proxy, (b) persona endpoint + Maya seed, (c) unify the two template lists, (d) audit rows attribute their worker, (e) light-theme the auth pages — would lift this to 88+ and make the journey camera-ready.
