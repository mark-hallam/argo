---
name: propose_cpt
description: Given procedure documentation, propose CPT codes with modifiers, advisory RVU estimate, and per-code documentation citation. Drafts only; never submits.
inputs:
  - procedure_documentation
  - encounter_date
  - place_of_service
outputs:
  - proposed_codes
  - modifiers
  - rvu_estimate
  - flags
  - confidence
---

# Skill: Propose CPT Procedure Codes

Given the procedure-note span, propose CPT (and HCPCS Level II where
a supply / drug / DME line warrants it) codes for the supervising
certified coder.

1. **Proposed codes** — flat list ordered by service line. For each:
   - `code` — CPT or HCPCS Level II; name the code system.
   - `description` — official short description.
   - `citation` — `[doc: <doc_id-or-page>, span: L<start>-L<end>,
     date_accessed: <ISO>]`.
   - `evidence_class` — `documented` (procedure / supply described
     by name with sufficient detail) or `implied`.

2. **Modifiers** — per-line with policy rationale. Common: `-25`
   (significant separately identifiable E&M same day), `-59`
   (distinct procedural service), `-26` (professional component),
   `-TC` (technical), `-LT` / `-RT` (laterality), `-50` (bilateral).
   Cite the trigger; never apply as habit.

3. **RVU estimate** — advisory only. Work + practice + malpractice
   RVUs from the calling LLM's training-data recall. Set
   `advisory_only_not_for_billing: true` on every line.

4. **Confidence** — high (every code `documented`, descriptor
   matches), medium (`implied` present), low (descriptor ambiguous).

Hard rules:

- Never propose without a documentation citation; flag
  `[insufficient detail — flagged for coder review]` and drop.
- E&M level mismatch — if documentation supports a lower E&M
  (Evaluation & Management) level than requested, do NOT propose
  the higher level. Surface both with chart spans; route as
  upcoding risk.
- Unbundling — if proposed codes form a known NCCI (National
  Correct Coding Initiative) pair, STOP. Surface the canonical
  bundled code and route.
- DME (Durable Medical Equipment) and Medicare Part B drug pricing
  (ASP, 340B) escalate — separate MAC, quarterly rule changes.

I propose codes but never submit — the certified coder signs off
before anything goes to a payer.
