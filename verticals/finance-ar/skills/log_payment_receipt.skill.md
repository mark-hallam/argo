---
name: log_payment_receipt
version: "0.1.0"
description: "Logs an inbound payment confirmation against an open invoice: matches by invoice_id + amount, appends to memory.payments[], and updates the invoice status. Flags partial payments, wrong-customer payments, and overpayments rather than auto-applying them."
provenance:
  source: vertical/finance-ar
  hash: sha256-pending
---

# When to use

Operator pastes a payment confirmation (Stripe webhook, bank
notification, or manual note). Matches it to an open invoice and
persists the row. Pairs with `log_payment_receipt` in `vertical.toml`.

# Inputs you expect

- payment confirmation: `invoice_id` (or remittance memo), `amount_usd`, `received_at`, `source`, optional `customer` and `evidence_url`
- `memory.invoices[]` — match the open invoice
- `memory.payments[]` — cumulative-paid for partial detection

# Output shape

A single fenced JSON block. `outcome` selects the downstream path;
the flag fields tell the runner what to surface to the human.

```json
{
  "invoice_id": "<id>",
  "amount_usd": 0.00,
  "received_at": "<YYYY-MM-DDTHH:MM:SSZ>",
  "source": "stripe|manual|wire|other",
  "outcome": "applied|partial|overpayment|wrong_customer|no_match|duplicate",
  "applied_status": "paid|partial|none",
  "remainder_usd": 0.00,
  "overpayment_usd": 0.00,
  "flag": null,
  "memory_write": {"entry": "payments", "row": {"invoice_id": "<id>", "source": "<...>", "amount_usd": 0.00, "received_at": "<...>", "evidence_url": "<...>", "applied_status": "paid|partial|none"}},
  "requires_human_review": false
}
```

# Rules

- **Match by `invoice_id` first**, then `amount_usd + customer` if
  memo missing. Never match on amount alone — the same amount maps
  to many invoices.
- **Partial** (`amount_usd < amount_outstanding`): `outcome:
  "partial"`, `applied_status: "partial"`, `remainder_usd =
  amount_outstanding - amount_usd`, `flag: "partial_payment"`.
  Append the payment row; leave the invoice open at the new balance.
- **Wrong-customer** (payment customer != invoice customer):
  `outcome: "wrong_customer"`, `flag: "wrong_customer"`,
  `requires_human_review: true`, persist with `applied_status:
  "none"`. Human reroutes.
- **Overpayment** (`amount_usd > amount_outstanding`): `outcome:
  "overpayment"`, `applied_status: "paid"`, `overpayment_usd =
  amount_usd - amount_outstanding`, `flag: "refund_required"`,
  `requires_human_review: true`. Refunds are never automated.
- **No match** (`invoice_id` not in inventory): `outcome: "no_match"`,
  `flag: "unmatched_payment"`, `requires_human_review: true`.
  Persist with `applied_status: "none"`.
- **Idempotency.** Existing `payments[]` row with same
  `invoice_id + source + received_at + amount_usd` → `outcome:
  "duplicate"`, no write.

# Examples

Input: Stripe webhook, `invoice_id: INV-2026-0327`, `amount_usd:
1180.00`, invoice outstanding $1,180.00. →

```json
{
  "invoice_id": "INV-2026-0327",
  "amount_usd": 1180.00,
  "received_at": "2026-05-12T14:02:11Z",
  "source": "stripe",
  "outcome": "applied",
  "applied_status": "paid",
  "remainder_usd": 0.00,
  "overpayment_usd": 0.00,
  "flag": null,
  "memory_write": {"entry": "payments", "row": {"invoice_id": "INV-2026-0327", "source": "stripe", "amount_usd": 1180.00, "received_at": "2026-05-12T14:02:11Z", "evidence_url": "https://dashboard.stripe.com/payments/pi_xyz", "applied_status": "paid"}},
  "requires_human_review": false
}
```
