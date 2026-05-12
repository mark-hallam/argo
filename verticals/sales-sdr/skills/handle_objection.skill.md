---
name: handle_objection
version: "0.1.0"
description: "Classify an inbound prospect objection against the BANT-derived 5 (Budget/Authority/Need/Timing/Fit) and draft a measured response. AE-escalation is always available as a fallback."
provenance:
  source: vertical/sales-sdr
  hash: sha256-pending
---

# When to use

A prospect responds with friction — "too expensive", "not the
right person", "no budget this quarter", "we already have X",
"send me info and I'll get back to you". Classify, draft a
measured reply, and surface the threshold past which an Account
Executive should take over.

# Inputs you expect

- the prospect's exact objection text
- prior thread context (last 1-2 messages)
- product positioning notes (`memory.icp_rubric.positioning` if set)
- operator's AE handle (for the escalation path)

# Output shape

A single fenced JSON block:

```json
{
  "requires_approval": true,
  "objection_class": "Budget|Authority|Need|Timing|Fit",
  "objection_class_evidence": "<the phrase that triggered the class>",
  "response_draft": "<60-120 words, acknowledges + reframes + ONE specific ask>",
  "escalation_threshold": "<when to stop self-handling and loop the AE in>",
  "fallback": {
    "escalate_to_ae": true,
    "ae_handle": "<from input>",
    "trigger_phrase": "<single sentence to use when handing off>"
  }
}
```

# Rules

- **BANT + Fit are the canonical 5.** Use these exact names —
  `Budget`, `Authority`, `Need`, `Timing`, `Fit`. No synonyms,
  no nested sub-classes. If the objection genuinely spans two
  classes (common with Budget + Timing), pick the load-bearing
  one and note the other in `escalation_threshold`.
- **AE escalation is always available.** `fallback.escalate_to_ae`
  is structurally `true` — the SDR never owns a deal past the
  qualified-conversation stage. Even on a clean self-handle, the
  fallback path stays present.
- **One ask per draft.** Acknowledge → reframe → ONE specific
  next ask (a 15-min call, a one-line answer, a forward to the
  right person). Multiple asks dilute the reply.
- **Approval-gated.** `requires_approval: true` is always
  present. The operator signs off before the draft goes out.

# Examples

- **Budget objection** ("too expensive right now"): class
  `Budget`, draft acknowledges the pricing concern, reframes
  around ROI on the verified signal, asks for a 15-min scoping
  call. `escalation_threshold: "if prospect cites a hard
  procurement freeze ≥ 1 quarter, escalate to AE"`.
- **Authority objection** ("I'm not the right person"): class
  `Authority`, draft thanks them and asks for the right person's
  name + a one-line warm intro. `escalation_threshold: "if no
  intro after one follow-up, escalate to AE for direct outreach"`.
