---
name: reconcile_bank_statement
version: "0.1.0"
description: "Reconciles an operator-supplied bank statement against the current ledger. Surfaces matched, unmatched-bank, and unmatched-ledger entries with confidence scores; never auto-posts."
provenance:
  source: vertical/bookkeeping-assistant
  hash: sha256-pending
---

# When to use

Call when the operator pastes a bank statement (CSV, OFX, or
free-text list) and asks for reconciliation against the
journal for a given period. Output drives operator approval
before any unmatched entry is posted to the ledger.

# Inputs you expect

- bank statement rows: `date`, `description`, `amount_usd`,
  `direction` (`debit` / `credit`), optional `ref_id`
- current journal entries for the same period (or a window
  spanning ±7 days to absorb settlement lag)
- statement period (`YYYY-MM-DD..YYYY-MM-DD`)
- account `id` the statement belongs to (e.g. `1100 — Checking`)

# Output shape

```json
{
  "account_id": "1100",
  "period": "2026-04-01..2026-04-30",
  "matched": [
    {"bank_ref": "B-014", "journal_id": "J-2204", "confidence": "high", "amount_usd": 89.99}
  ],
  "unmatched_bank": [
    {"bank_ref": "B-021", "date": "2026-04-12", "amount_usd": 412.50, "description": "ALASKA AIR 0277", "suggested_category": "6810 — Travel/Airfare", "confidence": "medium"}
  ],
  "unmatched_ledger": [
    {"journal_id": "J-2218", "date": "2026-04-29", "amount_usd": 75.00, "memo": "Reimbursable — Lyft"}
  ],
  "approval_required": true,
  "summary": "27 matched, 3 unmatched-bank, 1 unmatched-ledger"
}
```

# Examples

- Vendor + amount + date (±2 days) all align → match with
  `confidence: high`; safe to auto-approve by policy.
- Amount + approximate date match but vendor string differs
  (e.g. "SQ *COFFEE" vs "Blue Bottle Coffee") → match with
  `confidence: medium`; surface for operator confirmation.
- Bank entry has no plausible ledger counterpart → emit in
  `unmatched_bank` with a `suggested_category` derived from
  `categorize_transaction`'s vendor heuristics.
- Ledger entry has no bank counterpart (e.g. uncashed check,
  pending reimbursement) → emit in `unmatched_ledger` for
  operator review; never auto-delete.

# Approval posture

Approval-gated. This skill produces a reconciliation
envelope only; it never writes to the ledger. Posting any
`unmatched_bank` entry as a new journal line requires
operator approval via the `post_journal_entry` action, even
when `confidence: high`. Multi-currency statements are out
of v1 scope — refuse and escalate when any row's currency is
not USD.
