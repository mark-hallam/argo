# Mission load-bearing wire — `recruiter-screener` vertical

**Owner**: claude. **Depends on**: existing vertical pattern (verified at `verticals/marketplace-seller-browser/` + 19 shipped verticals).
**Authored against**: origin/main `6960eb12` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-recruiter-screener`.

> SOURCE: MISSION_BACKLOG.md Section 11 #144 — "Recruiter-screener vertical." Resume triage + initial outreach + scheduling assistance for inbound applicants. Gates anything spending budget or sending external mail behind operator approval.

## Scope

NEW `verticals/recruiter-screener/` (content-only) mirroring `verticals/marketplace-seller-browser/`:

1. `vertical.toml`:
   - `[vertical].id = "recruiter-screener"`, version `0.1.0`
   - `[role]` persona = senior in-house recruiter, calibrated, non-discriminatory
   - `[tools].allowed` = `["web_search", "web_extract", "file_read"]`
   - `[mcp_servers]` = `{ greenhouse = {...}, gmail = {...} }` (both optional)
   - `[escalation].human_approval_required` = `["send_outreach_email", "advance_to_phone_screen", "reject_candidate"]`
   - `[escalation].escalate_after_failures = 2`

2. `prompts/system.md` (≤ 200 LOC) — covers: role-fit rubric, calibration anchors per seniority, explicit non-discrimination policy (no protected-class signals influence ranking), citation-grade reasons for advance/reject decisions, when to NEVER auto-decide (any borderline candidate, anyone with disability disclosure, anyone outside US/CA jurisdictions without legal review).

3. `prompts/persona.md` (≤ 100 LOC).

4. `skills/score_resume.md` — given JD + resume, produce 1-3-5 fit score + 3 specific match reasons + 3 gap notes.

5. `skills/draft_outreach.md` — given candidate context + JD, produce personalized outreach (90s read).

6. `skills/find_screening_slot.md` — given candidate's availability hints + recruiter's calendar, propose 3 slots.

7. `skills/calibrate_against_pipeline.md` — given this candidate + 5 prior similar candidates' outcomes, flag whether the score is consistent.

8. `memory/schema.toml` — namespaces: `pipeline_history`, `jd_calibrations`, `rejected_signals` (non-PII summary patterns).

9. `eval/seed.jsonl` (≥ 7 fixtures): 3 clear-advance, 2 borderline-escalate, 2 reject-needed.

10. `auto_approval.toml` — scoring + calibration auto. Outreach + advance + reject gated.

11. `inventory.json` — sample JD calibrations for v1 demo.

This is NOT in scope: ATS deep-integration beyond Greenhouse adapter; offer-negotiation logic; reference-check automation; sensitive-disclosure handling (always escalates per system prompt).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/recruiter-screener/vertical.toml` (NEW)
- `verticals/recruiter-screener/prompts/system.md` (NEW, ≤ 200 LOC)
- `verticals/recruiter-screener/prompts/persona.md` (NEW, ≤ 100 LOC)
- `verticals/recruiter-screener/skills/*.md` (4 NEW skill files, ≤ 80 LOC each)
- `verticals/recruiter-screener/memory/schema.toml` (NEW, ≤ 40 LOC)
- `verticals/recruiter-screener/eval/seed.jsonl` (NEW, ≥ 7 fixtures)
- `verticals/recruiter-screener/auto_approval.toml` (NEW, ≤ 40 LOC)
- `verticals/recruiter-screener/inventory.json` (NEW, ≤ 80 LOC)

## Files you must NOT touch

- Any other `verticals/<name>/` directory
- Any `crates/**` source
- `crates/argo-store/migrations/**`

## Acceptance criteria

- [ ] `argo vertical list` includes `recruiter-screener`
- [ ] `argo vertical lint recruiter-screener` reports no errors
- [ ] `argo vertical show recruiter-screener --json` returns valid envelope
- [ ] All 4 skill files have correct frontmatter
- [ ] eval/seed.jsonl has ≥ 7 fixtures
- [ ] auto_approval.toml gates outreach + advance + reject behind human
- [ ] No code changes
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-recruiter-screener`
- PR title: `vertical: recruiter-screener (mission-vertical-recruiter-screener)`

## Notes for the implementer

- The non-discrimination policy in system.md is load-bearing. Be explicit: no protected-class signals (age, gender, race, disability, national origin, religion, pregnancy status, marital status, etc.) influence ranking.
- For eval/seed.jsonl: use anonymized resumes ("5 years backend, Python+Postgres, contributed to PR #X on open-source"). No real names.
- The "escalate any borderline candidate" criterion is intentional — false-negatives are cheap, false-rejects cost the candidate.
- Calibration skill works against `pipeline_history` namespace — operators populate over time.
