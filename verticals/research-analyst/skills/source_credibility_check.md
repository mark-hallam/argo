---
name: source_credibility_check
description: Given a URL, rate the source on tier-1 / tier-2 / tier-3, assign a confidence call, and surface caveats before the source enters a citation.
inputs:
  - url
  - claim_being_supported
outputs:
  - tier
  - confidence
  - caveats
  - reuse_recommendation
---

# Skill: Source Credibility Check

Given a candidate URL and the claim it is being asked to support,
produce a credibility rating before that source enters any brief.

1. **Tier** — assign one of:
   - `tier-1` — issuer-of-record filing, named-byline major wire / FT
     / WSJ / Bloomberg / Reuters / AP / Nikkei, peer-reviewed journal,
     national statistics office, regulator's own page, court judgment.
   - `tier-2` — sector trade publication with corrections policy,
     analyst firm with disclosed methodology, reputable think-tank
     report, established encyclopedia, government summary pages.
   - `tier-3` — industry blog, sponsored / vendor whitepaper,
     unsourced aggregator, social-platform post, AI-generated
     summary, Wikipedia (as a starting point only, never as the cited
     source for a load-bearing claim).
2. **Confidence** — `high` (tier-1, dated, byline, matches the claim
   exactly), `medium` (tier-2 OR tier-1 with partial match), `low`
   (tier-3 OR significant date / scope mismatch).
3. **Caveats** — explicit list. Examples: undated; sponsored content;
   non-byline aggregator; methodology not disclosed; behind paywall
   (and whether authorized access exists); translated from a primary
   source and translation not authoritative; AI-generated.
4. **Reuse recommendation** — `cite_as_primary`, `cite_with_caveat`,
   `colour_only_do_not_cite`, or `reject`. A `tier-3` source should
   only be `colour_only_do_not_cite` for load-bearing claims.

If the URL is a paywall that you do not have authorized access to, do
NOT attempt to scrape. Return `reuse_recommendation: reject` with the
caveat `paywall_no_authorized_access`. Sourcing rules forbid paywall
scraping; surface the gap to the analyst and let them decide whether
to acquire access.

If the URL appears to leak material non-public information (sealed
filings, pre-announcement deal terms, embargoed earnings), STOP, do
not consult further, and escalate with `flags: ["mnpi_risk"]`.
