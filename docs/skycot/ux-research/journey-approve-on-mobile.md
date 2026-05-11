# Journey — Approve on Mobile

> The 10-second decision on a phone, between meetings, in line at
> the coffee shop. The highest-frequency touchpoint of the entire
> product. The reason mission claim #6 exists. If this journey
> doesn't work, the product doesn't work — every other surface
> compounds onto this one being smooth.

## The promise

> "Your AI teammate asks. You answer in one tap, from wherever
> you are, on whatever channel you prefer. The answer sticks.
> You can change your mind for 30 seconds. You will never have
> to explain yourself twice."

## Actors

- **The Approver** (primary). See [approver.md](approver.md).
  Usually the Delegator on mobile in a different context than
  policy-setting; sometimes a different person entirely.
- **The Worker** asking the question.
- **The Channel** (Slack / iMessage / SMS / Push / WhatsApp /
  Voice / Watch). The Approver does not know — and should not
  need to know — that this is a channel; from their side, it's
  *the way Mariah asks them*.

## Trigger

A worker reaches a decision point that policy says requires
human approval. Could be:

- An action that's never been seen before (no policy match).
- An action above an explicit threshold (e.g. price floor, dollar
  amount).
- An action where the worker's confidence is below a threshold
  (e.g. "I'm 60% sure this is the right response — please confirm").
- An action where the Approver explicitly said "always ask me
  about X."

## The card — the load-bearing artifact

Whatever channel the Approver is on, they see ONE artifact: an
**approval card**. The card is the contract. Get this right and
the rest follows.

Structure (mandatory, in this order, mobile-first):

```
┌─────────────────────────────────────┐
│ [avatar]  Mariah                    │
│ wants to send this listing          │   ← who + verb, 1 line
├─────────────────────────────────────┤
│ Bauer Vapor X5, used 2x · $90      │   ← the thing, glanceable
│ Reply to buyer: "Yes still avail-  │
│ able, $90 firm. Pickup in Auckland │
│ this weekend."                     │
├─────────────────────────────────────┤
│ If YES → goes to FB Marketplace,   │   ← consequence
│ visible to buyer in 30s.           │
│ If NO → I'll wait for your direc-  │
│ tion.                              │
├─────────────────────────────────────┤
│ [ Approve ]  [ Refuse ]  [ Why? ]  │   ← three actions max
└─────────────────────────────────────┘
```

That is the entire card. No expandable sections, no tabs, no
"more options". The "Why?" tap is the *only* progressive
disclosure — and it opens to a single sentence: *"I'm asking
because you said 'always check counter-offers' on May 4th."*

## The journey — channel-by-channel

The card *renders* differently across channels, but its
**semantics** are identical. The Approver's experience is the
same; the platform handles the channel-specific surface.

### Slack

Native Slack message with `attachments[].actions` for the
buttons. Buttons reply to the gateway via Slack's actions
webhook (`mission-wire-slack-approval-roundtrip` MERGED).

Time-to-decide: ~5s if Slack is already open; ~10s if Slack
needs to wake up.

### iMessage (BlueBubbles)

Tap-back reactions: 👍 = approve, 👎 = refuse. The card body
arrives as a single iMessage with all four lines of the
structure above. "Why?" is a follow-up message they request
with the reaction 🤔.

Time-to-decide: 3s. iMessage is the fastest channel for the
Approver — no app to open, no link to tap.

### Push notification (PWA or native app, future)

Action buttons on the lock screen: `Approve` `Refuse`. The
notification body holds the card structure compressed to fit
the OS notification limits (~4 lines). Tap-to-expand reveals
the full card without opening the app. Tap an action button
without opening anything.

Time-to-decide: 4s.

### SMS

Plain-text card. Reply `Y` to approve, `N` to refuse, `?` for
why. (Twilio shipped in `mission-notify-channels-rest-of-pack`
#957.)

Time-to-decide: 7s — fastest channel for the Approver-who-is-
driving (as a passenger) or-whose-phone-is-locked.

### Voice (Twilio)

Outbound call with TwiML reading the card aloud. DTMF: `1`
approve, `2` refuse, `9` details. (Shipped in #896.)

Time-to-decide: 20s. Voice is the longest channel by far;
reserved for truly high-stakes approvals or accessibility cases.
NOT the default.

### Apple Watch glance (future)

Two-button Watch face: `Approve` `Refuse`, with the card body
as the complication text. Tap-and-hold for "Why?".

Time-to-decide: 2s. The lightest-weight channel of all; only
appropriate for the Approver who has consented to wearable
notifications.

## The decision

Whichever channel the Approver responded on:

1. **The platform records the decision** with attribution
   (approver identity, channel, timestamp, HMAC token where
   relevant — already wired for Slack / iMessage / Twilio).
2. **The worker proceeds** within ~1 second of decision
   receipt. If the worker is no longer running (action timed
   out, session ended), the decision is logged but no-op;
   the audit trail shows "decision recorded post-timeout."
3. **The 30-second undo window opens**, surfaced as a
   countdown badge on the channel the decision arrived on
   (Slack: a green "approved — tap to undo" replacement;
   iMessage: a follow-up "approved · 30s to undo"). Tapping
   undo within the window rolls back the action — or, if the
   action has already executed, queues a compensating action.
4. **The decision is idempotent.** If the Approver replies on
   Slack AND taps a push notification (because both arrived
   before they decided), only the first applies; the second
   surfaces "already decided" without re-running.

## What can go wrong

| Risk | Mitigation |
|---|---|
| Approver taps NO when they meant YES (mobile, fat thumb) | 30-second undo window; optimistic action execution gated on countdown elapsing. |
| Approver does not respond within the worker's timeout window | Policy-driven fallback: escalate to next approver in chain, or default to refuse-then-park. NOT default-to-approve. |
| Channel fails to deliver (Slack outage, iMessage to non-Apple recipient) | Fall through to next channel in the worker's channel-preference list (`approval-channel-fallback-chain` brief exists in streams/). |
| Approver receives the same card on 3 channels and they all show up at once | Idempotent decision recording; first response wins; other channels self-update to "already decided." Should be silent / not buzz again. |
| Approver is in a different timezone than they set quiet-hours for (traveling) | Quiet-hours respects the *approver's* current zone if their channel carries it (iMessage / push do, SMS does not). Fallback: respect the policy zone and let them override per-session. |
| The card body contains PII the Approver shouldn't see (e.g. a co-worker approving a customer-data action) | Redact at card-render time based on approver's role + the action's data classification. Approver sees "Mariah wants to update customer [ID 4821]'s shipping address" not the full PII. |
| The "Why?" answer leaks policy details the Approver shouldn't know | "Why?" surfaces the policy *name* and the *trigger*, not the full policy body. Full policy body lives in the Operator console. |

## Critical moments

The three points where the Approver disengages permanently:

1. **First card that takes > 10 seconds to read.** They'll
   approve without reading the next one. Then the one after.
   Then they're rubber-stamping. The product's safety model
   is now decorative.
2. **First wrong-button tap with no undo.** They tell the
   Delegator to "just let it run on its own." Now the approval
   surface is decorative.
3. **First card from a worker on a channel they didn't opt into.**
   They mute the channel. Now we've lost the channel for every
   future card.

The product's job is to make these three moments structurally
impossible.

## Success metrics for this journey

- **Median time-to-decide**: ≤ 8 seconds across channels.
  (Voice is the outlier and excluded.)
- **No-response rate**: ≤ 5% of cards. Higher means we're
  asking too often or via channels they're not on.
- **Undo rate**: ≤ 2% of approvals. Higher means cards aren't
  clear enough or buttons aren't far enough apart.
- **Per-channel parity check**: approval rate by channel
  should be within 10% of each other for the same action type.
  If Slack approvals are 90% and SMS approvals are 60% for the
  same kind of ask, the SMS card has a bug.
- **"Why?" tap rate**: 5–20% is healthy. 0% means we're not
  trusted to be asked; > 30% means the cards aren't
  self-explanatory.

## Cross-channel invariants

These hold regardless of channel:

- **One decision, one action.** Idempotency across channel-
  fanout.
- **Same context.** The card body is byte-identical across
  channels (modulo channel-imposed length limits).
- **Same audit trail.** Decision + approver + channel +
  timestamp + HMAC, recorded once.
- **Same undo.** 30s window applies regardless of channel.
- **Same redaction.** PII handling is at the card layer, not
  per-channel.

## Dependencies on existing platform pieces

All MERGED unless noted:

- Slack approval roundtrip: `mission-wire-slack-approval-
  roundtrip`.
- iMessage signed tokens: shipped.
- SMS/Voice/WhatsApp Twilio: `mission-notify-channels-rest-of-
  pack` + #896 + #913.
- WebInbox approval state machine: shipped.
- Quiet hours across channels: `mission-quiet-hours-channels`.
- Approval-channel fallback chain: brief exists in streams/,
  status unclear — verify before implementation.
- **Missing for this journey**: the rendered card design across
  channels; the unified Approvals Inbox view (web, when the
  Approver does open the app to see history); the undo-window
  mechanic and its rollback path.

## Open questions

`(assumed — needs validation)`

- Is the 30-second undo window the right duration? Could be
  10s (faster execution) or 60s (more forgiving).
- Does iMessage's tap-back reaction model actually work for
  real Approvers, or do they expect a text reply?
- Is the "Why?" third button worth the visual weight, or is
  it noise for 95% of cards?
- Does the post-decision badge on the channel (the "approved
  · 30s to undo" line) confuse the Approver into thinking the
  decision is pending?
- For voice — is anyone actually going to use it? Or is it
  shipped-but-dead?

Validate with 10 high-volume Approvers (mix of channels) over
a 1-week shadow before locking the card design.
