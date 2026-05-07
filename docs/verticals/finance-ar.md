# Vertical: finance-ar

V2.2 demo vertical (Stream `claude-vertical-finance-ar`). Fourth
demo vertical — see also `customer-support`, `sales-sdr`, and
`marketing-intern` (all from `claude-demo-multi-vertical`).

## Persona
Riley — AR clerk for a small B2B SaaS shop. Polite, meticulous,
quick to flag a case for human judgment rather than send another
nudge.

## Action surface
- `review_invoices` — daily duty. Buckets every open invoice into
  `current` / `due_soon` / `overdue_7` / `overdue_14` / `overdue_30`
  / `overdue_45`. Autonomous-safe.
- `send_reminder` — drafts a polite reminder email and emits an
  approval-gated `gmail.send` action. Operator approves the body
  before the message goes out.
- `escalate_overdue` — at 45 days past due, hands the case to a
  human (typically the operator or a finance lead). Approval-gated;
  no money moves.
- `log_payment_receipt` — records an inbound Stripe (or other)
  payment-received event into the `payments` memory entry. Flips the
  invoice's status to `paid` or `partial`.
- `generate_aging_report` — renders the weekly AR aging report (CSV
  + PDF) via the `document_creation` executor. Approval-gated before
  the PDF is shared.

## Memory schema
- `invoices` — bucketed review state per open invoice.
- `reminders` — reminder thread history (one row per send).
- `payments` — inbound payment events.
- `escalations` — human handoffs at 45+ days.
- `ar_learnings` — reusable AR patterns the operator confirms work.

## Duty
`daily_review` fires `0 9 * * 1-5` (weekdays at 09:00 local). The
duty-scheduler primitive landed in
[#899](https://github.com/mark-hallam/Argo-Agent/pull/899); the
gateway-side spawn wires up via
`codex-duty-scheduler-gateway-wiring` (Phase A LOAD-BEARING,
unblocked once gateway/mod.rs is split — see
`claude-gateway-mod-split`).

## Demo
```sh
./scripts/demo_finance_ar.sh
```

The demo copies the vertical into a tempdir workspace, runs
`argo vertical doctor`, and prints the operator's next-step
checklist. No external API keys required for the doctor scan.

## Approval gates
Every buyer-facing action (`send_reminder`, `escalate_overdue`,
`generate_aging_report`) is approval-gated. `review_invoices` and
`log_payment_receipt` are autonomous-safe (read or memory-only). The
`auto_approval.toml` cap is `$5/day` total + `$0.10/action`.

## Bootstraps via Genesis
The matching archetype is `BookkeepingAssistant`. Genesis charter
keywords like "accounts receivable", "AR clerk", "invoices",
"reminders", or "payment follow-up" classify here:

```sh
argo genesis classify --charter "AR clerk who chases overdue invoices for a SaaS"
# → BookkeepingAssistant
```

## Deferred follow-up work (out of scope for this brief)
- Stripe webhook handler that auto-emits `log_payment_receipt`
  actions on `payment_intent.succeeded` events. The action declaration
  is wired here; the inbound webhook glue lives in a follow-up brief.
- Full PDF aging report rendering. The `generate_aging_report`
  action declaration targets `executor.kind = "document_creation"`
  with `format = "pdf"`; the action wiring is in place but the
  per-bucket PDF template lives in a follow-up.
- 20-scenario eval suite. Shipped with 5 scenarios mirroring the
  existing demo verticals (customer-support has 3, marketplace-seller
  has 4); the brief asked for 20 — additional scenarios can be added
  per `tests/vertical_smoke` patterns once cassettes for
  `gmail.send` + `stripe.list_invoices` are recorded.
