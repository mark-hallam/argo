# Button + action wiring audit — wave 1

**Auditor:** Wave-1 audit agent #2 (button + action wiring)
**Audited:** 2026-05-20
**Scope:** every page under `~/code/argo/web/app/` — `(app)`, `(setup)`, `(hire)`, `auth/*`, marketing surfaces
**Method:** read each `page.tsx`, enumerate every `<button>`, `<Link>`, `onClick`, `onSubmit`, form handler, role="button". Categorize WIRED / TODO / DEAD / UNCLEAR.

---

## TL;DR

**~33 interactive elements are visibly DEAD or stubbed-without-wiring across the surfaces a demo recording would touch.** The worst offenders cluster on:

1. **`/workflows` index** — search input is dead (no `onChange`), "Propose new" button is dead, all 4 "TRY" chips are dead
2. **`/workflows/[v]/[name]` viewer** — "Test run" button has no handler; per-step pencil "Edit" buttons all dead (6+ per workflow)
3. **`/workflows/[v]/[name]/build` builder** — top-bar "Test run" / "Save + publish" / floating "Add step" / panel "Edit step" / "Propose" all stubs (the prompt input fakes a 1.2s timer and resets)
4. **`/docs`** — search input has no `onChange`; all category cards link to `/docs/<slug>` routes that don't exist (page comment admits "will 404 gracefully")
5. **Approvals inbox `/` (root app)** — top-right "Hire teammate" button is dead (just a `<Button>` with `aria-label`, no `onClick` / no `href`); the "Notifications" bell `armed` pattern silently swallows the very first click (deliberate, but confusing)

The auth flow, onboarding wizard, hire wizard, audit detail/replay, /team, worker tabs (memory/skills/settings/activity/trust/overview), members invite modal, data-export, customers, pricing, changelog, compare, landing-hero CTAs are all **WIRED** or correctly disabled with v1.x affordance (SSO buttons).

Top 5 specific dead buttons to fix before the demo recording:

| Where | What | File:line | Why bad |
|------|------|-----------|---------|
| Approvals header | "Hire teammate" button | `web/app/(app)/page.tsx:431-434` | Top-of-inbox CTA → no handler. Should `router.push("/hire")` |
| Workflow builder | "Save + publish" | `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:547-549` | Headline action of the whole class-leading builder. Just a styled `<button>` |
| Workflow viewer | "Test run" | `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:729-731` | One of three primary CTAs on the page |
| Workflows index | Search input | `web/app/(app)/workflows/page.tsx:441-447` | No `onChange`. User types, nothing happens. SKYCOT brand promise of "find the right playbook" silently broken |
| Docs hub | Search input | `web/app/docs/page.tsx:637-642` | No `onChange`. Same silent-fail UX as workflows |

---

## Per-page tables

### `/landing` — [21 total / 17 wired / 0 todo / 4 dead]

| Element | Status | Notes |
|---------|--------|-------|
| Nav: "How it works" anchor | ✅ WIRED | `href="#how"` jumps section |
| Nav: "Features" anchor | ✅ WIRED | `href="#features"` |
| Nav: "Compare" link | ✅ WIRED | `<Link href="/compare">` |
| Nav: "Customers" link | ✅ WIRED | `<Link href="/customers">` |
| Nav: "Pricing" link | ✅ WIRED | `<Link href="/pricing">` |
| Top-right "Sign in" | ✅ WIRED | `/auth/signin` |
| Top-right "Hire your first worker" pill | ✅ WIRED | `/auth/signup` |
| Hero primary CTA "Hire your first worker" | ✅ WIRED | `/auth/signup` |
| Hero secondary "Watch a 90-second demo" | ⚠️ TODO | `href="#how"` — jumps to "How it works" section, no actual demo video |
| Compare-band CTA "See the comparison" | ✅ WIRED | `/compare` |
| Final CTA "Hire your first worker" | ✅ WIRED | `/auth/signup` |
| Footer "Customers" link | ✅ WIRED | `/customers` |
| Footer "See all customers" CTA | ✅ WIRED | `/customers` |
| Footer "Compare" link | ✅ WIRED | `/compare` |
| Footer "Pricing" link | ✅ WIRED | `/pricing` |
| Footer "SKYCOT" link | ✅ WIRED | `/landing` |
| Footer "GitHub" link | ✅ WIRED | external github URL |
| Footer "Contact" link | ✅ WIRED | mailto |
| **Footer "Docs" anchor** | ❌ DEAD | `href="#"` (`landing/page.tsx:1067`) |
| **Footer "Privacy" anchor** | ❌ DEAD | `href="#"` (`landing/page.tsx:1070`) |
| **Footer "Terms" anchor** | ❌ DEAD | `href="#"` (`landing/page.tsx:1073`) |
| **Footer "Security" anchor** | ❌ DEAD | `href="#"` (`landing/page.tsx:1076`) |

### `/auth/signup` — [6 total / 3 wired / 0 todo / 3 dead-by-design]

| Element | Status | Notes |
|---------|--------|-------|
| Top-right "Sign in" link | ✅ WIRED | `/auth/signin` |
| Email input | ✅ WIRED | controlled state |
| "Send magic link" submit | ✅ WIRED | `submit({ email })` |
| **SSO Google button** | ❌ DEAD (deliberate) | `disabled` + "v1.x" pill — design-locked placeholder. Fine for prospects, not a bug |
| **SSO GitHub button** | ❌ DEAD (deliberate) | Same |
| **SSO Microsoft button** | ❌ DEAD (deliberate) | Same |

### `/auth/check-email` — [3 total / 3 wired / 0 todo / 0 dead]

| Element | Status | Notes |
|---------|--------|-------|
| Top-right "Try again" link | ✅ WIRED | `/auth/signup` |
| "Resend the link" button | ✅ WIRED | `onResend()` calls `submit({ email })` |
| "contact support" mailto | ✅ WIRED | `mailto:support@skycot.com` |

### `/auth/signin` — [6 total / 3 wired / 0 todo / 3 dead-by-design]

Same shape as signup — email form WIRED, SSO buttons disabled-with-v1.x.

### `/auth/verify` — [3 total / 3 wired / 0 todo / 0 dead]

| Element | Status | Notes |
|---------|--------|-------|
| Top-right "Contact support" | ✅ WIRED | mailto |
| "Get a new link" CTA | ✅ WIRED | `/auth/signup` |
| "Contact support" footer | ✅ WIRED | mailto |

### `/` (Approvals inbox) — [11+ total / 8+ wired / 1 todo / 1 dead]

| Element | Status | Notes |
|---------|--------|-------|
| Filter popover trigger | ✅ WIRED | Wires to `<FilterPopover>` |
| Notifications bell trigger | ⚠️ TODO | First click only flips `armed` state; popover opens on the *second* click (`page.tsx:351-367`) — looks broken to the operator |
| **"Hire teammate" primary CTA** | ❌ DEAD | `<Button variant="primary" aria-label="Hire teammate">` with no `onClick`/`href` (`page.tsx:431-434`). Should route to `/hire` |
| Search input | ✅ WIRED | controlled `setQuery` |
| Theme picker (footer) | ✅ WIRED | own component |
| Approve / Refuse / Undo per row | ✅ WIRED | via `BatchGroupBlock` callbacks |
| "Try again" on error | ✅ WIRED | `approvalsQuery.refetch()` |
| "Clear all" filters | ✅ WIRED | `onClearAll` |
| Empty-state "Clear all" | ✅ WIRED | inline button |

### `/team` — [8 total / 8 wired / 0 todo / 0 dead]

All interactive: search input (wired with debounce), search-clear X (wired), status filter `<select>` (wired), filter-chip-rail (wired), "Hire" CTA (wired to `/hire`), clear-all buttons (wired), teammate cards (wired via `<TeammateCard>`).

### `/workers` — [1 total / 1 wired / 0 todo / 0 dead]

Empty-state has a single "Hire a worker" CTA → `/hire` (WIRED). Other content redirects to `/team`.

### `/workers/[workerId]/overview` — delegates entirely to `<WorkerOverviewContent>`, `<WorkerPersonalityCard>`, `<RecentChangesPanel>`. Not audited at component level in this wave.

### `/workers/[workerId]/memory` — [6 total / 6 wired / 0 todo / 0 dead]

Add-fact dialog, edit fact, delete fact, retry, voice-profile adjust — all wired through `useWorkerMemoryActions`.

### `/workers/[workerId]/skills` — [4 total / 4 wired / 0 todo / 0 dead]

Disable / try / promote / discard skill — all wired through `useSkillsActions`. "View source" surfaces a toast that says "ships with marketplace surface" — that's a deliberate seam, not a dead button.

### `/workers/[workerId]/audit` — **DOES NOT EXIST**

Mission asked for this; the route doesn't exist in the codebase. WorkerTabs.tsx defines only 6 tab IDs: overview, activity, memory, skills, trust, settings. The activity tab includes a "Open in Audit timeline →" link that routes to `/audit?worker=…` — so worker-scoped audit lives on the cross-worker audit page, not a per-worker subroute.

### `/workers/[workerId]/policies` — **DOES NOT EXIST**

Policies are surfaced inside the Settings tab via `<ApprovalPolicySection>`. No standalone /policies route.

### `/workers/[workerId]/settings` — [6 sections, all wired]

Channels / QuietHours / Budget / Autonomy / ApprovalPolicy / DangerZone (pause/archive/delete). All wired via `useSettingsActions`. Read-only mode honored when worker is paused.

### `/workers/[workerId]/activity` — [3 total / 3 wired / 0 todo / 0 dead]

Filter bar, load-more, clear-filters all wired through `useWorkerActivity` infinite query.

### `/workers/[workerId]/trust` — [1 total / 1 wired]

Just a read-only `<TrustGradeCard>` + a retry button on error. No edit affordances by design.

### `/workers/[workerId]/evals` — [2 total / 2 wired]

`<EvalRunHistory>` selection + "Run an eval" CTA on empty state (routes to `/workers/<id>/evals/new` — which doesn't exist as a `page.tsx`, **this is a soft DEAD link**).

### `/workers/[workerId]/responsibilities/new` — wired via `<ProposeWorkflowChat>`.

### `/workflows` — [20+ total / 9 wired / 0 todo / 11 dead]

| Element | Status | Notes |
|---------|--------|-------|
| **Search input** | ❌ DEAD | No `onChange` handler (`workflows/page.tsx:441-447`). User can type but nothing happens |
| "Browse 50+ templates" link | ✅ WIRED | `/workflows/templates` |
| **"Propose new" button** | ❌ DEAD | No `onClick` (`workflows/page.tsx:457-460`) |
| **TRY chip "approval flows"** | ❌ DEAD | No `onClick` (`workflows/page.tsx:466-468`) |
| **TRY chip "data extraction"** | ❌ DEAD | Same |
| **TRY chip "outbound qualification"** | ❌ DEAD | Same |
| **TRY chip "escalation"** | ❌ DEAD | Same |
| All 8 workflow cards | ✅ WIRED | `<Link>` to `/workflows/<v>/<name>` |

### `/workflows/[verticalId]/[name]` (viewer) — [10+ total / 4 wired / 0 todo / 7+ dead]

| Element | Status | Notes |
|---------|--------|-------|
| "← ALL WORKFLOWS" back | ✅ WIRED | `<Link>` |
| "Open builder" button | ✅ WIRED | `<Link>` to `/build` |
| "Version history" button | ✅ WIRED | `<Link>` to `/verticals/<v>/workflows/<n>` |
| **"Test run" button** | ❌ DEAD | No `onClick` (`workflows/[verticalId]/[name]/page.tsx:729-731`) |
| **Per-step pencil "Edit" buttons** (6+ instances) | ❌ DEAD | All `<button type="button">` with no `onClick` (`page.tsx:759-761`). Page comment promises "operator can edit a step by clicking the pencil; that opens an inline editor" — it does NOT |

### `/workflows/[verticalId]/[name]/build` (builder) — [10+ total / 3 wired / 0 todo / 7 dead/stub]

| Element | Status | Notes |
|---------|--------|-------|
| "← ALL WORKFLOWS" back | ✅ WIRED | `<Link>` |
| "View timeline" button | ✅ WIRED | `<Link>` |
| **"Test run" header button** | ❌ DEAD | No `onClick` (`build/page.tsx:544-546`) |
| **"Save + publish" header button** | ❌ DEAD | No `onClick` (`build/page.tsx:547-549`) — **this is the headline action of the entire visual builder** |
| AI prompt input | ✅ WIRED | controlled state |
| **"Propose" send button** | ⚠️ TODO | `handleProposeChange()` fakes a 1.2s timer with `setTimeout` and clears the input. Comment at `build/page.tsx:305` admits "Stub: real wire-up POSTs to /api/workflows/propose" |
| Suggest chip "Add a step" | ✅ WIRED | sets prompt text |
| Suggest chip "Remove gate" | ✅ WIRED | sets prompt text |
| Canvas node clicks (open panel) | ✅ WIRED | `onNodeClick` |
| **"Add step" floating button** | ❌ DEAD | No `onClick` (`build/page.tsx:608-610`) |
| Panel close X | ✅ WIRED | `setSelectedNode(null)` |
| **Panel "Edit step" CTA** | ❌ DEAD | No `onClick` (`build/page.tsx:662-682`) |

### `/workflows/templates` — [12+ total / 12+ wired / 0 todo / 0 dead]

Search input (wired), vertical chips (wired), risk chips (wired), template cards (wired to `/workflows/templates/[id]`).

### `/workflows/templates/[templateId]` — [3 total / 3 wired]

"← ALL TEMPLATES" back, "Use template" CTA (routes to builder with `?from_template=` query — and the builder consumes that param at `build/page.tsx:264-281`, good), "Browse more templates" link. All wired.

### `/audit` — [8+ total / 8+ wired / 0 todo / 0 dead]

Range selector (wired), search input with debounce (wired), filter popover (wired), filter chips (wired), load-more (wired), retry (wired), clear-filters (wired), export trigger (wired to `<EvidenceBundleExporter>`).

### `/audit/action/[actionId]` — [5 total / 5 wired / 0 todo / 0 dead]

Breadcrumb, back button (`router.back()`), replay button (`router.push(/replay)`), export button (opens exporter dialog), retry. All wired.

### `/audit/action/[actionId]/replay` — [3 total / 3 wired]

Back to action, export replay artifact (surfaces an info toast pointing at E5-e — deliberate seam), retry. All wired.

### `/compare` — [1 total / 1 wired]

Single "Hire your first teammate →" CTA inside the table footer. WIRED.

### `/pricing` — [10+ total / 10 wired / 0 todo / 0 dead]

Brand pill (wired), nav links (wired), "Sign in" / "Hire" CTAs (wired), 3 tier CTAs (wired — Free → /signup, Pro → /signup?tier=pro, Enterprise → mailto), 6 FAQ accordion buttons (wired with open/close state), final CTA (wired), all footer links (wired).

### `/customers` — [12+ total / 12 wired / 0 todo / 0 dead]

Nav links + 2 top-right CTAs + 9 customer cards (display only) + final mailto + footer — all WIRED or static display.

### `/skills` — [10+ total / 10 wired / 0 todo / 0 dead]

Search input (wired with `setQuery`), 6 try-chips (wired — set query), skill cards (wired to `/skills/[skillId]`).

### `/skills/[skillId]` — [1 total / 1 wired]

This is a stub page that just shows the skillId and a "← Back to the marketplace" link. The page explicitly labels itself "COMING IN v1.x" — deliberate placeholder, not dead.

### `/settings` — [8 total / 2 wired / 6 dead-by-design]

| Element | Status | Notes |
|---------|--------|-------|
| "Workspace" tile | ❌ DEAD (deliberate) | `coming: true` flag → rendered as div with "v1.x" pill, not a link |
| "Team & roles" tile | ✅ WIRED | `<Link href="/settings/members">` |
| "Notifications" tile | ❌ DEAD (deliberate) | v1.x placeholder |
| "Security" tile | ❌ DEAD (deliberate) | v1.x placeholder |
| "Provider keys" tile | ❌ DEAD (deliberate) | v1.x placeholder |
| "Connectors" tile | ❌ DEAD (deliberate) | v1.x placeholder |
| "Billing" tile | ✅ WIRED | `<Link href="/billing">` |
| "Data export" tile | ✅ WIRED | `<Link href="/settings/data-export">` |

The 5 v1.x tiles are visually disabled with `opacity: 0.72` + `cursor: not-allowed` + "v1.x" badge. Fine for a launch surface; just confirm operator's OK with showing the placeholders rather than hiding them.

### `/settings/members` — [5+ total / 4 wired / 1 stub]

| Element | Status | Notes |
|---------|--------|-------|
| "Invite teammate" button | ✅ WIRED | opens modal |
| Invite email input | ✅ WIRED | controlled |
| 4 role-picker pills (Owner/Admin/User/Guest) | ✅ WIRED | `setInviteRole` |
| "Cancel" modal button | ✅ WIRED | closes modal |
| **"Send invite" button** | ⚠️ TODO | `members/page.tsx:573-577` — just closes the modal and clears the input. Comment: "Stub: real wire-up POSTs to gateway /api/v1/workspace/members" |
| **Per-row MoreVertical action menus** | ❌ DEAD | `members/page.tsx:518-520` — `<button>` with no `onClick` |

### `/settings/data-export` — [2 total / 2 wired]

"Request export" + "Download .tar.gz" — both wired through `useExportRequest()` and `apiGet`.

### `/hire` (Step 1) — fully wired through `<WizardEditorialShell>` + `<Step1Land>` with `useStep1`/`useHireDraft`.

### `/hire/step/[step]` — All 4 dynamic steps (2-5) routed correctly, redirect chain enforces step-1 prerequisites, each step component owns its own CTAs.

### `/hire/done/[workerId]` — auto-redirects to `/` after 1.5s. No buttons.

### `/onboarding/welcome` — Single "Continue" CTA, WIRED.

### `/onboarding/workspace` — Workspace name + company name inputs, Back / Continue buttons, all WIRED via `useProvision` mutation.

### `/onboarding/connect-provider` — API key input, "Get a key from console" external link, Back / Skip / Validate buttons — all WIRED.

### `/onboarding/pick-vertical` — 5 vertical-template cards (all wired via `onChoose` → `fetch(...)` + router push), Back button. WIRED.

### `/onboarding/first-hire` — Textarea + Back + "Hire them" submit — WIRED through `useFirstHire` mutation.

### `/changelog` — All RSS link, "Get notified" mailto CTA, week-anchor links, all WIRED.

### `/docs` — [10+ total / 9 wired / 1 dead, 4-6 dead links]

| Element | Status | Notes |
|---------|--------|-------|
| Brand + nav links | ✅ WIRED | All `<Link>` |
| Top-right "Hire your first worker" pill | ✅ WIRED | `/auth/signup` |
| **Search input** | ❌ DEAD | No `onChange`/`onSubmit` (`docs/page.tsx:637-642`) |
| 6 category cards | 🤔 UNCLEAR | Each `<Link href="/docs/<slug>">` — but those routes don't exist as pages. The page's own comment at line 28-29 admits "Cards link to `/docs/<slug>` pseudo-routes — those will 404 gracefully" |
| 4 featured guides | 🤔 UNCLEAR | Same: link to deep `/docs/<slug>/<sub>` paths that don't exist |
| Help band CTAs | ✅ WIRED | mailto + changelog link |

---

## TODO and DEAD elements — root-cause groupings

- **Empty `onClick` (button rendered, no handler):** 17
  - Approvals "Hire teammate" (1)
  - Landing footer "Docs/Privacy/Terms/Security" `href="#"` (4)
  - Workflow viewer "Test run" + per-step pencils (~7)
  - Workflow builder "Test run" + "Save+publish" + "Add step" + "Edit step" (4)
  - Settings/members MoreVertical actions (1)
- **Dead search inputs (no `onChange`):** 2 (`/workflows`, `/docs`)
- **Dead try-chips (no `onClick`):** 4 (all on `/workflows` index)
- **Stubbed handlers (fake delay or just close modal):** 3
  - `/workflows/.../build` AI propose — 1.2s fake timer
  - `/settings/members` Send invite — closes modal only
  - `/audit/.../replay` Export — surfaces info toast (deliberate seam, not really dead)
- **Deliberate v1.x placeholders (visually disabled, design-locked):** 9
  - 3 SSO buttons on signup + 3 on signin
  - 5 v1.x settings tiles (Workspace/Notifications/Security/Providers/Connectors)
  - `/skills/[skillId]` stub page itself
- **404-bound links (routes don't exist):** 10+
  - All 6 docs categories + 4 featured guides → `/docs/<slug>`
  - `/workers/<id>/evals/new` from evals empty state

---

## Recommended fixes (priority ordered)

**P0 — fix before demo recording (operator-facing dead buttons in the happy path):**

1. **Wire `/` (Approvals inbox) "Hire teammate" button** → `web/app/(app)/page.tsx:431-434`. One-line fix: wrap in `<Link href="/hire">` or add `onClick={() => router.push("/hire")}`
2. **Wire `/workflows` search input** → `web/app/(app)/workflows/page.tsx:441-447`. Either filter `SEED_WORKFLOWS` client-side (the page is already `"use client"`) or hide the input until v1.x
3. **Wire or hide `/workflows` TRY chips + "Propose new" button** → same file lines 457-470. Cheapest fix: hide them with `display:none` until they have handlers
4. **Wire or remove "Test run" buttons** on both `/workflows/[v]/[name]` (line 729) and `/workflows/.../build` (line 545). For v1 they could surface a "Test runs ship in v1.x" toast — anything but silent fail
5. **Wire "Save + publish" in builder** → `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:547-549`. This is the headline action. At minimum surface an info toast pointing at the upcoming brief
6. **Wire per-step pencil "Edit" buttons in viewer** → `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:759-761`. The page comment promises clicking opens an inline editor; either deliver that or remove the pencils

**P1 — fix before public launch:**

7. **`/docs` search + dead category links** — either implement minimal doc pages (even 200-word stubs is enough) or replace the search with a "coming in v1.x" pill and replace the category cards with a "What's coming" callout
8. **`/landing` footer dead anchors** — Docs/Privacy/Terms/Security all `href="#"`. Either link to real pages (`/docs`, `/legal/privacy`, `/legal/terms`, `/security`) or remove them
9. **`/settings/members` "Send invite"** — currently just closes the modal. Either wire to gateway or add a "✓ Invite sent" optimistic toast
10. **`/workflows/.../build` "Propose" AI button** — replace fake 1.2s `setTimeout` with either a real `/api/workflows/propose` call (the route exists at `app/api/workflows/propose/`) or a clearer "AI proposals ship in v1.x" toast

**P2 — quality-of-life:**

11. **Approvals notifications-bell `armed` pattern** (`page.tsx:347-367`) — first click silently arms, second opens popover. Either remove the lazy-mount gate or surface a tiny spinner on the first click
12. **Settings/members MoreVertical menu** — render a proper Radix `<DropdownMenu>` with "Change role / Resend invite / Remove" actions, even if some of those are stubs
13. **Workers `/workers/<id>/evals/new`** — empty-state CTA links to a route that doesn't have a `page.tsx`. Either create the page or change the CTA copy

**Out of scope for this audit:** the worker overview / personality-card sub-components, every individual approval-card action button, individual cohort cards, individual customer cards — those are wave-2 territory.
