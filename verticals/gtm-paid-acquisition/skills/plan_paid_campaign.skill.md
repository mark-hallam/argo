---
name: plan_paid_campaign
version: "0.1.0"
description: Plan a paid campaign end-to-end — target audience, channel mix (Meta/Google/LinkedIn), budget split, and one success metric with a threshold — plus the ad-creative variants and the analysis-and-launch plan. Drafting and planning are autonomous; launching and budget changes spend money and are approval-gated. Every figure is sourced or marked unavailable; thin samples stay low-confidence.
provenance:
  source: vertical/gtm-paid-acquisition
  hash: sha256-pending
---

# When to use

Use when the operator asks Quinn to plan a paid-acquisition campaign, or
at the start of a paid-acquisition cycle. This is Quinn's anchor skill: it
walks the full loop — campaign plan → ad-creative drafts → a sourced
analysis plan → the gated launch/budget proposals — and never spends
anything itself. Planning, drafting, and analysis are autonomous;
`launch_campaign` and `adjust_budget` are approval-gated and this skill
only produces the proposals.

# Inputs you expect

- `objective` — what the campaign is for (e.g. paid signups for a segment).
- `target_metric` — the single success metric (`cac`, `roas`, or
  `paid_signups`) with a threshold.
- `channels` — the candidate channels to consider (Meta, Google,
  LinkedIn).
- `prior_metrics` — sourced CAC / ROAS / spend / signups history per
  channel (optional). Pulled from the ads / analytics connectors when
  wired, from operator paste-ins or memory pointers otherwise. Any figure
  without a source is treated as **unavailable** — never invented.

# Output shape

```json
{
  "campaign_plan": {
    "objective": "Paid signups for mid-market ops",
    "audience": "...",
    "channel_split": [
      { "channel": "google", "budget_pct": 50, "rationale": "high intent" },
      { "channel": "meta",   "budget_pct": 30, "rationale": "retargeting" },
      { "channel": "linkedin", "budget_pct": 20, "rationale": "role targeting" }
    ],
    "success_metric": "cac",
    "threshold": 100,
    "daily_budget_usd": "unavailable"
  },
  "ad_creative": [
    { "channel": "google", "variant_label": "A",
      "headline": "...", "body": "...", "cta": "..." }
  ],
  "analysis_plan": {
    "metrics_to_watch": ["cac", "roas", "paid_signups"],
    "min_sample_before_call": 0,
    "source_per_metric": { "cac": "analytics", "roas": "unavailable" }
  },
  "gated_actions": [
    { "action": "launch_campaign", "channel": "google",
      "daily_budget_usd": 0, "requires_approval": true },
    { "action": "adjust_budget", "campaign": "...",
      "requires_approval": true }
  ]
}
```

`daily_budget_usd` is **`"unavailable"`** until the operator sets it — the
plan proposes the split, the operator sets the spend. `gated_actions`
always carries `requires_approval: true` for `launch_campaign` and
`adjust_budget`. Show the full channel split — never collapse the plan to
a single number.

# Hard rules — truth boundary (NO THEATER)

- **Cite every number.** Each prior metric and each analysis figure carries
  its source. A number with no source is **`"unavailable"`** — never
  estimate, round to a plausible value, or fill a gap. "unavailable" is a
  correct answer; a made-up figure is not.
- **Never claim a campaign worked without data.** Report a campaign result
  only from a sourced measurement; "we ran it" is not "it worked".
- **Never overfit a small sample.** A creative or channel with thin support
  sets `confidence: "low"` and is never inflated into a conclusion. Set
  `min_sample_before_call` so a noisy early week doesn't get called a win.
- **Never fabricate a quote or a statistic** to dress up ad copy.
- **Launching is approval-gated.** `launch_campaign` SPENDS operator funds
  and sets `requires_approval: true`. Never auto-launch.
- **Budget never moves here.** `adjust_budget` MOVES money and sets
  `requires_approval: true`. The worker NEVER executes a spend change; the
  proposal stops at the operator.

# Channel routing

Pick the channel mix from the objective and the sourced CAC/ROAS history:

- high-intent search capture → **Google**
- broad demand + warm-audience retargeting → **Meta**
- precise B2B / role-based enterprise reach → **LinkedIn**

If history is unavailable, propose a balanced split and mark it
low-confidence — never present a guess as a data-backed allocation.

# Example

Synthetic only — no real metrics.

```
objective: "Paid signups for mid-market ops"
target_metric: { metric: "cac", threshold: 100 }
channels: ["meta", "google", "linkedin"]
prior_metrics:
  google:   { cac: 82,  roas: 3.4, source: "analytics" }
  meta:     { cac: 140, roas: 2.1, source: "analytics" }
  linkedin: { cac: unavailable }   # connector down
```

```json
{
  "campaign_plan": {
    "objective": "Paid signups for mid-market ops",
    "audience": "Ops leaders at 50-500 person companies",
    "channel_split": [
      { "channel": "google", "budget_pct": 60, "rationale": "lowest sourced CAC ($82) and ROAS above target" },
      { "channel": "meta",   "budget_pct": 40, "rationale": "retargeting; CAC above target, watch closely" }
    ],
    "success_metric": "cac",
    "threshold": 100,
    "daily_budget_usd": "unavailable"
  },
  "ad_creative": [
    { "channel": "google", "variant_label": "A",
      "headline": "Hire your ops teammate in minutes",
      "body": "An AI teammate that does the work, with a hash-chained audit trail.",
      "cta": "Start free" }
  ],
  "analysis_plan": {
    "metrics_to_watch": ["cac", "roas", "paid_signups"],
    "min_sample_before_call": 100,
    "source_per_metric": { "cac": "analytics", "roas": "analytics", "linkedin_cac": "unavailable" }
  },
  "gated_actions": [
    { "action": "launch_campaign", "channel": "google", "daily_budget_usd": 0, "requires_approval": true },
    { "action": "launch_campaign", "channel": "meta", "daily_budget_usd": 0, "requires_approval": true }
  ]
}
```

LinkedIn is excluded from the split this cycle: its CAC is unavailable
(connector down), so allocating spend to it would be a guess, not a
data-backed call. Daily budgets stay `unavailable` until the operator
approves a launch — Quinn never sets the spend herself.
