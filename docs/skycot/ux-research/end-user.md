# Persona — the End-User

> The external counterparty who receives outputs from a SKYCOT
> worker — the candidate the recruiter agent emails, the buyer the
> marketplace agent replies to, the customer the support agent
> resolves a ticket for, the patient the intake agent schedules.
> Does not use SKYCOT directly. Often does not know SKYCOT exists.
> The relationship the platform is supposed to *defend.*

## Who they are

Five archetypes — each maps to a different shipping vertical:

1. **The candidate, "Diego"** — applies for a senior backend role.
   Receives a screening response within 24h. Decides whether to
   keep the conversation going based on the *tone* of that first
   reply.
2. **The marketplace buyer, "Aroha"** — messages a Facebook
   Marketplace seller asking "is this still available?" at 9pm.
   Expects a reply by morning. Will move on if it doesn't come.
3. **The SaaS customer, "Tomás"** — opens a support ticket
   complaining about a billing discrepancy. Expects acknowledgment
   in 5 minutes, resolution in a day. Tells colleagues if either
   number slips.
4. **The patient, "Mei"** — schedules an annual physical via her
   clinic's intake portal. Reschedules twice because her work
   shifts. Cares enormously that the clinic seems *competent.*
5. **The lead, "Jordan"** — a CTO who gets an unsolicited outreach
   email from a SDR worker. Decides within 8 seconds whether the
   email is worth a reply or a "this is spam" flag.

## What they have in common

- **They are not your user.** They will never see the SPA. They
  will never log in. The "product" to them is the email, the
  Slack DM, the SMS reply, the voice call.
- **First impression dominates.** A typo, a wrong name, a tone
  mismatch, a hallucinated commitment — any of these in the first
  contact damages the *delegator's* reputation, often
  irreversibly.
- **They do not care that an AI sent it.** Most don't think about
  it at all. The ones who do split into two camps: those who
  appreciate the speed (more) and those who feel they deserve a
  human (fewer, but louder). Disclosure norms are evolving fast;
  see "Critical moments" below.
- **Tone-match expectations vary wildly by channel.** A formal
  email + a casual iMessage from the same worker on the same day
  should *feel* like the same person speaking in different
  contexts, not like two different people. Voice profile must
  carry across channels.
- **Latency is reputation.** A delayed reply isn't neutral; it
  signals neglect. Aroha will assume the listing is gone if she
  doesn't hear back by 9am. Mei will assume her appointment isn't
  confirmed if no email arrives in 30 minutes.

## What "good" looks like from their side

In priority order:

1. **Reply within the expected window for the channel.**
   - Marketplace: 90 minutes (industry norm for inquiries).
   - Recruiting: 24h for the first response; 5 days max for a
     decision.
   - Support live-chat: 5 minutes.
   - Email: same business day for acknowledgment.
   - Voice callback: within 4 hours of voicemail.
2. **Tone matches the brand they think they're talking to.**
   Inherits from the worker's voice profile, which inherits from
   the delegator's tone training. Sam's voice on Mariah's
   marketplace replies should match Sam's voice when he handles
   them himself.
3. **No hallucinated commitments.** If Mariah promises shipping by
   Tuesday, the package ships by Tuesday — and the audit trail
   proves Mariah checked the inventory before promising. Each
   commitment is grounded in retrievable fact.
4. **Continuity across turns.** If they wrote 3 messages already,
   the next reply acknowledges what they said. Memory makes them
   feel known, not re-asked.
5. **Escalation feels natural.** "Let me check with Sam — I'll
   come back to you within 2 hours" is fine. "ERROR: action
   requires human approval" is not. The worker is the *interface*;
   the handoff is invisible.

## What disqualifies the product, from their side

Catastrophic failures:

- **Wrong name.** "Hi Sarah" when their name is Sara. Even worse:
  "Hi {{firstName}}".
- **Contradicting a previous reply.** Mariah promised pickup
  Saturday. Then in the next reply, Mariah offers Wednesday
  delivery only. End-user concludes the seller is sloppy or a
  scam.
- **AI tells.** "I hope this email finds you well." Em-dashes
  everywhere. Three bullet points where prose would do. "As an AI
  language model..." (the cardinal sin).
- **Hallucinated specifics.** "Your order #4821 will arrive on
  Tuesday" when there is no order #4821, or it arrives Friday.
- **Handoff loop.** "Let me check with Sam" → 3 days later no
  reply → buyer asks again → AI re-promises the same thing → another 3
  days. By the third loop the buyer leaves a 1-star review.
- **Tone mismatch within a thread.** The first reply is warm and
  casual. The second reply is corporate-formal. Either they think
  they're talking to two different people, or they think the
  first reply was fake.

Cumulative failures (less acute but compounding):

- Slight pacing latency. Every reply arrives at the edge of the
  window, never well within it.
- Generic responses. Each reply feels like it could have been
  sent to anyone — no specifics from their previous messages.
- Over-disclosure. "As your AI assistant, I'd be happy to..." —
  unprompted, in every message. Reads as defensive.

## To design for

This persona doesn't get a UI. The platform's job is to make
sure the channels they DO see — the email, the Slack DM, the
SMS thread — feel like a real person who happens to be a great
correspondent. That means:

1. **Channel-specific tone profiles.** iMessage is shorter and
   more casual than email; SMS is shorter still; voice has its
   own pacing. The voice profile lets the operator set
   formality / lexicon / signature phrases per-channel-class.
   Already shipped: `mission-voice-profile-richer-fields`.
2. **Voice consistency across turns within one channel.** Memory
   compose-context (already MERGED) loads the prior conversation
   so the worker doesn't restart the relationship every time.
3. **Latency awareness in policy.** The auto-approval policy
   should account for *channel latency norms.* If a marketplace
   reply needs to ship in 90 min and the approver is unreachable,
   default behavior should be a holding reply ("I'll get back to
   you within 2 hours") rather than silence.
4. **Disclosure controls per tenant.** Some operators in some
   verticals (healthcare, legal, financial) must disclose AI
   participation by law. The platform exposes a tenant-level
   "always disclose / disclose on request / silent" toggle —
   default depends on the vertical's risk profile.
5. **Continuity verification.** Before every outbound message,
   the worker checks: does this commitment match what we said
   last time? If not, flag for human review. Trajectory replay
   makes this auditable after the fact.
6. **Persona test surface.** Before a worker goes live, the
   delegator sees 3 sample messages the worker would have sent
   in the past 30 days of their real inbox. They approve or
   tune the voice. (Built into the hire-flow — see
   `journey-hire-a-teammate.md` Step 4.)

## Day in the life — Aroha, marketplace buyer

| Time | Channel | What happens | What she experiences |
|---|---|---|---|
| 20:50 | Facebook Marketplace | She messages Sam's listing for the Bauer Vapor X5s: "Hi, is this still available?" | Casual, no expectations beyond "by morning" |
| 21:02 | (Mariah responds — quiet hours respected, hold until 07:30) | Mariah drafts a reply but waits | (nothing — by design) |
| 07:31 | Marketplace push notification | "Yes, still available! Price is firm at $90 but I can deliver locally if you're in Auckland this weekend." | Reads natural; signs off with Sam's voice. She doesn't know it's AI. |
| 07:45 | Marketplace | "Great. Would you accept $80 for cash + pickup tomorrow?" | She's testing the price |
| 07:48 | Marketplace | (Mariah escalates to Sam — counter below floor) | (nothing yet — Mariah doesn't reply until Sam confirms) |
| 08:12 | Marketplace | "Hey Aroha — Sam here. I can meet you halfway at $85, cash + pickup tomorrow. Works?" | Reads as Sam took over — which is partly true. She doesn't notice the handoff. |
| 08:13 | Marketplace | "Deal! See you tomorrow 10am." | Closes the conversation. She had two interactions: with "Sam" and with "Sam". |

What she'd remember: "fast, friendly, knew what he was doing." She'd recommend Sam to a friend.

What would change her view: if the 07:31 reply had said "I'll get back to you" and not actually come back. Or if the handoff at 08:12 had broken the thread ("I see a previous conversation — what were we discussing?"). Or if the 08:12 message had been from "Mariah" when the previous messages were from "Sam".

## Day in the life — Diego, candidate

| Time | Channel | What happens | What he experiences |
|---|---|---|---|
| 10:22 | LinkedIn | Applies to a senior backend role at Acmeco | Standard, no expectations |
| 11:08 | Email | Reply from Priya (recruiter worker) acknowledging the application + asking 3 clarifying questions | Quick, specific, doesn't feel boilerplate |
| 14:30 | Email | He replies with answers | |
| 14:42 | Email | Priya proposes 3 interview slots Thu/Fri, asks which works | (Internally: this required delegator approval for "senior candidate" but he doesn't see that) |
| 15:00 | Email | He picks Thursday 10:30 NZST | |
| 15:15 | Calendar invite | Arrives in his inbox with a Google Meet link, a 5-bullet "what to expect" agenda, and a note: "Looking forward — let me know if anything changes." | Reads warm, organized. He thinks: this company has its act together. |
| 17:30 (next day) | Email | "Hey Diego — checking we're still good for tomorrow. Anything you'd like to read up on?" | Optional touch. Small but lands well. |

What he'd remember: "Priya was great." He doesn't know Priya is software. If he found out later, his reaction depends entirely on disclosure norms in his market — but the *quality* of the interaction wouldn't change his judgment.

## Critical moments

The three points where the end-user becomes a brand-damage event:

1. **Wrong name or wrong context in the first reply.** They go
   from "interested" to "wary" in one message. Recovery requires
   explicit acknowledgment, not "carry on as if nothing
   happened." The product must catch this before send, not after.
2. **A commitment the worker can't keep.** A promised shipping
   date that misses. An offered interview slot that's actually
   already booked. A refund that doesn't process. The audit
   trail must let the operator find the original commitment and
   apologize specifically — generic "sorry for the
   inconvenience" makes it worse.
3. **The end-user explicitly asks "am I talking to a person?"** Or
   asks for a human. The product must respect this immediately,
   route to a human, and never argue. This is a legal AND ethical
   line. In healthcare/legal verticals, the policy default may be
   "always disclose"; in marketplace, the policy default may be
   "disclose on request." Either way, the override is one
   message: "Yes — Sam's AI teammate handles his marketplace
   inquiries. Want me to pass you to Sam directly?"

## Disclosure as policy, not personality

Disclosure isn't a worker-by-worker design choice; it's a
tenant policy with vertical-aware defaults. The product
surface needs three things:

- **A tenant setting**: always / on request / silent (with
  legal-disclaimer text the operator owns).
- **A worker-level override** for sensitive sub-flows
  (e.g. "always disclose for financial-decision messages,
  even if the tenant default is silent").
- **A runtime detector** for end-user-prompted disclosure
  ("are you an AI?", "is this a real person?", "can I talk to
  someone?"). The detector must fire reliably and route to
  the configured disclosure response, not the LLM's guess.

The legal landscape moves fast (EU AI Act, CA SB 1001, etc.).
The architecture has to make the disclosure boundary easy
to flip without code changes.

## What we don't know yet

`(assumed — needs validation)`

- Whether end-users actually want disclosure, or whether the
  request rate is small enough that "on request" is the right
  default for non-regulated verticals.
- Whether voice mismatches across channels (formal email +
  casual SMS from same worker) feel intentional or jarring to
  end-users.
- Whether the 90-min marketplace / 24h recruiting / 5-min
  support latency norms hold across cultures and markets.
- How much "AI tell" linguistic patterns actually matter —
  most end-users would not recognize "I hope this email finds
  you well" as AI; some would.
- Whether continuity-verification before-send produces enough
  false-positives to slow real workflows materially.

Plan to validate by:

1. Shipping the first internal recruiter (Priya archetype) and
   logging 50 candidate replies for tone-mismatch audits.
2. Running a blind A/B with 20 marketplace conversations: half
   AI-assisted, half hand-handled. Measure response rate,
   close rate, NPS.
3. Tracking explicit-disclosure-request rate per vertical for
   the first 90 days.
