---
name: run_weekly_growth_review
version: "0.1.0"
description: Compile the weekly funnel report card across all five stages, propose one growth experiment, route delegations to the right teammate, and surface a budget proposal if needed — every number sourced or marked unavailable.
provenance:
  source: vertical/gtm-growth-lead
  hash: sha256-pending
---

# When to use

Use once per week when the operator asks Gwen to run the growth
review, or at the start of the GTM cadence. This is Gwen's anchor
skill: it walks the full loop — funnel report → one experiment →
delegation(s) → optional budget proposal — and never executes anything
itself. Delegation and budget moves are approval-gated; this skill only
produces the proposals.

# Inputs you expect

- `week_iso` — the ISO week being reviewed (for the audit trail).
- `funnel_metrics` — the week's numbers per stage (attract, convert,
  activate, retain, learn), each with a source. Pulled from the
  analytics / CRM connectors when wired, from operator paste-ins or
  memory pointers otherwise. Any stage without a source is treated as
  unavailable — never invented.
- `prior_snapshot` — last week's `funnel_snapshots` row (optional), so
  trends can be stated against a sourced baseline.

# Output shape

```json
{
  "funnel_report": {
    "week_iso": "2026-W25",
    "attract": { "value": 0, "source": "analytics" },
    "convert": { "value": 0, "source": "crm" },
    "activate": { "value": "unavailable", "source": null },
    "retain": { "value": 0, "source": "analytics" },
    "learn": { "value": "unavailable", "source": null },
    "weakest_stage": "activate"
  },
  "proposed_experiment": {
    "hypothesis": "...",
    "target_metric": "activation_rate_pct",
    "threshold": 40,
    "owner_teammate": "customer-onboarding",
    "success_measure": "..."
  },
  "delegations": [
    { "teammate": "customer-onboarding", "summary": "...",
      "funnel_stage": "activate", "requires_approval": true }
  ],
  "budget_proposal": {
    "amount_usd": 0, "from_channel": "...", "to_channel": "...",
    "rationale": "...", "requires_approval": true
  }
}
```

`budget_proposal` is optional — omit it entirely when no spend shift is
warranted. `delegations` is an array; route each scoped task to the
teammate who owns the weakest stage. `weakest_stage` drives the
experiment owner and the delegation target.

# Hard rules — truth boundary (NO THEATER)

- **Cite every number.** Each funnel figure carries its `source`. A
  number with no source is **`"unavailable"`** — never estimate, round
  to a plausible value, or fill a gap. "unavailable" is a correct
  answer; a made-up figure is not.
- **Never claim an experiment worked without data.** Report
  experiment results only from a sourced measurement; "ran it" is not
  "it worked".
- **Delegation is approval-gated.** Every entry in `delegations` sets
  `requires_approval: true`. Never auto-fire a delegation.
- **Never delegate money, legal, or a named-customer commitment.**
  Those stop at the operator.
- **Budget never moves here.** `budget_proposal` is a proposal handed
  to the operator with `requires_approval: true`. The worker NEVER
  executes a spend shift.

# Teammate routing

Pick the delegation target by the weakest funnel stage:

- attract weak → `marketing-agent`
- convert weak → `sales-sdr`
- activate / retain weak → `customer-onboarding`
- need to know why / which segment → `gtm-voice-of-customer`

If two stages are weak, route to the earliest in the funnel.

# Example

Synthetic only — no real metrics.

```
week_iso: "2026-W25"
funnel_metrics:
  attract:  { value: 1200, source: "analytics" }
  convert:  { value: 60,   source: "crm" }
  activate: { value: unavailable }   # connector down
  retain:   { value: 0.82, source: "analytics" }
  learn:    { value: unavailable }
```

```json
{
  "funnel_report": {
    "week_iso": "2026-W25",
    "attract": { "value": 1200, "source": "analytics" },
    "convert": { "value": 60, "source": "crm" },
    "activate": { "value": "unavailable", "source": null },
    "retain": { "value": 0.82, "source": "analytics" },
    "learn": { "value": "unavailable", "source": null },
    "weakest_stage": "activate"
  },
  "proposed_experiment": {
    "hypothesis": "A guided first-run checklist lifts day-1 activation.",
    "target_metric": "activation_rate_pct",
    "threshold": 40,
    "owner_teammate": "customer-onboarding",
    "success_measure": "Sourced activation rate from analytics, week-over-week."
  },
  "delegations": [
    { "teammate": "customer-onboarding",
      "summary": "Ship a guided first-run checklist for new sign-ups.",
      "funnel_stage": "activate", "requires_approval": true }
  ]
}
```
