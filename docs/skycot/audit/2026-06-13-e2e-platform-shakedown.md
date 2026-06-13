# E2E Platform Shakedown — 2026-06-13 (conductor, operator-contracted)

Live walk of every surface in Chrome (desktop 1440px + mobile 390px), logged-out
stranger journey first, then authenticated app surfaces. NO-THEATER lens
throughout. Status: **IN PROGRESS** — marketing phase complete, app phase
pending gateway restart.

## Method

- Chrome (Claude-in-Chrome), localhost:3000 SPA + :7400 gateway (dev).
- Every public route probed for HTTP status; key pages walked visually +
  full-copy review; forms actually submitted; CTAs actually clicked.
- Console + network watched per page.

## Findings — marketing / logged-out journey

### F1 · HIGH · Dead "Watch the demo" play button (confirmed theater)
`/landing#demo` renders a full video-player section ("Watch a teammate get
hired") with a play button that does NOTHING on click — no video, no modal, no
error. Second-most-tempting CTA on the page is a dead end. Fix: record the
~90s capture, or until then remove the section / point the button at the
interactive demo workspace.

### F2 · HIGH · Waitlist form shows raw "Internal Server Error"
The "first 100 teams" email-capture (the page's #1 conversion element) surfaced
a red raw `Internal Server Error` on submit while the gateway was down. The
form is real (it posts), but failure UX loses the lead silently. Fix: friendly
copy + client-side retry/queue; never lose an early-access email to a transient
backend blip. RE-TEST with gateway healthy before closing.

### F3 · HIGH · `/customers` fictional testimonials read as real
Six fictional companies with named fictional people giving quoted testimonials
and specific metrics. The "REPRESENTATIVE PROFILE" chip + one-line disclaimer
is outweighed by quote-styled cards with names/titles. Worst: compliance claims
in fictional mouths ("HIPAA-compliant out of the box" — fictional Chief
Compliance Officer; AFSL claims similarly). A skim or screenshot reads as real
customers. Fix: drop the fake person-quotes; reframe as scenario cards in
third person; never place HIPAA/AFSL claims in fabricated quotes. Contrast:
`/security` does honesty perfectly ("no aspirational claims", SOC 2 Type II
explicitly NOT claimed) — bring /customers to that standard.

### F4 · MED · Root `/` lands a stranger inside the app as "Guest"
Logged-out visit to `/` renders the Approvals app shell (empty dashboard,
"Guest · Personal workspace") instead of the marketing landing. The marketing
page lives at `/landing`. For the public flip, `/` must serve marketing to
logged-out visitors. (Dev auth-bypass may explain it — verify against the
production-auth shape before the flip.)

### F5 · MED · Inconsistent route gating for logged-out users
`/team`, `/audit`, `/approvals`, `/genesis` render as Guest while `/skills`,
`/billing`, `/settings`, `/workflows`, `/pitch`, `/coach`, `/cohorts`,
`/marketing`, `/gtm` redirect to `/auth/signin`. Confusing half-open surface;
pick one rule (all app routes gated unless demo session explicitly started).

### F6 · MED · Marketing CTA "Watch Portia refuse to overclaim" → sign-in wall
The differentiator-02 card CTA points at `/pitch`, which redirects logged-out
visitors to sign-in. The flagship truth-boundary demo is unreachable for the
prospect it exists to convince. Fix: public read-only pitch demo route or
redirect into the demo workspace.

### F7 · LOW · Terms page references status.skycot.com (does not exist yet)
Terms honestly self-label as pre-launch principles (good) but cite
`status.skycot.com` + 99.9% uptime target; status page is a Week-2 checklist
item. Align timing.

### F8 · LOW · Docs hero copy includes a plain-English expansion of "API"
"API (Application Programming Interface — the contract programs use to talk
to SKYCOT)" on the docs landing reads oddly for the developer audience that
page serves. Keep plain-language for operator surfaces; docs can say API.

### Verified GOOD (no action)
- Landing hero/brand: locked purple→pink, Audiowide, strong copy; mobile 390px
  layout holds; "Made in Melbourne" footer.
- Nav links all resolve (Roster → /teammates renders the 7-persona roster).
- Pricing page tiers ($0 / $49 per teammate / Enterprise) MATCH the implemented
  v1-tiers + the newly-wired enforcement (#2871): free=100 actions/mo,
  pro=5000. "Try with the mock provider. No credit card." is honest.
- /compare claims spot-checked against the codebase: KMS envelope encryption
  claim is REAL (crates/argo-store/src/encryption_kms.rs + tests); ⌘K palette,
  hash-chain verify, single-binary all real. One to watch: "earn trust + get
  promoted" — the graduation mechanism ships default-OFF
  (ARGO_EXECUTION_GRADUATION), so "earns trust over time" is built-but-dormant.
- /security page is the honesty benchmark for the whole site.
- /terms /privacy self-labeled placeholders, principles operative (matches
  launch-checklist legal posture).
- No console errors captured on the marketing pages visited.

## Findings — app surfaces (PENDING gateway restart)

### F0 · context · Gateway was running a stale binary
Found `schema_version_mismatch` (worker test runs migrated the dev DB past the
running release build). Rebuilt + restarted; also discovered the restart
script does NOT set `ARGO_DEMO_MODE`, which produces F9. The same staleness
class will exist in prod whenever migrations land — the deploy runbook's
restart step handles it; the DEV loop needs the same hygiene.

### F9 · HIGH · Demo-workspace CTA dead-ends with developer jargon
"Or try a demo workspace · no email required" (signup page, the primary
pre-SES conversion path) failed with literal copy: "Demo mode isn't enabled on
the gateway. Boot it with ARGO_DEMO_MODE=1." A prospect cannot act on an env
var. Fix: (a) decide demo-mode posture for the gated deploy and bake it into
the restart script/runbook; (b) replace the error with human copy + fallback
to the email path.

### F10 · HIGH · Probation-held tools never reach the Approvals inbox
The flagship journey works (see GOOD below) and ends with Ivy's first run
held by the probation window — toast says "Approve the held tools below."
But `/approvals` (whose header promises "EVERY decision your AI teammates
queue for a human shows up here") shows 0 pending. The hold lives only on the
worker page. The very first thing a new user is asked to approve does not
appear in the approvals surface. Wire probation holds (and any held-tool
grants) into the inbox as P2-low rows, or change the inbox promise.

### F11 · MED · /billing: spend attributed to "Unknown teammate" + meter spins forever
Ivy's run shows as real spend ($0.0086) but attributed to "Unknown teammate"
(the DEC-038 per-teammate attribution didn't stamp/join her run — check the
demo Run-now path against fix-runnow-process-job-worker-attribution). The
usage panel is a perpetual "Loading usage..." — the meter API exists (#2871)
but the SPA wiring is the known-pending DEC-053 item; until wired it must
degrade to an honest empty state, not an infinite spinner.

### F12 · LOW · /team header contradicts itself
Hero: "Your team is ready. 0 working right now · 22 actions this week" while
the list header says "1 teammate working". Two different "working"
definitions on one screen; "22 actions this week" for a 10-minute-old
workspace also reads odd (it counts audit rows incl. system events — label it
or count teammate actions only).

### F13 · LOW · Fresh-hire cosmetics
Ivy's avatar is a bare letter tile; the subtitle under her name is the
lowercase slug ("ivy") rather than a role title ("Collections Coordinator").
Tiny, but it's the first screen after the hire — the magic dips.

### F14 · LOW · tenant/theme endpoint 500s
`GET /api/v1/tenant/theme` returned 500 during the walk (pre-restart window).
Re-check; if real, the per-tenant theming fetch fails silently on every page.

### Verified GOOD — the flagship journey is REAL end-to-end
Walked as a stranger: demo workspace → 5-step onboarding wizard (locked F1
editorial design, witty honest copy) → Genesis "Talk it through" intake
(real `POST /api/v1/genesis/conversation`; intelligent contextual questions;
live "What we've captured" panel extracting charter / responsibilities /
cadence / never-do carve-outs into editable chips) → 7-stage build pipeline
(persona → charter → skills → SOP → eval seeds → safety gate → ready) →
truth-boundary policy synthesized from MY answers (gated claim classes:
customer-pii, financial, contracts, source-code) → role-contract review
(per-action risk + approval badges exactly matching my carve-outs;
"integrations draft-only at birth"; review checkbox required) → Confirm &
hire → real worker page (personality, escalation, trust tabs) → Run now →
probation window holds tools → real hash-chained audit rows (22/22 hashes
present, $0.009 spend in view, PII-redaction banner, honest "CLI verify not
run in browser" boundary). Time: under 5 minutes, zero console errors.
This is the investor demo, working, today — F9/F10 are the polish between
this and "epic".

## Top-10 priority order (operator view)

1. F10 probation holds → approvals inbox (product promise broken on first journey)
2. F9 demo-mode CTA dead-end + posture decision for the gated deploy
3. F1 dead demo play button on /landing
4. F3 /customers fictional testimonials (compliance claims in fake mouths)
5. F2 waitlist failure UX (works when healthy; fails ugly + auth-redirect bug)
6. F11 billing attribution "Unknown teammate" + meter spinner
7. F4/F5 logged-out route posture (/ lands in app shell; inconsistent gating)
8. F6 "Watch Portia" CTA → signin wall (public read-only pitch demo)
9. F7/F8/F12/F13 copy + cosmetic polish batch (changelog staleness, receptionist
   mention on /about, team header math, avatar/role-title)
10. F14 tenant/theme 500 re-check

## Re-test checklist after fixes
- Submit waitlist with gateway stopped → friendly error + lead not lost
- Click demo CTA with demo-mode off → human copy + email fallback
- Fresh hire → run → held tool appears in /approvals AND resolves from there
- /billing shows Ivy by name with her spend + real meter row


## Re-test 2026-06-13 (post-fix)

Re-tested against gateway build `0e401f2` (healthy on :7400, demo mode on) +
SPA on :3000, in Chrome via browser automation. F1–F9 verified PASS by the
prior agent this morning; F10/F11/F12 re-run live this session; F13 carried
as PARTIAL.

| # | Finding | Verdict | Evidence (this session unless noted) |
|---|---------|---------|--------------------------------------|
| F1 | /landing dead demo play button | PASS | Verified by prior agent this morning |
| F2 | Waitlist failure UX | PASS | Verified by prior agent this morning |
| F3 | /customers fictional testimonials | PASS | Verified by prior agent this morning |
| F4 | Logged-out / lands in app shell | PASS | Verified by prior agent this morning |
| F5 | Inconsistent route gating | PASS | Verified by prior agent this morning |
| F6 | "Watch Portia" CTA → signin wall | PASS | Verified by prior agent this morning |
| F7 | Changelog staleness | PASS | Verified by prior agent this morning |
| F8 | /about receptionist mention | PASS | Verified by prior agent this morning |
| F9 | Demo-mode CTA dead-end | PASS | Verified by prior agent this morning |
| F10 | Probation holds → approvals inbox | PASS | Full journey re-walked: demo workspace → onboarding ("Shakedown Retest") → Genesis talk-it-through (overdue invoices / Rex / never threaten legal action) → Skip the rest & build → review checkbox → Confirm & hire → Run now. Toast: "Rex ran, but their tools are held by the probation window". /approvals inbox showed BOTH holds (P2 "wants to use memory" + "wants to use todo", "Held while on probation"); approved both from the inbox; pending 2→0, approval rate 100% (2 resolved); worker Activity tab shows `permission.demo_override_granted` × 2 Succeeded + `ran a manual turn` Succeeded. Hold shown AND resolved from the inbox. |
| F11 | Billing attribution + meter | PASS | /billing Spend card: "$0.0369 — Real provider spend, attributed per teammate" with row "Rex $0.0369" (named worker, no "Unknown teammate"). Usage meter renders real numbers ("Actions this month 0 / 100 · Resets 01/07/2026"), no infinite "Loading usage…". |
| F12 | /team header math | PARTIAL | Header "WORKING 0 / 0 working right now" IS consistent with the list (1 teammate, Rex, status Idle). But action counts contradict: header says "7 actions this week" while Rex's card says "19 actions taken today" — 19 today cannot fit inside 7 this week. Working-count fixed; action-count math still wrong. |
| F13 | Avatar initials / role subtitle | PARTIAL | Avatar initials fixed ("RE" renders on worker page + team card). Genesis still stamps "Genesis Rex 019ebea4d72d1b81" as the role subtitle instead of a human role title (seen on worker header + team-card badge). |

**Tally: 11/13 PASS, 2 PARTIAL (F12, F13), 0 FAIL. No regressions in F1–F9 scope.**

### NEW findings (this session)

- **N1 — /team action-count sources disagree.** Header rollup ("7 actions this
  week") vs teammate card ("19 actions taken today") cannot both be true.
  Likely two different counters (audit events vs billable actions). This is
  the remaining F12 gap.
- **N2 — Billing meter doesn't tick on probation-held runs.** "Actions this
  month 0 / 100" stayed at 0 while Rex ran twice and took "19 actions today"
  per /team, and real spend ($0.0369) accrued. If the meter only counts
  approved/billable actions that's defensible, but 0 actions next to nonzero
  spend reads as broken. Also two "Actions this month" rows render (plan
  0/100 + metered 0/Unlimited) — confusing duplication.
- **N3 — Post-approval re-run holds the same tools again.** After approving
  both held tools, a second "Run now" re-held memory/todo with the same
  probation toast ("your OK rides on the next run"). If the 24h probation by
  design holds every action, the toast copy overpromises; if approval is
  meant to clear those tools for subsequent runs, the clear isn't applied.
  Either fix the copy or honour the grant.
- **N4 — Genesis "talk it through" answer-preview cosmetic glitch.** The
  carve-out answer ("Never threaten legal action") briefly rendered inside
  the "When does this teammate run?" input in the captured panel before the
  cadence answer overwrote it. Self-corrects; cosmetic only.
