---
name: draft_newsletter
version: "0.1.0"
description: "Drafts the weekly newsletter section combining competitive_scan output and an operator-supplied product changelog. Approval-gated; every claim cited."
provenance:
  source: vertical/marketing-intern
  hash: sha256-pending
---

# When to use

Call after `competitive_scan` has populated
`memory.competitor_scans[]` for the current ISO week AND the
operator has supplied an internal product changelog. Output is
a draft only — `send_newsletter` is approval-gated by the
vertical (see `vertical.toml`).

# Inputs you expect

- this week's `competitor_scans[]` rows (or a filtered subset)
- operator-provided changelog: `[{change, link, date}]`
- newsletter audience tag (default: "all-subscribers")

# Tools you may invoke

- `memory` — read prior `competitor_scans[]` and pinned
  `newsletter_voice_examples[]`

# Output shape

A markdown block, then a JSON metadata block:

```markdown
**Lede (2 lines):**
<line one — the week's headline angle>
<line two — why it matters to the reader>

**This week:**
- <bullet 1, ≤30 words, ends with `[source: URL]`>
- <bullet 2, ≤30 words, ends with `[source: URL]`>
- <bullet 3, ≤30 words, ends with `[source: URL]`>

**Sign-off:**
<one short, plain line — no exclamation marks>
```

```json
{"requires_approval": true, "audience": "<tag>", "claim_count": 3, "uncited_claims": 0, "sources": ["<URL>"]}
```

# Hard rules

- **5-bullet ceiling, 3-bullet target.** Operators read every
  approval; longer drafts get rubber-stamped and ship with
  errors. If you have 6 candidates, drop the weakest two.
- **Every claim cited.** A bullet without `[source: URL]` is
  invalid. `uncited_claims` in the metadata MUST be `0` for
  the draft to be submittable.
- **No fabricated competitor moves.** If `competitor_scans[]`
  is empty, say so plainly in the lede ("Quiet week from
  competitors — here's what we shipped instead.") rather
  than inventing filler.
- **Approval-gated.** Always emit `requires_approval: true`.
  This skill never triggers `send_newsletter` directly.
- **Brand voice = persona.** Match `prompts/persona.md` —
  no clever turns of phrase, no exclamation marks.

# Examples

```markdown
**Lede:**
Two competitors raised prices this week.
Here's how that lines up with what we shipped.

**This week:**
- Acme Inbox moved Pro from $29 → $39/month [source: https://acmeinbox.example/pricing]
- Beta Chat shipped a threading view [source: https://betachat.example/blog/threading]
- We released CSV export in Reports v1.4 [source: https://yours.example/changelog/1.4]

**Sign-off:**
Reply if anything here would help your team this week.
```

# Memory

Append the draft + metadata to `memory.newsletter_drafts[]` so
the operator can audit which scan rows fed which approval.
