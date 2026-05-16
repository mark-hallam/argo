# claude-launch-coordination-playbook — ProductHunt + HackerNews + Tech Twitter coordination (Wave 4 P2)

**Owner**: claude. **Depends on**: every other Wave 1-4 brief MERGED.
**Authored against**: origin/main `489e0580` on `2026-05-16`.

## Scope

P2. Launch-coordination playbook for the moment operator decides to push for the viral wave (post-weekend, post-design-partner-validation).

Contents:
- Pre-launch checklist (validate every gate green; load-test the hosted instance; confirm support staffing).
- ProductHunt launch day plan: 12am PT Tuesday post; founder + advisors voting; first-hour engagement; hourly updates.
- HackerNews timing: 7am PT, submitting from a credible account (operator's), founder in comments responsiveness.
- Tech Twitter (X): 5 prepared threads; coordinated drop with advisors/network.
- Press list: 10 publications + journalists who cover AI/agent space.
- War-room readiness: Discord + ticket queue + ops dashboard.

Deliverable: docs only (operator-executable plan).

LOC: ~150.

## Files you may touch

- `docs/skycot/launch/README.md` (NEW). ≤80 LOC.
- `docs/skycot/launch/pre-launch-checklist.md` (NEW). ≤80 LOC.
- `docs/skycot/launch/producthunt-plan.md` (NEW). ≤80 LOC.
- `docs/skycot/launch/hackernews-plan.md` (NEW). ≤60 LOC.
- `docs/skycot/launch/tech-twitter-threads.md` (NEW) — 5 thread drafts. ≤120 LOC.
- `docs/skycot/launch/press-list.md` (NEW) — operator fills with their relationships. ≤30 LOC.
- `docs/skycot/launch/war-room-runbook.md` (NEW). ≤80 LOC.

## Acceptance criteria

- [ ] Full playbook documented; operator can execute without further authoring.
- [ ] Pre-launch checklist covers every Wave 1-4 ship gate.
- [ ] 5 Tech Twitter threads drafted (operator approves/redlines).
- [ ] War-room runbook addresses: signup-rate spike, error-rate spike, provider-quota exhaustion, DoS.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-launch-coordination-playbook`
- PR title: `docs: launch coordination playbook (claude-launch-coordination-playbook)`

## Notes

- This brief is the LAST one. Authored after all other waves to capture lessons learned in the playbook.
- Don't launch publicly until: 3 design partners onboarded + 30-day signup activation rate > 50% + zero P0/P1 audit findings open.
