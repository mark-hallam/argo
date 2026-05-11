---
name: find_runbook
description: Search known runbook locations (inventory.json + operator-attached runbook repo) for the alert symptom and return the best-match runbook id with a confidence score.
inputs:
  - service
  - alert_name
  - symptom_summary
outputs:
  - matches
  - best_match_id
  - confidence
  - runbook_excerpt
---

# Skill: Find a runbook for the symptom

Search the known runbook locations for the alert symptom. Read-only.

1. **Read `inventory.json`** at the vertical root for the in-tree
   runbook table. Each entry carries `runbook_id`, `service`,
   `symptoms[]`, `severity_min`, `path`, and `human_required` flag.
2. **Match on `service` AND symptom keywords.** Tokenise the alert
   name + summary; match against `symptoms[]` substrings.
   Case-insensitive. Multi-keyword matches outrank single-keyword.
3. **If multiple runbooks match,** rank by symptom-keyword overlap
   ratio. Return the top 3 with confidence scores. Do not pick one
   blindly — the operator sees the ranked list.
4. **If `confidence < 0.7`,** mark `best_match_id = null` and
   classify the page as `human_required` on return; an ambiguous
   match is worse than no match.
5. **Cite the runbook excerpt.** Quote the matching symptom line
   verbatim so the operator can validate the match in one glance.
6. **Never inline the runbook's mutation steps in autonomous mode.**
   If the runbook calls for `restart_service` or `rollback_deploy`,
   the skill returns the runbook id; the actual mutation goes through
   the approval gate, never auto-applied.

Output shape:

```json
{
  "matches": [
    {"runbook_id": "rb-db-pool-saturation", "confidence": 0.84},
    {"runbook_id": "rb-slow-query-tail",    "confidence": 0.41}
  ],
  "best_match_id": "rb-db-pool-saturation",
  "confidence": 0.84,
  "runbook_excerpt": "Symptoms: db.pool.saturation_pct > 90% sustained 2min; error rate 2-5% with timeout errors in app logs."
}
```

When no runbook matches at any confidence, emit
`{"matches": [], "best_match_id": null, "confidence": 0.0}` and let
the caller route to `escalate_to_human`.
