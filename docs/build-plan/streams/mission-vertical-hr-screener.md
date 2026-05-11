# Mission load-bearing wire — HR-screener vertical

**Owner**: claude. **Depends on**: existing vertical-template scaffold (`verticals/recruiter-screener/` MERGED #1340 as the canonical non-discrimination pattern).
**Authored against**: origin/main `fef33298` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-hr-screener`.

> SOURCE: MISSION_BACKLOG.md Section 4 Claim 11 #157 ("HR-screener vertical"). **Tier A (vertical).** Internal-HR concerns: employee referral screening, policy-violation triage, exit-interview structuring, workforce-survey analysis. **EEOC + ADA compliance load-bearing** — sibling to recruiter-screener's hiring-side non-discrimination but covering EXISTING employees + workforce. Drafts only — every action that touches an employee record, a benefit, or a workforce-policy enforcement gates behind HR-officer + (often) legal review.

## Scope

NEW content-only `verticals/hr-screener/` (11 files, ≤ 500 LOC total — pattern budget: system ≤ 140, persona ≤ 35, skills 4×≤65, schema ≤ 35, auto_approval ≤ 25, inventory ≤ 55, vertical.toml ≤ 55, eval/seed.jsonl ≥ 9 fixtures):

- `vertical.toml` — id=`hr-screener`, version=`0.1.0`, optional bamboohr/workday/ms365 MCP, `[escalation].human_approval_required = ["modify_employee_record", "approve_pto_request", "initiate_disciplinary_action", "share_workforce_data_externally", "send_company_wide_communication"]`
- `prompts/system.md` — **EEOC + ADA load-bearing**: never analyze workforce data along protected-class lines (race / gender / age / disability / religion / national-origin / pregnancy / veteran-status / genetic-info — 9 categories) UNLESS the request explicitly comes from EEO-1 reporting context AND the operator is in `eeo_approved_roles` memory; treat all medical / disability accommodation requests as confidential (ADA-protected); never identify a specific complainant in workforce-survey analysis; escalation triggers (named-individual-in-survey-response / accommodation-request-without-interactive-process / retaliation-pattern-detected / wage-gap-by-protected-class / proposed-discipline-without-PIP-progression)
- `prompts/persona.md` — Sloane, HR business partner; assumes good faith from both employee + manager; never speculates beyond the documentation; explicitly names which laws apply (Title VII / ADA / FMLA / NLRA) when drafting actions
- 4 skills:
  - `triage_employee_concern` — concern report in; severity rating + escalation-Y/N w/ legal-citation + recommended-next-step + named-individual-redacted-summary out
  - `summarize_workforce_survey` — survey-response set in; sentiment-themes + anonymized-quote-bank + top-3-action-recommendations out (NEVER per-respondent attribution unless `respondent_count_for_theme >= 5`)
  - `draft_policy_clarification` — ambiguous-policy + scenario in; plain-language clarification + edge-cases-not-covered list + recommended-policy-update-language out
  - `structure_exit_interview` — outgoing-employee-context in; interview-question-set + suggested-tone-calibration + topics-to-handle-with-legal-present out
- `memory/schema.toml` — `policy_handbook` (current policy text + last-amended date), `eeo_approved_roles` (operator-ids approved for aggregate EEO-1 reporting view; tiny — typically 1-3 senior HR), `accommodation_history` (current-accommodation registry; HIPAA-sensitive — never echo into drafts), `survey_themes_cache` (rolling theme cluster from recent surveys for trend detection)
- `eval/seed.jsonl` — 9+ fixtures: 1 triage-low-severity (just-FYI), 1 triage-medium (PIP-recommended), 1 triage-high (potential-retaliation, escalate), 1 survey-summary-w-anonymization, 1 survey-w-N=3-quote-blocked, 1 policy-clarification, 1 exit-interview, 3 escalation (protected-class-analysis-without-EEO-context / accommodation-without-process / wage-gap-detected)
- `auto_approval.toml` — wake list includes the 4 triage/summarize/draft/structure skills only; modify_employee_record / approve_pto_request / initiate_disciplinary_action / share_workforce_data_externally / send_company_wide_communication intentionally absent
- `inventory.json` — sample `policy_handbook` (3 synthetic policies), sample `eeo_approved_roles` (1 entry), sample `accommodation_history` (2 synthetic — both anonymized), sample `survey_themes_cache` (3 rolling themes)

NO Rust. NO migrations. NO crate changes. Pattern is identical to the existing verticals — re-use shapes 1:1.

This is NOT in scope: actual HRIS connector wiring (BambooHR / Workday / ADP — separate MCP-connector briefs); benefits-administration workflows (separate; complex per-vendor); HRBP-on-call rotation logistics (separate); 401k / equity / compensation modeling (separate finance vertical); union-context labor relations (separate, NLRA-specific); the actual workforce-data store (a real impl would integrate with the HRIS; the vertical assumes a downstream tool provides aggregated data).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/hr-screener/vertical.toml` (NEW, ≤ 55 LOC)
- `verticals/hr-screener/prompts/system.md` (NEW, ≤ 140 LOC)
- `verticals/hr-screener/prompts/persona.md` (NEW, ≤ 35 LOC)
- `verticals/hr-screener/skills/triage_employee_concern.md` (NEW, ≤ 65 LOC)
- `verticals/hr-screener/skills/summarize_workforce_survey.md` (NEW, ≤ 65 LOC)
- `verticals/hr-screener/skills/draft_policy_clarification.md` (NEW, ≤ 65 LOC)
- `verticals/hr-screener/skills/structure_exit_interview.md` (NEW, ≤ 65 LOC)
- `verticals/hr-screener/memory/schema.toml` (NEW, ≤ 35 LOC)
- `verticals/hr-screener/eval/seed.jsonl` (NEW, ≥ 9 fixtures)
- `verticals/hr-screener/auto_approval.toml` (NEW, ≤ 25 LOC)
- `verticals/hr-screener/inventory.json` (NEW, ≤ 55 LOC)

## Files you must NOT touch

- Any `crates/**` Rust source
- `crates/argo-store/migrations/**`
- Any other `verticals/<other>/**` directory

## Acceptance criteria

- [ ] `argo vertical list` includes `hr-screener 0.1.0`
- [ ] `argo vertical lint hr-screener` exits 0 errors (warns of the recruiter-screener baseline shape are acceptable)
- [ ] `argo vertical show hr-screener --json` returns valid envelope (id / name / version / persona / memory_entries / eval_seed_count ≥ 9 / vertical_toml)
- [ ] All 4 skill files have YAML frontmatter (`name` + `description` + `inputs` + `outputs`)
- [ ] System prompt names EEOC + ADA + 9 protected classes explicitly; binds non-analysis-without-EEO-context rule to it
- [ ] N≥5 anonymization threshold for survey-respondent attribution is named in the system prompt + tested in eval fixtures
- [ ] `eval/seed.jsonl` has ≥ 9 fixtures, ≥ 3 of which are escalation cases (protected-class-without-EEO / accommodation-without-process / wage-gap)
- [ ] `auto_approval.toml` wake list contains only the 4 triage/summarize/draft/structure skills; modify / approve / initiate / share / send gated through `[escalation].human_approval_required`
- [ ] No Rust changes; no migrations
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-hr-screener`
- PR title: `vertical: hr-screener (EEOC/ADA drafts-only triage + survey + policy + exit) (mission-vertical-hr-screener)`

## Notes for the implementer

- Treat the just-merged `verticals/recruiter-screener/` (#1340) as a 1:1 template for the non-discrimination pattern — but extend to EXISTING-employee concerns instead of hiring. The 9 EEOC protected classes are the load-bearing list; document them by name in the system prompt.
- N≥5 anonymization is a standard threshold for survey-respondent attribution (smaller cohorts can be deanonymized via demographic inference). The vertical enforces this at the prompt level; eval fixture #5 tests that an N=3 response is correctly blocked from quoting.
- ADA + accommodation requests are CONFIDENTIAL — even within HR. The vertical should never echo accommodation-history data back into a draft; eval fixture should test this.
- "Wage-gap by protected class" is a SPECIFIC ESCALATION — not a normal analysis. If the request even touches this territory, the vertical refuses + escalates with the legal citation. This is the "you must have a comp-equity audit running and be approved, not just doing exploratory analysis" guardrail.
- Sloane's persona: gives the law a name. "Per Title VII §703," "Per ADA Title I," etc. This isn't legal-advice (that's UPL — see legal-paralegal); it's transparency about which framework the recommendation rests on.
- The `eeo_approved_roles` memory list is intentionally TINY — 1-3 senior HR officers — because the protected-class analysis surface is the most sensitive in the vertical. Document this in module/inventory docs.
- DO NOT add a "predict who will quit" feature. That's predictive HR analytics — covered separately if needed, and ethically fraught (privacy + bias risk). Out of scope.
- The exit-interview skill should specifically call out which topics need legal-counsel-present (severance terms, post-employment competition, whistleblower-context, retaliation-context) — that's the load-bearing differentiator.
