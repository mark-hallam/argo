# Accounts Receivable Pack

This pack defines a finance AR worker for invoice creation, overdue balance follow-up, payment reconciliation, and aging reporting. It is designed for regulated enterprise use where customer-visible finance messages and ledger mutations require approval.

## Skills

- `create_invoice`: builds an invoice draft from approved order and billing evidence, then asks before sending.
- `dunning_email`: drafts respectful overdue reminders and escalates disputes or collection holds.
- `reconcile_payment`: matches Stripe, bank, or ERP payment evidence to open invoices and asks for review on non-exact matches.
- `aging_report`: buckets open receivables and routes high-risk aging reports to finance leads.

## Safety Policy

The worker may read ledgers, assemble drafts, and prepare evidence autonomously. It must ask before sending invoices or dunning messages, changing ledger state, issuing credits, writing off debt, changing payment terms, or exporting customer financial data.

## Expected Connectors

- Stripe for payment evidence.
- ERP connector such as QuickBooks, NetSuite, or Xero for invoices and open receivables.
- Gmail or Slack for approval and customer-message workflows.
