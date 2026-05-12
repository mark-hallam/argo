---
name: escalate_past_due
version: "0.1.0"
description: "Packages a 45+ day past-due invoice for a human handoff: invoice + amount + reminder history + last contact + recommended action. Output is the envelope the escalate_overdue action consumes; the human, not the agent, decides next steps."
provenance:
  source: vertical/finance-ar
  hash: sha256-pending
---

# When to use

Triggered for any invoice whose `days_past_due >= 45` (the charter
threshold — see `vertical.toml::actions.escalate_overdue`). Do NOT
fire below 45 days, and do NOT fire twice — check
`memory.escalations[]` first.

# Inputs you expect

- `invoice_id` — id of the past-due invoice
- `memory.invoices[]` — amount, customer, due_date, days_past_due
- `memory.reminders[]` — full reminder history for this invoice
- `memory.payments[]` — any partial payments already received

# Output shape

A single fenced JSON block — the escalation envelope:

```json
{
  "invoice_id": "<id>",
  "customer": "<name>",
  "amount_usd": 0.00,
  "amount_outstanding_usd": 0.00,
  "days_past_due": 0,
  "due_date": "<YYYY-MM-DD>",
  "reminder_history": [{"sent_at": "<YYYY-MM-DD>", "bucket": "overdue_7|overdue_14|overdue_30", "subject": "<...>"}],
  "last_contact": {"at": "<YYYY-MM-DD>", "channel": "email|phone|none", "summary": "<one sentence>"},
  "partial_payments": [{"received_at": "<YYYY-MM-DD>", "amount_usd": 0.00}],
  "recommended_action": "collections_call|payment_plan_offer|write_off",
  "rationale": "<one sentence — why this recommendation>",
  "requires_approval": true
}
```

# Rules

- **45-day threshold is fixed.** Charter says 45; no drift to 44/46.
  Below threshold → `{"skipped": true, "reason": "below_threshold"}`.
- **Outstanding != face value.** `amount_outstanding_usd = amount_usd
  - sum(partial_payments)`. A $1,000 invoice with $400 received
  escalates as $600 outstanding.
- **Recommendation is a closed set.** `collections_call` (active
  customer, no payment intent), `payment_plan_offer` (responsive but
  cash-constrained), `write_off` (no contact in 60+ days OR amount
  below collection threshold). Never invent a fourth.
- **Never email the customer from this skill.** Escalation is
  routing; the human decides the next outbound touch.
- **One escalation per invoice.** If `memory.escalations[]` already
  has a row for this `invoice_id`, return `{"skipped": true,
  "reason": "already_escalated"}`.

# Examples

Input: INV-2026-0119, $4,200, 52 days past due, 3 reminders sent, no
response since the overdue_14 bounce-back. →

```json
{
  "invoice_id": "INV-2026-0119",
  "customer": "Halcyon Labs",
  "amount_usd": 4200.00,
  "amount_outstanding_usd": 4200.00,
  "days_past_due": 52,
  "due_date": "2026-03-21",
  "reminder_history": [
    {"sent_at": "2026-03-28", "bucket": "overdue_7",  "subject": "Reminder: Invoice INV-2026-0119"},
    {"sent_at": "2026-04-04", "bucket": "overdue_14", "subject": "Reminder: Invoice INV-2026-0119"},
    {"sent_at": "2026-04-20", "bucket": "overdue_30", "subject": "Reminder: Invoice INV-2026-0119"}
  ],
  "last_contact": {"at": "2026-04-04", "channel": "email", "summary": "Bounce-back, mailbox full."},
  "partial_payments": [],
  "recommended_action": "collections_call",
  "rationale": "Three unanswered reminders and a bounced mailbox suggest a live address but no payment intent.",
  "requires_approval": true
}
```
