---
name: recommend_policy_review
description: Given a cluster of related triaged employee concerns (≥3 within a 90-day window), recommend whether existing policy needs review. Output names the policy, the evidence count, and a suggested revision category (clarification / scope-expansion / new-policy / no-action). Surfaces patterns only — NEVER drafts policy text. Approval-gated before any downstream policy-doc edit propagates.
inputs:
  - triaged_concerns
  - policy_handbook
  - window_days
outputs:
  - policy_name
  - evidence_count
  - suggested_revision_category
  - rationale
  - flags
---

# Skill: Recommend Policy Review

Surface policy-shaped patterns from triaged concerns. The skill is
analysis-only: it points operators at policies that may need a
second look. It NEVER drafts policy language and NEVER routes a
policy edit — that work belongs to HR-officer + legal counsel, on a
fresh `draft_policy_clarification` call after operator review.

1. **Cluster the triaged concerns.** Group inbound concerns by the
   policy they touch (cited section in `policy_handbook`). A
   concern without a cited policy is set aside; do not invent a
   mapping. Window defaults to 90 days; honor `window_days` if
   supplied. Cluster size is the count of distinct concerns, not
   the count of comments.
2. **Apply the 3-in-90-days threshold.** Only clusters with
   `evidence_count >= 3` AND all concerns inside the rolling
   `window_days` qualify. Below-threshold clusters surface as
   `suggested_revision_category: no-action` with the count so the
   operator can monitor — do not suppress them silently; they may
   reach threshold next window.
3. **Pick a revision category.** Exactly one of:
   - `clarification` — policy exists, language is ambiguous, the
     same misreading recurs across concerns. Lowest-cost remedy.
   - `scope-expansion` — policy covers the named area but the
     pattern shows an edge case the current scope omits (e.g.
     remote-work flex policy that omits hybrid-by-quarter cases).
   - `new-policy` — the pattern is not covered by ANY existing
     handbook section; a fresh policy is warranted. Highest-cost
     remedy; reserve for genuinely uncovered ground.
   - `no-action` — the pattern is real but not policy-shaped (e.g.
     individual manager behavior), OR cluster is below threshold.
4. **Rationale.** 2-4 sentences naming the policy section, the
   recurring misreading or gap, and which category the evidence
   supports. Cite concern IDs by stable identifier — never by the
   reporter's name or role label.
5. **`next_action`** is always `share_workforce_data_externally`
   with `requires_approval: true` when the recommendation is
   surfaced to HR leadership; the policy-draft step itself is a
   separate operator-routed call to `draft_policy_clarification`.

Hard rules:

- **Never auto-draft policy text.** This skill returns a
  recommendation envelope only. If the operator presses for draft
  language, redirect to `draft_policy_clarification` — which is
  itself approval-gated.
- **Never recommend a change that conflicts with superseding law.**
  Title VII / ADA / FMLA / FLSA / NLRA / state-equivalent. If the
  pattern would push a policy below a statutory floor, set
  `flags: ["recommendation_conflicts_with_statute"]` and escalate
  to legal counsel before surfacing.
- **No protected-class clustering without the EEO-1 gate.**
  Splitting the cluster along race / gender / age / disability /
  religion / national-origin / pregnancy / veteran-status / genetic-
  info dimensions requires `eeo_approved_roles` + EEO-1 context.
  Without both, set `flags: ["protected_class_analysis_without_
  eeo_context"]` and refuse the breakout — surface the aggregate
  pattern only.
- **Three concerns is a floor, not a target.** Do not lower the
  bar to manufacture a recommendation. A genuinely uncovered area
  with two concerns surfaces as `no-action` + watch-flag; the
  operator decides whether to fast-track.
- **Identity-redacted evidence only.** Concern IDs reference the
  triaged, named-individual-redacted summaries from
  `triage_employee_concern` — never the un-redacted originals.
- **Approval-gated propagation.** Even after operator review, the
  policy-doc edit propagates only after `requires_approval: true`
  clears HR-officer + (if regulated area) legal-counsel sign-off.
