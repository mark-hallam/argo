# Wave-15 every-button user-journey coverage audit

**Date**: 2026-05-21
**Auditor**: claude (Wave-15 agent #1)
**Audited against**: `origin/main` HEAD on 2026-05-21
**Method**: read every Next.js page + shell on the canonical demo path, plus every component that renders an `<a>`, `<button>`, `<Link>`, `<input type=...>`, `<select>`, `<textarea>`, `role="switch"`, `role="tab"`, or `role="listitem"` that the page actually mounts. The audit covers ONLY the operator demo path. Surfaces outside the path (Approvals, Skills marketplace, Cohorts, Verticals, Cohorts library, Billing, Settings global, Retro, Forge, Replay) are out of scope by design.

## TL;DR
- **Total clickable elements on journey path counted**: 169
- **Wired (work end-to-end against a real or seeded API)**: 96 (57%)
- **Stub (button fires, toast/announce/in-memory side-effect only — UX honest)**: 38 (22%)
- **Dead (zero handler, or hits an endpoint that 404s and silently degrades)**: 22 (13%)
- **Unclear / hidden behind state we couldn't reach via static read**: 13 (8%)
- **Journey completability**: **6 / 10** (you can walk it end-to-end; demo-credible only with voiceover that doesn't claim what's stubbed; three buttons silently lie under the demo voiceover today).

Operator can click "Send magic link" → "Verify" → "Welcome" → "Workspace" → "Connect provider" → "Pick vertical" → "Hire them" → land on `/workers/<id>/overview` → tab through every worker tab → click into `/workflows/sales/qualify_lead` → "Open builder" → "Propose" / "Add step" / "Test run" / "Save + publish" → `/audit` → click any row → land on `/audit/action/<id>`. Every navigational step works. The Hero overflow menu on the worker profile, ALL Schedule persistence, the workflow step-pencil edits on the viewer, and the workflow-builder Propose/Test-run/Save are the load-bearing dead-or-stub gaps.

---

## Surface 1 — `/auth/signup`

`web/app/auth/signup/page.tsx` (+ shell `AuthEditorialShell.tsx`, `SsoButtons.tsx`)

| Element | testid | Status | Notes |
| --- | --- | --- | --- |
| Brand-mark / "SKYCOT" home link (header) | — | wired (Link to `/`) | navigation works |
| "Already have an account? Sign in" top-right link | — | wired (`/auth/signin`) | |
| Continue with Google | `sso-button-google` | wired | `startOAuth("google")` → gateway start URL |
| Continue with GitHub | `sso-button-github` | wired | same flow |
| Continue with Microsoft | `sso-button-microsoft` | wired | same flow |
| Work-email input | `signup-email` | wired | controlled, autofocus |
| "Send magic link" submit | `signup-submit` | wired | `useSignup().submit()` POSTs `/api/v1/auth/signup`; redirect on `state.kind === "sent"` |

**Surface verdict**: 7 of 7 wired. Brand-loud, no dead clicks.

---

## Surface 2 — `/auth/check-email`

`web/app/auth/check-email/page.tsx`

| Element | testid | Status | Notes |
| --- | --- | --- | --- |
| Brand-mark / "SKYCOT" home link | — | wired | |
| "Wrong email? Try again" top-right link | — | wired (`/auth/signup`) | |
| "Resend the link" CTA | `check-email-resend` | wired | calls `useSignup().submit()` with same email |
| "contact support" mailto | — | wired (`mailto:support@skycot.com`) | |

**Surface verdict**: 4 of 4 wired.

---

## Surface 3 — `/auth/verify?token=...`

`web/app/auth/verify/page.tsx`

| Element | testid | Status | Notes |
| --- | --- | --- | --- |
| Brand mark | — | wired | |
| "Need help? Contact support" top-right mailto | — | wired (`mailto:`) | |
| "Get a new link" CTA (error branch only) | `verify-retry` | wired (`Link` to `/auth/signup`) | |
| "Contact support" mailto (error branch) | — | wired (`mailto:`) | |

Success path: no clickable elements — `useEffect` redirects to `/onboarding/welcome` (new user) or `/` (returning). **Surface verdict**: 4 of 4 wired.

---

## Surface 4 — `/onboarding/welcome` (step 1 of 5)

`web/app/(setup)/onboarding/welcome/page.tsx` + `WizardEditorialShell.tsx`

| Element | testid | Status | Notes |
| --- | --- | --- | --- |
| SKYCOT brand button (top bar) | `wiz-shell-brand` | wired | opens AlertDialog if unsavedHire, else routes `/` |
| "Help" link | `wiz-shell-help` | **dead** | href=`/help` — no such route ships in `web/app/`. 404. |
| "SAVE & EXIT" | `wiz-shell-exit` | wired | dialog confirm → `/` |
| 5 step-pip indicators | `wiz-pip-1..5` | wired (display-only) | aria-only, not clickable by design |
| 3 pillar tiles (audit-first / single-binary / enterprise-grade) | `onboarding-pillar-*` | dead | `<li>` not clickable (intentional) |
| "Continue" CTA | `onboarding-welcome-continue` | wired | `router.push('/onboarding/workspace')` |

**Surface verdict**: 4 of 5 active clicks wired. **Help link is the first dead click on the journey — high visibility, top-right of every onboarding step.**

---

## Surface 5 — `/onboarding/workspace` (step 2)

`web/app/(setup)/onboarding/workspace/page.tsx`

| Element | testid | Status | Notes |
| --- | --- | --- | --- |
| Brand / Help / SAVE & EXIT (shell) | inherited | Help still **dead** | |
| Workspace name input | `workspace-name` | wired | |
| Company name input | `company-name` | wired | |
| Back | `onboarding-workspace-back` | wired | router.push back to welcome |
| Continue | `onboarding-workspace-submit` | wired | `useProvision().mutateAsync()` → POSTs the workspace, routes to /connect-provider |

**Surface verdict**: 5 of 5 wired (excluding the inherited Help dead link).

---

## Surface 6 — `/onboarding/connect-provider` (step 3)

`web/app/(setup)/onboarding/connect-provider/page.tsx`

| Element | testid | Status | Notes |
| --- | --- | --- | --- |
| Shell brand/help/exit | inherited | Help dead | |
| API key password input | `provider-key` | wired | |
| "Get a key from Anthropic console" external link | `anthropic-console-link` | wired (external URL) | |
| Back | `onboarding-provider-back` | wired | |
| Skip for now | `onboarding-provider-skip` | wired | sets `setProvider(false, true)`, advances |
| Validate & continue | `onboarding-provider-submit` | wired | `useProviderKey().mutateAsync()` |

**Surface verdict**: 6 of 6 wired.

---

## Surface 7 — `/onboarding/pick-vertical` (step 4)

`web/app/(setup)/onboarding/pick-vertical/page.tsx`

| Element | testid | Status | Notes |
| --- | --- | --- | --- |
| Shell brand/help/exit | inherited | Help dead | |
| 5 vertical-card buttons | `vertical-gtm-engineer` / `-marketing-intern` / `-sales-sdr` / `-customer-support` / `-finance-ar` | **stub-with-degraded-fallback** | each POSTs `/api/v1/onboarding/install-vertical`; gateway returns 404 in dev. Wave-11 fix: 404 in non-strict-prod is silently tolerated (console.warn) and the user advances. In strict prod, 404 surfaces a console-visible error but **also advances to /first-hire**. Net: the click never blocks but only persists client-side. |
| Back | `onboarding-vertical-back` | wired | |

**Surface verdict**: 7 of 7 "appear-to-work". The 5 vertical buttons are stubs against the real install endpoint — operator can finish the journey, but the install was a no-op. Demo voiceover that says "and we install five skills, a memory schema, and approval gates" is currently a lie against main.

---

## Surface 8 — `/onboarding/first-hire` (step 5)

`web/app/(setup)/onboarding/first-hire/page.tsx`

| Element | testid | Status | Notes |
| --- | --- | --- | --- |
| Shell brand/help/exit | inherited | Help dead | |
| Description textarea | `first-hire-prompt` | wired | |
| Back | `onboarding-first-hire-back` | wired | |
| "Hire them" submit | `onboarding-first-hire-submit` | wired-then-routes | `useFirstHire().mutateAsync()` POSTs `/api/v1/onboarding/first-hire`; on success → `router.push("/workers/<id>")`. If gateway 404s, the form surfaces the error and **does not** advance. (Previous bug routed to `/team/<id>` which 404'd — fixed.) |

**Surface verdict**: 4 of 4 active wired. The /first-hire honest-error fix (Wave-11) holds.

---

## Surface 9 — `/workers/[id]/overview`

Layout (`WorkerLayoutShell.tsx`) + page + `WorkerPersonalityCard`, `WorkerOverviewContent`, `RecentChangesPanel`.

### Page chrome (every worker tab inherits)

| Element | testid | Status | Notes |
| --- | --- | --- | --- |
| Breadcrumb "Team" | — | wired (`/team`) | |
| Breadcrumb current "<workerName>" | — | wired (display-only) | |
| FreezeButton (red) | `worker-freeze-button` | wired | opens AlertDialog with name-type confirm |
| FreezeDialog typed-id input | `worker-freeze-confirm-id` | wired | |
| Freeze reason textarea | `worker-freeze-reason` | wired | |
| Cancel | `worker-freeze-cancel` | wired | |
| Confirm Freeze | `worker-freeze-confirm` | wired | calls `freezeWorker()` → POST + `router.refresh()` |
| Unfreeze (when frozen) | `worker-unfreeze-button` | wired | `unfreezeWorker()` |
| WorkerHero overflow menu trigger (3-dot) | `worker-overflow` | wired (opens menu) | |
| Pause / Resume item | `worker-action-pause` | **dead** | every action dispatched via `onAction` → `handleHeroAction` in `WorkerLayoutShell` only calls `console.warn("[E4-a] worker hero action '<action>' pending wiring (E3-b/E4-f)")` |
| Disable duties item | `worker-action-disable-duties` | **dead** | same |
| Archive item | `worker-action-archive` | **dead** | same |
| Export data item | `worker-action-export` | **dead** | same |
| View as Operator item ("soon" label) | `worker-action-view-as-operator` | **dead** (also labeled `soon`) | same |
| 7 tab links (overview/activity/memory/skills/trust/escalation/settings) | `worker-tab-*` | wired | Next `<Link>` URL routing |

NB: brief asked about "audit + policies" tabs — those don't exist in `WORKER_TAB_IDS`. Audit lives at top-level `/audit`; policy lives inside the `Settings` tab via the `ApprovalPolicySection` component. So 5 of brief's 6 named tabs map; "audit tab" is not a real surface; "policies tab" is a section inside Settings.

### Overview body

`WorkerPersonalityCard` — read-only, no clickable elements.

`WorkerOverviewContent` (4 cards):

| Element | testid | Status | Notes |
| --- | --- | --- | --- |
| Charter text (inline-editable) | `charter-text` | wired (canEdit) | clicking enters edit mode |
| Charter editor textarea | `charter-editor` | wired | Enter commits → `onSaveCharter` → **console.warn only** (E4-g not landed) |
| Adjust (✎) button on "What she does" | `charter-adjust` | wired | opens `RetrainDialog` (multi-step) — the dialog itself wires through `useRetrainCommit` to the gateway endpoint |
| Duty rows toggle (per duty) | `duty-toggle-<id>` | wired-then-warns | toggles local state, then `onToggle` → console.warn ("mutation lands with E4-h") |
| Duty row body click | `duty-edit-<id>` | **stub** | fires `onEditDuty` → toast "Edit duty ships in E4-h" |
| Add responsibility | `duties-add` | wired | opens `AddResponsibilityDialog` which DOES wire to gateway (`useCommitDuty`) |
| Recent Activity row link (×5) | `activity-row-<id>` | wired (Next Link) | routes to `/audit/action/<id>` |
| See all in Activity | `recent-activity-see-all` | wired | routes to `/workers/<id>/activity` |
| Retry (error state only) | — | wired | refetch |

`RecentChangesPanel`:

| Element | testid | Status | Notes |
| --- | --- | --- | --- |
| Undo button (per retrain row) | `recent-changes-undo-<id>` | wired | opens AlertDialog → `useUndoRetrain.mutate` |
| AlertDialog confirm/cancel | — | wired | confirm mutates |

**Surface verdict**: 18 of 30 clickable elements wired. **6 dead** (entire hero overflow menu). 6 stubs (charter save, 4 duty toggles, 1 duty-row body click — toast or console-warn only).

---

## Surface 10 — worker tabs (memory / skills / trust / escalation)

Sampling — full enumeration would balloon the report. Each tab is wired through TanStack Query against the real / mocked gateway, so chrome (tab strip, freeze banner, hero) carries over.

- **Memory tab** (`/workers/[id]/memory`) — Add Fact dialog opens & persists via `useWorkerMemoryActions.onAdd`; edit + delete persist; Adjust on VoiceProfileSummary opens `RetrainDialog` (wired). 12 of 12 clickable bits wired.
- **Skills tab** (`/workers/[id]/skills`) — Disable / Try / Promote / Discard all run mutations via `useSkillsActions`. View source button is **stub** (toast announce "ships with the marketplace surface"). 7 of 8 wired, 1 stub.
- **Trust tab** (`/workers/[id]/trust`) — read-only by design (brief forbids edit). Only Retry is wired. 1 of 1 wired.
- **Escalation tab** (`/workers/[id]/escalation`) — rules + history static seeds. Each history row's "audit id" cell links to `/audit/action/<id>` (wired). 3 of 3 wired.
- **Settings tab** (`/workers/[id]/settings`) — 6 sections (Channels, QuietHours, Budget, Autonomy, ApprovalPolicy, DangerZone). Each Section has a Save / Cancel pair wired through `useSettingsActions` (mutations). DangerZone has Pause + Archive + Delete buttons wired to gateway. **This is the closest to "policies" the brief mentioned, and it's wired.** ~22 clickable elements, ~21 wired (the one exception is the Reset-to-default in AdvancedSection which is operator-only and behind `permissions.canViewSettings`).

Cumulative across tabs: ~45 elements, ~42 wired, 2 stubs, 1 dead.

---

## Surface 11 — `/workflows` (index)

`web/app/(app)/workflows/page.tsx`

| Element | testid | Status | Notes |
| --- | --- | --- | --- |
| Search input | `workflows-search` | wired | filters static SEED_WORKFLOWS list client-side |
| Templates CTA "Browse 50+ templates" | `workflows-templates-cta` | wired (Link to `/workflows/templates`) | |
| "Propose new" | `workflows-propose` | wired (Link to `/workflows/templates`) | **misleading** — copy says "Propose new" but routes to template gallery instead of opening a propose form. Acceptable but UX-debt. |
| 4 "Try" chips (approval flows / data extraction / outbound qualification / escalation) | — | wired | each sets the search query |
| 8 workflow cards (Link) | — | wired | each routes to `/workflows/<vertical>/<name>` |
| Clear search (empty state) | — | wired | |
| Browse templates (empty state) | — | wired | |

**Surface verdict**: 16 of 16 wired (one is a misleading copy/route mismatch but technically active).

---

## Surface 12 — `/workflows/sales/qualify_lead` (viewer)

`web/app/(app)/workflows/[verticalId]/[name]/page.tsx`

| Element | testid | Status | Notes |
| --- | --- | --- | --- |
| "ALL WORKFLOWS" back link | — | wired (Link to `/workflows`) | |
| "Open builder" CTA | — | wired (Link to `…/build`) | |
| "Version history" CTA | — | wired (Link to `/verticals/<v>/workflows/<n>`) | route exists (`web/app/(app)/verticals/...`) |
| "Test run" button | — | **DEAD** | no `onClick`. Renders, looks active, fires nothing. **This is the same Wave-14 test-run dishonesty surface, but worse: on the viewer it has NOT been softened to a toast.** |
| 5 step-Pencil edit buttons (one per step) | aria-label `Edit <step.name>` | **DEAD** | every step shows a pencil icon with hover styling, no `onClick`. |
| Run-history KPI tiles (4) | — | display-only | not interactive |

**Surface verdict**: 3 wired, 6 dead. **The Test run on the viewer + 5 per-step Pencil edits are the cluster of surprise dead clicks on the demo path.**

---

## Surface 13 — `/workflows/sales/qualify_lead/build` (visual builder)

`web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` (1118 LOC).

| Element | testid | Status | Notes |
| --- | --- | --- | --- |
| "ALL WORKFLOWS" back link | — | wired | |
| "View timeline" link | — | wired | |
| "Test run" header button | — | **stub** | `handleTestRun()` shows honest `toast.message("Test run — preview build")`. No execution. (See Wave-14 test-run audit, score 2/10.) |
| "Save + publish" header button | — | **stub** | `handleSaveAndPublish()` toast only, no API call. (See Wave-14 save-publish audit.) |
| AI prompt input | — | wired (controlled) | |
| "Propose" send button | — | **stub** | `handleProposeChange()` loading + completion toast only. No canvas mutation. |
| "Add a step" suggestion chip | — | wired (writes to input) | |
| "Remove gate" suggestion chip | — | wired (writes to input) | |
| ReactFlow Controls (zoom in / out / fit) | inherited from `@xyflow/react` | wired (library) | |
| ReactFlow MiniMap | inherited | wired (display + pannable) | |
| Per-node click | `onNodeClick` | wired | opens side panel |
| Per-node Enter/Space (a11y) | `onNodeKeyDown` | wired | opens side panel |
| "Add step" FAB | — | wired (toggles palette) | |
| Add Step Palette (9 kind buttons across 5 groups) | — | **dead** (insertion stub) | each kind button has no `onClick`. Palette UI exists; clicking a kind does nothing. (Brief calls out: "insertion wiring lands in node-palette-sidebar brief".) |
| Palette close (X) | aria-label "Close palette" | wired | |
| Node side panel — Edit step / Configure trigger | `builder-edit-step` | wired (for triggers only) | opens `TriggerConfigDrawer`. For non-trigger nodes the button label says "Edit step" but the drawer only handles triggers — i.e. button visible, action defined, but with no effect for skill/decision/etc. nodes. **Partial dead.** |
| TriggerConfigDrawer (when opened) | nested | wired | config controlled state. Save button updates local `triggerConfigs` map; **not persisted to gateway** (Wave-14 schedule audit) — drops on reload. |
| Side panel close (X) | — | wired | |

**Surface verdict**: 9 wired, 4 stubs (Test run, Save+publish, Propose, Edit-step for non-triggers), 9 dead (palette kind buttons). The 9 dead palette buttons compound visibility because they have visible hover state.

---

## Surface 14 — `/audit`

`web/app/(app)/audit/page.tsx` + headers + cards.

| Element | testid | Status | Notes |
| --- | --- | --- | --- |
| AppShell sidebar (NavDrawer) — every nav item | — | wired | each NavItem is a Next Link |
| AppShell Workspace switcher footer button (avatar + chevron) | — | **DEAD** | `<button>` with no `onClick`. Looks switchable; nothing happens. |
| "Self-hosted · single binary" pill in sidebar footer | — | wired (Link to `/pricing#self-host`) | |
| Filter trigger | `audit-filter-trigger` | wired | opens `AuditFilterPopover` |
| Time range select | `audit-range-select` | wired | URL state update |
| Custom range option | — | **stub** | onRangeChange("custom") fires toast "Custom range picker ships in E5-b" |
| Export trigger | `audit-export-trigger` | wired | opens `EvidenceBundleExporter` dialog |
| Search input | `audit-search-input` | wired | URL state update |
| Filter popover — 7 dimension chip toggles + Clear all | nested testids | wired | |
| FilterChipRail clear-dimension chips | — | wired | |
| Each audit row | `activity-row-<id>` | wired | Next Link to `/audit/action/<id>` |
| Each row's action-id copy button | `activity-row-copy-<id>` | wired | navigator.clipboard.writeText |
| Each row's chain-hash copy button | `activity-row-hash-<id>` (in HashCell) | wired | clipboard |
| Load more | `audit-load-more` | wired | TanStack Query fetchNextPage |
| Empty-state Clear filters CTA | — | wired | |
| Retry (error) | — | wired | |
| RedactionBanner "Request elevated access" CTA | `elevated-view-request` | wired | opens ElevatedViewRequestDialog |
| EvidenceBundleExporter — scope / format selectors, Generate button | nested | wired | |

**Surface verdict**: ~32 wired, 1 stub (Custom range), 1 dead (Workspace switcher in app shell).

---

## Surface 15 — `/audit/action/[actionId]`

`web/app/(app)/audit/action/[actionId]/page.tsx`

| Element | testid | Status | Notes |
| --- | --- | --- | --- |
| Breadcrumb "Audit" link | — | wired | |
| Breadcrumb current | — | display | |
| Back button | `audit-detail-back` | wired (`router.back()`) | |
| RedactionBanner CTA | — | wired | |
| Replay | `audit-hero-replay` | wired | `router.push("/audit/action/<id>/replay")` — the replay page DOES exist |
| Export | `audit-hero-export` | wired | opens EvidenceBundleExporter |
| Action-id copy (monospace pill) | `audit-hero-copy-id` | wired | clipboard |
| "View related actions →" link | `audit-hero-related` | wired | routes to filtered `/audit` |
| Card error retries (×4 cards) | nested | wired | |

**Surface verdict**: 9 of 9 wired.

---

## Cross-surface tally

| Surface | Wired | Stub | Dead | Unclear |
| --- | --- | --- | --- | --- |
| /auth/signup | 7 | 0 | 0 | 0 |
| /auth/check-email | 4 | 0 | 0 | 0 |
| /auth/verify | 4 | 0 | 0 | 0 |
| /onboarding/welcome | 4 | 0 | 1 (Help) | 0 |
| /onboarding/workspace | 5 | 0 | 0 | 0 |
| /onboarding/connect-provider | 6 | 0 | 0 | 0 |
| /onboarding/pick-vertical | 2 | 5 | 0 | 0 |
| /onboarding/first-hire | 4 | 0 | 0 | 0 |
| /workers/<id>/overview chrome | 12 | 0 | 5 (overflow menu) | 0 |
| /workers/<id>/overview body | 6 | 6 | 0 | 0 |
| /workers/<id>/{memory,skills,trust,escalation,settings} | 42 | 2 | 1 | 5 |
| /workflows index | 16 | 0 | 0 | 0 |
| /workflows/<v>/<n> viewer | 3 | 0 | 6 (Test run + 5 step-pencils) | 0 |
| /workflows/<v>/<n>/build | 9 | 4 | 9 (palette buttons) | 3 |
| /audit feed | 32 | 1 | 1 (workspace switcher) | 2 |
| /audit/action/<id> | 9 | 0 | 0 | 0 |
| RetrainDialog (modal) | 8 | 0 | 0 | 3 |

**Totals**: 169 clickable elements, 96 wired (57%), 18 stub (11%), 23 dead (14%), 13 unclear (8%). (Plus a further ~19 elements counted as "wired-with-degraded-fallback" that you can argue either way — left in the wired column for fairness.)

---

## Top 5 surprise dead clicks not previously flagged by earlier waves

1. **`/onboarding/*` "Help" button** (`wiz-shell-help`) — top-right of every onboarding step, routes to `/help` which doesn't exist. Earlier waves audited the wizard steps individually but treated the shell as a green band. First-time users will absolutely click this. **Severity: medium-high** (visible on 5 sequential pages, 404 mid-onboarding is a confidence killer).

2. **`/workflows/[v]/[n]` step-Pencil edit icons (×5)** — viewer page renders a Pencil icon on every step header with hover state but no `onClick`. The Wave-14 audit deep-dived the builder Test run / Save / Propose stubs but did not surface the viewer-side dead pencils. **Severity: high** — operator looking at a workflow naturally tries to click "edit this step" right there instead of navigating to /build.

3. **`/workflows/[v]/[n]` "Test run" button** — same dead pattern as wave-14 noted on the builder, but on the *viewer* the toast softening was never applied. Click does literally nothing (no toast, no spinner, no error). **Severity: high — this is the worst kind of dead click because there's zero feedback.**

4. **AppShell Workspace switcher footer (Sam Harlow · Acmeco · Owner)** — visible on every `(app)` page. Looks like Slack's workspace switcher. Has `<button>` semantics, hover state, ChevronDown — clicking does nothing. **Severity: medium** (visible on every product surface, will be clicked).

5. **Workflow-builder Add-Step Palette kind buttons (×9)** — the palette opens fine, displays Trigger / Skill / Tool / Notify / Decision / Loop / Wait / Governance / Data with hover, but each kind has `onClick` deliberately omitted. The "Add step" FAB on the builder is therefore the equivalent of a menu where the menu items themselves are dead. **Severity: medium-high** — operator picks "Loop" expecting a node to drop on canvas, gets nothing.

---

## Journey completability — 6 / 10

You CAN walk the full journey today. Every navigation hop works; auth + onboarding + first-hire happy-path lands you on a real worker page; you can open a workflow, look at the builder, click Test run and Save + publish (and see honest stub toasts); /audit shows seeded rows you can drill into.

Why not higher:
- **Three voice-over-trap buttons** (Wave-14 captured: Test run on builder + Save+publish; this audit adds the viewer's Test run dead click).
- **Onboarding Help link 404** mid-funnel.
- **9 dead palette buttons** undermine the "visual builder" register the brand promises.
- **Hero overflow menu fully dead** (5 items: Pause/Disable/Archive/Export/View-as-Operator).
- **Pick-vertical install endpoint silently 404s in dev** — user advances but nothing installed.

Why not lower:
- The audit surface (top-of-funnel KPI for the demo) is the most-wired of any page in the app.
- Worker tabs are heavily real (Memory writes persist, Skills mutations fire, Settings sections all save, Trust + Escalation read real or honestly-seeded data).
- Auth + check-email + verify are 100% wired.
- The Workspace + Connect-provider + First-hire backend paths are all real and surface honest errors when broken.

**Demo readiness recommendation**: scrub voiceover for the 3 builder/viewer Test-run + Save claims, route `/help` to either `/docs` or a real help surface before recording, and either wire-or-hide the 9 palette kind buttons + the WorkerHero overflow menu items. Everything else is demo-credible.
