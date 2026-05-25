# fix-onboarding-welcome-soc2-day-one-copy

**Owner**: claude or codex.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-welcome-soc2-day-one-copy`.
**Source**: e2e-ux-audit Agent 1 finding F7 (`docs/skycot/ux-audits/2026-05-25-1100-onboarding-hire-an-ai-teammate.md`).

🟡 **P1 — compliance overclaim language at `/onboarding/welcome`: "SOC 2-ready from day one".**

## Root cause

`web/app/(setup)/onboarding/welcome/page.tsx:40` pillar card body: "Every action signed and replayable — SOC 2-ready from day one."

SOC 2 readiness is a multi-month audit, not "day one". Operator memory `feedback_no_theater_in_demo` (operator-locked 2026-05-24) flags compliance-overclaim language as a category. This was previously called out as F16 in the prior audit and is carrying forward.

## Findings this brief closes

- Agent 1 F7 [P1] — "SOC 2-ready from day one" copy on `/onboarding/welcome` overclaims (carries from prior audit F16)

## Fix recipe

1. Change the line at `web/app/(setup)/onboarding/welcome/page.tsx:40` from:
   > "Every action signed and replayable — SOC 2-ready from day one."
   
   to:
   > "Every action signed and replayable — SOC 2-ready audit trail."
   
   (Matches the landing trust pill copy — "SOC 2-READY AUDIT TRAIL". Drops the "from day one" overreach.)
2. Sweep `grep -r "SOC 2-ready from day one\|SOC 2 ready from day one\|SOC2 from day one" web/` for any sibling occurrences.

## Acceptance criteria

- [ ] No occurrence of "SOC 2-ready from day one" (or variants) anywhere in `web/`.
- [ ] The pillar card copy reads "SOC 2-ready audit trail" instead.
- [ ] Snapshot test on `/onboarding/welcome` pins the new copy.
- [ ] Sweep clean: `grep -ri "soc.*2.*day one" web/` returns zero matches.

## May-touch

- `web/app/(setup)/onboarding/welcome/page.tsx:40`
- Any sibling pages found in the sweep
- Snapshot test

## Out of scope

- Other compliance copy elsewhere (sweep only — but this brief doesn't catalog every compliance claim)
- Trust-pillar visual design (locked per `project_onboarding_design_locked`)

## Effort

~5-15 LOC. ~10-15 min.
