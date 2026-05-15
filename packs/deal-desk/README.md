# Deal Desk Analyst Pack

This pack defines a deal-desk teammate that validates incoming quotes,
checks discount policy, flags non-standard commercial and legal terms,
routes approval packets, and drafts concise deal summaries.

## Contents

- `pack.toml` declares the `deal-desk` pack and vertical entry.
- `worker.toml` sets the default display name, charter, tools, and
  approval gates.
- `skills/*.skill.json` contains five Skill IR graphs:
  `validate-quote`, `check-discount-policy`,
  `flag-non-standard-terms`, `route-deal-desk-approval`, and
  `draft-deal-summary`.
- `memory/schema.json` and `memory/seed.json` provide discount,
  standard-term, approval-matrix, validation-rule, and deal-history
  memory.
- `evals/golden.json` contains 20 golden fixtures.

## Demo Flow

1. Receive a quote from CRM or a workspace intake packet.
2. Run `validate-quote` to verify required fields, price book, SKU
   compatibility, and co-terming.
3. Run `check-discount-policy` and `flag-non-standard-terms` to
   identify margin, finance, legal, and executive exceptions.
4. Run `route-deal-desk-approval` to build the approval packet and ask
   the right approvers.
5. Run `draft-deal-summary` so sales leadership sees the deal value,
   concessions, risks, missing evidence, and recommended next action.

Customer-facing quotes, discount exceptions, payment exceptions,
non-standard terms, and executive approval packets stay approval-gated
by default.
