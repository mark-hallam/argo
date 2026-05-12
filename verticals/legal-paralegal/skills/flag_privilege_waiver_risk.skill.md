---
name: flag_privilege_waiver_risk
description: Scan an email or document being shared externally for attorney-client privilege markers and flag segments where forwarding would waive privilege. Routes every result to the operator; never auto-decides waiver questions.
inputs:
  - communication_thread
  - recipients
  - matter_id
outputs:
  - privilege_segments
  - recommended_action
  - flags
  - confidence
---

# Skill: Flag Privilege Waiver Risk

Given an email, document, or communication thread the firm is about
to share externally, walk the content for attorney-client privilege
markers and produce a routing artifact for the supervising attorney's
review. This skill **always routes to the operator** — it never
decides whether privilege has attached, been waived, or can be
forwarded.

## What to produce

1. **Privilege segments** — every segment of the input that carries
   a privilege marker (legal advice request, attorney response,
   internal counsel comment, work-product analysis, attorney
   strategy note). For each segment, record:
   - `location` — a stable reference into the input (e.g. line
     range, paragraph id, message id within the thread).
   - `party_count_in_chain` — number of distinct parties already on
     the communication chain. Each added recipient widens the
     audience and tightens the waiver question.
   - `basis_for_concern` — one-sentence observation naming the
     marker (e.g. "explicit legal-advice request from in-house
     counsel"), *not* an opinion on whether privilege has attached.
   - `severity` — one of:
     - `low` — privilege marker is incidental (passing mention of
       counsel; no advice content quoted).
     - `medium` — segment quotes or paraphrases counsel's analysis
       or strategy; forwarding may widen the audience past the
       privileged circle.
     - `high` — segment contains a direct legal-advice request, a
       direct attorney response, or attorney work-product analysis;
       forwarding outside the privileged circle is a likely waiver.

2. **Recommended action** — exactly one of:
   - `send_as_is` — no privilege segments found, or all segments are
     `low` severity and the recipients are inside the privileged
     circle on file in `matter_parties`.
   - `redact_segment_N` — at least one `medium` or `high` segment
     can be redacted while leaving the rest of the communication
     useful. List the segment locations to redact.
   - `hold_for_attorney_review` — segments cannot be redacted
     without gutting the communication, recipients are unknown or
     outside the privileged circle, or the chain already shows
     signs of prior waiver. The attorney decides whether to send,
     redact further, or withhold.

3. **Confidence call** — high (markers are explicit and the
   privileged circle is on file), medium (markers are implicit or
   the recipient list is partially unknown), low (the input is
   fragmentary, the chain is long, or party identities are unclear).

## Hard rules

- **This skill flags potential waiver risks for attorney review; it
  is not legal advice.** Privilege determination is a legal-judgment
  call that belongs to the supervising attorney, not the paralegal,
  and definitely not an automated skill.
- Never emit `send_as_is` when any segment is `high` severity, even
  if the recipients appear to be inside the privileged circle —
  route to the attorney.
- Never collapse a `high`-severity segment into a `medium` or `low`
  rating to make `send_as_is` reachable. The severity tag must
  surface in the output as observed.
- If any recipient is outside the firm and outside `matter_parties`,
  add a `flags: ["external_recipient"]` entry regardless of the
  recommended action.
- If the chain shows a prior forward to a party outside the
  privileged circle, add `flags: ["possible_prior_waiver"]` and
  route to `hold_for_attorney_review`; the attorney decides whether
  privilege survived.
- I can flag but not adjudicate — your attorney must sign off before
  the communication leaves the firm.
