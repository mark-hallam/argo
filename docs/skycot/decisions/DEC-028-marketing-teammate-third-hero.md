---
id: DEC-028
date: 2026-05-31
status: active
supersedes: none
---

# DEC-028 — Marketing teammate is the third real working hero teammate (Genesis-built, draft-only)

## Decision

SKYCOT builds a **Marketing teammate** as the **third hero teammate** — alongside
**Portia** (the Pitch Agent, [[DEC-016]]) and **Grant** (the GTM/inbound teammate,
[[DEC-018]]). Operator-locked 2026-05-31 (GO-AHEAD captured same-turn).

The Marketing teammate **owns top-of-funnel marketing + social** as a standing
*responsibility*, not a one-off task:

- Getting **articles written** (thought-leadership / company content).
- **Social listening** — finding posts and mentions of SKYCOT (and adjacent
  conversations the company should be in) and surfacing them.
- **Drafting responses** to people online (replies, comments, engagement).
- Building the **company profile + thought-leadership** presence over time.

## How it's built

- **Built THROUGH Genesis.** The teammate is created from a plain-English role
  description that Genesis ([[DEC-010]], [[DEC-025]]) synthesizes into a real
  teammate — so building it **doubles as live proof that Genesis works** (the
  recursive showcase: SKYCOT's own marketing teammate is birthed by the same
  Genesis the product sells).
- **Writes in Mark's documented voice.** The content/draft outputs follow the
  canonical voice profile at
  [`docs/skycot/marketing/VOICE_PROFILE.md`](../marketing/VOICE_PROFILE.md).

## MVP discipline — draft-only + approval-gated + listening-first

This is the load-bearing constraint, and it is non-negotiable for the MVP:

- **Listening-first.** The first capability is finding mentions / relevant
  conversations and surfacing them — observation before action.
- **Draft-only.** Everything the teammate produces (articles, replies, posts) is
  a **draft**. The operator approves before **anything** goes public.
- **NO autonomous posting.** A public action is a brand + legal risk; an
  unsupervised post in the company's name could ruin trust. The teammate's
  truth-boundary is **hard-bound** (no fabricated claims), and this ties directly
  to [[DEC-025]]'s "a generated teammate must not ruin a business" bar — the
  public-facing surface is exactly where that bar matters most.
- **Earn autonomy later.** Like every SKYCOT teammate, it can graduate to more
  autonomy once trust is demonstrated — but it **starts** fully approval-gated.

The shape mirrors Grant ([[DEC-018]]): find → **draft** → human approves →
(later) send. Every draft is a real LLM output run through the truth boundary and
recorded on the **hash-chained, tamper-evident audit ledger** — so every draft is
auditable, NO THEATER ([[DEC-007]]).

## Boundary with Grant (DEC-018)

The two GTM-adjacent teammates are complementary, not overlapping:

- **Grant ([[DEC-018]]) = inbound / qualify-the-lead.** He works the bottom of the
  funnel: qualifies waitlist signups, drafts brand-safe inbound follow-ups.
- **Marketing teammate = top-of-funnel awareness / content / voice.** It **fills
  the funnel that Grant then qualifies** — articles, social listening, public
  engagement, thought-leadership.

## Timing

**Fast-follow after the Genesis safety stack (P0–P6) lands.** Genesis must be
production-quality + safe ([[DEC-025]]) before a Genesis-built teammate is pointed
at the company's public-facing presence. The thin-slice scoping work is queued in
[`docs/skycot/overnight-build-out.md`](../overnight-build-out.md) under
"Post-P6 follow-ups."

## Why

- **Recursive proof.** SKYCOT's own marketing — run by a SKYCOT teammate built
  through Genesis — is the most credible demonstration of the platform (the same
  pattern as Portia pitching SKYCOT and Grant working its inbound).
- **The ideal showcase of the differentiators where they matter most.** It puts
  the **truth-boundary** + **approval-first** moat ([`docs/skycot/marketing/DIFFERENTIATORS.md`](../marketing/DIFFERENTIATORS.md))
  on a **public-facing** surface — the highest-stakes place for "every action is
  drafted, approved, and auditable."

Related: [[DEC-010]] (Genesis is MVP-must-have), [[DEC-018]] (Grant, the 2nd hero
teammate; the funnel boundary), [[DEC-025]] (Genesis done right — don't ruin a
business; the safety bar this teammate's draft-only discipline serves),
[[DEC-007]] (NO THEATER — drafts are real, the approval is real, the audit is real).

## Confirmed scope + v0 shipped (operator-confirmed 2026-05-31)

Operator confirmed the v0 build scope 2026-05-31: **name = Marcus**; use-cases =
**both** (paste-a-mention/URL → drafted reply AND topic → drafted article);
channels = **both** (X/Twitter short AND LinkedIn long-form). v0 shipped the same
turn:

- **Endpoint** `POST /api/v1/marketing/draft-response`
  (`crates/argo-cli/src/handlers/gateway/routes_marketing_draft_response.rs`) —
  clones Grant's proven route: live-LLM draft (Sonnet floor) → same truth-boundary
  Portia/Grant use → pending approval row + hash-chained audit row →
  **never posts**. 15 cargo unit tests (both modes × both channels, truth-boundary
  swap, draft-only). The operator hand-posts the approved draft (NO posting
  integration — greenfield per [`MARKETING_TEAMMATE_PLAN.md`](../marketing/MARKETING_TEAMMATE_PLAN.md)).
- **Charter** `verticals/marketing-agent/vertical.toml` ("Marcus") — public/transmit
  verbs (`post_reply`/`publish_article`/`share_on_linkedin`/`post_to_x`) are
  `requires_approval = true` + High risk + `executor = "human"` (the Genesis
  safety-judge floor; the LLM can't relax it); `draft_reply`/`draft_article` are
  autonomous-safe.
- **Console** `/marketing` SPA page (mirrors `/gtm`; LOCKED brand) + `lib/api/marketing.ts`.
- **Roster** Marcus added as the 3rd named hero (alongside Portia + Grant); demo
  roster bumped 7 → 8 consistently across `personas.rs` + `sample-personas.ts` +
  count assertions.
- **Deferred (fast-follow):** auto social-listening (no public-social/web-search
  connector exists) — v0 operator pastes the mention; a live hire-through-Genesis
  demo.
