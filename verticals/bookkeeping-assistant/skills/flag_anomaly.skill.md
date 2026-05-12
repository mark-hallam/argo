---
name: flag_anomaly
version: "0.1.0"
description: "Surfaces anomalies in recent transactions: duplicates, category-spend spikes (>2σ from 90-day median), missing recurring entries, and suspicious round-number patterns. Outputs severity-tagged findings for operator review."
provenance:
  source: vertical/bookkeeping-assistant
  hash: sha256-pending
---

# When to use

Call on a rolling window (e.g. weekly close prep, or before
month-end) over the last 30 days of categorized journal
entries. The output feeds operator review and may seed
follow-up actions (re-categorize, void duplicate, investigate
fraud); this skill never mutates the ledger.

# Inputs you expect

- recent journal entries for the lookback window (default 30 days)
- 90-day history for the same accounts (for variance baseline)
- recurring-rules table (vendor, expected cadence, expected amount)

# Output shape

```json
{
  "window": "2026-04-13..2026-05-13",
  "findings": [
    {"kind": "duplicate", "severity": "advisory", "journal_ids": ["J-2204", "J-2206"], "evidence": "same vendor + amount + date"},
    {"kind": "category_spike", "severity": "advisory", "account": "6300", "spend_usd": 1820.00, "median_90d_usd": 410.00, "z_score": 3.4},
    {"kind": "missing_recurring", "severity": "urgent", "rule_id": "recurring/payroll-biweekly", "expected_on": "2026-05-08"},
    {"kind": "round_number_pattern", "severity": "advisory", "count_in_week": 4, "amounts_usd": [100.00, 200.00, 100.00, 300.00], "note": "≥3 round-100 entries in a 7-day window"}
  ],
  "summary": "1 urgent, 3 advisory, 0 info"
}
```

# Examples

- Two entries with the same vendor, amount, and date within
  72 hours → emit `duplicate` (`advisory`); operator decides
  whether to void.
- A `6300 — Meals` month-to-date spend at z-score 3.4 vs the
  90-day median → emit `category_spike` (`advisory`).
- A recurring `payroll-biweekly` rule expected on
  `2026-05-08` with no matching entry by today → emit
  `missing_recurring` (`urgent`) — payroll gaps materially
  hurt the business.
- ≥3 round-100 entries to the same or related accounts within
  a 7-day window → emit `round_number_pattern` (`advisory`)
  with an auditor-readable note; small-business books
  sometimes show suspicious round-number entries that hide
  skim, so the operator should investigate before close.

# Approval posture

Internal-output, not approval-gated — this skill never posts
or mutates anything. Severity vocabulary is fixed at three
levels:

- `info` — a pattern worth knowing but not acting on.
- `advisory` — operator should review before close.
- `urgent` — likely material to cash flow or audit posture;
  surface immediately.

Category-spike threshold is fixed at `z_score > 2.0` against
the 90-day median; round-number threshold is fixed at `≥3
round-100 entries within a 7-day window`. Thresholds are
deterministic so the operator can spot-check findings.
