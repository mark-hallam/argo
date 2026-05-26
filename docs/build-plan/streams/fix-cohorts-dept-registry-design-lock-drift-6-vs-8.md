# fix-cohorts-dept-registry-design-lock-drift-6-vs-8

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-cohorts-dept-registry-design-lock-drift-6-vs-8`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/whole-site-mvp-claims-coverage.md` F12 [P1].

🟠 **P1 — `dept-registry.ts` declares 8 departments. Locked D3 design memory says "6 dept tabs at the top." Spec/code drift; with current seed the drift is hidden (only Marketing renders) but once seed expands the design will deviate from the lock.**

## Root cause

`web/components/skycot/cohorts/dept-registry.ts:18-50` declares 8 departments: Support / Finance & Ops / Sales / Project & Ops / People & HR / IT & Dev-ex / Marketing / Legal. Memory `project_cohorts_page_design_locked` says "6 dept tabs at the top." Either:
- (a) The lock was authored against an earlier 6-dept set and the registry has since grown by 2; OR
- (b) The lock should be amended to 8 because the operator extended the design intent.

Operator decision required. Pending that, the registry should be aligned to whichever is authoritative.

## Code evidence

`web/components/skycot/cohorts/dept-registry.ts:18-50` — 8 entries.
Memory `project_cohorts_page_design_locked` says 6 tabs.

## Findings this brief closes

- whole-site-mvp-claims-coverage F12 [P1] — dept registry 8 vs locked design 6

## Fix recipe

1. **Operator confirm**: is it 6 (lock-authoritative) or 8 (registry-authoritative)? Ask before editing.
2. **If 6**: drop 2 departments from `dept-registry.ts`. Likely candidates to drop: Legal (currently empty in seed; v1 is enterprise-not-consumer per `project_v1_strategic_decisions_2026_05_14` so legal-ops cohorts could be People+HR sub-cohort), IT & Dev-ex (could merge into Project & Ops). Final 6: Support / Finance & Ops / Sales / Project & Ops / People & HR / Marketing.
3. **If 8**: amend the locked-design memory to 8 dept tabs. Update `feedback_*` memory file at `~/.claude/projects/-Users-operator-code-argo/memory/project_cohorts_page_design_locked.md` to mention 8 tabs.
4. **Pair with `fix-cohorts-empty-state-render-locked-d3-magazine-preview`** (already exists) — that brief handles the empty-state render for departments with no cohorts.
5. **Web test** at `web/tests/cohorts-dept-registry-canon.test.ts` asserts the registry exports exactly the operator-canonical count.

## Acceptance criteria

- [ ] Operator confirms canonical dept count (6 vs 8).
- [ ] `dept-registry.ts` matches the canonical count.
- [ ] If 8 chosen: the lock memory is updated to match.
- [ ] Web test asserts the registry exports exactly N entries.
- [ ] No visual regression on `/cohorts` with current seed.

## May-touch

- `web/components/skycot/cohorts/dept-registry.ts` (trim or keep)
- `~/.claude/projects/-Users-operator-code-argo/memory/project_cohorts_page_design_locked.md` (amend if 8)
- `web/tests/cohorts-dept-registry-canon.test.ts` (NEW)

## Out of scope

- Cohorts page magazine layout (already locked)
- Cohorts empty state (separate brief: `fix-cohorts-empty-state-render-locked-d3-magazine-preview`)
- Cohorts data seed (separate brief: `fix-demo-seed-cli-for-real-roster`)

## Effort

~30 LOC + 1 test + 1 memory edit. ~30min once operator confirms.
