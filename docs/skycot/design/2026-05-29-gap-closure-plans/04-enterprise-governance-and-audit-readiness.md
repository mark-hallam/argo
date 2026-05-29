# Design plan — Enterprise governance + audit-readiness (make the moat legible)

**Status:** ready-for-go implementation plan. Authored 2026-05-29 (overnight).
**Closes:** competitive gap #1 (enterprise agent governance + dual-identity) —
the **highest-priority** gap in the analysis vs. Microsoft Agent 365 + Salesforce.
**Effort:** audit-readiness packaging = LOW–MEDIUM (mostly surfacing what we
already have) · dual-identity = MEDIUM–HIGH · cross-runtime + threat-detection =
LARGE (defer).

> **Strategic read:** the gap analysis was explicit — *"higher leverage to make
> our audit + truth-boundary moat LEGIBLE to buyers (compliance packaging +
> audit-readiness framing) than to chase parity on every checkbox."* So the
> headline move here is **packaging, not a big build**: we already have the
> strongest substance (hash-chained audit, truth boundary, capability-RBAC,
> multi-tenancy); buyers evaluating us next to Microsoft/Salesforce just can't
> *see* it yet. This plan leads with legibility, then the one real build
> (dual-identity), and defers the genuinely-large lifts.

---

## Plain-English summary (for the operator)

Microsoft (Agent 365) and Salesforce now lead their pitch with **agent
governance**: "every agent has an identity, here's the audit-readiness console,
here's threat-detection." We actually have *stronger* governance substance than
they do in places (our tamper-evident audit + truth boundary are things they
**don't** ship) — but a buyer can't tell, because we present it as scattered
features, not as a **governance story**.

Three moves, biggest-bang-first:

- **A — Package what we already have (LOW effort, do first).** A single
  "Trust & Governance" view + a one-click **audit-readiness report** (PDF/JSON)
  that says, for a tenant: every action is logged + cryptographically chained
  (provably un-tampered), here's the capability-RBAC matrix, here's the truth
  boundary, here's the approval coverage. This is **mostly surfacing existing
  data** — minimal new backend — and it's the highest-leverage thing we can do
  for enterprise credibility. Doubles as landing-page proof for the
  differentiators you flagged.
- **B — Dual agent-identity (MEDIUM, the one real build).** Microsoft's floor:
  an agent can act **on behalf of a user** (the user's identity propagates
  through every downstream call + audit row) **or** with its **own autonomous
  identity**. We have worker identity + tenant plumbing; we'd add the
  "acting-as" distinction + propagate it into the audit chain. Makes "who really
  did this, under whose authority?" answerable — exactly what a regulated buyer
  asks.
- **C — Cross-runtime governance + behavioral threat-detection (LARGE, defer).**
  Managing agents across other vendors' runtimes (Bedrock/Copilot) + anomaly
  detection (jailbreak/risky-tool-call flagging). Genuinely big; not where we
  win first. Park it.

**Recommendation:** do **A now** (cheap, high-credibility, reusable as marketing
proof), schedule **B** as a real near-term build, and **defer C**. The analysis
also noted Microsoft's governance is itself fragmented across three consoles with
no third-party auto-discovery and detect/block still in preview — so the maturity
gap is narrower than their marketing; **A closes most of the *perception* gap.**

---

## What exists today (verified against the repo + analysis)

- **Hash-chained, tamper-evident audit** of every action — *no researched
  competitor matched this.* (`argo-audit`, `verify_chain`.)
- **Claim-level truth boundary** — gates what an agent may *assert*, not just
  access — *a category we appear to own.* (`argo_capability::evaluate_claim`.)
- **Capability-based RBAC + multi-tenancy** plumbed through every primitive
  (`tenant_id` everywhere; `role_gate.rs` at the gateway).
- **Approval-first across 9 channels.**
- **Missing:** a documented **delegated-vs-autonomous identity** model; any
  **cross-runtime** governance; **behavioral threat-detection**; and — the cheap
  win — a **buyer-facing governance/audit-readiness surface** that makes the
  above legible in one place.

## Decisions the operator should weigh

- **Scope of move A:** a read-only **report/export** (fastest — reuses the audit
  + RBAC + truth-boundary data we have) vs. a richer interactive **/trust
  dashboard** (more design + build). Recommend the **report first** (days, not
  weeks), dashboard later.
- **Dual-identity (move B) semantics:** do we model "acting-as" as a first-class
  field on every action (cleaner, more invasive) or as an audit-payload
  annotation first (lighter, incremental)? Recommend **annotation-first**, then
  promote to first-class once the audit story proves out.

## Implementation phases

### A — audit-readiness packaging (LOW–MEDIUM, do first)
- `GET /api/v1/governance/readiness?tenant=…` → a structured report: action
  count + audit-chain integrity result (`verify_chain`), capability-RBAC grants,
  truth-boundary config, approval coverage %, retains/retrain history. **All from
  existing tables** — assembly, not new capture.
- `argo governance report --tenant <t> --out report.json|pdf` CLI for a
  download an enterprise buyer/auditor can hold.
- Optional SPA `/trust` page rendering the report (brand-restrained register, like
  `/audit`). Reuse the differentiator copy from `docs/skycot/marketing/DIFFERENTIATORS.md`.
- **Verify:** report over the seeded tenant shows real counts + a PASS chain
  integrity; tenant-scoped (a different tenant's report is empty/clean).

### B — dual agent-identity (MEDIUM)
- Add an `acting_as: { kind: "on_behalf_of" | "autonomous", principal }`
  annotation, stamped into the audit payload at the action edge (reuse the
  existing principal-stamp seam, `principal_id_for_audit`).
- Surface it in the audit detail + the readiness report ("X actions on behalf of
  user U; Y autonomous").
- **Verify:** an on-behalf-of action and an autonomous action produce
  distinguishable, tenant-scoped audit rows; the readiness report counts both.

### C — cross-runtime + threat-detection (LARGE, defer)
- Cross-runtime registry/sync + Defender-style anomaly detection. Own brief tree
  + likely a vertical-agnostic core capability. Not a near-term win.

## Risks / honesty notes
- **A is mostly assembly** — the hazard is over-claiming in the report. Keep it
  factual: report what the audit/RBAC/truth-boundary data actually shows, never a
  marketing gloss (NO THEATER applies to compliance artifacts *especially*).
- Don't let "governance" balloon into C. The win is **legibility of existing
  substance + the one identity build**, not chasing Microsoft's full surface.
- Re-verify gap #1 before committing — Agent 365 GA'd ~4 weeks before the
  analysis; the baseline moves monthly.

## Brief decomposition (ready to author)
`governance-readiness-report-endpoint` → `governance-report-cli` →
`web-trust-readiness-page` → `governance-dual-identity-acting-as-annotation`.
Each carries the freshness header + names its consumer per DRIFT_PREVENTION.md.
