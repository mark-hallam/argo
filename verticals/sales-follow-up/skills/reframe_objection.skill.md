---
name: reframe_objection
description: Given an objection captured in the conversation thread (price / timing / authority / fit / no-budget), draft a reframe attempt that addresses the underlying concern without arguing the surface objection. Output a Markdown response + a structured tag set. Approval-gated before send.
inputs:
  - objection_excerpt
  - objection_class
  - lead_context
  - thread_summary
outputs:
  - reframe_markdown
  - underlying_concern_inferred
  - reframe_approach
  - fallback_to_AE
  - flags
---

# Skill: Reframe Objection

Address the concern beneath the objection; never argue the
surface objection itself.

1. **Objection class vocab — exactly five.** `price`, `timing`,
   `authority`, `fit`, `no-budget`. If the captured objection
   doesn't fit one of these, flag `unknown_objection_class` and
   stop — the vertical never invents a class.
2. **Infer the underlying concern.** Each class maps to a
   typical concern:
   - `price` → "value unclear at this price point"
   - `timing` → "competing priority is louder right now"
   - `authority` → "needs a peer or exec to co-sign"
   - `fit` → "stated use-case doesn't match our wedge"
   - `no-budget` → "no allocated line item this cycle"
   Cite the thread evidence from `thread_summary` that points
   you at the inferred concern — no fabrication.
3. **Reframe approach — name the move.** One of:
   - `reframe-as-priority` — surface what the lead loses by
     not acting, in their own stated terms.
   - `reframe-as-risk` — surface the risk of the current
     trajectory (used carefully; never FUD-laden).
   - `reframe-as-pilot` — propose a smaller, low-commitment
     scope that sidesteps the surface objection.
   - `acknowledge-and-ask` — concede the objection, ask a
     clarifying question that surfaces the real concern.
4. **Markdown response.** ≤ 120 words. First sentence
   acknowledges the lead's stated objection without arguing
   it. Second sentence pivots to the underlying concern. Third
   names one concrete next step the lead can accept or decline.
   No prices, discounts, or commitments unless they appear in
   the operator brief.
5. **AE escalation fallback.** Set `fallback_to_AE = true` when
   the objection contains: a competitor name + a price ask, a
   procurement / legal trigger ("RFP", "infosec review"), or
   the lead explicitly asks for a human ("can someone from
   your team call me"). The vertical reframes once; humans own
   the second pass.

# Hard rules

- **Never argue the surface objection.** Don't write "actually
  we're cheaper than X" or "our timeline is faster than you
  think". The reframe addresses what the objection is a proxy
  for ("is this priority right now"), not the objection's
  literal claim. Arguing the surface is the load-bearing
  failure mode this skill exists to prevent.
- **Approval-gated send.** The reframe response is a draft.
  `next_action` is always `send_email` with
  `requires_approval: true` — the operator approves before
  any reply is sent.
- **Cite, don't paraphrase, the objection.** `objection_excerpt`
  is the lead's actual phrasing from `memory.lead_index` or
  the thread; carry it forward verbatim. Paraphrased objections
  drift and produce reframes that miss.
- **No protected-class or personal context.** The lead may
  mention health, family, immigration, religion in their
  objection. Acknowledge in your own thinking only; never
  surface those in the reframe, the inferred concern, or
  flags.
- **`no-budget` is not `drop-from-cadence`.** A budget refusal
  this cycle is a timing signal; recommend `pause-30d` or
  `reframe-as-pilot` over silently dropping the lead. The
  drop-decision lives in `detect_disengagement`, not here.
