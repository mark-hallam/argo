---
name: research_investor
description: Given investor name + firm, produce a one-pager covering thesis fit, portfolio overlap, check-size range, and intro paths. Sourced from web research + thesis_alignments memory; does NOT contact the investor.
inputs:
  - investor_name
  - firm_name
  - stage_hint
outputs:
  - one_pager
  - thesis_fit_score
  - intro_paths
  - _confidence
---

# Skill: Research Investor

Produce a structured one-pager on an investor or firm. Reads from
`thesis_alignments` memory first (prior research is authoritative
when fresh), then supplements with `web_search` / `web_extract` for
public sources.

1. **Memory check.** Look up `thesis_alignments` for the firm. If a
   recent entry exists (≤ 90 days), use it as the base and supplement
   only with what's missing. Older entries are reference, not truth.
2. **Public sources.** Search firm site, public partner profiles,
   recent portfolio announcements, public LP letters or fund
   announcements, Crunchbase / Pitchbook public surface. Do NOT
   scrape paywalled content; cite only public URLs.
3. **Thesis fit.** Compare firm's stated thesis to the founder's
   company (sector, stage, GTM motion, geography). Score on
   `[0.0, 1.0]`. Anything below 0.5 → recommend deprioritize.
4. **Portfolio overlap.** Surface any portfolio companies that are
   adjacent, competitive, or potentially conflicted. A direct
   competitor in portfolio is usually a hard pass-signal.
5. **Check size + stage.** Public surface usually states a fund size
   and typical check range. If absent, record as "unknown" — do not
   guess from fund size alone.
6. **Intro paths.** Mutual connections, prior founders portfolio
   has backed, advisors in common. Surface 2–3 specific intro routes
   with the warmth signal (1st-degree warm / 2nd-degree warm / cold).
7. **NEVER contact the investor in this skill.** Output is for
   founder review only. Outbound is gated to `send_email_external`.

Output shape:

```json
{
  "one_pager": {
    "firm": "Acme Capital",
    "lead_partner": "Pat Doe",
    "fund_size_usd": 250000000,
    "check_range_usd": [3000000, 10000000],
    "stage_focus": ["Series A", "Series B"],
    "sector_focus": ["B2B SaaS", "developer tools"],
    "thesis_summary": "...",
    "portfolio_overlap": ["adjacent: CompanyX", "competitive: none"],
    "recent_investments": ["..."]
  },
  "thesis_fit_score": 0.82,
  "intro_paths": [
    {"path": "via founder of CompanyX", "warmth": "1st-degree-warm"},
    {"path": "via advisor Alice", "warmth": "2nd-degree-warm"}
  ],
  "_confidence": {"thesis_summary": 0.9, "check_range_usd": 0.7},
  "_sources": ["https://acmecap.example.com/portfolio", "..."]
}
```
