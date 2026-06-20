---
name: score_account_health
version: "0.1.0"
description: Score an account's health from real usage signals, citing the source for every signal, then route the next move — draft check-in, internal churn flag, or expansion proposal — with customer sends and discounts left approval-gated.
provenance:
  source: vertical/gtm-customer-success
  hash: sha256-pending
---

# When to use

Use when the operator asks Sasha to assess an account, run a check-in,
or prep a QBR. This is Sasha's anchor skill: it scores account health
from sourced usage signals, names the health tier, and routes the next
move. Scoring, drafting a check-in, flagging churn internally, and
drafting an expansion proposal all run here. Sending anything to a
customer and offering any discount do NOT — those are approval-gated and
this skill only produces the proposals.

# Inputs you expect

- `account_id` — the account being scored (for the audit trail).
- `usage_signals` — the account's real signals, each with a source:
  login frequency, feature adoption, seat utilisation, support volume,
  sentiment. Pulled from the usage / CRM connectors when wired, from
  operator paste-ins or memory pointers otherwise. Any signal without a
  source is treated as unavailable — never invented.
- `prior_health` — last period's `account_health` row (optional), so
  trends can be stated against a sourced baseline.

# Output shape

```json
{
  "health_score": {
    "account_id": "acct_demo",
    "health_tier": "at_risk",
    "signals": {
      "login_frequency": { "value": "down 60% MoM", "source": "usage" },
      "feature_adoption": { "value": 2, "source": "usage" },
      "seat_utilisation": { "value": "unavailable", "source": null },
      "support_volume": { "value": 7, "source": "crm" },
      "sentiment": { "value": "unavailable", "source": null }
    },
    "confidence": "medium"
  },
  "next_move": {
    "action": "flag_churn_risk",
    "rationale": "...",
    "requires_approval": false
  },
  "drafts": {
    "checkin": { "body": "...", "requires_approval_to_send": true },
    "expansion_proposal": null
  }
}
```

`drafts` are produced here but never sent — sending a check-in or
proposal needs `send_customer_message` (approval-gated) and any discount
needs `offer_retention_discount` (approval-gated). `next_move` names the
autonomous action (score / draft / flag / propose) when no customer
contact is required.

# Hard rules — truth boundary (NO THEATER)

- **Cite every signal.** Each usage figure carries its `source`. A
  signal with no source is **`"unavailable"`** — never estimate, round
  to a plausible value, or fill a gap. "unavailable" is a correct
  answer; a made-up usage number is not.
- **Never fabricate a metric, quote, or statistic.** Health tiers come
  from sourced signals only. If you can't source it, you can't score on
  it.
- **Thin evidence stays low-confidence.** Two data points is not a
  trend. Mark thin evidence low-confidence and name what you'd need;
  never inflate it into a health conclusion.
- **Churn flags are INTERNAL.** `flag_churn_risk` raises an internal
  flag only — the customer is NEVER contacted by a flag.
- **Customer contact is approval-gated.** Sending any message to a
  customer is `send_customer_message` with `requires_approval: true`.
  Never auto-send.
- **Discounts are approval-gated.** Offering a discount or save is
  `offer_retention_discount` with `requires_approval: true` — money +
  commitment stop at the operator. Never auto-offer.

# Health-tier routing

Pick the next move by the sourced health tier:

- **healthy** → optional `draft_checkin` (light touch) or
  `propose_expansion` if usage is saturating limits.
- **watch** → `draft_checkin` grounded in the slipping signal; monitor.
- **at_risk** → `flag_churn_risk` internally with a save play; draft a
  check-in for the operator to approve sending.

A discount or any customer send is ALWAYS the operator's call, whatever
the tier.

# Example

Synthetic only — no real account data.

```
account_id: "acct_demo"
usage_signals:
  login_frequency:  { value: "down 60% MoM", source: "usage" }
  feature_adoption: { value: 2,              source: "usage" }
  seat_utilisation: { value: unavailable }   # connector down
  support_volume:   { value: 7,              source: "crm" }
  sentiment:        { value: unavailable }
```

```json
{
  "health_score": {
    "account_id": "acct_demo",
    "health_tier": "at_risk",
    "signals": {
      "login_frequency": { "value": "down 60% MoM", "source": "usage" },
      "feature_adoption": { "value": 2, "source": "usage" },
      "seat_utilisation": { "value": "unavailable", "source": null },
      "support_volume": { "value": 7, "source": "crm" },
      "sentiment": { "value": "unavailable", "source": null }
    },
    "confidence": "medium"
  },
  "next_move": {
    "action": "flag_churn_risk",
    "rationale": "Logins down 60% with rising support volume — sourced at_risk signal.",
    "requires_approval": false
  },
  "drafts": {
    "checkin": {
      "body": "Hi — noticed the team's been quieter lately; can we help unblock anything?",
      "requires_approval_to_send": true
    },
    "expansion_proposal": null
  }
}
```
