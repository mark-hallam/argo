---
name: flag_term_sheet_redlines
version: "0.1.0"
description: Given a term sheet draft + the founder's preferred terms (from `memory.fundraising_targets`), redline disagreements across 5 dimensions — valuation, liquidation preference, option-pool size, board seats, anti-dilution. Outputs structured JSON + a Markdown summary for the founder. ALWAYS approval-gated — term sheets are legal commitment instruments.
provenance:
  source: vertical/fundraising-coordinator
  hash: sha256-pending
---

# When to use

A term sheet has arrived from an investor and the founder wants a
fast read on where it disagrees with their pre-stated targets, BEFORE
forwarding to counsel. This skill is research — it surfaces gaps; it
never advises acceptance, rejection, or negotiation strategy. The
operator (founder + counsel) decides.

# Inputs you expect

- `term_sheet_ref` — path or memory key to the draft (parsed fields,
  not a PDF blob — extraction is a separate connector).
- `fundraising_targets` — from `memory.fundraising_targets`. Must
  include preferences for all 5 dimensions or the skill refuses.
- `round_kind` — `seed` / `series-a` / `series-b` / ... (sets
  conventional ranges for the per-dimension context lines).

# Output shape

```json
{
  "round_kind": "series-a",
  "redlines": [
    {"dimension": "valuation",            "target": {"pre_money_usd": 40000000}, "term_sheet": {"pre_money_usd": 32000000}, "delta": "-20%",  "severity": "high",   "evidence": {"source": "term_sheet.valuation", "value": 32000000}},
    {"dimension": "liquidation_preference","target": {"multiple": 1.0, "participating": false}, "term_sheet": {"multiple": 1.5, "participating": true}, "delta": "1x non-part → 1.5x participating", "severity": "high", "evidence": {"source": "term_sheet.liquidation"}},
    {"dimension": "option_pool",          "target": {"pct": 10.0, "timing": "post-money"}, "term_sheet": {"pct": 15.0, "timing": "pre-money"}, "delta": "+5pp + pre-money shuffle", "severity": "high", "evidence": {"source": "term_sheet.option_pool"}},
    {"dimension": "board_seats",          "target": {"founder": 2, "investor": 1, "independent": 1}, "term_sheet": {"founder": 2, "investor": 2, "independent": 1}, "delta": "investor +1 (5-seat board)", "severity": "medium", "evidence": {"source": "term_sheet.board"}},
    {"dimension": "anti_dilution",        "target": {"type": "broad-based-weighted-average"}, "term_sheet": {"type": "full-ratchet"}, "delta": "broad-based → full-ratchet", "severity": "high", "evidence": {"source": "term_sheet.anti_dilution"}}
  ],
  "markdown_summary": "## Term sheet redlines\n\n- **Valuation**: -20% vs target ($32M vs $40M pre-money)\n- **Liquidation preference**: 1.5x participating vs 1x non-participating target ...\n",
  "requires_approval": true,
  "next_action": {"name": "route_to_counsel", "requires_approval": true}
}
```

# Approval posture

**Always approval-gated.** `requires_approval: true` is
non-negotiable. A term sheet is a legal commitment instrument; no
memory entry, env var, or flag flips this. The skill emits the
redline; the founder + counsel decide.

# Rules

- **Cover all 5 named dimensions.** Valuation, liquidation
  preference, option-pool size, board seats, anti-dilution. Missing
  any dimension in either input → refuse with
  `_refusal_reason` naming the missing field.
- **Flag gaps, never propose acceptance.** Output is a delta map.
  Do not recommend "this is fine to accept" or "you should push back
  to X". Surface the disagreement; let the operator decide.
- **Severity is a heuristic, not legal advice.** Use `high` /
  `medium` / `low` based on numeric magnitude and round-conventional
  ranges. Document the rule of thumb in the markdown summary; do not
  state it as a legal conclusion.
- **Markdown summary mirrors JSON.** Same five rows, founder-readable.
  No editorialization beyond the delta + severity.

# Hard rules

- **Not legal advice.** This skill is information-routing. It does
  not interpret the legal effect of any term; it compares two
  structured records. Counsel review is the operator's next step.
- **No outbound from this skill.** Never draft a counter-proposal,
  never email the investor, never modify the term sheet. The output
  is for founder + counsel review only.
- **Refuse on incomplete inputs.** Missing target preference for any
  named dimension → `{"_refusal_reason": "populate memory.fundraising_targets.<dim> first"}` and stop.
