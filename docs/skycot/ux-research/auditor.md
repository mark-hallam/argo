# Persona — the Auditor (stub)

> The compliance officer or external auditor who reads SKYCOT's
> audit trail and signs off (or doesn't) on its use within an
> organization. Internal SOC2 / ISO 27001 readiness; external
> SOC2 Type II attestation; GDPR Article 15 data-access
> requests; HIPAA in the healthcare-intake vertical.

## Who they are

- **Internal compliance officer** at the customer's org. May be
  the legal team in a small co., a dedicated compliance team in
  a regulated industry.
- **External auditor** engaged for SOC2 Type II or equivalent.
  Visits 1–2 times per year. Reads what the platform produces;
  does not configure anything.
- **Regulator-side counterparty.** Rare, slow, expensive when
  it happens. The platform's behavior under regulatory request
  is the highest-stakes moment of its existence.

## What they need

- **Read-only access to a complete, signed audit trail.**
  Every action, every decision, every approval, every secret
  access. Schema-stable, exportable, queryable.
- **Reproducibility.** "Show me exactly what happened on
  2026-04-12T15:42:00Z for tenant X" produces a deterministic
  bundle every time.
- **Action replay.** Not just the log of what happened — the
  ability to re-run an action in a sandbox with the same
  inputs the worker saw, to verify the outcome was correct.
  (Trajectory replay UI brief exists; not yet wired to a
  surface.)
- **Provenance for AI-generated content.** Which model, which
  prompt, which skill version, which memory snapshot produced
  this output. (Already shipped: `mission-prompt-cache-hit-rate-
  observability` + audit-row stamping.)
- **Identity attribution.** Every action attributable to either
  a human (with signed approval) or an autonomous policy (with
  the policy hash that authorized it). No anonymous actions.

## Day in the life (sketch)

- 90% of their time: not in SKYCOT at all. Doing other audit
  work.
- 10% of their time: querying the audit trail, exporting
  evidence bundles, occasionally requesting a clarification
  from the Operator.

## To design for

- A search surface for the audit trail that an SQL-comfortable
  user can use without writing SQL. Filters on tenant, worker,
  action type, time window, channel, model.
- An export format that legal teams trust: single signed file
  per request, schema documented, hash chain across records.
- A "minimum-necessary" defaults answer to GDPR: by default,
  audit logs are queryable; by default, they redact PII unless
  the auditor's role has the unredacted-view grant.

## Status

Stub. Most of the backend (compliance-soc2, compliance-export,
audit-row-stamping) is already shipped. The auditor's UI is the
Operator console's "Compliance" tab (see `operator.md`). Will
be expanded when that tab's design needs to be wireframed.
