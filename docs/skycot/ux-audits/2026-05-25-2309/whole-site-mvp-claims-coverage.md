# Whole-site MVP rollup + 6-MISSION-claim coverage

**Run**: 2026-05-25T23:09 (Melbourne local). Read-only e2e UX audit against `http://127.0.0.1:3000` (Next.js dev) and gateway on `http://127.0.0.1:7400` (schema v81). Auth session via dev `demo-mint` cookie as workspace `demo+e2e`.

**Scope** (per agent invocation, the whole-site rollup not covered by the 4 sister agents): `/`, `/landing`, `/customers`, `/compare`, app shell `/cohorts`, `/settings` + sub-tabs, `/workflows/templates`, `/skills-canvas` (if present), `/team`, `/brand`, theme-toggle behaviour, and a viewport-responsiveness sweep at 375 / 768 / 1440 / 1920.

**Not in scope** (other agents own these): `/workers` + `/workers/[id]` (Scenes 1+2), `/onboarding/*` (Scene 3), `/skills` marketplace + `/workflows` (Scenes 4+5), `/approvals` + `/audit` (Scene 6).

---

## TL;DR

22 findings: **3 P0**, **9 P1**, **8 P2**, **2 P3**. The marketing surfaces are demo-ready (no P0s there) and the workflows-template gallery is the strongest page in the rollup. The three P0s sit inside the app shell: a broken `next-themes` toggle that ignores user input across the entire SPA, a hydration mismatch on `/settings/members` that drops most styling, and `/workflows` still showing the `MOCK` badge plus a non-canon worker name (`BY RIO`) where Scene 5 will narrate. The locked `D top-tabs` design for `/settings` is not implemented — current `/settings/` is a card-grid landing page with no horizontal tab navigation between sub-pages. `/brand` reference still uses pre-D3 tokens (blue accent, no Audiowide wordmark, no violet→pink gradient).

## Headline blockers

- **F1 [P0] Theme toggle is non-functional across the SPA** — clicking the sidebar toggle writes `theme=dark` to `localStorage` (the *current* value, not the next) and never flips the `<html>` class. SPA is permanently dark whether the operator wants it or not.
- **F2 [P0] `/workflows` page header reads `WORKFLOWS · MOCK`** and one of the visible workflow cards is authored "BY RIO" (non-canon worker name; canonical set is Mariah/Peta/Priya/Jamal/Nori). Scene 5 of the v1 demo runbook narrates this page on camera.
- **F3 [P0] `/settings/members` ships an unstyled / hydration-mismatched DOM** — `<MemberRow>` markup loads but the `styled-jsx` class never paints; the whole "Team & roles" page renders as a vertical wall of text with no card chrome.

---

## All findings, in order

### Marketing surfaces (/, /landing, /customers, /compare)

### F1 [P0] — Theme toggle never switches modes; persists wrong value

The sidebar `[data-testid=appshell-theme-toggle]` button. The `aria-label` reads "Switch to light theme" while in dark mode. Clicking calls `setTheme("light")` per `web/components/skycot/ThemeToggle.tsx:29`, but after the click: `<html class="… dark">` is unchanged, `data-theme="skycot-dark"` is unchanged, the `aria-label` stays "Switch to light theme", and `localStorage.theme` writes `"dark"` (the *current* value, not `"light"`). Verified across 3 clicks on /approvals at 1440 viewport — class never flips. Likely the `ThemeProvider` is configured with `attribute="class"` but the FOUC-prevention script in `<head>` is writing back `"dark"` before the React render commits, OR `value={{light:"skycot-light", dark:"skycot-dark"}}` is wired but the storage key the FOUC script reads is different from what `next-themes` sets.
Reproduction:
- Visit `/approvals` (or any app-shell route) at viewport ≥1280 so the sidebar is visible
- Click the moon icon next to the user pill (bottom-left)
- Observe `<html>` className remains `… dark` and `localStorage.theme === "dark"`
- Repeat — same result on every click
File: `web/components/skycot/ThemeToggle.tsx:29` (caller) + likely `web/app/layout.tsx` or `web/components/skycot/ThemeProvider` (the broken FOUC/storage wiring)
Suggested fix: align the `next-themes` `storageKey`, `attribute`, and `value` props with the FOUC `<script>` in the root layout; verify by checking that one click changes `localStorage.theme` to `"light"` AND removes the `dark` class on `<html>`.

### F2 [P0] — `/workflows` shows "MOCK" badge + non-canon worker "RIO"

Visible on the /workflows index (which is the Scene 5 destination per the runbook). Top eyebrow chip reads `WORKFLOWS · MOCK`. One of the visible cards (`qualify_lead`, GTM-engineer, 7 steps, v4, 3 days ago) is attributed "BY RIO" — that name does not appear in the v1.0 canonical roster (`memory project_pm_cohorts_peta_patrick` + `memory project_workers_page_design_locked`). Both items are dressing-side under the function-vs-dressing rule but **the camera lands on this page in Scene 5**, so they are P0 demo blockers. The drill-in test: this is what the viewer sees while the operator narrates "your AI workforce" in Scene 5.
Reproduction:
- Sign in as `demo+e2e`
- Navigate `/workflows`
- Eyebrow chip reads `WORKFLOWS · MOCK`
- Scroll to the third card (`qualify_lead`) — author chip reads `BY RIO`
File: `web/app/(app)/workflows/page.tsx` + the fixture source (likely `web/lib/workflows-data.ts` or seeded gateway data)
Suggested fix: remove the `· MOCK` chip from the page header before recording; rewrite the `RIO` author chip to one of {Mariah, Peta, Priya, Jamal, Nori}. Sister agent for Scene 5 will likely call this out too — confirm before any rebrand.

### F3 [P0] — `/settings/members` hydration mismatch + unstyled render

The page compiles and serves a 200, but after hydration the React tree throws "A tree hydrated but some attributes of the server rendered HTML didn't match the client properties" inside `MembersPage` → `MemberRow` → `MemberExpiryPill expiresAt="2026-06-24…"`. Visually: zero card chrome, all member rows + role tier headers run as a single vertical text dump, role badges show only as inline-highlight backgrounds (Owner=purple bar, Admin=pink bar, User=cyan bar, Guest=grey bar with no chip outline). The `styled-jsx` class `jsx-385397…` is emitted on the DOM but the corresponding `<style>` block does not appear to apply.
Reproduction:
- Sign in as `demo+e2e`
- Visit `/settings/members`
- Observe stacked-text layout; open DevTools console — `[error] A tree hydrated but some attributes of the server rendered HTML didn't match…`
File: `web/app/(app)/settings/members/page.tsx` + `web/app/(app)/settings/members/MemberRow.tsx` + `members-styles.ts`
Suggested fix: the most common cause is a `Date.now()` / locale-sensitive date format in `MemberExpiryPill expiresAt` rendered differently SSR vs client. Move the date formatter behind a `mounted` guard, or pre-compute the formatted string on the server and pass it through.

### F4 [P1] — `/settings` index does not implement the locked D top-tabs design

Memory `project_settings_page_design_locked` calls for "D top-tabs (horizontal tabs + sub-tabs + 2-col content with aside cards), Danger zone right-aligned in tab row". Current `/settings/page.tsx` ships a 2-column card-grid landing page (Workspace / Team & roles / Notifications / Security / Provider keys / API keys / Notification channels / Knowledge / Connectors / Billing / Data export). Three cards say `Coming soon` (Workspace, Notifications, Provider keys). When you drill into any sub-route (e.g. `/settings/security`, `/settings/connectors`), there are no horizontal tabs at the top connecting the sub-pages — each is a standalone surface with no parent-tab navigation.
Reproduction:
- Visit `/settings` — see card grid, no tabs
- Click `Security` card — lands on `/settings/security`, no tab strip
- Manually navigate to `/settings/connectors` — different page chrome, still no tab strip
File: `web/app/(app)/settings/page.tsx` + `web/app/(app)/settings/layout.tsx`
Suggested fix: implement the D top-tabs from the locked design as a shared `layout.tsx` element across all `settings/*` routes. Move the "Coming soon" cards into a settings-overview tab rather than the index.

### F5 [P1] — `/brand` reference page is pre-D3-consistency-pass

Memory `project_brand_consistency_pass` mandates Audiowide hero + violet→pink gradient + warm radial-blob background + `#1e1b4b` ink across all in-product files, locked 2026-05-19. The `/brand` reference at `/brand` shows: wordmark rendered in **lowercase blue Inter** ("skycot"), `--brand-primary` token displayed as a flat `#3B82F6`-ish blue (no gradient), no Audiowide on the page, no gradient anywhere. Per the function-vs-dressing rule this is an internal designer doc and the demo never navigates there, so this is NOT a demo blocker — but it actively misleads the next designer who lands on it.
Reproduction:
- Visit `/brand`
- Wordmark panel renders `skycot` in blue Inter, not `SKYCOT` in white Audiowide on a gradient surface
- "Color" token grid shows `--brand-primary` as plain blue chip
File: `web/app/(app)/brand/page.tsx` + the design-tokens reference (`docs/skycot/brand/DESIGN_TOKENS.md` per the page copy)
Suggested fix: rebuild `/brand` against the D3 brand-consistency recipe; either delete the page entirely (it has no demo / no customer use) or align it with the locked tokens so it isn't a misleading "source of truth".

### F6 [P1] — `/cohorts` empty state hides the locked D3 magazine-tile design

Memory `project_cohorts_page_design_locked` describes "6 dept tabs + asymmetric lead/stacked/compact (1 lead + 2 stacked + 4 compact)". When `cohorts.length === 0` the page shows the generic `<EmptyState illustration={CohortsIllustration} headline="No cohorts" cta={{label:"Create cohort"}}>` instead of any preview of the locked layout (`web/app/(app)/cohorts/page.tsx:79-90`). For a fresh-tenant demo, /cohorts looks identical to any generic empty SaaS page. Not a Scene-5 blocker since the runbook doesn't navigate to /cohorts, but it weakens the "Platform builds workers" claim if an investor probes the sidebar.
Reproduction:
- Sign in as a tenant with zero cohorts
- Visit `/cohorts`
- See generic empty-state card, not the locked magazine-tile layout
File: `web/app/(app)/cohorts/page.tsx:79-90`
Suggested fix: either seed at least one cohort for the demo tenant so the locked design renders, or render a "preview-mode" magazine grid in the empty state.

### F7 [P1] — Settings → Security shows "Failed to fetch" on SAML status

`/settings/security` renders the SSO panel with `Not configured` badge but inline error text `Failed to fetch`. The two CTAs (`Configure SAML`, `Download SP metadata XML`) still render. Likely the gateway endpoint for SAML status isn't wired or the SPA is calling a wrong URL.
Reproduction:
- Visit `/settings/security`
- See `Failed to fetch` red text under the SAML panel
File: `web/app/(app)/settings/security/page.tsx` + the SAML-status hook
Suggested fix: confirm gateway endpoint exists (`/api/v1/settings/saml-status` or similar); if not, render a more graceful "Not yet wired" placeholder instead of a network-error.

### F8 [P1] — Marketing nav cross-page theme drift

`/landing` and `/customers` render light theme (Audiowide-on-cream-paper, violet→pink gradient on the hero word). `/compare` renders dark theme (foreground text on near-black, gradient muted). Hover state on a marketing CTA from a dark page back to landing causes a visible flash. Since these are marketing pages an unauthenticated visitor lands on, treat the whole set as one theme cohort.
Reproduction:
- Visit `/landing` → light
- Click "Compare" in marketing nav → dark
- Click `SKYCOT` brand mark → light
File: `web/app/landing/page.css`, `web/app/customers/page.css`, `web/app/compare/page.css` (and likely the marketing-layout wrapper)
Suggested fix: lock all marketing pages to the same theme (operator preference is light per `project_brand_consistency_pass`); if dark is intentional on `/compare` as an editorial choice, do it for the whole marketing surface, not just one page.

### F9 [P1] — Hero "teammate cards" overlap with SKYCOT logo block on `/` (1440)

On the root marketing page at 1440 width, the floating worker-summary cards (`Mariah: 3 listings live, 2 buyers waiting on you.` / `Priya: 8 candidates screened today.` / etc) sit ON TOP of a gradient SKYCOT-logo block in the upper-right. The center card (`Jamal: 12 tickets resolved, median reply 4m.`) is partly occluded by the logo block. Looks like a z-index / positioning bug between two absolutely-positioned hero ornaments.
Reproduction:
- Open `/` at 1440 width (logged out or with demo-mint cookie)
- Hero region — cards group right of the headline overlaps with a gradient "SKYCOT" block
File: `web/app/landing/page.tsx` (or root `page.tsx`) + `web/app/landing/page.css`
Suggested fix: nudge the SKYCOT logo block z-index below the worker cards, or offset their grid columns so they don't share the same right edge.

### F10 [P1] — `/audit/action/<id>/replay` fabricates reconstruction text

While testing route navigation I drifted into `/audit/action/019e5dbaec037fa2819eab3f27f3f41e/replay` (an action that doesn't exist; the prior page showed "Action not found"). The replay page rendered: `ORIGINAL: Reconstructed original output for 019e5dbaec037fa2819eab3f27f3f41e.` + `REPLAYED: Reconstructed original output for 019e5dbaec037fa2819eab3f27f3f41e. Policy tightening would block this output today.` + a "Policy regression detected" banner — even though there is no action behind that ID. This is **function-side theater**: the page generates plausible-looking forensic content from the URL parameter. Note: this is sibling-agent scope (Scene 6 / audit), flagging here as a passing note because I hit it during navigation. Scene 6 narrator must NOT drill into a non-existent replay during recording.
Reproduction:
- Visit `/audit/action/anything/replay`
- See fabricated "Reconstructed original output for anything." text
File: `web/app/(app)/audit/action/[actionId]/replay/page.tsx` (or similar)
Suggested fix: when the action ID has no underlying audit row, render an explicit "No replay available — action not found" instead of a fabricated reconstruction. Defer detailed scope to the Scene-6 audit agent.

### F11 [P2] — Marketing CTA "Watch a 90-second demo" has no destination wired

`/landing` shows two CTAs in the hero: "Hire your first teammate" → `/auth/signup` (works), and "Watch a 90-second demo" — clicking it does nothing visible (no modal opens, no scroll to a video). Since the v1.0 demo MP4 isn't being publicly released, this CTA has no asset to land on yet. Not a demo blocker (demo doesn't show /landing) but a public-facing dead link.
Reproduction:
- Visit `/landing` at desktop
- Click "Watch a 90-second demo" in the hero
- Nothing visibly happens
File: `web/app/landing/page.tsx`
Suggested fix: hide the CTA, or wire it to a placeholder modal saying "Demo coming soon — book a 1:1" with a calendar link. Defer until after MP4 records.

### F12 [P2] — `/settings` "Coming soon" badges on core surfaces

Three of the eleven settings cards display `Coming soon`: Workspace, Notifications, Provider keys. Notification channels has its own card next to the disabled Notifications card. Provider keys is the v1-locked surface for managing Anthropic/OpenAI keys per `crates/argo-config/src/providers.rs`. Demo doesn't navigate here, but for an investor who clicks Settings as a sanity check, three "coming soon" labels in eleven cells read as half-finished.
Reproduction: visit `/settings`, scan card badges
File: `web/app/(app)/settings/page.tsx`
Suggested fix: remove the "Coming soon" pills; either implement the card content or drop the card from the index.

### F13 [P2] — `/cohorts` "Create cohort" CTA links to a route that doesn't exist

The empty-state `<EmptyState cta={{label:"Create cohort", href:"/cohorts/new"}}>` (`web/app/(app)/cohorts/page.tsx:87`). `/cohorts/new` is not a defined route in `web/app/(app)/cohorts/` (only `page.tsx`, `layout.tsx`, `error.tsx`, `page.test.tsx`). Clicking the CTA will 404 into the brand-themed not-found page (which is itself well-designed — see F18).
Reproduction: `/cohorts` (empty) → click "Create cohort" → lands on `/cohorts/new` → 404
File: `web/app/(app)/cohorts/page.tsx:87`
Suggested fix: build `/cohorts/new` (the cohort-creation wizard), OR change the CTA to a modal-based create flow on the current page.

### F14 [P2] — Marketing nav anchors point at fragments that may not exist

`/landing` marketing nav links: `How it works` → `#how`, `Features` → `#features`. Pages /customers, /compare don't include those anchors. Clicking `How it works` from /compare won't jump correctly — it'll navigate to `/compare#how` then probably just no-op.
Reproduction: Visit `/compare`. Click "How it works" in the nav. Nothing scrolls; URL becomes `/compare#how` but no element with that id exists.
File: `web/components/marketing/MarketingNav.tsx`
Suggested fix: rewrite nav hrefs to `/landing#how` / `/landing#features` absolutely (currently they appear to be relative-hash links).

### F15 [P2] — `/team` empty state duplicates the `/workers` hire-template grid

`/team` empty state below the Audiowide hero shows: "Hire your first AI teammate" + 4 "OR START FROM A TEMPLATE" cards (Marketplace seller / Recruiter / Customer support / Sales SDR — outbound). The exact same hire-template grid appears in the onboarding wizard step 5 (`/onboarding/first-hire`) and probably on `/workers` (Scene 1 agent will confirm). Three different surfaces serving the same affordance with the same templates is confusing — operator may end up trying to hire from /team when the canonical surface is /workers.
Reproduction: visit `/team` with no team members; scroll past the hero
File: `web/app/(app)/team/page.tsx`
Suggested fix: keep `/team` as a list-of-humans surface (operator + invited operators) and route the "hire a teammate" affordance to `/workers/new` or the onboarding wizard. Don't render the same cards in three places.

### F16 [P2] — `/workflows/templates` "Propose new" CTA on /workflows is a sonner-stub

Memory `project_v1_demo_mp4_ship_gate.md` notes "Propose button is sonner-toast stub". Visible on `/workflows` index. Documented; included here as a reminder that Scene 5 narrator should NOT click "Propose new" on camera. Not a finding per se, but worth pinning so the runbook stays accurate.

### F17 [P2] — Wide-viewport (1920) on app shell wastes horizontal real-estate

App shell at 1920 width: main content column appears constrained to ~max-w-6xl (≈1152px) with empty grey margin on both sides. Investor-class monitors are 1440-2560 in 2026; the demo recording may be on a wider display than the dev preview. Cards + sparklines on /approvals look adrift in the middle.
Reproduction: resize to 1920×1080, visit `/approvals`, see ~380px of empty margin on each side
File: `web/components/skycot/AppShell.tsx` + per-page wrappers
Suggested fix: bump max-width to 1440 or 1600 for app-shell content surfaces; or center the cards-row tighter with a max-width content rail.

### F18 [P3] — 404 page is excellent but the suggested destinations could be smarter

`/skills-canvas` (a route that doesn't exist) renders the global 404 with hero "That page isn't here." in Audiowide gradient, three quick-links: Approvals / Workers / Docs. Beautiful — but operators arriving here from a /cohorts/new 404 (see F13) probably want Cohorts, not Approvals. Consider context-aware suggestions based on the incoming path segment.
File: `web/app/not-found.tsx`
Suggested fix: nice-to-have; defer.

### F19 [P3] — Marketing-nav social-proof eyebrow numbers don't refresh

The eyebrow chip on `/landing` shows `V1.0 SHIPPED · HIRING OUR FIRST 100 TEAMS` while `/customers` shows `2,847 TEAMS THIS MONTH`. Both are dressing-side per the positioning doc and OK for the POC demo. Mention only to note the inconsistency in case a future public-release pass wants to unify.

### F20 [P1] — Hydration warning on app-shell pages broader than just /settings/members

Console error harvest showed React hydration warnings on multiple app-shell pages, not just /settings/members — the full transcript was 66KB. Common root cause likely: client-only state (next-themes resolved theme, sonner toaster, motion provider) mounted before SSR markup matches. Symptom on /settings/members is the worst because that page also uses `styled-jsx`, but the underlying issue likely affects render correctness of many pages.
File: probably `web/app/layout.tsx` (theme + motion providers) + per-page `<style jsx>` blocks
Suggested fix: walk the providers in the root layout, ensure each has `suppressHydrationWarning` where appropriate (next-themes especially), and audit any `<style jsx>` blocks that rely on runtime values.

### F21 [P1] — Sidebar workspace pill shows "demo+e2e / Personal workspace"

Bottom-left of app shell. The workspace name "demo+e2e" is the demo tenant slug, fine for testing. The label "Personal workspace" beneath it is the **fixed** label that always appears regardless of the actual workspace type (an enterprise tenant would still see "Personal workspace"). Multi-tenant claim weakened.
Reproduction: any app-shell page; bottom-left
File: `web/components/skycot/AppShell.tsx` (the UserPill component)
Suggested fix: read the workspace type from the tenant metadata and render "Personal workspace" / "Team workspace" / "Enterprise workspace" accordingly.

### F22 [P2] — Marketing footer / legal links not validated

I did not click through to `/privacy`, `/terms`, `/security`, `/sla`, `/help` from the marketing nav (out of scope). Routes exist per `web/app/` listing. Recommend a separate pass before any public release of `/landing`.

---

## Resolved since last audit

None — this is the first whole-site MVP rollup at this timestamp; prior 2026-05-25-1100 audits ran against different scopes.

---

## Orientation score: **70 / 100**

The marketing surfaces are demo-ready and brand-consistent. The app shell carries one load-bearing P0 (theme toggle) and two narrower P0s (/settings/members hydration, /workflows MOCK + RIO). The single biggest risk is the theme-toggle being broken for everyone, not just the demo — that's a customer-visible regression in the SPA. Locked-design adherence is uneven: cohorts empty state, /settings index, and /brand all diverge from their locked memories.

---

## MISSION-claim coverage rollup

- **Claim 1 — Teammate, not tool** — 🟢 **WORKS in marketing scope.** /landing and /customers lean hard into the "named teammate, not workflow" framing; the hero of /compare ("A teammate isn't a workflow. It's the thing the workflow serves.") and the four customer testimonials all use first-name worker references (Mariah / Jamal / Priya / Patrick). The "How it works" four-step (`Describe → Meet → Approve → Audit`) reinforces this. Caveat: I did not audit /workers (sister agent owns Scenes 1+2) — the in-app expression of this claim is theirs to score.
- **Claim 2 — Delegation is the unit** — 🟡 **PARTIAL in MVP rollup scope.** I don't see delegation surfaced as a first-class action anywhere in my scope. /workflows/templates exposes 50 template cards (delegation-shaped playbooks), and the marketing /compare table has "AI delegates back to operator" comparisons. But the in-app delegation surface (Scene 5's workflow runner) is owned by the skills/workflows sister agent. From the rollup pages alone, delegation is *described* (workflow templates, approval gates in marketing copy) but not *visibly invoked*.
- **Claim 3 — Platform builds workers** — 🟡 **PARTIAL.** The hire-template grid is duplicated across /team, /onboarding/first-hire, and (likely) /workers — three places where you can start a worker (F15). The brand-loud "Hire your first teammate" hero on /team makes the platform-builds-workers claim feel real. BUT: /cohorts in fresh-tenant demo shows a generic empty state (F6) instead of the locked D3 magazine layout that would actually *show* a built workforce. The "platform builds" message is told, not shown, in the rollup scope.
- **Claim 4 — Skills shared + learned** — 🟢 **WORKS** in the surface I observed. /workflows/templates renders 50 templates in an A1-marketplace pattern (hero search "Search 50+ templates by name, step, or description" + vertical filter chips + 50 of 50 results), proving the marketplace-feels-like-a-marketplace experience holds at /workflows/templates even before /skills (which is owned by the sister agent) is audited. Each card shows step-count, version, risk-tier, and CRM-touched fields. Caveat: I did not click "Use template" → builder; the install/clone-into-builder flow is sister-agent scope.
- **Claim 5 — Any-PC execution** — 🔴 **INVISIBLE** in the rollup scope. Settings → Connectors lists 29 MCP-shaped integrations (Airtable / Asana / Calendly / ClickUp / Discord / Figma / Fireflies / Google Calendar / GitHub Issues / … with `ENTERPRISE` badges + Connect buttons) — that's the MCP runtime path made visible. But computer-use (the macOS executor, claim-5's other half) isn't surfaced anywhere in my scope. Settings → Provider keys is "Coming soon" (F12). The CU substrate is invisible to the user from the rollup pages. Sister agents for /workflows + /audit may show it via workflow runs that invoke macOS — they own the verdict for the in-app render of claim 5.
- **Claim 6 — Multi-channel + multi-tenant** — 🟡 **PARTIAL.** Multi-channel is well-told on /landing ("Slack, iMessage, email, web inbox — all wire in the same approval flow") and Settings → Notification channels surfaces a card. Multi-tenant is weaker: F21 (the workspace pill labels every tenant as "Personal workspace" regardless of type) actively undermines the multi-tenant claim. /settings/members IS the multi-tenant surface (roles + cohort scope), but F3 (hydration broke its render) means you literally cannot see the multi-tenant design properly today.
