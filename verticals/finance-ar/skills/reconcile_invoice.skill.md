---
name: reconcile_invoice
version: "0.1.0"
description: "Reconciles an apparent payment discrepancy: customer claims they paid, the ledger says open. Pulls invoice + payment history, computes the gap, and recommends one of four closed-set outcomes. Customer-facing reply is approval-gated."
provenance:
  source: vertical/finance-ar
  hash: sha256-pending
---

# When to use

Customer pushes back on a reminder: "I already paid", "this was
waived", "we never received this invoice". Run BEFORE drafting any
reply. The reconciliation summary is what the operator approves;
the reply itself routes through `draft_reminder` only after the
operator picks an outcome.

# Inputs you expect

- `invoice_id` — the disputed invoice
- the customer's claim (free-text or pasted email)
- `memory.invoices[]` — amount, status, due_date
- `memory.payments[]` — to compare against billed amount
- `memory.reminders[]` — confirm what the customer was sent

# Output shape

A single fenced JSON block. `recommended_outcome` is a closed set
of four values — never synthesize a fifth.

```json
{
  "invoice_id": "<id>",
  "customer": "<name>",
  "billed_usd": 0.00,
  "received_usd": 0.00,
  "gap_usd": 0.00,
  "payment_evidence": [{"received_at": "<YYYY-MM-DD>", "amount_usd": 0.00, "source": "stripe|manual|wire|other", "evidence_url": "<...>"}],
  "customer_claim": "<verbatim quote, trimmed>",
  "recommended_outcome": "apply_payment|dispute|waive|no_action",
  "rationale": "<one sentence — what the evidence supports>",
  "requires_approval": true
}
```

# Rules

- **Closed-set outcomes.** Exactly one of:
  - `apply_payment` — payment in `memory.payments[]` not yet applied
    to the invoice (gap = 0, status not updated).
  - `dispute` — no matching payment AND no evidence in the claim
    (no confirmation #, no date, no amount).
  - `waive` — billing error confirmed (duplicate, wrong SKU, agreed
    credit). Operator approval gates this.
  - `no_action` — vague claim or wrong invoice; request specifics.
- **`gap_usd = billed_usd - received_usd`.** Positive = customer
  owes; negative = we owe a refund (route via `log_payment_receipt`'s
  overpayment path, not here).
- **Quote the customer verbatim.** `customer_claim` is a direct
  quote (trimmed). Paraphrasing loses the audit trail.
- **Cite evidence per row.** Every `payment_evidence` entry carries
  an `evidence_url` (Stripe charge, bank ref) or a manual note id.
  No url + no id → drop the row.
- **Never auto-send the reply.** The customer-facing email goes
  through `send_reminder` AFTER the operator picks an outcome.

# Examples

Input: customer claims "I paid INV-2026-0119 last week, see Stripe
ch_abc123". Ledger shows the invoice `open` for $4,200; Stripe
charge ch_abc123 in `memory.payments[]` for $4,200 received
2026-05-05 with `applied_status: "none"`. →

```json
{
  "invoice_id": "INV-2026-0119",
  "customer": "Halcyon Labs",
  "billed_usd": 4200.00,
  "received_usd": 4200.00,
  "gap_usd": 0.00,
  "payment_evidence": [{"received_at": "2026-05-05", "amount_usd": 4200.00, "source": "stripe", "evidence_url": "https://dashboard.stripe.com/payments/ch_abc123"}],
  "customer_claim": "I paid INV-2026-0119 last week, see Stripe ch_abc123",
  "recommended_outcome": "apply_payment",
  "rationale": "Stripe charge ch_abc123 matches the invoice amount and date; the payment is logged but not yet applied.",
  "requires_approval": true
}
```
