# Journey — Delegate Ongoing Work (stub)

> The flow AFTER hire. The Delegator says "Mariah, also handle X
> from now on" — and the platform absorbs the new responsibility
> into the existing teammate without a fresh hire flow.

## The promise

> "Adding a new responsibility to an existing teammate is a
> conversation, not a configuration."

## Trigger

- Delegator notices a new repetitive task they want offloaded.
- Or: a duty fires and produces a result the Delegator
  wants to make more autonomous ("you've approved this 30
  times — make it automatic").

## The shape (sketched)

1. From the worker profile, "Add a responsibility" → free-text
   prompt: "What else do you want her to handle?"
2. Genesis-lite path: classifies the new responsibility, drafts
   a new duty (cron + entry-action), proposes an approval policy,
   shows 2–3 example actions.
3. Delegator approves, refines, or rejects.
4. Mariah's charter, duties, and KPI list update in place. Memory
   namespace extends; existing memory not invalidated.

## What we still need to figure out

- How to surface "trust ratchet" suggestions ("you've approved
  this 30 times — auto?") without nagging.
- Whether ongoing-delegation is its own surface or lives inside
  the worker profile page.
- How to handle conflict: new responsibility overlaps with an
  existing one. Does the worker pick one path? Ask the Delegator?
- How to detect *implicit* delegation: the Delegator chats with
  Mariah in plain English ("can you also start handling X?") and
  the platform learns the new charter from conversation rather
  than from a settings page.

## Status

Stub. Will be expanded when the worker profile page wireframes
need this flow defined. Probably ~Q3 of the frontend
implementation phase.
