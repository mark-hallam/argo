---
name: update_candidate_pipeline
version: "0.1.0"
description: Persist candidate state transitions across the 8-state recruiter pipeline with reasoning + timestamps. Shape-compatible with recruiter-screener for cross-vertical hand-off.
provenance:
  source: vertical/recruiter
  hash: sha256-pending
---

# When to use

Use whenever a candidate's stage changes — after a screening
decision, an interview booking, an onsite, an offer, or a terminal
outcome. Writes the new state to `memory.candidate_pipeline[]` with
reasoning + a timestamp. This is the audit trail; do not skip a
transition.

# Inputs you expect

- `candidate_id` — stable opaque ID.
- `from_state` — current state; must match the latest pipeline row.
- `to_state` — new state. Must be in the 8-state vocabulary and a
  legal transition.
- `reason` — 1-2 sentences citing the action that triggered the
  transition (e.g. "Operator approved advance after phone screen").
- `requisition_id` — opaque ID for the audit trail.

# 8-state vocabulary

Canonical states (future `argo-store` schema should mirror):

1. **`sourced`** — candidate identified, no contact yet.
2. **`contacted`** — outreach sent, awaiting response.
3. **`responded`** — candidate engaged; screening can begin.
4. **`screened`** — phone screen complete; advance decision pending.
5. **`onsite`** — full interview loop in progress.
6. **`offered`** — offer extended (post `flag_offer_submission` +
   human approval).
7. **`hired`** — terminal: offer accepted, start_date set.
8. **`closed`** — terminal: did not hire. Carries a
   `closed_reason: passed | withdrew` sub-field. `passed` =
   company declined; `withdrew` = candidate withdrew.

Legal transitions: forward edges along the chain, **plus** `closed`
is reachable from any non-terminal state. `hired` is reachable only
from `offered`. Transitions are not reversible.

# Output shape

```json
{
  "candidate_id": "synthetic-001",
  "transition": {
    "from_state": "screened", "to_state": "onsite",
    "reason": "Operator approved advance after phone-screen 2026-05-12.",
    "transitioned_at": "2026-05-12T18:30:00Z",
    "closed_reason": null
  },
  "memory_write": {
    "key": "candidate_pipeline", "op": "append",
    "row": {
      "candidate_id": "synthetic-001", "state": "onsite",
      "reason": "...", "last_touched_at": "2026-05-12T18:30:00Z",
      "requisition_id": "req-042"
    }
  },
  "next_action": { "name": "persist_pipeline_row", "requires_approval": false },
  "flags": []
}
```

# Cross-vertical compatibility

`memory.candidate_pipeline` shape matches `recruiter-screener` —
same `candidate_id`, `state`, `last_touched_at` keys. The 8-state
vocabulary is the superset; `recruiter-screener` operates on
`sourced → contacted → responded → screened` and hands off at
`screened`.

# Hard rules

- **Persisting is autonomous-safe.** Writes audit metadata only;
  `requires_approval: false`. Actions that *cause* transitions
  (advance, reject, offer) are gated by their own skills.
- **Illegal transitions → stop.** If `to_state` is not a legal
  successor, set `flags: ["illegal_transition"]` and do not write.
- **Stale `from_state` → stop.** If the latest pipeline row for
  `candidate_id` is not `from_state`, set `flags:
  ["stale_from_state"]` and escalate — concurrent writes upstream.
- **No protected-class signals in `reason`.** The audit log is
  subpoenable. Cite the action and approval, never demographic
  context.
- **Synthetic data only in examples.** Never real PII.

# Examples

Synthetic — Alex Doe (`synthetic-001`) transitions `screened →
onsite` after operator approval. The skill appends one row to
`memory.candidate_pipeline` citing the approval event. Downstream
skills re-read the row to confirm current stage.
