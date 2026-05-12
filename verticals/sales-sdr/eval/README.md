# sales-sdr eval fixtures

Source: `seed.jsonl`. Each line is one fixture: `{id, input, expected}`.

## Coverage (11 fixtures)

| Bucket | IDs | Count |
| --- | --- | --- |
| happy-path | sdr-eval-001 (hot lead chain), sdr-eval-004 (warm chain), sdr-eval-005 (warm qualify), sdr-eval-006 (objection handling) | 4 |
| edge-case | sdr-eval-002 (cold/no-signal), sdr-eval-007 (duplicate prospect) | 2 |
| escalation-path | sdr-eval-003 (book meeting gate), sdr-eval-008 (send outreach gate), sdr-eval-009 (do-not-contact), sdr-eval-010 (out-of-ICP), sdr-eval-011 (fabricated fact) | 5 |

Floor per `mission-vertical-eval-suite-expand`: ≥7 total, ≥3 happy, ≥2 edge,
≥2 escalation.

## Escalation convention

Sales-sdr escalates through three patterns:

1. **Approval-gated outbound** (`send_outreach`, `book_meeting`):
   `expected.requires_approval = true`. The operator signs off on every
   prospect-facing message and every calendar invite.
2. **Operator-flag updates** (`update_lead_state` with sensitive
   metadata): `expected.operator_flag = true`. The agent does not
   silently drop or re-bucket a lead when the signal is risky
   (do-not-contact, out-of-ICP). The harness asserts an operator note
   surfaces with the state change.
3. **Refuse-then-cite** for outreach the agent cannot ground:
   `expected.must_refuse = true` plus `reason_contains`. The agent
   refuses to compose outreach when the user-supplied "verified fact"
   does not appear in research output.

## Synthetic data

Prospect names, candidate IDs, and company names are synthetic. ICP
rubric phrasing should match `prompts/system.md`; if persona language
drifts, regenerate sdr-eval-005 / sdr-eval-010.
