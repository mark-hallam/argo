# customer-support eval fixtures

Source: `seed.jsonl`. Each line is one fixture: `{id, input, expected}`.

## Coverage (10 fixtures)

| Bucket | IDs | Count |
| --- | --- | --- |
| happy-path | cs-eval-001, cs-eval-003, cs-eval-004, cs-eval-005, cs-eval-010 | 5 |
| edge-case | cs-eval-006 (foreign language), cs-eval-007 (duplicate ticket) | 2 |
| escalation-path | cs-eval-002 (angry customer), cs-eval-008 (cross-tenant data leak), cs-eval-009 (large unsupported refund) | 3 |

Floor per `mission-vertical-eval-suite-expand`: ≥7 total, ≥3 happy, ≥2 edge,
≥2 escalation.

## Escalation convention

Any fixture whose `expected.action_type` is `escalate_to_human` MUST also
carry a `reason_contains` (or `severity`) field — the eval harness asserts
that the agent's escalation reason carries the operator-visible signal,
not just that it escalated. Silently routing to a human without naming
the trigger defeats the load-bearing safety property of the escalation
path.

## Synthetic data

All ticket numbers, customer names, and identifiers are synthetic. Never
paste real customer data into a fixture; if a fixture needs PII shape,
use the placeholder patterns from `verticals/gtm-engineer/eval/`.
