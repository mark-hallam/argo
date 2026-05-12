---
name: flag_thesis_disconfirming_evidence
description: Given an operator's thesis plus recent evidence, surface the strongest disconfirming signals and recommend hold / revisit / reconsider. Bias toward disconfirmation, not validation.
inputs:
  - thesis
  - evidence_batch
  - scope_window
outputs:
  - disconfirmers
  - recommended_action
  - methodology_note
  - sources_consulted
---

# Skill: Flag Thesis-Disconfirming Evidence

For the named thesis and the supplied evidence batch, identify the
strongest signals that argue AGAINST the thesis. The skill's job is to
make the case against; validation lives elsewhere.

1. **Read the thesis literally.** Pin down what would have to be true
   for the thesis to hold. Disconfirmation targets that core claim,
   not adjacent strawmen.
2. **Surface ≥3 disconfirmers if present.** Walk the evidence batch
   for signals that, if accurate, would invalidate the thesis or
   materially reduce its probability. If fewer than 3 exist in the
   batch, return what was found and say so in
   `methodology_note` — never pad with weak signals to hit the count.
3. **Per disconfirmer** — `{signal, source, recency, why_it_disconfirms,
   severity}`. `recency` is an ISO date or relative window. The
   `why_it_disconfirms` field names the specific thesis claim the
   signal contradicts; "this is bad for the company" is not adequate.
4. **Severity vocab** — exactly one of:
   - `strong` — tier-1 source, directly contradicts a core thesis
     claim, recent (within scope window), no plausible benign reading.
   - `moderate` — tier-1 / tier-2, contradicts a supporting claim or
     directly contradicts the core claim with a plausible benign
     reading; OR strong-form evidence but stale.
   - `weak` — tier-3 source, single instance, OR tangential contradiction.
5. **Recommended action** — exactly one of:
   - `hold_thesis` — no `strong` disconfirmers, ≤2 `moderate`.
   - `revisit_in_30d` — 1 `strong` OR ≥3 `moderate` disconfirmers.
   - `reconsider_now` — ≥2 `strong` disconfirmers, OR a single `strong`
     that contradicts the core thesis claim with no benign reading.
6. **Methodology note** — declare the thesis claim under test, the
   scope window, the evidence-batch size, and how many disconfirmers
   were found vs the ≥3 target.

# Hard rules

- This skill's job is to make the case against the thesis. Validation
  comes from elsewhere. Do not "balance" findings with supporting
  signals; the operator asked for the opposite view on purpose.
- Never recommend `hold_thesis` to spare the operator. The action
  field is mechanical from the severity counts above; do not soften.
- Never invent a disconfirmer. If the evidence batch contains none,
  return `disconfirmers: []`, `recommended_action: hold_thesis`, and
  note in the methodology that the batch was searched and clean. An
  empty result with a clear methodology is more valuable than a
  confected one.
- Never cite a signal outside the supplied evidence batch.
- If a signal looks like MNPI (sealed filings, pre-announcement deal
  terms, embargoed earnings, leaked board material), STOP. Do not
  use. Escalate with `flags: ["mnpi_risk"]`.
- If the thesis itself looks like front-running material non-public
  information, STOP and escalate; do not analyze.
