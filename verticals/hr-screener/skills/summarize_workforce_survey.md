---
name: summarize_workforce_survey
description: Given a workforce-survey response set, produce sentiment themes + an anonymized quote bank (N>=5 cohort threshold) + top-3 action recommendations. Per-respondent attribution is never permitted; quotes from cohorts under 5 are suppressed entirely. Summarization is autonomous-safe.
inputs:
  - survey_responses
  - survey_metadata
  - cohort_breakdown
outputs:
  - sentiment_themes
  - anonymized_quote_bank
  - top_3_action_recommendations
  - suppressed_themes
  - flags
---

# Skill: Summarize Workforce Survey

Produce themes + an anonymized quote bank for one survey window.

1. **Sentiment themes.** Cluster responses into themes. For each
   theme, attach `respondent_count_for_theme` (the cohort size that
   surfaced the theme). Themes are descriptive ("communication
   cadence in remote teams"), not evaluative ("management is
   failing").
2. **N >= 5 anonymization threshold.** For each theme:
   - If `respondent_count_for_theme >= 5`, the theme is eligible
     for a quote in the anonymized quote bank.
   - If `respondent_count_for_theme < 5`, suppress the quote
     entirely and list the theme in `suppressed_themes` with
     `flags: ["respondent_count_below_threshold"]`. Do not
     paraphrase the contents; the contents are gone.
3. **Anonymized quote bank.** For each eligible theme, surface 1-2
   verbatim quotes with no attribution beyond the theme label.
   Strip names, role titles fine enough to identify, and
   demographic markers from quote bodies before surfacing.
4. **Named-individual handling.** If a respondent named a specific
   individual in their free text, redact to "an individual
   respondent" or "a named individual" in any draft. Never surface
   the name regardless of cohort size; flag
   `named_individual_in_survey_response` and escalate.
5. **Top-3 action recommendations.** Each recommendation cites the
   theme it addresses + the cohort size + a concrete next step
   with risk level. None of the recommendations may be one of the
   five gated actions without `requires_approval: true`.

Hard rules:

- **Per-respondent attribution is never permitted.** Themes
  summarize aggregates; even for an N>=200 theme, the quote bank
  attributes to the theme, not to a respondent.
- **Protected-class breakouts require the EEO-1 gate.** Splitting
  themes along race / gender / age / disability / religion /
  national-origin / pregnancy / veteran-status / genetic-info
  dimensions requires `eeo_approved_roles` + explicit EEO-1
  context. Without both, set
  `flags: ["protected_class_analysis_without_eeo_context"]`
  and refuse.
- **No outlier-quote shortcut.** A particularly vivid or moving
  quote from a cohort of 2 does not escape the N>=5 rule. The
  threshold is the threshold.
- **Cohort math is on respondents, not responses.** A respondent
  who left 4 comments still counts as 1 respondent for the
  threshold.
