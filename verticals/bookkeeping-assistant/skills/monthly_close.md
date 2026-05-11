---
name: monthly_close
description: Produce a read-only P&L and balance-sheet trial for a closing period, flagging uncategorized rows and accounts whose ending balance moved >25% MoM.
inputs:
  - period
  - chart_of_accounts
  - journal_entries
outputs:
  - pnl_summary
  - balance_summary
  - exceptions
  - close_blocked
---

# Skill: Monthly Close Trial

Use this skill to assemble a non-mutating monthly close trial for the
operator to review before any posting.

1. **Set the period.** Accept `YYYY-MM`. Reject anything outside the
   last 13 months as a stale-close attempt and escalate.
2. **Roll up the P&L.** Group categorized journal entries by account:
   - Revenue accounts (`4xxx`) net of contra-revenue (`4900`).
   - Cost of goods sold (`5xxx`).
   - Operating expense accounts (`6xxx`).
   - Other income/expense (`7xxx`/`8xxx`).
3. **Roll up the balance sheet.**
   - Assets (`1xxx`), liabilities (`2xxx`), equity (`3xxx`).
   - Always confirm `assets = liabilities + equity` for the close
     period; if it doesn't balance, set `close_blocked = true` with
     `reason = "trial_balance_mismatch"`.
4. **Flag exceptions.** Surface, but do not post:
   - Any account whose ending balance moved more than 25% MoM.
   - Any `unmapped` rows from the categorization pass (close cannot
     finalize until they're resolved).
   - Any account that has gone negative for the first time.
5. **Output the period summary** with totals, the MoM delta table, the
   exception list, and a one-line operator-facing verdict ("close-ready
   pending 3 unmapped rows" / "blocked: balance sheet does not tie").

Output shape:

```json
{
  "period": "2026-04",
  "pnl_summary": {"revenue": 0.00, "cogs": 0.00, "opex": 0.00, "net": 0.00},
  "balance_summary": {"assets": 0.00, "liabilities": 0.00, "equity": 0.00},
  "exceptions": [{"account": "6300", "kind": "mom_delta_pct", "value": 0.42}],
  "close_blocked": false
}
```

This skill is read-only — it never posts a journal entry and never
modifies the live ledger. Posting and adjusting entries are
approval-gated actions handled outside this skill.
