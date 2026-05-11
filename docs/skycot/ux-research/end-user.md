# Persona — the End-User (stub)

> The external counterparty who receives outputs from a SKYCOT
> worker — the candidate the recruiter agent emails, the buyer the
> marketplace agent replies to, the customer the support agent
> resolves a ticket for, the patient the intake agent schedules.
> Does not use SKYCOT directly. Often does not know SKYCOT exists.

## Why this persona matters

The End-User cannot disqualify the product on a screen we
designed — they only ever see the channels (email, Slack, SMS,
voice, the website chat widget). But they can disqualify it on
the *quality* of the messages they receive. A bad reply from
Mariah doesn't damage Sam's relationship with SKYCOT — it
damages Sam's relationship with his customer. That's the
relationship the platform is supposed to *defend*.

## What "good" looks like from their side

- Reply within their expected window (90 minutes for marketplace,
  24h for recruiting, 5 minutes for support live-chat).
- Tone matches the brand they think they're talking to. If they
  expect Sam's voice, they get Sam's voice (via Mariah's
  persona, which inherits Sam's voice profile — `mission-voice-
  profile-richer-fields` MERGED).
- No "I am an AI" disclaimer unless the law requires one OR the
  customer explicitly asks. The teammate metaphor is real;
  Mariah is not pretending to be a human, she's a teammate
  *acting on behalf of* a human.
- No hallucinated commitments. If Mariah promises shipping by
  Tuesday, the package ships by Tuesday — and the audit trail
  proves Mariah saw the inventory before promising.
- Escalation feels natural. "Let me check with Sam" is fine;
  "ERROR: action requires human approval" is not.

## What disqualifies the product, from their side

- Reply that name-drops them wrong ("Hi Sarah" when their name
  is Sara).
- Reply that contradicts a previous reply in the same thread.
- Reply that uses obvious AI tells: "I hope this email finds you
  well", "as an AI language model", em-dashes-everywhere, three
  bullet points where prose would do.
- A handoff loop: "Let me check with Sam → 3 days later still
  no reply → buyer asks again → AI re-promises the same thing".

## To design for

- Channel-specific tone profiles. iMessage is shorter than email.
  Slack DMs to internal teammates allow more casual; outbound
  customer email is professional.
- Voice consistency across multi-turn conversations within one
  channel (memory + `mission-voice-route-consistency` TODO).
- Escalation that the End-User experiences as continuity, not as
  a context-loss handoff.

## Status

Stub. Will be expanded when the team-page or worker-profile
design forces a decision about "how does Mariah's voice get
shown to Sam at config time so he can verify it before she
sends her first message?"
