---
name: aging_report
version: "0.1.0"
description: "Builds a weekly AR aging report: bucket open invoices by 0-30/31-60/61-90/90+ days, totals + counts per bucket, top-10 oldest, and a per-bucket next-step recommendation. Output is approval-gated before publishing."
provenance:
  source: vertical/finance-ar
  hash: sha256-pending
---

# When to use

Run weekly (typically Monday morning) before any reminders or
escalations go out. The output is the source of truth for that week's
collection priorities. Operator must approve the rendered report
before it leaves the workspace — see the `generate_aging_report`
action in `vertical.toml` (`requires_approval = true`).

# Inputs you expect

- `inventory.json` — the open-invoice ledger (id, customer, amount, due_date, status)
- `memory.invoices[]` — last review's bucketed rows (for status carry-over)
- today's date (UTC) — used to compute `days_past_due`

# Output shape

Two artifacts. The Markdown report is the operator-facing one; the
JSON summary is what downstream actions read.

```json
{
  "as_of": "<YYYY-MM-DD>",
  "buckets": [
    {"label": "0-30",  "days_min": 0,  "days_max": 30, "count": 0, "total_usd": 0.00, "next_step": "reminder"},
    {"label": "31-60", "days_min": 31, "days_max": 60, "count": 0, "total_usd": 0.00, "next_step": "reminder"},
    {"label": "61-90", "days_min": 61, "days_max": 90, "count": 0, "total_usd": 0.00, "next_step": "escalation"},
    {"label": "90+",   "days_min": 91, "days_max": null, "count": 0, "total_usd": 0.00, "next_step": "write_off_review"}
  ],
  "top_oldest": [
    {"invoice_id": "<id>", "customer": "<name>", "amount_usd": 0.00, "days_past_due": 0}
  ],
  "totals": {"count": 0, "total_usd": 0.00},
  "requires_approval": true
}
```

The Markdown report mirrors this: a table per bucket, a top-10 list,
and a one-line recommendation per bucket.

# Rules

- **Deterministic bucketing.** `days_past_due = today - due_date`.
  Bucket on integer day count. An invoice 30 days past due lands in
  `0-30`; 31 days lands in `31-60`. No off-by-one drift between weeks.
- **Bucket totals must match the per-row sum.** Operator spot-checks
  arithmetic; a mismatch is a rejection.
- **Top-10 by `days_past_due` desc, tie-break by `amount_usd` desc.**
  Stable order so week-over-week diffs are readable.
- **Next-step is fixed per bucket** — `0-30` → reminder,
  `31-60` → reminder, `61-90` → escalation, `90+` → write_off_review.
  Don't synthesize ad-hoc recommendations per row; that's the
  reconcile/escalate skills' job.
- **Exclude paid / written-off invoices.** Only `status: open` or
  `status: partial` rows count; partial rows carry the unpaid balance.
- **Never publish unapproved.** The skill emits the artifact pair
  only; the runner gates on the operator's approval before any share.

# Examples

Input: 4 open invoices — INV-1 ($200, 12d past due), INV-2 ($1,500,
44d), INV-3 ($800, 67d), INV-4 ($350, 102d). →

```json
{
  "as_of": "2026-05-12",
  "buckets": [
    {"label": "0-30",  "days_min": 0,  "days_max": 30, "count": 1, "total_usd": 200.00,  "next_step": "reminder"},
    {"label": "31-60", "days_min": 31, "days_max": 60, "count": 1, "total_usd": 1500.00, "next_step": "reminder"},
    {"label": "61-90", "days_min": 61, "days_max": 90, "count": 1, "total_usd": 800.00,  "next_step": "escalation"},
    {"label": "90+",   "days_min": 91, "days_max": null, "count": 1, "total_usd": 350.00, "next_step": "write_off_review"}
  ],
  "top_oldest": [
    {"invoice_id": "INV-4", "customer": "Acme",   "amount_usd": 350.00,  "days_past_due": 102},
    {"invoice_id": "INV-3", "customer": "Beacon", "amount_usd": 800.00,  "days_past_due": 67},
    {"invoice_id": "INV-2", "customer": "Cobalt", "amount_usd": 1500.00, "days_past_due": 44},
    {"invoice_id": "INV-1", "customer": "Delta",  "amount_usd": 200.00,  "days_past_due": 12}
  ],
  "totals": {"count": 4, "total_usd": 2850.00},
  "requires_approval": true
}
```
