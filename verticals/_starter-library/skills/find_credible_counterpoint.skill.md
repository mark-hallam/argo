---
name: find_credible_counterpoint
version: "0.1.0"
description: "Finds the strongest credible argument against a position the operator is leaning toward — steel-manned, not strawmanned."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

The operator has a position and wants to pressure-test it before
committing — a strategy, a vendor pick, a launch plan, a public
statement. Use any time the request is "what's the best argument
against this?" or "what am I missing?"

# Inputs

- `position`: the one-sentence position the operator holds.
- `context`: the constraints, goals, audience the position is
  meant to serve.
- `acceptable_sources`: same shape as `cite_sources_for_claim` —
  what counts as credible in this domain.

# Output

The strongest counterpoint, written as it would be made by a
thoughtful opponent (not a generic devil's advocate). Includes:
the core counter-claim, 1-2 supporting reasons with citations
where possible, and the conditions under which the counterpoint
would dominate the operator's position.

Followed by a short **What this changes** note: which inputs to
the operator's position would have to be wrong for the
counterpoint to win.

# Examples

- Position `"we should build in-house rather than buy"` →
  steel-manned buy case with TCO data, risk profile, and the
  conditions (e.g., "if engineering capacity is below X")
  under which buying wins.
- Position with no credible counter (rare but possible) →
  return a `note: "no credible counter found in
  acceptable_sources; consider whether the position is well-
  framed"` rather than inventing one.

# Approval posture

Read-only. The counterpoint is for the operator's consideration,
not a vote against their position.

# Notes

- Steel-man, don't strawman. The opponent's case must be the
  strongest version, not a caricature easy to dismiss. If the
  output is easy to refute, the skill failed.
- Disclose the source class for every supporting reason. An
  unsourced counter is suspect.
- Avoid "on the other hand" hedging — commit to the counter as
  if it were the right answer; the operator's job is to decide.
