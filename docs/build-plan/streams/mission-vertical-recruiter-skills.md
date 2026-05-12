# Mission load-bearing wire — Recruiter vertical: 5+ skills

**Owner**: claude. **Depends on**: existing `verticals/recruiter/` charter stub. Sibling-style of `verticals/recruiter-screener/` (4 existing skills) — `recruiter` is the umbrella vertical, `recruiter-screener` is the inbound-screening narrower variant.
**Authored against**: origin/main `d866e01c` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-recruiter-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137, scoped to recruiter. Today the vertical ships with 1 skill (`screening_rubric.md`). MISSION claim 7 expects parity-with-gtm-engineer. This brief brings recruiter to 6 skills.

## Scope

5 NEW skill files under `verticals/recruiter/skills/`. Skills cover the dominant recruiter flow not already covered by `screening_rubric`:

1. **`score_resume.skill.md`** — given a resume + job rubric, score against rubric dimensions (skills match / experience / progression / culture-fit-signals / red-flags). Output JSON: `{overall_score: 0-100, dimension_breakdown, recommended_action: advance|reject|hold-for-review, rationale, red_flags: [...]}`. Approval-gated before sending advance/reject signals.
2. **`draft_screening_email.skill.md`** — given a candidate + screening-question set, draft an outreach email matching the operator's brand voice. Templates by stage (initial-outreach / follow-up / scheduling). Approval-gated before sending.
3. **`schedule_interview.skill.md`** — given candidate availability + panel availability + interview type (phone-screen / technical / panel / hiring-manager), propose 3 slot options factoring time-zone, panel-conflict-free, prep-time-buffer. Output JSON. Approval-gated before booking.
4. **`flag_offer_submission.skill.md`** — when an offer is being prepared, package the data for human approval: candidate profile + offer details (comp / start-date / role / level) + comp benchmarks + risk flags (counter-offer-likely / compete-with-competitor / outlier-comp). Always routes to a human approval — no auto-send.
5. **`update_candidate_pipeline.skill.md`** — persist candidate state transitions: sourced → contacted → responded → screened → onsite → offered → hired / passed / withdrew. Writes to `memory.candidate_pipeline[]` with reasoning + last_touched_at. Shape compatible with `recruiter-screener` for hand-off between the two verticals.

Each skill ~50-90 LOC. Total: ~350-450 LOC.

This is NOT in scope: live ATS integration; live calendar write (separate Tier A connector); auto-sending candidate-facing emails (charter mandates approval-gating); offer-letter generation (separate brief).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/recruiter/skills/score_resume.skill.md` (NEW, ≤ 90 LOC)
- `verticals/recruiter/skills/draft_screening_email.skill.md` (NEW, ≤ 80 LOC)
- `verticals/recruiter/skills/schedule_interview.skill.md` (NEW, ≤ 90 LOC)
- `verticals/recruiter/skills/flag_offer_submission.skill.md` (NEW, ≤ 90 LOC)
- `verticals/recruiter/skills/update_candidate_pipeline.skill.md` (NEW, ≤ 90 LOC)

## Files you must NOT touch

- `verticals/recruiter/skills/screening_rubric.md` — existing skill stable
- `verticals/recruiter/vertical.toml` — schema is fine
- `verticals/recruiter/persona.toml`, `system_prompt.md` — locked
- `verticals/recruiter-screener/**` — sibling vertical, different scope; don't drift its skills

## Acceptance criteria

- [ ] 5 new `.skill.md` files
- [ ] Each has frontmatter: `name`, `version: "0.1.0"`, `description`, `provenance: { source: vertical/recruiter, hash: sha256-pending }`
- [ ] Each has sections: `# When to use`, `# Inputs you expect`, `# Output shape`, `# Examples`
- [ ] `score_resume` outputs deterministic dimension scores (0-100 per dimension) + an explicit `red_flags` array (separate from low scores)
- [ ] `score_resume` advance/reject signals are marked `requires_approval: true`
- [ ] `draft_screening_email` includes the 3 stage templates explicitly (initial-outreach / follow-up / scheduling)
- [ ] `flag_offer_submission` always routes to human approval (no auto-send path)
- [ ] `update_candidate_pipeline` 8-state vocabulary documented + shape-compatible with `recruiter-screener` (verify by reading recruiter-screener's skills)
- [ ] Synthetic candidate data only — no real PII in `# Examples`
- [ ] No file > 100 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-recruiter-skills`
- PR title: `verticals: 5 skills for recruiter (mission-vertical-recruiter-skills)`

## Notes for the implementer

- Read `verticals/recruiter-screener/skills/*` BEFORE writing — the 8-state pipeline vocabulary and the screening-question shape should match. Cross-vertical hand-off only works if the shapes are stable.
- The recruiter persona is "thorough + bias-aware". Skills should reinforce both:
  - Thorough: `score_resume` shows the breakdown, not just a single number.
  - Bias-aware: `score_resume` red_flags should be evidence-based (resume gaps, job-hopping pattern, etc.) — NEVER demographic proxies. Document this constraint in `# Hard rules`.
- `flag_offer_submission` is the load-bearing safety skill. Comp negotiations + counter-offer prediction are high-stakes. The skill outputs an envelope for the human; never proposes offer numbers directly. The risk_flags array is the safety belt.
- `schedule_interview` must respect time-zone — every proposed slot includes the candidate's TZ + every panelist's TZ + the local clinic-time wall-clock. Don't drop TZ info in the output; downstream calendar tooling needs it.
- For `update_candidate_pipeline`, the 8-state vocabulary maps to a typed enum in `argo-store` if/when a CandidatePipeline schema lands. For now, document the names in the skill so future schema work has a source of truth.
- Demo readiness: a working recruiter demo runs `score_resume` → (if advance) `draft_screening_email` → (approval) → `schedule_interview` → (post-interview) `update_candidate_pipeline(onsite)` → eventually `flag_offer_submission`. The 5 new skills + the 1 existing skill cover that chain.
