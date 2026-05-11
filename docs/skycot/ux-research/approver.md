# Persona — the Approver

> The highest-frequency touchpoint of the entire product. Usually
> the Delegator on mobile, mid-meeting, with 10 seconds to decide.
> Sometimes a different person (a co-founder, a manager) who only
> ever sees SKYCOT through approval cards. If we lose this
> persona, we lose mission claim #6 — and with it, the entire
> "AI workers humans can bring into their team" promise.

## Who they are

Most often: the Delegator (see [delegator.md](delegator.md)) on
their phone, in a different context than when they set the policy.

Sometimes a different person entirely:

- **The co-founder Approver.** Sam runs the storefronts; his
  business partner Lena handles money. Mariah asks Sam for
  listing approvals but routes price-floor decisions to Lena.
  Lena has never opened the SKYCOT app — she sees the product
  ONLY through Slack messages with approve/deny buttons.
- **The team-lead Approver.** Marcus delegates a worker to his
  support manager Jamal, who approves on behalf of the team.
  Jamal touches SKYCOT 40 times a day, exclusively as approver.

## What they have in common

- **Always in motion.** Phone, between meetings, walking, in a
  car (passenger or stopped). Rarely seated at a desk.
- **Limited context at decision time.** They saw the policy 2
  weeks ago. They do NOT remember what they decided then. The
  approval card has to re-supply context every time.
- **Decision time budget: 10 seconds.** If the card takes
  longer than that to read + decide, they will either (a)
  approve everything without reading, or (b) defer and forget.
  Both failure modes are catastrophic.
- **No tolerance for re-asking.** If they hit "approve" and the
  app says "are you sure?" — they will tell their team to turn
  this thing off. Modals are death.
- **Mobile-first, often phone-only.** Designing for desktop and
  then "responsive-ing" to mobile produces an unusable approval
  surface. Design for the lock-screen notification first, the
  watch widget second, the phone-app screen third, the desktop
  fourth — in that order.

## What they need

1. **Full context in one card.** Who's asking (which worker),
   what they want to do (one sentence), why now (the trigger),
   what happens if YES (one sentence), what happens if NO
   (one sentence). No tab-switching, no expanding sections.
2. **One-tap decision from the notification itself.** Approve /
   refuse / "details" buttons on the push notification, on the
   Slack message, on the SMS reply, on the iMessage tap-back —
   never "open the app to decide".
3. **An undo window.** They will tap the wrong button at least
   twice a week. They need 30 seconds to reverse the decision
   before the action executes. Optimistic UI with a quiet
   countdown badge is fine.
4. **Trust the worker more over time.** If they've approved
   "draft a marketplace reply" 50 times this month, the 51st
   should escalate to autonomy ("Mariah will start handling
   these without asking — say 'wait, not yet' in the next 24h
   to override"). The product should ratchet trust automatically,
   not require Lena to find a settings page.
5. **A "why am I getting this?" answer in one tap.** If Lena
   gets pinged about a price decision, she needs to be able to
   ask "wait, who set up this rule?" and see the answer
   immediately. Otherwise she resents the AI for interrupting
   her.

## What they do NOT want

- A web app they have to log into.
- A modal that says "are you sure?".
- A push notification that says "tap here to review" (the
  decision must live IN the notification).
- 4 buttons. Two buttons (approve / refuse) plus a "details" tap
  is the max.
- To be the QA team. If they're saying NO more than 5% of the
  time, the worker's policy is wrong, not the approver's job.
- To have the same conversation twice. "Mariah, never list at
  below $50" said once should hold forever.

## Anti-patterns to watch for in design

- **Approval requests that read like config diffs.** "Worker mariah-1
  proposes action listing.send with payload {…}" is unparseable
  on a phone. Translate to plain English at the approval-card
  layer: "Mariah wants to send this listing to Facebook Marketplace
  at $90."
- **Burying the consequence.** The single most important sentence
  on the card is "if you say yes, X happens." Lead with it.
- **One approval card per action, no batching.** When the worker
  proposes 8 similar actions in a row, the approver should be
  able to "approve all 8" in one tap, with a quick swipe-through
  to spot-check. The card-per-action pattern was an early-iteration
  mistake; batching is required for the high-frequency persona
  to survive.
- **Treating the approval surface as the same product as the
  delegator's web app.** The approval surface is its own product.
  It might be a Slack bot, an iMessage thread, a PWA notification,
  an Apple Watch glance. None of those are "your SaaS app rendered
  small."
- **Showing the worker's name and persona only in the message
  header, not the card body.** The approver needs to feel they're
  saying yes to *Mariah*, not to *SKYCOT*. The teammate metaphor
  lives or dies here.

## Day in the life — Lena, co-founder Approver

| Time | Channel | What she sees | What she does | Time spent |
|---|---|---|---|---|
| 09:00 | Slack notification | "Mariah wants to counter-offer at $85 (buyer asked $80, your floor is $75). Reply 1 to approve, 2 to refuse." | Taps `1` in the Slack thread without opening the app | 4s |
| 10:30 | iMessage tap-back | "Mariah: list 'Bauer Vapor X5, used 2x, $120' on Facebook Marketplace?" with thumbs-up / thumbs-down reactions | Hits thumbs-up | 3s |
| 12:15 | Apple Watch glance | "Mariah needs a $200 price-floor override approval" with Approve / Refuse buttons | Refuses | 2s |
| 14:00 | SMS while driving (passenger) | "Mariah: 6 similar listings to send. Reply Y to approve all, N to review each." | Replies `Y` | 5s |
| 16:45 | Push notification on phone | "Mariah found a buyer for the Vapor X5 you listed Mon. Buyer asked: 'Will you ship to UK?'. Mariah suggests: 'Yes, $25 shipping, 5–7 days.' Approve?" | Taps Approve on the lock screen | 4s |
| 22:00 | (would normally fire) | Quiet hours active — no notification. Approval moves to morning queue with a green "you can review with breakfast" indicator. | (nothing) | 0s |

Total touch time today: ~18 seconds across 5 decisions.

## Critical moments

Three points where Lena disengages:

1. **An approval card she can't parse in 5 seconds.** The next
   one will be approved without reading. Then the one after.
   Then the model decays into rubber-stamping. Every card is a
   contract with her attention.
2. **A wrong-button tap with no undo.** First time she taps NO
   when she meant YES and the worker can't recover, she
   permanently distrusts the approval flow and asks Sam to
   "just let it run on its own". Now we've lost the safety
   model.
3. **An interruption during her quiet hours.** First time SKYCOT
   pings her at 23:00, she mutes the entire channel. Now we've
   lost the channel.

## Cross-channel parity

Lena does not know — and should not need to know — that her
approvals fan out across Slack, iMessage, SMS, push, Watch. From
her perspective: *Mariah asks me; I answer; the answer sticks.*

The product must guarantee:

- Same decision applied however she answers (Slack reply vs SMS
  reply vs Watch tap vs push action).
- Decision is idempotent — if she approves on Slack AND on push
  (because both notifications arrived), the action runs once.
- Decision is auditable to a single human even when it arrived
  via a channel that doesn't natively carry identity (SMS-from-
  approved-number, signed iMessage token).

The platform already has the wiring for this (`mission-wire-
slack-approval-roundtrip` MERGED, iMessage signed tokens MERGED,
SMS+voice+whatsapp wired). The UX layer just has to honor it.

## What we don't know yet

`(assumed — needs validation)`

- Whether the 30-second undo window is too long (mariah's already
  shipped the message) or too short (lena was still typing her
  override).
- Whether Apple Watch is a real channel or vanity (true approver
  volume on Watch glance vs phone push).
- Whether "approve all 8" batching introduces over-approval risk
  in practice.
- Whether the trust-ratchet ("you've approved this 50 times,
  going autonomous") feels reassuring or alarming.

Plan to shadow 3 high-volume Approvers (one Slack-only, one
iMessage-heavy, one SMS-only) for a half-day each before locking
the approval-card design.
