---
name: cite_sources_for_claim
version: "0.1.0"
description: "Finds and attaches credible sources to a factual claim, returning citations with quoted evidence — refuses if no source supports the claim verbatim."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

The operator has a draft claim and wants it backed (or
challenged) before it goes out — in a report, a customer-facing
doc, a regulatory filing, a press release. Use any time the
question is "where does this come from?" or "can we say this?"

# Inputs

- `claim`: the one-sentence factual assertion.
- `acceptable_sources`: list of source classes the operator will
  accept (`"peer-reviewed"`, `"official-statistics"`,
  `"vendor-docs"`, `"industry-analyst"`, `"news-wire"`). The
  operator's domain determines this.
- `recency_window` (optional): the source must be no older than
  this (`"24 months"`, `"5 years"`). Default: no limit.

# Output

A list of 1-3 citations, each with: source name, URL or
internal-doc reference, publication date, and a verbatim quoted
sentence that supports the claim. If no qualifying source exists,
return an empty list + a `note` explaining why ("claim is too
specific", "all sources fall outside `recency_window`", "no
source class in `acceptable_sources` covers this topic").

# Examples

- Claim `<industry-stat assertion>`, acceptable_sources
  `["official-statistics", "industry-analyst"]` → 2-3
  citations with quotes.
- Claim that turns out to be folklore (no qualifying source) →
  empty list + clear "no qualifying source found" note —
  signal to the operator to soften or drop the claim.

# Approval posture

Read-only. Skill never asserts the claim is "true" or "safe to
publish" — only that supporting sources exist. The operator (or
legal/compliance review) decides.

# Notes

- Refuse to "almost-cite" — a source that mentions the topic but
  doesn't make the specific claim is NOT a citation; it's a near-
  miss and must be flagged as such, not used.
- Quote verbatim. Paraphrased quotes are a credibility risk
  because they can drift from what the source actually said.
- Prefer primary sources (the agency that published the stat)
  over secondary (a news article citing the agency).
- If `acceptable_sources` is empty, refuse and ask the operator
  to specify — "any source" is rarely what the operator wants.
