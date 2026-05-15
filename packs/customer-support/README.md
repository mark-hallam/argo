# Customer Support Pack

This pack defines a full customer-support worker that extends the existing Tier-1 support flow into Tier-2 research, refund preparation, account-change handling, and escalation-safe customer replies.

## Included Skills

- `triage`: classifies inbound tickets by intent, severity, SLA risk, and owner queue.
- `kb_lookup`: selects approved knowledge-base and account evidence before drafting.
- `respond`: drafts customer-safe replies grounded in approved evidence.
- `escalate`: prepares complete Tier-2, billing, security, or engineering handoff packets.
- `refund`: prepares refund or credit packets without mutating billing systems.
- `account_change`: validates account-change requests and routes them for approval.

## Memory

The seed data includes support policies, approved KB articles, customer account snapshots, recent incident language, refund rules, and handoff templates. The pack is dry-run by default and uses memory fixtures instead of live Zendesk, Stripe, or Salesforce mutations.

## Evaluation Coverage

`evals/golden.json` contains 12 fixtures across incident triage, SSO troubleshooting, API rate-limit guidance, customer reply drafting, Tier-2 escalation, refund prep, owner transfer, downgrade/proration policy, security escalation, and mixed billing/account-change routing.

## Safety Model

Human approval is required before refunds or credits, account permission/owner/plan changes, customer-facing outage statements, and legal, security, or privacy escalations. External replies must be grounded in an approved KB article, account fact, incident statement, or internal approval note.
