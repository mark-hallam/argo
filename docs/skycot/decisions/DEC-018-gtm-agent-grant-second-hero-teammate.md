---
id: DEC-018
date: 2026-05-29
status: active
supersedes: none
---

# DEC-018 — GTM teammate "Grant" is the second real working teammate

## Decision

SKYCOT ships a **GTM (go-to-market) teammate named Grant** as the **second
real working teammate** after Portia (the Pitch Agent, [[DEC-016]]). Grant
owns **inbound growth** as a standing *responsibility* — not a one-off task —
and his first real duty is drafting brand-safe inbound follow-ups to waitlist
signups, gated through human approval.

Grant is a hero teammate in the demo roster (7th persona; the roster is now
Mariah / Peta / Priya / Jamal / Nori / Portia / Grant) and has an
operator-facing console at **`/gtm`**.

## Why

- The operator's directive (2026-05-29): "I would like a GTM engineer that
  will take SKYCOT to market … AI teammates that take on **responsibilities,
  not tasks**." Grant is the embodiment of the Claim-2 differentiation.
- It is recursive proof, like Portia: SKYCOT's own GTM teammate growing
  SKYCOT (Portia pitches the company; Grant works its inbound pipeline).
- Naming convention ([[DEC-016]]): the name's first letter echoes the role.
  GTM → a **G**-name → **Grant**.

## How it's built (Phase B shipped)

- Backend: `POST /api/v1/gtm/draft-followup`
  (`crates/argo-cli/src/handlers/gateway/routes_gtm_draft_followup.rs`). Given
  `{signup:{name,email,context?}}`, Grant drafts a follow-up via the **live
  LLM** (operator's Claude subscription — [[DEC-012]] `claude-code-cli`,
  Sonnet floor [[DEC-013]]), runs the draft through the **same truth-boundary**
  the Pitch Agent uses (`extract_forbidden_class`), writes an
  `approval_request` to the workspace tenant (so it surfaces in `/approvals`),
  and emits an `AuditKind::ApprovalAsked` audit row. **Never auto-sends.**
- Reuses Portia's proven LLM + truth-boundary + audit primitives — this is
  "second vertical on the proven pattern," not new infrastructure. No new
  audit-kind variant (reuses `ApprovalAsked`).
- SPA: `/gtm` console (`web/app/(app)/gtm/page.tsx` + `web/lib/api/gtm.ts` +
  nav entry), mirror of `/pitch`. Seed persona in
  `crates/argo-demo-seed/src/personas.rs` (vertical `gtm`, team "Growth").

## Truth boundary (brand safety)

Grant is bound by the same approved-facts policy as Portia: no fabricated
valuation / revenue / customer-count claims. A draft that strays into a
guarded claim is **held for founder review** (disposition `Escalate`) instead
of being filed. NO THEATER ([[DEC-007]]): drafts are real LLM output, the
approval is real, nothing is faked.

## Out of scope (later slices)

- Actually SENDING the approved email (deploy-time; demo is draft→approve).
- Real waitlist-signup ingestion (a webhook); for now the `/gtm` console +
  the API drive it with a signup payload.
- Grant's other growth duties (social-content drafting, weekly growth brief).

Related: [[DEC-016]] (Portia), [[DEC-012]] (claude-code-cli), [[DEC-013]]
(Sonnet floor), [[DEC-007]] (no theater), [[project_track_p_v1_mvp_scope]].
