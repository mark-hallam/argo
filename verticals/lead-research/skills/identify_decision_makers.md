---
name: identify_decision_makers
version: "0.1.0"
description: "Given a company, returns up to 5 likely buying-committee contacts with titles, committee role, and LinkedIn URLs when web_extract permits. Role-pattern fallback when personal data is thin."
provenance:
  source: vertical/lead-research
  hash: sha256-pending
capabilities: ["web_search", "web_extract", "file_read"]
---

# When to use

Call after `research_company` (or whenever the SDR needs the
contacts block refreshed for an existing `accounts` row). The output
feeds `draft_research_brief`.

# Inputs you expect

- company name (required)
- ICP definition (to know which committee roles matter for the deal)
- existing `accounts` row when available (avoids re-researching the
  firmographic block)

# Output shape

```json
{
  "company": "<name>",
  "contacts": [
    {
      "name": "<person name|null>",
      "title": "<title>",
      "committee_role": "decision-maker|influencer|user|gatekeeper",
      "linkedin_url": "<url|null>",
      "basis": "verified|role-inference",
      "why_they_matter": "<one sentence grounded in the ICP>"
    }
  ],
  "uncertainty": ["<what you don't know but the SDR should>"]
}
```

# Rules

- **Maximum five contacts.** If you can verify ten, return the five
  highest-leverage ones. The SDR can't action a list of fifteen.
- **No invented people.** A named contact requires a verifiable
  public surface (company team page, LinkedIn profile, conference
  speaker bio, podcast appearance). When you can't verify a person,
  emit a role-only row with `name: null` and `basis:
  "role-inference"`.
- **No creepy personalisation.** Don't surface arrest records,
  health, family, politics, or any personal trait that isn't both
  public AND business-relevant.
- **Cite LinkedIn only when web_extract returned it.** Don't guess
  the URL pattern; a wrong URL is worse than no URL.
- **Populate `uncertainty`.** The SDR uses this to know what NOT to
  over-personalise in outreach.

# Memory

Append committee findings to the `accounts` row's `contacts` array.
Role-only rows accumulate role-pattern intelligence across companies
without inflating the named-contact count.
