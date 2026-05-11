---
name: categorize_transaction
description: Map a raw bank-feed or card-feed transaction string to a GAAP-cash-basis journal entry with a confidence score and a recurring-rule id when matched.
inputs:
  - txn_id
  - description
  - amount_usd
  - direction
  - account
outputs:
  - journal_entry
  - confidence
  - rule_id
  - escalation_required
---

# Skill: Categorize a Single Transaction

Use this skill to turn one raw transaction line into a journal-ready
entry.

1. **Match recurring rules first.** Read `recurring_rules` from memory.
   If the description matches a recurring signature (vendor regex,
   amount within ±15%, same cadence), return that account split and a
   `confidence` of `high`.
2. **Otherwise classify by vendor heuristics.** Common signatures:
   - SAFEWAY / WHOLE FOODS / TRADER JOES → `6300 — Meals & Groceries`
     (50% deductible, mark owner-portion if personal-use suspected).
   - COMCAST / VERIZON / AT&T → `6100 — Utilities`.
   - ADOBE / GITHUB / AWS / STRIPE-FEE → `6200 — Software & Subs`.
   - SHELL / CHEVRON / EXXON → `6400 — Fuel & Vehicle`.
   - STAPLES / OFFICE DEPOT / AMAZON-BUSINESS → `6500 — Supplies`.
   - GUSTO / ADP / RUN / Payroll wire → **escalate** (do not classify).
3. **Always escalate** when:
   - `abs(amount_usd) > 1000` (single-txn cap → `categorize_large_txn`).
   - Vendor unknown after rule + heuristic match (`confidence < 0.6`).
   - Description suggests refund, transfer, or owner draw.
   - Description is a card-feed cash-advance / ATM withdrawal.
4. **Emit the journal entry** with: `account_debit`, `account_credit`,
   `amount_usd`, `memo`, `confidence`, optional `rule_id`.

Output shape:

```json
{
  "journal_entry": {
    "account_debit": "6200",
    "account_credit": "1100",
    "amount_usd": 89.99,
    "memo": "Comcast Business monthly — utilities",
    "confidence": "high"
  },
  "rule_id": "recurring/comcast-business-monthly",
  "escalation_required": false
}
```

Never post the entry from this skill — posting is a separate
`post_journal_entry` approval-gated action.
