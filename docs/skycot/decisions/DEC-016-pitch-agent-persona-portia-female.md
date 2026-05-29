---
id: DEC-016
date: 2026-05-29
status: active
supersedes: none
---

# DEC-016 — Pitch Agent persona is "Portia" (female)

## Decision

The Pitch Agent's teammate persona is named **Portia** and is a **female**
persona. This sets `default_display_name = "Portia"` (was `"Pitch"`) in
`packs/pitch-agent/worker.toml` and `agent_name_default = "Portia"` in
`packs/pitch-agent/ui/cohort.toml`.

## Why

- The operator's teammate-naming convention is: names that start with /
  sound like the first letter of the role they perform. Pitch → a **P**-name.
- Existing P-names are taken: Peta (Project Coordinator), Patrick (Status
  Reporter), Priya (recruiter). A new, distinct P-name was needed.
- The operator chose **Portia** (2026-05-29). It is on-theme: Shakespeare's
  Portia delivers the most famous persuasive-advocacy speech in English — the
  archetype of an eloquent advocate, fitting a pitch/persuasion role.
- The operator specified the persona should be **female**.

## How to apply

- Use "Portia" and she/her wherever the Pitch Agent teammate is named in
  product copy, persona cards, the `/pitch` surface, and the teammate-eval
  harness ([[DEC-017]]).
- "Pitch Agent" remains the *role/pack* label (like "Marketplace Lister" is
  Mariah's role); **Portia** is the teammate's name.
- Portia is the **reference teammate** for the performance-eval harness
  ([[DEC-017]]) — she already has the mature 141-case investor-Q&A suite.

Related: [[project_pitch_agent_v1_demo]], [[DEC-008]] (Pitch in MVP scope).
