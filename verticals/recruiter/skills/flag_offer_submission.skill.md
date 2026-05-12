---
name: flag_offer_submission
version: "0.1.0"
description: Package an offer proposal for human approval — candidate profile, offer details, comp benchmarks, and risk flags. Never auto-sends; always routes to a human.
provenance:
  source: vertical/recruiter
  hash: sha256-pending
---

# When to use

Use when an offer is being prepared for a candidate (post-onsite,
pre-send). Assembles every signal a hiring manager needs to make
the call. **Always** routes to a human; there is no auto-send code
path from this skill.

# Inputs you expect

- `candidate_profile` — `{id, current_stage, score_breakdown,
  seniority_call}` from pipeline history.
- `offer_details` — `{comp_total, comp_breakdown: {base, equity,
  bonus, sign_on}, start_date, role, level, location}`.
- `comp_benchmarks` — `{p50, p75, p90}` for the role + level +
  location from the operator's benchmark source.
- `signals` — optional `{competitor_offers: [...],
  candidate_stated_target: {amount, source}, time_in_market_days}`.

# Output shape

```json
{
  "candidate_id": "synthetic-001",
  "offer_envelope": {
    "role": "Senior Backend Engineer",
    "level": "L5",
    "comp_total": 280000,
    "comp_breakdown": {"base": 200000, "equity": 60000, "bonus": 15000, "sign_on": 5000},
    "start_date": "2026-07-01",
    "location": "Remote / US",
    "vs_benchmark": {"p50": 250000, "p75": 285000, "p90": 320000, "placement": "between p50 and p75"}
  },
  "risk_flags": [
    { "type": "counter_offer_likely", "severity": "medium", "evidence": "Candidate stated target $310k cites current TC $290k." },
    { "type": "outlier_comp_low", "severity": "low", "evidence": "Offer below p50 by 8%." }
  ],
  "recommended_human": "hiring_manager + recruiter_director",
  "next_action": {
    "name": "submit_offer_for_approval",
    "requires_approval": true,
    "auto_send_allowed": false
  }
}
```

# Risk flag taxonomy

The skill enumerates `risk_flags` from this fixed taxonomy:

- `counter_offer_likely` — candidate disclosed competing offer OR
  stated target ≥ p75 AND `time_in_market_days < 14`.
- `compete_with_competitor` — `signals.competitor_offers` non-empty.
- `outlier_comp_high` — `comp_total > benchmark.p90 × 1.1`.
- `outlier_comp_low` — `comp_total < benchmark.p50 × 0.9`.
- `equity_heavy` — equity > 40% of `comp_total`.
- `start_date_compressed` — `start_date < now + 14 days`.

Anything outside this taxonomy is **not** a flag for this skill.

# Hard rules

- **Never proposes offer numbers.** The skill consumes
  `offer_details` from the operator; it never invents `base`,
  `equity`, `bonus`, or `sign_on`. The envelope is for human
  review, not auto-generation.
- **Never auto-sends.** `next_action.auto_send_allowed: false` is
  load-bearing. There is no code path that flips this to true.
- **Human approval is mandatory.** `requires_approval: true` and
  `recommended_human` names which humans must sign off (default:
  hiring_manager + recruiter_director).
- **No protected-class signals in `risk_flags`.** Age, gender,
  family status, etc. cannot enter the rationale. Counter-offer
  prediction is based on stated evidence (target amount,
  competitor disclosure), never demographic proxies.
- **No salary-history input.** `signals.candidate_stated_target`
  is allowed; `salary_history` is not (illegal in many
  jurisdictions as an offer input). If the upstream payload
  includes it, drop it and set `flags: ["salary_history_dropped"]`.
- **Synthetic data only in examples.** No real PII; use
  `candidate-id: synthetic-001`.

# Examples

Synthetic — Senior Backend Engineer offer at $280k total, benchmark
p75 = $285k. Output sets `outlier_comp_low: low` because $280k is
−1.7% from p75 but +12% over p50 — actually well within band, so
no outlier flag. Counter-offer flag fires if candidate stated
target $310k. Operator sees the envelope + flags and decides.
