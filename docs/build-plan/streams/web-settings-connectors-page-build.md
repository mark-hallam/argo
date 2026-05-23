# web-settings-connectors-page-build

**Owner**: claude.
**Depends on**: nothing structural (additive Next.js page).
**Authored against**: origin/main `bdf57f458` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-settings-connectors-page-build`.

🟡 **P1 — 23 of 29 REAL MCP connectors have no operator-facing surface.** Source: PR #2108 evaluator finding F-01 (MCP connector real-vs-stub inventory, 2026-05-23).

> "29 real connector crates compile + ship + pass tests. Only 6 connector ids appear in the hire-wizard catalog. `web/app/(app)/settings/connectors/page.tsx` does not exist. Operators cannot SEE — let alone connect — Notion, Linear, PagerDuty, GitHub Issues, Asana, Airtable, Zendesk, Discord, Stripe, Trello, Typeform, Figma, Google Meet, Calendly, Mailchimp, SendGrid, Postmark, Monday, Fireflies, ClickUp, iMessage, OneDrive, Salesforce."

This is the operator-facing seam that turns the 29 REAL connectors from "shipped" into "usable" — without it the work in 29 crates is invisible. Mission claim 6 (multi-channel) + mission claim 4 (skills shared) both depend on operators being able to wire up these credentials.

## Scope

Build `web/app/(app)/settings/connectors/page.tsx` as a catalog grid + per-connector connect/disconnect flow:

1. **Catalog view** — grid of 29 connector cards, each shows: connector logo (download from svgl.app per operator memory + store at `web/public/logos/<connector>.svg`), display name, one-line description, current state ("Not connected" / "Connected as <email>"), Connect / Disconnect button.
2. **Connect flow** — Connect button opens a modal/drawer that POSTs to the existing gateway OAuth-start endpoint for that connector (read the connector route registry — most are `POST /api/v1/connectors/<id>/oauth/start`). For credential-based connectors (API-key shape), modal renders a labeled input + save form.
3. **Disconnect flow** — calls existing gateway delete endpoint; refreshes card state.
4. **Filter / search** — top hero-search bar matching the locked Skills/marketplace pattern (28px gradient icon + 18px input + ⌘K shortcut + count pill + suggested-search chips per `project_skills_page_design_locked` memory).
5. **Empty state** — none expected (29 connectors), but if filter yields zero, show "No connectors match <query>" + clear-filter button.
6. **Tier badges** — Pro / Enterprise badge per connector based on the v1.0 tier mapping (read from a static `tier.ts` file in the page module — Slack/iMessage/Email = Pro per round-12 audit + pricing page; everything else = Enterprise until operator decides).

## Design language

Adopt the locked **D3 brand-consistency-pass** recipe per `project_brand_consistency_pass` memory: Audiowide hero typography + purple→pink gradient + warm radial-blob bg + #1e1b4b ink. Match the Skills page pattern from `project_skills_page_design_locked` (A1 marketplace + hero search).

## Files you may touch

- `web/app/(app)/settings/connectors/page.tsx` — NEW (~300-400 LOC).
- `web/app/(app)/settings/connectors/connector-card.tsx` — NEW component (~150 LOC).
- `web/app/(app)/settings/connectors/connect-modal.tsx` — NEW component (~200 LOC).
- `web/app/(app)/settings/connectors/connector-catalog.ts` — NEW static catalog (29 entries, name + description + tier + auth-shape) (~150 LOC).
- `web/app/(app)/settings/connectors/tier.ts` — NEW tier mapping (~30 LOC).
- `web/lib/api/connectors.ts` — NEW gateway client (list + connect-start + disconnect) (~80 LOC).
- `web/public/logos/<connector>.svg` — 29 NEW logo files (downloaded from svgl.app per operator memory).
- Add nav entry under `web/app/(app)/settings/layout.tsx` (or its sidebar component) for Connectors.

## Files you must NOT touch

- Gateway routes (read-only — connectors already expose OAuth endpoints).
- Any of the 29 connector crates under `crates/argo-mcp-connector-*`.
- `web/app/(app)/settings/api-keys/`, `members/`, `security/` (sibling settings tabs — separate concerns).

## Acceptance criteria

- [ ] Page loads at `/settings/connectors` and renders 29 connector cards.
- [ ] Each card has logo, name, tier badge, Connect/Disconnect button, current state.
- [ ] Connect button on at least one OAuth connector (gmail) wires through to the gateway OAuth-start endpoint and the browser ends up at the connector's authorization URL (verified via Chrome MCP / Playwright).
- [ ] Connect button on at least one API-key connector (notion or stripe) opens a credential-entry modal that POSTs to the credential save endpoint.
- [ ] Disconnect button removes the credential + refreshes card state.
- [ ] Filter input narrows the grid on substring match in name OR description.
- [ ] Brand recipe applied (Audiowide hero, purple→pink gradient, warm bg, #1e1b4b ink).
- [ ] Page is in the settings sidebar.
- [ ] One Playwright spec exercises the connect-modal-open flow on a stub connector.
- [ ] `pnpm typecheck` + `pnpm lint` clean in `web/`.
- [ ] BUILD_STATUS.md updated with PR number.

## Test plan

- Unit: connector-catalog.ts has 29 entries (snapshot test).
- Integration: Playwright spec renders the grid + opens connect modal on one card.
- Manual: operator-facing screenshot in PR body shows the grid in Chrome.

## Parallelism

- Manager-worker allowed: no.
- Max children: 1.

## Out of scope

- Real OAuth completion flow (callback handling) — gateway already owns the callback route.
- Multi-account-per-connector (one credential per tenant per connector is v1.0 scope).
- Per-connector scope-picker UI (defer).

~700-900 LOC across new files (component + page + logos are most of the surface).
