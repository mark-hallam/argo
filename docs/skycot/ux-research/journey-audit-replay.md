# Journey — Audit & Replay (stub)

> The forensic flow. The Operator (or Auditor) needs to answer:
> "What did this worker do, why, and on whose authority?" — and
> reproduce the action in a sandbox if needed.

## The promise

> "Every action SKYCOT has ever taken is one query away,
> attributable to a human approver or a named policy, and
> replayable from the audit log alone."

## Trigger

- Compliance request (SOC2, GDPR Article 15, internal audit).
- Incident response ("did the support agent really tell that
  customer we'd refund them?").
- Cost investigation ("why did this worker burn 800 LLM calls
  yesterday?").
- Retraining: the Operator wants to find every example of
  Mariah's "wrong" behavior to build a better policy.

## The shape (sketched)

Three surfaces, tightly linked:

1. **Activity feed** — chronological timeline of all actions
   across workers/tenants. Filterable. The "scrolling
   answer" to *what happened.*
2. **Action drill-in** — click any action → full context:
   inputs, model version, prompt, memory snapshot, skill
   version, approval trail, output, downstream consequence.
3. **Replay** — re-run the action in a sandbox with the same
   inputs. Useful for "was this the right behavior?" review.
   Already largely shipped: `claude-trajectory-replay-ui` and
   `claude-audit-replay` briefs exist.

## What we still need to figure out

- Search vocabulary. Does the Operator filter by action type,
  worker, channel, model, time, cost, approver, all of the
  above? Defaults matter — most-used filter visible first.
- Redaction defaults. Auditor sees redacted PII unless their
  role grants unredacted; Operator sees their own tenant's
  data unredacted. The audit-trail UI must enforce this at the
  render layer, not the data layer.
- Replay isolation. Is the sandbox fully isolated from
  production (no real Slack message sent, no real charge
  placed), or are some side-effects necessary to test? Lock
  the contract.
- Cross-tenant audit. Some Operators need fleet-wide view; in
  multi-tenant deployments, this requires explicit cross-tenant
  grant.

## Status

Stub. Most backend pieces exist (compliance-soc2, compliance-
export, trajectory-replay, audit-row-stamping). The UI is the
Operator console's "Audit" tab plus a worker-profile-page
"Activity" tab. Expanded when those tabs are wireframed.
