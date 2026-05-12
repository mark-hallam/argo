# Mission load-bearing wire — Tax-Prep vertical: 5+ skills

**Owner**: claude. **Depends on**: existing `verticals/tax-prep/` charter stub.
**Authored against**: origin/main `e2441425` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-tax-prep-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137, scoped to tax-prep. Today the vertical ships with 1 skill (`deduction_eligibility.md`). MISSION claim 7 expects parity-with-gtm-engineer. This brief brings tax-prep to 6 skills. **IRS-filing safety is the load-bearing constraint** — every client- or IRS-facing artifact is approval-gated (8879 mapping per charter).

## Scope

5 NEW skill files under `verticals/tax-prep/skills/`:

1. **`review_w2.skill.md`** — given an uploaded W-2 image/PDF (operator-attached), extract line items (wages, withholding, employer, employee SSN-last4 only, etc.). Output JSON. **Autonomous per charter** (no approval gate). Marks `extraction_confidence` per field; low confidence routes to operator review.
2. **`draft_return.skill.md`** — given a client's gathered docs (W-2 line items, 1099 summaries, deduction proposals from `deduction_eligibility`), draft a 1040 + schedules. Output structured JSON mapping to form lines + Markdown summary of "what changed since last year" + "what we couldn't determine". Approval-gated (no auto-file).
3. **`schedule_review.skill.md`** — propose 3 meeting times for client review of the draft, factoring preparer availability + appointment-type duration + time-zone. Autonomous per charter. Output JSON.
4. **`file_return.skill.md`** — package the approved return for IRS e-file. **Always approval-gated** — maps to the 8879 e-file authorization signature. Output: filing envelope with status + IRS transmitter receipt placeholders. Refuses if any line is `confidence: low` or any deduction `requires_approval: true` wasn't approved.
5. **`flag_amendment_opportunity.skill.md`** — when reviewing prior-year returns, flag potential amendment opportunities (missed deductions, life-event timing, retroactive credits). Output: amendment_envelope with: prior_form, change_category (deduction / credit / income / status), estimated_refund_change, 3-year-statute-of-limitations check. Approval-gated.

Each skill ~50-90 LOC. Total: ~350-450 LOC.

This is NOT in scope: real IRS e-file API integration; OCR engine selection (operator-provided extraction); state returns (federal only in v1); business returns (Schedule C only — no 1120/1065).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/tax-prep/skills/review_w2.skill.md` (NEW, ≤ 90 LOC)
- `verticals/tax-prep/skills/draft_return.skill.md` (NEW, ≤ 90 LOC)
- `verticals/tax-prep/skills/schedule_review.skill.md` (NEW, ≤ 70 LOC)
- `verticals/tax-prep/skills/file_return.skill.md` (NEW, ≤ 90 LOC)
- `verticals/tax-prep/skills/flag_amendment_opportunity.skill.md` (NEW, ≤ 90 LOC)

## Files you must NOT touch

- `verticals/tax-prep/skills/deduction_eligibility.md` — existing skill stable
- `verticals/tax-prep/vertical.toml`, prompts — locked
- Any other vertical's skill directory

## Acceptance criteria

- [ ] 5 new `.skill.md` files
- [ ] Each has frontmatter: `name`, `version: "0.1.0"`, `description`, `provenance: { source: vertical/tax-prep, hash: sha256-pending }`
- [ ] Each has sections: `# When to use`, `# Inputs you expect`, `# Output shape`, `# Examples`, `# Approval posture`
- [ ] `review_w2` + `schedule_review` marked autonomous per charter; other 3 marked `requires_approval: true`
- [ ] `file_return` explicitly refuses on low-confidence lines or un-approved deductions (cite the 8879 mapping)
- [ ] `flag_amendment_opportunity` includes the 3-year-statute-of-limitations check
- [ ] No skill auto-files with IRS (no `requires_approval: false` on `file_return`)
- [ ] No real PII in examples (synthetic SSNs only)
- [ ] No file > 100 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-tax-prep-skills`
- PR title: `verticals: 5 skills for tax-prep (mission-vertical-tax-prep-skills)`

## Notes for the implementer

- The 8879 e-file authorization is the IRS form that maps "the human approved this filing". `file_return` should explicitly cite 8879 in its docstring as the load-bearing safety mapping.
- `review_w2` is autonomous because the data is already on the operator's machine (no client-facing effect). But low extraction confidence MUST route to operator review — don't silently file a W-2 line at 30% confidence.
- `flag_amendment_opportunity` is the highest-value autonomous-discovery skill. Tax preparers leave ~20% of legal refund money on the table by not amending. The 3-year SOL gate is the only timing constraint; flag everything else.
- The charter's "mixed-trust" framing is operator-meaningful — document each skill's approval posture in a clear `# Approval posture` section. Don't bury it in the example.
- Synthetic SSNs: use `123-45-6789` or the IRS-published test SSNs. Never put a real-looking pattern in examples.
