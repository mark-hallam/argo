# Support Specialist (Tier 1) Pack

This pack defines a Tier-1 support teammate that receives customer
tickets, classifies risk, resolves low-risk issues from verified KB
articles, drafts customer replies, escalates sensitive cases, and keeps
ticket status current.

## Contents

- `pack.toml` declares the `support-tier1` pack and vertical entry.
- `worker.toml` sets the default display name, charter, tool families,
  and approval policy.
- `skills/*.skill.json` contains five Skill IR graphs:
  `triage-ticket`, `resolve-from-kb`, `escalate-to-tier2`,
  `draft-resolution-message`, and `update-ticket-status`.
- `memory/schema.json` and `memory/seed.json` provide KB, escalation,
  and triage-pattern memory.
- `evals/golden.json` contains 20 golden fixtures.

## Demo Flow

1. Receive a ticket from Zendesk, Intercom, or Freshdesk.
2. Run `triage-ticket` to classify issue type, urgency, customer tier,
   and escalation flags.
3. For low-risk tickets, run `resolve-from-kb` and
   `draft-resolution-message`.
4. For sensitive tickets, run `escalate-to-tier2` with a lead-reviewed
   handoff packet.
5. Run `update-ticket-status` so the audit trail records the next
   owner and ticket state.

Customer-visible messages stay approval-gated by default. Refunds,
security incidents, legal threats, account deletion, outage claims, and
enterprise SLA exceptions always route through escalation.
