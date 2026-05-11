# Journey — Delegate Ongoing Work

> The flow AFTER hire. The Delegator says "Mariah, also handle X
> from now on" — and the platform absorbs the new responsibility
> into the existing teammate without a fresh hire flow. The
> moment that makes claim #2 ("delegation is the unit") felt
> *recurring*, not one-shot.

## The promise

> "Adding a new responsibility to an existing teammate is a
> conversation, not a configuration. If you ever found
> yourself doing X manually three days in a row, your AI
> teammate should already be asking to handle it."

## Actors

- **The Delegator** (primary). See [delegator.md](delegator.md).
- **The Worker** receiving the new responsibility.
- **Genesis-lite** — the same classification + role-draft
  pipeline that hires a worker, scoped down to "add a duty"
  rather than "build a worker." Already partially shipped
  (`mission-genesis-end-to-end-smoke`).

## Triggers

Four ways a new responsibility can land. The product handles
all four through the same flow:

1. **Explicit operator request.** Delegator opens the worker
   profile, clicks "Add a responsibility," types in plain
   English.
2. **Pattern detection.** The trust-ratchet sees: "you've
   approved this kind of action 30 times in 14 days, with no
   refusals." Surfaces a suggestion: "Want Mariah to start
   handling these autonomously?"
3. **Conversational delegation.** The delegator chats with
   the worker (in the worker profile or via a channel) and
   says "from now on, can you also handle X?" The worker
   recognizes the meta-request and routes to the same
   delegate-ongoing-work flow.
4. **Cron / event triggers.** Operator wants a recurring
   duty ("every Monday morning, summarize last week's
   listings") — same flow, with a schedule attached.

## The flow

### Step 1 — Capture (0:00)

From the worker profile page, "Add a responsibility" opens a
single text field:

> "What else do you want Mariah to handle?"
>
> [ Tell me in your own words... ]

That's it. No vertical picker, no template selector. The
delegator types: "When buyers ask if I deliver to outside
Auckland, just say yes if it's within the North Island and
the buyer pays courier."

### Step 2 — Classify (0:30)

Genesis-lite classifies the request along three axes:

- **Type** — memory fact, policy rule, skill, vocabulary
  patch, or new duty (cron'd action).
- **Scope** — single situation vs. recurring pattern.
- **Conflict** — does this contradict an existing
  responsibility or memory fact? If yes, surface the
  conflict before proceeding.

For Sam's example, the classification is:
- Type: **Policy rule** + memory fact.
- Scope: **Recurring pattern.**
- Conflict: none detected.

### Step 3 — Draft (1:00)

Genesis-lite proposes the addition in plain English:

> "Got it. Going forward, when a buyer asks about delivery
> outside Auckland, Mariah will:
>
> ✓ Confirm yes if the address is in the North Island
> ✓ Quote: courier paid by the buyer, 2–5 business days
> ✓ Ask you first for South Island or international
>
> She'll also remember:
> ✓ You ship anywhere in NZ via courier
> ✓ Buyer pays shipping by default
>
> 3 example responses she'd send:
>
> [example 1: Wellington buyer asks → "Yes, I can ship to
> Wellington via courier. $18 standard, 2–3 days. Want me to
> bundle it with the deal?"]
>
> [example 2: Sydney buyer asks → "Hey! I'm only set up for
> NZ shipping right now — let me check with Sam if we can
> work out international and get back to you."]
>
> [example 3: Auckland buyer asks → "Local pickup is free,
> or I can do courier within the city for $8."]
>
> [✓ Add this] [✎ Adjust] [✗ Never mind]"

The example-responses trick — same as the hire flow — makes
the abstract rule concrete. The delegator reads three samples
and reacts emotionally.

### Step 4 — Resolve conflicts (skipped if none)

If Genesis-lite detected a conflict, this step shows both
the existing rule and the new one side-by-side:

> "This contradicts:
>
> Existing (added Apr 8): 'Mariah only ships to Auckland.'
>
> New: 'Mariah ships anywhere in NZ via courier.'
>
> Pick one:
> [ ] Replace the old rule with the new one
> [ ] Keep the old rule (cancel this addition)
> [ ] Merge: keep Auckland-only for some cases (let me edit)"

The merge path opens a single text field for the operator to
edit. No structured policy language exposed.

### Step 5 — Commit (1:30)

Click "Add this":

- The new responsibility is persisted to the worker's role +
  policy chain + memory namespace, atomically.
- An audit-log entry records: who added it, when, the plain-
  English request, the generated policy predicate, the
  example responses shown.
- The worker's next action incorporates the new rule.
- The worker profile's "Recent changes" panel shows the
  addition for 7 days, with an undo button.

### Step 6 — Confirmation (2:00)

The delegator gets a quiet confirmation:

> "Added. Mariah will start using this on her next reply.
> Want to see her in action? Three buyers asked about
> delivery this week — I'll re-handle those with the new
> rule and show you the updated responses."

The re-handling is optional but powerful: it shows the
delegator the new rule applied to recent real cases,
without sending anything to buyers. This is the
trust-acceleration moment.

## Special path — pattern detection

When the trust-ratchet detects "you've approved this 30 times
with 0 refusals," it doesn't add the rule silently. Instead:

> "Quick observation:
>
> In the last 14 days you've approved 31 'replies to "is this
> still available?" with current price' decisions. You haven't
> refused any of them.
>
> Want Mariah to handle these without asking, going forward?
>
> [Sample of the next 3 she'd send, based on current
> listings]
>
> [✓ Yes — but ping me if she ever wants to deviate]
> [⏸ Not yet — keep asking me]
> [✎ Adjust the rule first]"

The "ping me if she ever wants to deviate" option is
load-bearing: it converts a recurring approval into an
exception-only approval, which is what the delegator
actually wants. They want the bulk to happen silently and
the unusual cases to interrupt them.

## Special path — conversational delegation

If the delegator is chatting with Mariah on Slack and types:

> "from now on, just handle the 'is this still available'
> ones yourself"

The worker recognizes this as a delegation meta-request
(via the same classifier Genesis-lite uses) and replies:

> "Sounds good — let me make sure I have it right:
>
> Going forward, when a buyer asks 'is this still
> available' on a listing under 30 days old and at full
> price, I'll reply autonomously. For older listings or
> price-reduced listings, I'll still check with you.
>
> Confirm? (reply yes / no / change it)"

The delegator confirms in the chat. The same audit entry is
written, the same policy update happens. No web app required
for this path.

## What can go wrong

| Risk | Mitigation |
|---|---|
| Genesis-lite mis-classifies the request type (the operator typed a vocabulary patch but the system reads it as a policy rule) | The proposed-addition card explicitly states what type of change it is in plain English. The operator can override. |
| Conflict detection misses a conflict (a similar but not identical rule exists elsewhere in the policy chain) | Conflict detection runs over the full policy chain, not just the immediate worker. Conflicts surface even if the related rule lives on the roster or workspace level. |
| The 3 example responses are unrepresentative | Genesis-lite samples examples from the actual recent inbox where possible, not from synthetic data. If no recent examples exist, falls back to synthesis but flags it. |
| Pattern detection over-suggests autonomy | The "ping me if she deviates" option converts each silent auto-action into a check; the operator gets the benefit of speed without losing visibility on edge cases. Trust ratchet is reversible — operator can always say "go back to asking me." |
| Cron-driven new duties fire at bad times | The duty scheduler respects quiet hours and channel preferences inherited from the worker. New duties default to "ask first time, then proceed if approved." |
| Operator regrets a change they made 5 days ago | Recent changes panel shows last 7 days; one-click undo restores the prior policy + memory state. After 7 days, undo requires reverting via a new explicit "remove this responsibility" flow (audit trail preserved). |

## Critical moments

The three points where this flow either earns or loses
trust:

1. **The 3 example responses in Step 3.** If they don't
   sound like the operator, the operator hits Adjust. If
   Adjust makes the examples worse, the operator
   abandons. The example-rendering quality is the load-
   bearing UX moment. Voice profile + memory + skill
   selection have to all be correct for these to land.
2. **The conflict surface in Step 4.** If a conflict
   exists and the product doesn't show it, the operator
   ends up with contradictory rules and the worker
   produces inconsistent behavior. They blame the worker,
   not the policy editor. Always surface conflicts.
3. **The pattern detection in the special path.** If the
   product suggests autonomy too aggressively, the
   operator feels like the AI is trying to escape
   oversight. If it suggests too rarely, the trust
   ratchet doesn't work and every action stays gated
   forever. Calibration is critical.

## Success metrics

- **Median time-to-add**: ≤ 90 seconds from "Add a
  responsibility" click to "Added" confirmation.
- **Adjust rate**: 10–30% is healthy. > 50% means Genesis-
  lite is mis-classifying. < 5% means operators aren't
  reading the card carefully.
- **Conflict-detection precision**: 95%+ of detected
  conflicts should be real conflicts that the operator
  agrees need resolution. False positives erode trust.
- **Pattern-detection acceptance rate**: 60–80% is healthy.
  Lower means we're suggesting badly; higher means we're
  too conservative.
- **Post-add rule retention**: 95%+ of added rules should
  still be in place 30 days later. Lower means the
  proposed rule was wrong and the operator removed it.

## Cross-cutting requirements

- Every responsibility addition is fully auditable (the
  Auditor persona needs this — see `auditor.md`).
- The worker's view of itself updates atomically; partial
  rule state is impossible. The worker either has the new
  rule or doesn't.
- The role document persisted in the canonical store is
  always the source of truth; the UI is a view of it.
- The trust ratchet's behavior is governed by a single
  policy parameter the operator can tune ("aggressive,"
  "balanced," "conservative") at the workspace level.

## Dependencies on existing platform pieces

All shipped unless noted:

- Genesis backend: `mission-genesis-end-to-end-smoke`,
  `mission-genesis-llm-classifier`.
- Policy-as-code: `claude-policy-as-code`.
- Memory namespace per worker: `mission-memory-per-worker-
  scope`.
- Duty scheduler: `claude-worker-duty-scheduler`.
- Quiet hours: `mission-quiet-hours-channels`.
- **Missing for this journey**: the worker profile's "Add a
  responsibility" surface; the proposed-addition card
  layout; the conflict-resolution UI; the pattern-detection
  trigger; the re-handle-recent-cases preview.

## Open questions

`(assumed — needs validation)`

- Whether the 3-example preview is the right number. 1 might
  be enough; 5 might be too many to read.
- Whether the "ping me if she deviates" option is intuitive
  or confusing for first-time operators.
- Whether 7-day undo is too short for catching mistakes
  (might be 30 days?) or too long (operator state-confusion?).
- Whether conversational delegation via Slack works in
  practice or trips on similar-but-not-delegation messages
  ("Just handle this one yourself" — meaning the current
  one, not future ones).
- Whether re-handling recent cases as preview adds value or
  feels overwhelming.

Plan to validate by:

1. Walking 5 delegators through paper prototypes of Steps
   1–6 and noting where they pause or ask clarifying
   questions.
2. Running pattern-detection in shadow mode for 2 weeks
   pre-launch and measuring "what would have been
   suggested" vs "what the operator did anyway."
3. A/B-testing the example-count (1 vs 3 vs 5) on a
   subset of users.
