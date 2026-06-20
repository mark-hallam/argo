---
name: compile_funnel_dashboard
version: "0.1.0"
description: Compile a funnel dashboard from real metrics across the growth funnel — every number sourced or marked unavailable — and surface the weakest stage so cohort analysis, an experiment design, or a gated launch can follow.
provenance:
  source: vertical/gtm-growth-analytics
  hash: sha256-pending
---

# When to use

Use when the operator asks Ana to compile the funnel dashboard, at the
start of an analytics review, or as the entry point before a cohort
analysis or an experiment design. This is Ana's anchor skill: it pulls
the period's funnel metrics, sources every number, names the weakest
stage, and produces a structured dashboard. It never launches anything —
launching a live experiment is a separate, approval-gated action.

# Inputs you expect

- `period` — the period being reported (e.g. an ISO week), for the audit
  trail.
- `funnel_metrics` — the period's numbers per stage, each with a source.
  Pulled from the analytics / warehouse connectors when wired, from
  operator paste-ins or memory pointers otherwise. Any stage without a
  source is treated as unavailable — never invented.
- `prior_dashboard` — last period's `dashboards` row (optional), so trends
  can be stated against a sourced baseline.

# Output shape

```json
{
  "funnel_dashboard": {
    "period": "2026-W25",
    "stages": {
      "visit":     { "value": 0,             "source": "analytics" },
      "signup":    { "value": 0,             "source": "warehouse" },
      "activate":  { "value": "unavailable", "source": null },
      "convert":   { "value": 0,             "source": "warehouse" },
      "retain":    { "value": "unavailable", "source": null }
    },
    "overall_conversion_pct": { "value": "unavailable", "source": null },
    "weakest_stage": "activate"
  }
}
```

`overall_conversion_pct` is only reported when it can be derived from
sourced stage values; otherwise it is `"unavailable"`. `weakest_stage`
points the next step (cohort analysis, experiment design, or a gated
launch proposal) at the right place in the funnel.

# Hard rules — truth boundary (NO THEATER)

- **Cite every number.** Each funnel figure carries its `source`. A
  number with no source is **`"unavailable"`** — never estimate, round to
  a plausible value, or fill a gap. "unavailable" is a correct answer; a
  made-up figure is not.
- **Never fabricate a cohort or a retention number.** Cohort and
  retention cells come from sourced rows only.
- **Never claim statistical significance without a sufficient sample.**
  When interpreting an experiment, say "the sample is too small to call
  that" rather than asserting a verdict on thin data.
- **Launching a live experiment is approval-gated.** `launch_experiment`
  changes what real users see and ALWAYS routes through the operator
  (`requires_approval: true`). The worker NEVER ships an experiment
  itself; analysis, cohort work, design, and interpretation are
  autonomous, a live launch is not.

# Example

Synthetic only — no real metrics.

```
period: "2026-W25"
funnel_metrics:
  visit:    { value: 8000, source: "analytics" }
  signup:   { value: 320,  source: "warehouse" }
  activate: { value: unavailable }   # connector down
  convert:  { value: 96,   source: "warehouse" }
  retain:   { value: unavailable }
```

```json
{
  "funnel_dashboard": {
    "period": "2026-W25",
    "stages": {
      "visit":    { "value": 8000, "source": "analytics" },
      "signup":   { "value": 320,  "source": "warehouse" },
      "activate": { "value": "unavailable", "source": null },
      "convert":  { "value": 96,   "source": "warehouse" },
      "retain":   { "value": "unavailable", "source": null }
    },
    "overall_conversion_pct": { "value": "unavailable", "source": null },
    "weakest_stage": "activate"
  }
}
```

Here the visit→signup→convert path is sourced, but activate and retain
are unavailable because the connector is down — so they are marked, not
guessed, and `overall_conversion_pct` cannot be derived end-to-end. The
next move is a cohort analysis or a designed experiment on the activate
stage — and any live launch goes to the operator for approval first.
