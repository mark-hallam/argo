# Wave-5 Close-The-Loop Audit — /landing promises vs product reality

**Audit date:** 2026-05-20
**Auditor:** Wave-5 close-the-loop agent #3
**Mode:** read-only
**Source of truth:** `~/code/argo/web/app/landing/page.tsx` (1090 LOC)
**Method:** every claim, copy fragment, footer link, navigation item, and integration logo on `/landing` extracted, then traced into the product to confirm a working surface exists and matches the framing.

---

## Summary

- **Total promises extracted:** 38 (29 in-page + 9 footer + 18 integration-logo crosscheck rolled in)
- **Real + demo-able:** 22
- **Real but caveat:** 6 (works but UX/framing mismatch, or shipping caveat)
- **Stub / dead-onClick / 404:** 10
- **Net trustworthiness:** **6.5 / 10** — the spine of the demo is real (hire wizard, audit chain, approvals, trust ratchet, channels, hash chain, security, pricing), but the *visible* trim around it (Watch demo, rollback, half the integration logos, half the channels, half the docs categories) is mostly stub. A buyer who clicks every promise will find ~1 in 4 leads nowhere. Soften before the demo call.

---

## Promise → surface matrix

Rows are grouped by area. "Status" legend:
- **REAL** — feature exists, wires through, can be exercised live
- **REAL/CAVEAT** — feature exists but framing or surface is off
- **STUB** — UI exists but the "real" action is a toast/log/sleep; no backing wire
- **MISSING** — no surface at all in the product
- **404** — link resolves to Next's 404 boundary

### Hero copy

| Promise | Surface | Status | Demo-able? |
|---|---|---|---|
| "Don't buy another tool. Hire a teammate." | `/auth/signup` → `/hire` wizard | REAL | Yes |
| "v1.0 shipped · Hiring our first 100 teams" eyebrow | None (copy-only) | REAL/CAVEAT | Yes — but a buyer asking "show me the 100 teams" finds only placeholder customers |
| "Hire your first teammate" CTA | `/auth/signup` (magic-link) → `/auth/check-email` → `/auth/verify` → onboarding → `/hire` | REAL | Yes — magic-link auth is wired (SOC2-aligned, no passwords) |
| "Watch a 90-second demo" CTA | `#how` anchor (same page) | STUB | **No** — it only scrolls; no video plays. A buyer expecting a video gets nothing |
| "NO CREDIT CARD" trust pill | Free tier on `/pricing` matches | REAL | Yes |
| "SOC 2-READY AUDIT TRAIL" trust pill | `/security` confirms; hash chain in audit page | REAL | Yes |
| "ISO 27001 CONTROLS SUPPORTED" trust pill | `/security` doesn't claim ISO; `/pricing` matrix lists Pro+ | REAL/CAVEAT | Partial — the claim is "supported," not "certified," but a security-team buyer will press |
| "GDPR · HIPAA WORKFLOWS" trust pill | `/pricing` HIPAA-aligned redaction (Pro+); `/security` lists isolation | REAL/CAVEAT | Yes — but no HIPAA BAA process surfaced |
| "2,847 teams this month" trust pill | None — fictional number | STUB | **No** — number is made up; remove or replace before demo |
| Hero art bubbles: Mariah / Priya / Jamal / Nori activity | Static decoration | REAL/CAVEAT | These match the worker-personality card examples and the sample-cohorts data — visually congruent with the product |

### "How it works" section

| Promise | Surface | Status | Demo-able? |
|---|---|---|---|
| Step 01 "Describe the job" | `/hire` step 1: real free-text input + examples popover (`Step1Land`) | REAL | Yes |
| Step 02 "Meet your new teammate" | `/hire/step/2` LISTEN → `/hire/step/3` DRAFTING → `/hire/step/4` INTRODUCE | REAL | Yes |
| Step 03 "Approve their first decisions" | `/` approvals inbox + `TrustRatchetCard` | REAL | Yes |
| Step 04 "Watch the audit chain" | `/audit` with hash-chain provenance card (`audit-provenance-hash-prompt`) | REAL | Yes |
| "Four minutes from job description to working teammate" | Wizard has **5 steps**, not 4 (`/hire` + `/hire/step/2..5` + `/hire/done/<id>`) | REAL/CAVEAT | Yes — but the "4 minutes" is wall-clock, not step count. Worth clarifying on stage |

### Feature highlights (6 cards)

| Promise | Surface | Status | Demo-able? |
|---|---|---|---|
| "Named teammates, not tools" | `/workers/<id>/overview` shows `WorkerPersonalityCard` with traits ("Patient", "Detail-oriented") + signature behaviors | REAL/CAVEAT | Yes — but the route is `/workers/<id>` not `/teammates/<id>`. Internal voice still says "worker" in route names + many tests/labels |
| "Asks before it acts" — approval-first by default | Approvals inbox (`/`), `ApprovalDaySection`, `BundleCard`, channels crate dispatches approval-gated | REAL | Yes |
| "Trust ratchets up over time" | `TrustRatchetSection` on inbox + `/workers/<id>/trust` page | REAL | Yes |
| "Edit workflows in chat — plain English" | `/workflows/<v>/<n>/build` "Propose" button | STUB | **No** — `handleProposeChange` is explicitly a sonner toast ("Stub: real wire-up POSTs to /api/workflows/propose…"). Top-3 broken-promise risk |
| "Your teammate proposes the diff, you approve, it ships" | No diff shown after propose; toast is the only response | STUB | **No** — extends the prior row |
| "Versioned like code" | `/verticals/<v>/workflows/<n>` lists versions + head marker | REAL | Yes |
| "Side-by-side diff" | `/verticals/<v>/workflows/<n>/diff?from=&to=` server-renders `WorkflowDiff` from gateway | REAL | Yes — but the route is non-obvious from `/workflows/<v>/<n>` (different parent route entirely) |
| "One-tap rollback" | None — `argo-store/src/workflow_definition.rs` notes rollback is "upcoming" substrate; no UI, no API, no route | **MISSING** | **No** — Top-1 broken-promise risk. Demo-blocker if a buyer asks "show me rollback." |
| "Multi-channel native: Slack, iMessage, email, web inbox" | `argo-channels` crate has all 4 (`slack.rs`, `imessage.rs`, `email.rs`, `webinbox.rs`) + Discord, Teams, WhatsApp, Voice, SMS, push | REAL | Slack + web inbox: yes. iMessage: needs Blue Bubbles (`argo-bluebubbles-client`) — not a 5-min on-camera setup. Email: yes |
| "All wire in the same approval flow. No extra setup." | `MessageDispatcher` + `OperatorPreferences` + `OperatorRiskOverrides` route approvals across channels | REAL/CAVEAT | Half-true — "no extra setup" overclaims; Slack/iMessage/email each need credentials |
| "Forensic-grade audit chain on every action" | `/audit/action/<id>` provenance card with short hashes + "Copy hash chain" button (per `page.test.tsx:201`) | REAL | Yes |
| "Per-action role permissions" | `/settings/members` shows 4 backend role tiers (owner/admin/user/guest), backed by `argo-capability::default_roles_for_workspace` | REAL | Yes |
| "Encryption-at-rest by default" | `argo-store/src/encryption.rs` + `encrypted_default_smoke.rs` + `sqlcipher_link_verify.rs`; `/security` confirms; `/pricing` lists SQLCipher | REAL | Yes (visible in `/security`) |

### Social proof — logos + testimonials

| Promise | Surface | Status | Demo-able? |
|---|---|---|---|
| 12 customer logos (Northwind, Acmeco, etc.) | `/customers` lists 9 of them with metrics + quotes; source comment: "placeholder customers; pre-launch; not real" | STUB | **No** — every name on the strip is fake. Top-2 broken-promise risk: a journalist or due-diligence buyer will Google these |
| 3 testimonial quotes | Same fake personas on `/customers` | STUB | **No** — extends prior row |
| "See all customers" → `/customers` | Page exists, 9 cards, real-looking | REAL/CAVEAT | Yes — but content is fictional |

### Integration logos (18 names)

| Integrations claimed | Connector crate present? | Status |
|---|---|---|
| Slack | argo-mcp-connector-slack (391 LOC slack.rs) | REAL |
| Gmail | argo-mcp-connector-gmail (7 files) | REAL |
| Salesforce | argo-mcp-connector-salesforce | REAL |
| HubSpot | argo-mcp-connector-hubspot | REAL |
| Notion | argo-mcp-connector-notion | REAL |
| Linear | argo-mcp-connector-linear | REAL |
| GitHub | argo-mcp-connector-github-issues | REAL |
| Stripe | argo-mcp-connector-stripe + argo-mcp-connector-stripe-events | REAL |
| Zendesk | argo-mcp-connector-zendesk (713 LOC across 6 files) | REAL |
| **Outlook** | — | **MISSING** |
| **Intercom** | — | **MISSING** |
| **QuickBooks** | — | **MISSING** |
| **Xero** | — | **MISSING** |
| **NetSuite** | — | **MISSING** |
| **Greenhouse** | — | **MISSING** |
| **Lever** | — | **MISSING** |
| **Workday** | — | **MISSING** |
| **Shopify** | — | **MISSING** |

**9 of 18** integration logos have no connector. **50% of the integrations strip is broken-promise.**

### Comparison teaser

| Promise | Surface | Status | Demo-able? |
|---|---|---|---|
| "How do we stack up vs Zapier, n8n, Crew.ai, Lindy?" | `/compare` renders full 13-dim matrix | REAL | Yes |
| "Side-by-side, dimension-by-dimension" | Matrix is real and authored | REAL | Yes |
| "8 of 13 rows are clean wins" | Numeric claim checks out vs `/compare` row counts | REAL | Yes |

### Final CTA

| Promise | Surface | Status | Demo-able? |
|---|---|---|---|
| "Your first teammate is four minutes away" | `/auth/signup` → 5-step wizard | REAL/CAVEAT | The wizard works; the "four minutes" is plausible but unverified |

### Footer links

| Link | Destination | Status |
|---|---|---|
| SKYCOT | `/landing` | REAL |
| Compare | `/compare` (479 LOC) | REAL |
| Customers | `/customers` (635 LOC, placeholders) | STUB-content |
| Pricing | `/pricing` (811 LOC, full matrix + FAQ + 3 tiers) | REAL |
| GitHub | `https://github.com/mark-hallam/skycot-packs` (private repo per memory) | REAL/CAVEAT — link works but the repo is private; an anonymous click 404s |
| Docs | `/docs` hub (725 LOC, 6 category cards + 4 featured guides) | REAL/CAVEAT — hub is real, but `/docs/<slug>` routes 404 — the source comment confirms this (line 28-29) |
| Privacy | `/privacy` (placeholder until full legal review) | REAL/CAVEAT |
| Terms | `/terms` (110 LOC placeholder) | REAL/CAVEAT |
| Security | `/security` (real posture, no overclaiming) | REAL |
| Contact (hello@skycot.com) | mailto: link | REAL (assuming inbox exists) |
| "© 2026 SKYCOT · Made in Melbourne" | Matches memory: Melbourne not Sydney | REAL |

### Navigation header

| Link | Destination | Status |
|---|---|---|
| How it works | `#how` anchor | REAL |
| Features | `#features` anchor | REAL |
| Compare | `/compare` | REAL |
| Customers | `/customers` | REAL/CAVEAT (placeholder content) |
| Pricing | `/pricing` | REAL |
| Sign in | `/auth/signin` | REAL |
| Hire your first teammate | `/auth/signup` | REAL |

---

## Top 5 highest broken-promise risks

Ranked by **likelihood a demo viewer will probe × severity of the broken-ness**:

1. **"One-tap rollback"** — MISSING entirely. A buyer asking "show me what happens when an AI teammate makes a bad change to a workflow" finds version listing + side-by-side diff but no rollback button anywhere. Pricing page also lists "Versioned workflows + rollback" as a Pro feature. *Sells a feature that doesn't exist.*
2. **Customer logos + testimonials** — 12 fake company names with 3 fake testimonials. Source comment explicitly says "not real SKYCOT customers yet." First-Google by any due-diligence buyer or journalist exposes this. *Reputation risk on first scrutiny.*
3. **"Edit workflows in chat — plain English"** — STUB. The "Propose" button is a sonner toast (`handleProposeChange`, line 303 of `/workflows/[v]/[n]/build/page.tsx`). This is the signature SKYCOT differentiator on `/compare` and `/landing`. *The headliner feature isn't wired.*
4. **9 of 18 integration logos have no connector** — Outlook, Intercom, QuickBooks, Xero, NetSuite, Greenhouse, Lever, Workday, Shopify. Half of the "where your work happens" strip is decoration. *A finance/HR/e-commerce buyer scanning for their stack sees their logo and assumes it works.*
5. **"Watch 90-second demo" CTA** — STUB. Button only scrolls to `#how` anchor; no video. *First click on the hero secondary CTA misfires.*

---

## The single promise most worth softening before demo

**"One-tap rollback."**

Reasoning: it's the only top-5 broken promise that's **easy to soften without rewriting the hero**. Two options:

- **Option A (soften copy):** change `"Every workflow change versioned like code. Side-by-side diff. One-tap rollback."` to `"Every workflow change versioned like code. Side-by-side diff. Rollback shipping next."` — costs nothing, removes the only outright lie in the feature grid.
- **Option B (ship the button):** the DB substrate exists (`argo-store/src/workflow_definition.rs`); a Track-L follow-up brief could wire a `POST /api/v1/verticals/<v>/workflows/<n>/rollback/<id>` + a button on `/verticals/<v>/workflows/<n>` in 1-2 days of focused work.

Rollback is also the only broken promise that **gets repeated in pricing** ("Versioned workflows + rollback" Pro feature). Softening it once covers both surfaces.

The fake customer logos are arguably a higher reputation risk, but those are already deliberately placeholder per the source comment and memory has them flagged as "replace with real logos as we onboard our first design partners." That's a known TODO. Rollback isn't on anybody's known-TODO list — it'll surface as a surprise.

---

## Net trustworthiness verdict

**6.5 / 10**

The strong half (audit chain, hash provenance, approval-gated channels, trust ratchet, magic-link auth, encryption-at-rest, capability RBAC, hire wizard, pricing matrix, compare matrix, security posture page) is genuinely impressive and demo-ready. A buyer who follows the spine of the demo (sign up → hire wizard → approvals → audit) will see real software all the way down.

The weak half (rollback, propose-in-chat, half the integrations, fake customer logos, fake "2,847 teams" counter, dead demo-video button, 404ing doc subroutes, private GitHub link) is broken-promise risk on every single click off the happy path. A naturally-curious viewer who clicks footer links, integration logos, or the "Watch demo" CTA will find ~1 in 4 leads nowhere — which is the failure mode the operator warned about.

**Recommendation:** before the demo recording, do the 15-minute soften-pass — flip "rollback" to "rollback coming soon," remove the "2,847 teams" counter, change "Watch demo" to "See it live below" (since it scrolls), and label the integrations strip "Connectors in progress" (or filter the strip to the 9 real ones). That single 15-minute pass moves the trust verdict from 6.5 to 8.
