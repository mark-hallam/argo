# Skill: draft AR reminder email

Use this skill when the operator needs a polite reminder body for an
invoice that just crossed a 7 / 14 / 30 day past-due threshold. The
output goes into `payload.body` of a `send_reminder` action; the
operator approves the exact text before send.

## When to apply

- Invoice `bucket` is one of `overdue_7`, `overdue_14`, `overdue_30`.
- `customer_email` is present in `inventory.json`.
- No `escalation` row exists for this invoice in the `escalations`
  memory entry (escalated invoices use a different path).

## What to write

A short email (≤ 140 words) with this shape:

1. **Subject**: `Reminder: Invoice {{invoice_id}} — ${{amount_usd}} due {{due_date}}`
2. **Greeting**: `Hi {{customer}} team,` — never just "Hi" or "Hello".
3. **Body, paragraph one**: state the invoice id, amount, and original
   due date. Acknowledge it was likely missed; offer to resend the PDF.
4. **Body, paragraph two**: name the bucket (7/14/30 days past due) in
   plain language, e.g. "It's now about a week past the due date."
5. **Closing**: ask for confirmation of the payment date or a heads-up
   on any blockers. Sign off as the AR clerk.

## What NOT to write

- No late fees, suspension threats, or collection language.
- No "as discussed" / "per our last call" phrasing — most threads have
  no prior call.
- No exact bank account numbers or wiring details inline; reference
  the original invoice instead.
- No urgency manipulation ("final notice", "immediate action").

## Example (overdue_7 bucket, INV-2026-0327)

```
Subject: Reminder: Invoice INV-2026-0327 — $1,180.00 due 2026-04-26

Hi Halcyon Labs team,

Following up on invoice INV-2026-0327 for $1,180.00, originally due
2026-04-26. It's now about a week past the due date — no worries if
this slipped through; happy to resend the PDF if it would help.

Could you let me know when payment is scheduled, or flag anything
on your end that needs sorting first?

Thanks,
Riley
Accounts Receivable, Acme Widgets
```
