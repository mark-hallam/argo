# Vendor Manager Pack

The Vendor Manager pack provides a dry-run enterprise worker for procurement teams. It drafts RFP responses, scores proposals, negotiates renewals, tracks supplier SLA risk, and routes approval-ready procurement packets.

## Install Check

Use the pack loader from M15 against this directory:

```bash
argo packs dev-link vendor-manager packs/vendor-manager
argo packs list
```

The worker remains in dry-run mode. Supplier-facing messages, spend commitments, contract changes, and supplier performance notices require human approval before external action.

## Included Skills

- `draft-rfp-response` builds supplier-facing RFP or clarification drafts from a request, rubric, and policy context.
- `score-vendor-proposal` compares proposals against weighted commercial, risk, security, implementation, and support criteria.
- `negotiate-renewal` prepares renewal counteroffers using usage signals, benchmarks, and fallback terms.
- `track-vendor-sla` detects SLA miss patterns and prepares evidence-backed escalation drafts.
- `route-procurement-approval` assembles approval packets with spend, policy, risk, contract, and evidence sections.

## Demo Path

Provide a new RFP intake, three supplier proposals, and the standard SaaS rubric. The worker drafts the response, scores the proposals, recommends the best option, and routes an approval packet without sending anything externally.

