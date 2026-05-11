# Journey — Retrain a Worker (stub)

> The Delegator notices Mariah keeps making the same kind of
> mistake. They want to fix it without rebuilding her from
> scratch. The platform absorbs the correction into the
> worker's memory, policy, or skill set — durably.

## The promise

> "Tell your teammate what you want changed, in plain English,
> once. It sticks across every future session and every channel."

## Trigger

- Repeated refusal of the same kind of action (Delegator hits
  NO on 5 similar approval cards in a week).
- Direct correction in conversation ("Mariah, never agree to
  ship to PO boxes").
- Negative outcome — a customer complaint, a refund, a
  withdrawn listing — that traces back to a specific worker
  action.
- The trust ratchet *almost* triggered (28 yes-es in 30 days)
  but the Delegator wants to delay autonomy.

## The shape (sketched)

The Delegator should NEVER have to:

- Edit a YAML policy file.
- Pick which "layer" (action / worker / roster / workspace) the
  rule belongs at.
- Phrase the rule as a TOML predicate.

Instead:

1. From the worker profile, "Adjust how she works" → free-text:
   "What do you want her to do differently?"
2. The platform classifies the correction:
   - **Memory fact** ("we don't ship to PO boxes") → appended
     to the worker's memory namespace.
   - **Policy rule** ("always ask me before counter-offers
     above 10%") → added to the worker's approval policy with
     a generated predicate.
   - **Skill correction** ("she's writing replies too formal")
     → adjusts the voice profile's `tone` / `formality` fields
     (already shipped: `mission-voice-profile-richer-fields`).
   - **Vocabulary patch** ("call them 'guests' not 'customers'")
     → adds to the lexicon.
3. The platform shows the proposed change in plain English:
   *"Going forward, Mariah won't agree to ship to PO boxes
   without asking you. Sound right?"*
4. Delegator confirms. Change applies immediately to the next
   action. Audit log records the change, who made it, when.

## What we still need to figure out

- Conflicts: new correction contradicts an existing rule. Show
  both. Let the Delegator pick.
- Generalization: did "don't ship to PO boxes" mean *just PO
  boxes* or *any non-residential address*? Genesis-lite asks
  one clarifying question, not three.
- Reversibility: every change is undoable for 7 days, surfaced
  in a "recent changes" view per worker.
- Trust impact: a policy correction should reset the trust
  ratchet on the affected action type. The Delegator is saying
  "I want to be asked again about this" — respect it.
- A/B for skill updates: if the correction tightens the voice
  profile, automatically queue a regression-detector check
  (`mission-skill-regression-detector` MERGED) to verify the
  new voice doesn't break previously-good replies.

## Status

Stub. The pieces exist in the backend (memory edit, policy
chain, voice profile fields, skill regression). The UX glue is
the missing layer. Expanded when the worker profile page's
"Adjust" surface is wireframed.
