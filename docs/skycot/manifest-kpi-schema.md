# Vertical Manifest — Declarative KPIs

The `[[kpi]]` block in `vertical.toml` lets a vertical declare its own
release-gate metrics without a Rust code change. Each entry feeds the
unified read path `VerticalManifest::all_kpis()` consumed by the eval
CI gate, the worker KPI tracker, and dashboards.

## TOML example

```toml
schema_version = "1"

[vertical]
id      = "ai-receptionist"
name    = "AI Receptionist"
version = "0.3.0"

# Declarative KPIs. Zero or more entries; order is preserved.
[[kpi]]
name        = "deflection_rate"
kind        = "percentage"
threshold   = 80.0
comparator  = "gte"
gate        = "release"
description = "Pct of inbound calls resolved without a human handoff."

[[kpi]]
name       = "first_response_p95_seconds"
kind       = "duration_ms"
threshold  = 30000.0
comparator = "lte"
gate       = "warn"
```

## Fields

| Field | Type | Notes |
| --- | --- | --- |
| `name` | string | Unique within the manifest. Validation rejects duplicates. |
| `kind` | enum | `percentage` \| `duration_ms` \| `count` \| `ratio` \| `score`. Drives downstream display formatting and unit suffix. |
| `threshold` | f64 | Must be finite. NaN / ±inf are rejected. |
| `comparator` | enum | `gte` (≥) \| `lte` (≤) \| `eq` (== within an implementation-defined epsilon). |
| `gate` | enum | `release` (blocks release on fail) \| `warn` (dashboards only) \| `inform` (telemetry only). |
| `description` | string? | Optional. Validation caps the length at 200 chars so dashboards render inline. |

## Back-compat with `[success_metrics]`

The pre-existing `[success_metrics]` block stays valid. When a manifest
declares zero `[[kpi]]` entries, the four legacy fields are synthesized
into equivalent KPIs by `all_kpis()`:

| Legacy field | Synthesized name | kind | comparator | gate |
| --- | --- | --- | --- | --- |
| `deflection_target_pct` | `deflection_target_pct` | percentage | gte | release |
| `median_response_seconds` | `median_response_seconds` | duration_ms (× 1000) | lte | release |
| `csat_threshold` | `csat_threshold` | score | gte | release |
| `required_audit_chain_intact` | `required_audit_chain_intact` | count (1.0 / 0.0) | eq | release |

When both `[[kpi]]` AND `[success_metrics]` are populated, the
declarative entries win — an operator migrating a legacy field into a
`[[kpi]]` block does not see it double-counted. Remove the legacy line
from `[success_metrics]` as part of the migration.

## Gate semantics

- `release` — A failing KPI blocks the release-gate check. Pair this
  with a KPI whose threshold is conservative enough that you would
  hold a release on its violation.
- `warn` — Surfaces in the operator dashboard but does NOT block
  release. Right level for metrics the team is watching but not yet
  ready to enforce.
- `inform` — Pure telemetry. Aggregated for trend analysis; no
  operator-facing surface.

Enforcement (deciding what to do with a failing KPI) lives in
downstream consumers. This schema only carries the declaration.
