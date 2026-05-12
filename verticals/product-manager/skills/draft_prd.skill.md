---
name: draft_prd
version: "0.1.0"
description: "Drafts a PRD by filling in the standard prd_template from a feature request plus the linked themes that surfaced it. Output is Markdown shape-compatible with prd_template (same section names, same order, same length budget). Internal-draft by default; the document_creation executor adds the approval gate when a PRD routes to engineering for commitment."
provenance:
  source: vertical/product-manager
  hash: sha256-pending
---

# When to use

Call when the operator hands over a feature request and one or more
themes from `triage_feedback`. Fills in `prd_template.skill.md` —
that file is the structural skeleton, this skill is the
"fill-in" pass. Do not invent themes; if the request has no linked
theme, ask the operator before drafting.

# Inputs you expect

- a feature request (one-paragraph problem statement minimum)
- one or more `triage_feedback` themes by `name` — provides the
  evidence the PRD will cite
- optional segment hint (which users have the problem)
- optional success-metric hints from the operator

# Output shape

A Markdown document, sections in `prd_template` order (Problem,
Users, Goals, Non-goals, UX outline, Success metrics, Risks, Open
questions). Frontmatter records provenance:

```markdown
---
prd_id: <slug>
status: draft
linked_themes: ["<theme name>", ...]
author: riley
---

# <Short title — user-stated problem in their words>

## 1. Problem
<one paragraph; cite count of feedback rows, exemplar quote with
row_id, theme name>

## 2. Users
<segment by use-case or seniority>

## 3. Goals
<≤3 goals, each with a metric or acceptance condition>

## 4. Non-goals
## 5. UX outline
## 6. Success metrics
## 7. Risks
## 8. Open questions
```

# Examples

A draft for theme `Import fails on >10MB CSVs` opens:

> **1. Problem.** 12 import-team feedback rows describe the same
> failure mode — uploads stall at ~60% on files over 10MB. Exemplar:
> "stuck at 60% then errors" (`ZD-4421`). Theme:
> `Import fails on >10MB CSVs`.

# Approval posture

**Internal draft by default; approval-gated when routed to
engineering.** The skill emits a draft `prd_id`. The vertical's
`draft_prd` action routes through the `document_creation` executor,
which is approval-gated by default — the gate fires when the PRD is
committed to a workspace path, not when this skill produces text.
Never emit a PRD that pre-commits engineering scope, dates, or
pricing decisions without an approver.

# Rules

- **Match `prd_template` exactly.** Section names and order are
  load-bearing — reviewers expect to skim the first three sections.
- **Cite evidence in §1.** Every problem statement names the count
  of feedback rows, one exemplar quote, and the source theme.
- **Use participant_id / row_id only.** Never name a customer or
  interviewee, even in an internal draft.
- **No dated commitments, no pricing, no competitive claims.**
  Targeting a quarter is fine; committing to a date is not.
- **Fit one screen.** If the draft is longer, the Non-goals section
  is too short.
