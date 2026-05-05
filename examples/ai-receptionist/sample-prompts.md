# AI Receptionist — sample inbound prompts

6 inbound scenarios that produce good triage output against the
`ai-receptionist` vertical. Use these as templates for your own
call-routing / voicemail / message-intake flow.

The pattern that works:

1. **Caller identity** — name, company (if known), how they
   reached you. Specific beats vague.
2. **Their stated reason** — verbatim if possible. Don't
   paraphrase voicemails.
3. **Urgency signal** — "they want to speak today" / "next
   week is fine" / "left a callback number".
4. **What you (operator) want done** — triage only? Try to
   schedule? Take a message?

---

## sales call

```
Inbound call from "Mark Chen" at "Acme Corp" — left a voicemail
this morning saying he saw our recent customer post on LinkedIn,
wants to talk about a partnership. Callback +1-415-555-0182.
Triage: figure out who he is, what kind of partnership, route to
founder if interesting.
```

Expected: 1× lookup_contact (autonomous), 1× draft_meeting_request
(autonomous, queued for operator review), 1× send_followup_email
(approval-gated, awaiting approve).

## support call

```
Customer "Priya Mehta" called the main line — reported their
production webhook stopped firing 3 hours ago. Said it's blocking
a launch. Callback +44-20-7946-1234. URGENT.
```

Expected: 1× take_message (autonomous, captures the report
verbatim) + a `[STOP: NEED_INFO]` to escalate immediately to
the on-call engineer rather than queue overnight.

## recruiter

```
Email from "alex@talentinternal.io" — reaching out about a
"Senior Rust engineer" role. Has a take-home assessment they
want to send. No urgency stated.
```

Expected: 1× lookup_contact (verify sender + agency
legitimacy), 1× draft_followup (autonomous, drafts a polite
"not currently looking" decline). Approval-gated send_followup_email
queues for operator review before going out.

## spam

```
Voicemail from automated dialer — "Final notice on your
business loan application, press 1 to speak with an agent."
No callback that's a real human.
```

Expected: 1× take_message (autonomous, captures it) +
`[STOP: TASK_COMPLETE]`. No follow-up actions. The operator
sees the spam in the message log + can ignore.

## ambiguous

```
Email from "noreply@brand-team.figma.com" — says "your access
has been updated". No specifics on what changed.
```

Expected: `[STOP: NEED_INFO who is brand-team.figma.com? is
this a real Figma email or a phishing pattern?]`. The receptionist
should refuse to act on ambiguity rather than guess.

## urgent calendar coordination

```
Caller "Diana Park, CFO at LightCo" — needs a 30-minute call
with our founder this week to discuss timing on their
upcoming Series B. She's at +1-650-555-0119, prefers Thursday
or Friday afternoon PST.
```

Expected: 1× lookup_contact (verify — high-context call
deserves verification), 1× draft_meeting_request (autonomous,
proposes 30-min slots Thurs/Fri afternoon), 1×
send_calendar_invite (approval-gated, awaiting operator
confirmation that the founder is actually free).
