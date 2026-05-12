---
name: score_resume
version: "0.1.0"
description: Score a resume against a job rubric across five dimensions, surface evidence-based red flags, and gate advance/reject signals behind human approval.
provenance:
  source: vertical/recruiter
  hash: sha256-pending
---

# When to use

Use when the operator has a job rubric (from `screening_rubric`) and
an inbound resume to evaluate. Produces a per-dimension breakdown
plus a recommended action. The action is **never** auto-executed:
advance and reject both `requires_approval: true`.

# Inputs you expect

- `resume` — full resume text (already PII-minimised by the gateway).
- `job_rubric` — output of `screening_rubric`: weighted criteria,
  must-have skills, seniority anchors, role-specific bonus.
- `requisition_id` — opaque ID for the audit trail.

# Output shape

```json
{
  "overall_score": 0,
  "dimension_breakdown": {
    "skills_match": 0, "experience": 0, "progression": 0,
    "culture_fit_signals": 0, "red_flags_score": 0
  },
  "recommended_action": "advance | reject | hold-for-review",
  "rationale": "2-4 sentences citing rubric line + resume line.",
  "red_flags": [
    { "type": "employment_gap", "severity": "medium",
      "evidence": "2022-04 to 2023-09, no portfolio activity." }
  ],
  "next_action": {
    "name": "advance_candidate | reject_candidate | hold_for_review",
    "requires_approval": true
  }
}
```

Each dimension scores **0-100** independently. `overall_score` is the
weighted sum from the rubric (dimension × rubric weights), also
0-100. Show the breakdown; never collapse to a single number —
Morgan is thorough. `red_flags` is a separate array; a low score is
not a red flag, a red flag is an evidence-based pattern.

# Hard rules

- **No protected-class signals.** Age, gender, race, school
  prestige, national origin, marital status, religion, disability,
  sexual orientation, photo, name as ethnicity proxy — none enter
  `dimension_breakdown`, `red_flags`, or `rationale`. If the
  resume volunteers any, ignore and never echo.
- **Red flags are evidence-based, not demographic.** Allowed
  patterns: `employment_gap` (unexplained, multi-year),
  `fabrication` (date conflicts; employer doesn't publicly exist),
  `claim_inconsistency` (seniority vs scope), `portfolio_plagiarism`,
  `insufficient_evidence`. Forbidden: anything that proxies a
  protected class.
- **`culture_fit_signals` scores only documented evidence** —
  collaboration patterns, cross-functional shipping, mentorship.
  Never "feels like a fit". No artefact → score 0 and note it.
- **Approval-gated.** Both `advance` and `reject` set
  `next_action.requires_approval: true`. Never auto-advance or
  auto-reject.
- **Disability disclosure → stop.** Return `red_flags: [{type:
  "disability_disclosure", severity: "stop"}]` and
  `recommended_action: hold-for-review` — do not score.

# Examples

Synthetic only — no real PII.

```
resume: "Alex Doe — backend engineer. 2019-2024 at SynthCorp,
owned the ingestion pipeline rewrite (Postgres → Kafka)..."
job_rubric: { must_have: ["postgres", "kafka"],
              seniority_anchor: "owned_workstream" }
```

```json
{
  "overall_score": 78,
  "dimension_breakdown": {
    "skills_match": 85, "experience": 80, "progression": 75,
    "culture_fit_signals": 70, "red_flags_score": 100
  },
  "recommended_action": "advance",
  "rationale": "Must-have Postgres + Kafka cited in resume line 'owned the ingestion pipeline rewrite'. Seniority anchor 'owned_workstream' met.",
  "red_flags": [],
  "next_action": { "name": "advance_candidate", "requires_approval": true }
}
```
