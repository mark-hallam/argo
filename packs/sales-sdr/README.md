# Sales SDR Pack

This pack defines a sales development representative teammate for
prospecting, outreach drafting, reply handling, qualification, and CRM
hygiene. It is designed for enterprise GTM workflows where outbound
messages, meeting booking, and customer-visible commitments require
human approval.

## Contents

- `pack.toml` declares the `sales-sdr` pack and vertical entry.
- `skills/*.skill.json` contains five Skill IR graphs: `enrich`,
  `draft_outreach`, `classify_reply`, `qualify`, and `log_to_crm`.
- `memory/schema.json` and `memory/seed.json` define ICP, messaging,
  qualification, and CRM hygiene memory.
- `evals/golden.json` contains 15 golden fixtures spanning enrichment,
  outbound drafting, reply classification, qualification, CRM logging,
  and two end-to-end flows.

## Demo Flow

1. Run `enrich` on a lead from Salesforce, ZoomInfo, or Apollo.
2. Run `draft_outreach` to produce an evidence-backed outbound draft.
3. Ask for manager approval before any customer-visible send.
4. Run `classify_reply` when the prospect responds.
5. Run `qualify` to decide whether an account executive handoff or
   nurture path is appropriate.
6. Run `log_to_crm` so Salesforce records only sourced fields, activity
   notes, qualification score, and next owner.

The worker never fabricates missing CRM fields, never sends outbound
messages without approval, and escalates opt-outs, legal threats,
security commitments, discount guarantees, and angry replies.
