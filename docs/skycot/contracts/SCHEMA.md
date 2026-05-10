# Delegation contract — `<role>.contract.toml` schema

A delegation contract is the single artefact that captures everything an
operator wants a worker to **be**: its operating charter, the duties it
owns, the KPI definitions it is measured by, the quarterly objectives
it is driving toward, and the success criteria for promotion / closure.

Today an operator must hand-bootstrap charter / KPI definitions / OKRs
through three separate CLIs (`argo worker charter set`,
`argo worker kpi …`, `argo worker okr set`). The contract file rolls all
of that into one reviewable, version-controllable TOML file that can be
diff'd in pull requests and applied with one command.

> Mission claim 2: "Delegation is the unit. 'Be my marketing intern' or
> 'handle inbound recruiter messages' is the contract — not 'run this
> one job.'"

## File layout

```toml
[contract]
role = "ar-clerk"                         # required; free-form label
charter = """
Pursue outstanding receivables under the approved policy.
Escalate disputes worth more than $10k to the AR lead.
"""                                       # required; markdown body

[[duties]]
id = "send-reminder"                      # required; stable id
description = "Send dunning reminder at +30/+45/+60 days past due."

[[duties]]
id = "escalate-dispute"
description = "Hand off any dispute > $10k to the AR lead within 1 business day."

[[kpis]]
name = "dso_days"                         # required; matches `argo worker kpi record <name>`
description = "Days sales outstanding"
target = 35.0                             # numeric target
comparison = "lte"                        # gte | lte | eq

[[objectives]]
period = "2026Q2"                         # YYYYQ[1-4]
title = "Drive DSO under 30"
description = "Optional long-form context."

[[objectives.key_results]]
kpi = "dso_days"                          # references a [[kpis]].name
target = 30.0
comparison = "lte"
weight = 1.0                              # > 0; relative weight in the period score

[success_criteria]
min_sessions = 50                         # optional; promotion gate
min_success_rate = 0.85                   # optional; in [0, 1]
```

### Section reference

| Section                       | Required | Multiplicity | Notes                                                                                                     |
| ----------------------------- | -------- | ------------ | --------------------------------------------------------------------------------------------------------- |
| `[contract]`                  | yes      | 1            | `role` and `charter` both required and non-empty.                                                         |
| `[[duties]]`                  | no       | 0..*         | `id` must be non-empty. Persisted in the worker's `kpis` JSON blob alongside the KPI definitions.         |
| `[[kpis]]`                    | no       | 0..*         | KPI **definitions**. Observed values are recorded separately via `argo worker kpi record`.                |
| `[[objectives]]`              | no       | 0..*         | Each objective must declare ≥ 1 `[[objectives.key_results]]`. `period` follows the existing `YYYYQ[1-4]` form. |
| `[[objectives.key_results]]`  | yes inside an objective | 1..* | `kpi` should match a `[[kpis]].name`; `comparison` is `gte` / `lte` / `eq`; `weight > 0`. |
| `[success_criteria]`          | no       | 0..1         | `min_success_rate` is a fraction in `[0, 1]`.                                                             |

`comparison` accepts only `gte`, `lte`, `eq` — same operator-facing
tokens the existing `--kr` CLI parser uses (see
`argo worker okr set --kr kpi:op:target:weight`).

## CLI

```text
argo contract validate <file>             # schema-only check, no DB
argo contract apply <file> --tenant <t> --worker <id>
argo contract apply <file> --dry-run      # preview the planned mutations
argo contract show <worker-id> --tenant <t>
```

`apply` is a thin orchestrator over the existing primitives: it
updates the worker's charter (markdown body) plus the `kpis` JSON blob
(which carries `role`, `duties`, KPI definitions, `success_criteria`),
then invokes `argo_store::worker_okr::set_objective` once per
`[[objectives]]` block. No new schema; no migrations.

`validate` and `--dry-run` never touch the store and never require a
running gateway.

`show` reads the worker's persisted state and renders a
semantically-equivalent TOML document — the same input you'd hand
back to `apply`. TOML key ordering may differ from the original file
on disk; the **content** round-trips bit-for-bit through
`toml::to_string_pretty` → `toml::from_str`.

## Starter templates

Bundled in this directory:

- `ar-clerk.contract.toml` — finance-AR receivables operator.
- `recruiter-screener.contract.toml` — inbound recruiter triage.
- `marketing-intern.contract.toml` — first-touch marketing assistant.

Copy one, edit it for your role, and apply:

```sh
cp docs/skycot/contracts/ar-clerk.contract.toml ./ar-clerk.contract.toml
$EDITOR ./ar-clerk.contract.toml
argo contract validate ./ar-clerk.contract.toml
argo contract apply ./ar-clerk.contract.toml \
    --tenant <tenant-uuid> --worker <worker-uuid>
```

## Errors

Validation surfaces structured errors with file path + line + column
for malformed TOML, and a `[[section]][i].field` path for semantic
errors. Examples:

```text
argo: ar-clerk.contract.toml:14:3: invalid type: found integer, expected a string
argo: [[kpis]][0].comparison: invalid comparison 'wat': expected gte|lte|eq
argo: [[objectives]][1] must declare at least one key_result
```
