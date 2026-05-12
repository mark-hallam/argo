---
name: prep_pitch_deck_review
version: "0.1.0"
description: Given the founder's current pitch deck + a target investor's known thesis/portfolio, produce a 5-bullet "what to emphasize" and 3-bullet "what to soften / cut" brief. Every recommendation cites the investor evidence it derives from. Internal-only — never surfaced to the investor.
provenance:
  source: vertical/fundraising-coordinator
  hash: sha256-pending
---

# When to use

The founder has a partner meeting or first call scheduled with a
named investor and wants to tune which slides land hardest for that
specific audience. Use AFTER `research_investor` has populated a
recent `thesis_alignments` entry (≤ 90 days) for the firm.

# Inputs you expect

- `investor_id` / `firm_name` — the target audience.
- `deck_ref` — path or memory key to the current pitch deck (slide
  titles + 1-line summaries at minimum).
- `thesis_alignments_entry` — from `research_investor`. The skill
  refuses if no entry exists for this firm; surface that to the
  founder so they queue `research_investor` first.
- optional `meeting_kind` — `first-call` / `partner-meeting` /
  `follow-up` (shifts emphasis defaults; partner meetings reward
  market-size depth, first calls reward founder/story).

# Output shape

```json
{
  "firm": "Acme Capital",
  "meeting_kind": "partner-meeting",
  "emphasize": [
    {"point": "Lead with the $4.2B serviceable market slide",
     "rationale": "Acme's stated thesis emphasizes TAM > $1B for Series A leads",
     "evidence": {"source": "thesis_alignments.acme_capital.thesis_summary", "quote": "we lead Series A in markets >$1B SAM"}},
    {"point": "...", "rationale": "...", "evidence": {"...": "..."}}
  ],
  "soften_or_cut": [
    {"point": "De-emphasize consumer-acquisition channels slide",
     "rationale": "Acme is B2B-only — recent 6 investments all B2B SaaS",
     "evidence": {"source": "thesis_alignments.acme_capital.recent_investments", "quote": "6/6 last investments B2B SaaS"}}
  ],
  "_internal_only": true,
  "_thesis_alignments_used": "thesis_alignments.acme_capital@2026-04-30"
}
```

# Rules

- **Five emphasize bullets, three soften/cut bullets.** Fixed
  counts. The founder's mental model is a quick scan; longer lists
  dilute the prep value.
- **Every bullet cites evidence.** Each recommendation carries an
  `evidence` block with the source (memory key, public URL, or
  portfolio entry) and a quote. Recommendations without citations
  are noise — refuse to emit them.
- **Reference the thesis_alignments entry, don't re-research.** The
  research step already happened in `research_investor`. Stale
  entries (> 90 days) → surface that and recommend a re-research
  before relying on the prep.
- **Portfolio overlap shapes "soften".** If a portfolio company is
  in an adjacent space, soften any slide that risks looking
  competitive. If a competitor is in portfolio, that's a hard signal
  to surface — recommend founder weigh whether to take the meeting.
- **Meeting-kind shifts defaults.** First call: emphasize founder
  story and one crisp wedge. Partner meeting: emphasize market size,
  unit economics, and the slide most likely to draw partner
  skepticism (so the founder pre-empts it).

# Hard rules

- **Internal-only.** The output is for founder pre-meeting review.
  Never surface this brief to the investor; never auto-attach it to
  any outbound `send_email_external` flow.
- **No deck edits from this skill.** Recommendation, not action.
  The founder owns the slide-by-slide changes.
- **Refuse gracefully on missing research.** If no
  `thesis_alignments` entry exists, output
  `{"_refusal_reason": "queue research_investor first"}` and stop.
