---
name: vendor_health_check
description: Given a vendor name, gather recent news, financial-health signals, and a sanctions check; emit an escalation flag if any signal trips.
inputs:
  - vendor_name
  - lookback_days
outputs:
  - vendor_signals
  - escalation_flags
  - confidence
---

# Skill: Vendor Health Check

A vetting pass on a vendor before recommending them in a draft RFQ,
bid comparison, or contract review. Read-only — never reaches out to
the vendor and never confirms a relationship.

1. **Sanctions check.** Compare `vendor_name` against
   `sanctions_watchlist` first. Any match — exact or partial on
   entity_name — sets `escalation_flags = ["sanctions_hit"]` and stops
   downstream signal collection. Surface the matched watchlist entry
   id. The watchlist is a static inventory in v1; absence-from-list
   is not the same as cleared, and the output must say so explicitly.
2. **Approval check.** Look up `vendor_name` in `approved_vendors`. If
   absent, set `unapproved_vendor` flag (does not stop the check —
   the operator may want signals before deciding to approve).
3. **Recent news.** Pull headlines from the last `lookback_days` (default
   180). Filter for: layoffs, restatements, breach disclosures,
   leadership turnover, acquisition / divestiture, lawsuits. Cite each
   signal as `[source: <url>, tier: <1|2|3>, date: <ISO>]`. Tier-3
   sources are colour, not load-bearing.
4. **Financial-health signals.** Tier-1 sources only (issuer filings,
   named-byline wires, audited reports). Capture: revenue trend, cash
   runway if disclosed, recent funding round, recent debt event, going-
   concern language. Say "no tier-1 signal available" rather than
   citing tier-3 as primary.
5. **Confidence.** high / medium / low / speculative, calibrated to
   the source mix. "high" requires ≥2 tier-1 sources within
   `lookback_days`.
6. **Escalation triggers.** Any of: `sanctions_hit`, `going_concern`,
   `material_breach_disclosed_within_90d`, `leadership_turnover_in_role_under_review`.

Output shape:

```json
{
  "vendor_signals": {
    "sanctions_status": "clean | matched | not_yet_checked",
    "approved_vendor_status": "in_roster | absent",
    "recent_news": [{"headline": "...", "source": "...", "tier": 1, "date": "..."}],
    "financial_signals": [{"signal": "...", "source": "...", "tier": 1, "date": "..."}]
  },
  "escalation_flags": [],
  "confidence": "medium"
}
```
