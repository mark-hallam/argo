# Journey — Retrain a Worker

> The Delegator notices Mariah keeps making the same kind of
> mistake. They want to fix it without rebuilding her from
> scratch, without editing a YAML policy file, and without
> losing what she already knows. The platform absorbs the
> correction into the worker's memory, policy, or skill set —
> durably, reversibly, auditably. The journey that makes the
> AI worker feel *trainable* rather than *replaceable.*

## The promise

> "Tell your teammate what you want changed, in plain English,
> once. It sticks across every future session, every channel,
> and every model upgrade. If you change your mind, you can
> undo it just as easily."

## Actors

- **The Delegator** (primary). See [delegator.md](delegator.md).
- **The Worker** being retrained. Memory, policy, skills, or
  voice profile updated atomically.
- **Genesis-lite** — the classifier that figures out which
  knob to turn (memory? policy? skill? voice?) from a plain-
  English correction.
- **Skill-regression-detector** (already MERGED:
  `mission-skill-regression-detector`) — runs as a
  sanity-check on voice / skill changes.

## Triggers

Five triggers, all funneling into the same flow:

1. **Repeated refusal.** The Delegator hits NO on 5 similar
   approval cards in a week. The pattern-detector
   suggests "want to teach Mariah to stop proposing these?"
2. **Direct correction in conversation.** The Delegator
   chats with Mariah ("never agree to ship to PO boxes")
   on Slack, or types it into the retrain field in the
   worker profile.
3. **Negative outcome.** A customer complains, a refund is
   demanded, a listing is withdrawn. The post-incident
   flow links from the audit-replay drill-in: "make sure
   this doesn't happen again."
4. **Voice drift.** The skill-regression-detector flags
   that Mariah's recent replies are 30% shorter than her
   baseline. Surfaces a prompt: "voice has drifted — want
   to recalibrate?"
5. **Pre-emptive change.** The Delegator wants to update a
   rule before it becomes a problem ("our pricing
   changes next week — Mariah should know"). They
   proactively open the retrain field.

## The promise the worker must never break

The Delegator should NEVER have to:

- Edit a YAML policy file.
- Pick which "layer" (action / worker / roster / workspace)
  the rule belongs at.
- Phrase the rule as a TOML predicate.
- Re-articulate a fact they've already stated elsewhere.
- Lose what the worker already knew.
- Worry about whether the fix will stick across model
  upgrades.

The platform's job is to make all of those problems
invisible.

## The flow

### Step 1 — Initiate (0:00)

Multiple entry points, all reaching the same surface:

**From the worker profile**, button: "Adjust how she works."
**From an approval-card refusal**, an inline prompt: "Want
Mariah to stop asking about this? Train it in."
**From an audit drill-in**, button: "Make sure this doesn't
happen again."
**From a Slack DM with the worker**, just type the
correction in plain English.

All four paths reach the same retrain field:

> "What do you want Mariah to do differently?
>
> [ ___________________ ]"

### Step 2 — Classify (0:30)

Genesis-lite reads the correction and classifies the
intended change:

| Type | Examples | Where it goes |
|---|---|---|
| **Memory fact** | "We don't ship to PO boxes." / "Our office is closed Dec 25–28." | Append to the worker's memory namespace |
| **Policy rule** | "Always ask me before counter-offers above 10%." / "Never agree to refunds over $200 without approval." | New rule in the policy chain |
| **Skill correction** | "She's writing replies too formal." / "Stop using exclamation marks." | Adjust voice profile fields |
| **Vocabulary patch** | "Call them 'guests' not 'customers'." / "Use 'on-call' not 'rostered'." | Append to the lexicon |
| **Action ban** | "Never escalate refunds to PagerDuty." | Subtract from the worker's allowed-actions list |

Edge cases — the correction may map to *multiple* types
(e.g. "always check with me on shipping outside Auckland
and assume buyer pays courier" = policy rule + memory
fact). The flow handles compound corrections by surfacing
them as multiple proposed changes.

### Step 3 — Propose in plain English (1:00)

Whatever the classification, the platform shows the
proposed change in plain English, never structured:

> "Got it. Going forward, Mariah won't agree to ship to PO
> boxes. If a buyer asks, she'll ask you first — or you
> can teach her how to handle this in one tap.
>
> She'll add this to her knowledge:
> ✓ We don't ship to PO boxes (no exceptions by default).
>
> She'll change this rule:
> ✓ Shipping requests to PO box addresses require your
>   approval.
>
> [Try it on 3 recent buyer messages]
>
> [✓ Yes, train it in]   [✎ Adjust the wording]
> [✗ Never mind]"

The "Try it on 3 recent buyer messages" preview is
load-bearing: the operator sees how the updated worker
would have handled three real past situations. If they
look right, the change is trustable.

### Step 4 — Conflict + generalization check (1:30)

The platform runs two checks before committing:

**Conflict check.** Does this contradict an existing rule
or memory fact? If yes, surface both:

> "This conflicts with: 'Ship anywhere in NZ via courier'
> (added Apr 12).
>
> 'PO box addresses' might be in NZ.
>
> Pick one:
> [ ] Update the old rule to say 'anywhere in NZ except PO
>     boxes'
> [ ] Keep both — the new rule wins for PO boxes
> [ ] Cancel this change"

**Generalization check.** Did the operator mean "just PO
boxes" or "any non-residential address"? Genesis-lite
asks at most ONE clarifying question:

> "Quick check: should this also apply to other non-
> residential addresses (warehouses, mail-drops, freight
> forwarders), or just PO boxes specifically?
>
> [ Just PO boxes ]   [ Any non-residential ]"

If the operator skips, defaults to the narrower
interpretation ("just PO boxes"). Easier to broaden later
than to retract overreach.

### Step 5 — Sanity check via skill-regression-detector (2:00)

For voice-profile or skill-correction changes, the
platform runs the skill-regression-detector against the
last 50 actions to check: would the new voice or skill
have produced regressions on previously-good outputs?

If regressions detected, surface them:

> "Heads up — this would change Mariah's reply on 4 of
> the last 50 listings, including one that closed at full
> price (May 9). Preview:
>
> [Before / After comparison for each of the 4]
>
> Still want to apply? [Yes — I see the differences]
> [No — let me rework the change]"

The detector exists already (`mission-skill-regression-
detector` MERGED); this step exposes it to the operator
at retrain time, not just as a background process.

### Step 6 — Commit (2:30)

Click "Yes, train it in":

- Memory, policy, skill, or voice update committed
  atomically.
- Audit row stamped: plain-English request, classified
  type, generated structural change, conflict resolution,
  generalization scope, regression-check result.
- A "Recent changes" entry appears in the worker profile
  for 14 days with a one-click revert.
- Worker's next action uses the new state.
- The 50-action behavioral baseline is reset; future
  regression checks start from this updated baseline.

### Step 7 — Confirmation + reverse-test (3:00)

Quiet confirmation, with a powerful follow-up offer:

> "Done. Mariah will start using this on her next reply.
>
> Want me to re-handle the 4 conversations from this
> week that would have triggered this rule? Nothing gets
> sent to buyers — I'll show you the updated drafts."

The reverse-test offer is the trust-acceleration moment:
the operator sees the new behavior on real past cases
without any external risk. If the new behavior looks
right, confidence is high.

### Step 8 — Trust-ratchet reset (background, not user-visible)

A policy correction on a specific action type **resets the
trust ratchet** for that action type. The operator just
told the worker "I want to be more involved in this kind
of decision again" — respect that. The product silently
moves the ratchet back to "ask first time, then
proceed if approved."

This is invisible to the operator but load-bearing: it
prevents the situation where a correction makes the
worker behave correctly on the specific case but the
broader trust-ratchet keeps the wrong autonomy level.

## Special path — conversational retrain via Slack

If the operator chats with Mariah on Slack:

> "Mariah, never agree to ship to PO boxes."

The worker recognizes this as a retrain meta-request (via
the same classifier Genesis-lite uses) and replies:

> "Got it. Adding 'no PO boxes' to my shipping rules.
> Should I also flag any address that looks like a
> warehouse or freight forwarder, or just PO boxes
> specifically?
>
> Reply 'just PO' / 'any non-res' / 'wait, let me think'"

The operator answers in chat. The same audit entry is
written. The same conflict + regression checks run; if
they fire, the worker reports back in the chat:

> "Heads up — this conflicts with 'ship anywhere in NZ'
> from Apr 12. I can update that rule to exclude PO
> boxes, or keep both with this taking precedence. Which
> works?"

The flow is fully chat-driven for the common case. The
web app is the place to go when something more complex
(e.g. seeing the 3-message preview, reviewing regression
diffs) is wanted.

## What can go wrong

| Risk | Mitigation |
|---|---|
| Classifier picks the wrong type (memory vs policy) | The proposed-change card names the type in plain English; operator can override via "Adjust the wording" or by trying a different phrasing |
| Generalization check picks the wrong default | Defaults to the narrower interpretation; broadening later is one-click. Retraction (after over-applying) is much harder. |
| Conflict-resolution choices the operator regrets | Recent changes panel keeps a 14-day undo; one click restores the prior state, with the audit trail preserved |
| Regression check produces false positives, scaring the operator away from good changes | Detector tunable; defaults to flagging only > 20% behavioral delta on previously-good outputs |
| The reverse-test (Step 7) re-handles cases that should NOT be re-evaluated (e.g. sensitive resolved customer issues) | Reverse-test is opt-in, not automatic; operator chooses whether to see it |
| The trust-ratchet reset surprises the operator (they expected autonomy to stay) | The "Recent changes" entry includes a note about the ratchet reset; operator can re-grant autonomy explicitly if they want |
| Cross-channel inconsistency: rule applied in Slack chat doesn't show up in the web app's "Recent changes" panel | All retrain entry points write to the same audit trail and Recent-changes view, regardless of channel of origin |

## Critical moments

The three points where the retrain flow earns or loses
trust:

1. **The 3-message preview in Step 3.** If the previews
   look right, the operator commits with confidence. If
   they look wrong, the operator hits Adjust. If Adjust
   doesn't improve them, the operator gives up and the
   worker keeps the bug.
2. **The conflict surface in Step 4.** Conflicts hidden
   from the operator produce contradictory rules and
   inconsistent worker behavior. Operators blame the
   worker, not the rule editor. Always surface conflicts.
3. **The 14-day undo.** The first time an operator hits
   Undo and it actually restores the prior state, trust
   is locked. The first time it *doesn't* work — partial
   revert, broken state — trust is broken.

## Cross-cutting requirements

- Every retrain commits to a single canonical store; the
  UI is a view. No partial state.
- Every retrain entry point produces audit-identical
  records (same fields, same provenance), regardless of
  whether the entry was web, CLI, or chat.
- Skill / voice changes always run through the regression
  detector before commit.
- Policy corrections reset the trust-ratchet for the
  affected action type.
- Memory edits are never silent destructive overwrites —
  always append + supersede + audit, never delete.

## Success metrics

- **Median time-to-train**: ≤ 3 minutes from open to
  commit.
- **Adjust rate** (operator hits "Adjust the wording"):
  10–30% is healthy. > 50% means classifier or preview is
  poor.
- **Undo rate**: 5–15% within 14 days is healthy. > 25%
  means corrections are landing wrong; investigate Step 3
  copy.
- **Post-retrain regression rate**: ≤ 5% of retrains
  should produce a regression that the operator catches
  later. If higher, the regression-detector is missing
  cases.
- **Conversational retrain success rate** (chat-driven
  flow completing without falling back to the web app):
  ≥ 70% for simple memory/policy changes.

## Dependencies on existing platform pieces

All shipped unless noted:

- Memory edit + audit: shipped.
- Policy-as-code chain: `claude-policy-as-code`.
- Voice profile fields: `mission-voice-profile-richer-
  fields`.
- Skill regression detector: `mission-skill-regression-
  detector`.
- Audit-row stamping: shipped.
- Genesis-lite classifier: `mission-genesis-llm-
  classifier` (extends for retrain intent).
- **Missing for this journey**: the retrain field UI; the
  proposed-change card layout; the 3-message preview
  generator; the conflict-resolution UI; the regression-
  diff surface; the trust-ratchet reset wiring; the
  conversational retrain detector + response.

## Open questions

`(assumed — needs validation)`

- Whether the 3-message preview is the right number,
  same as in the delegate-ongoing flow.
- Whether the generalization check (one clarifying
  question) feels helpful or pedantic.
- Whether the regression-detector should default to
  flagging > 20% or > 30% delta.
- Whether 14-day undo is too short / too long.
- Whether conversational retrain should support the full
  flow (regression preview, conflict resolution) or
  punt to the web app for anything beyond simple
  memory/policy.
- Whether trust-ratchet reset on retrain is the right
  default behavior; might frustrate operators who want
  the worker autonomous on adjacent cases.

Plan to validate by:

1. Running 10 retrain scenarios with synthetic
   delegator personas and measuring time-to-commit + post-
   commit satisfaction.
2. Shadowing 3 real operators for a week and counting:
   how often did they want to retrain but couldn't find
   the path? How often did they end up editing TOML
   manually?
3. Running the skill-regression-detector retrospectively
   against past skill changes; calibrate the threshold.
