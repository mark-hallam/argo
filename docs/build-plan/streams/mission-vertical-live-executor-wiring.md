# Mission load-bearing wire — Live-executor wiring per charter-stub vertical

**Owner**: claude. **Depends on**: per-vertical skill-expand briefs MERGED (skills exist) + eval-suite-expand brief MERGED (coverage).
**Authored against**: origin/main `1cca6a69` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 4 #142. Charter-stub verticals declare action types in their `vertical.toml` + skill files but don't wire any to a live executor (sandbox / MCP connector / channel). To call a vertical "shipped" we need ≥1 approval-gated live-executor action working end-to-end per vertical.

## Scope

For each charter-stub vertical, wire ONE primary action to a live executor (the demo-ready surface):

- **customer-support**: `send_reply` action → email/Slack channel send (approval-gated)
- **marketing-intern**: `draft_social` action → Slack message draft (operator copies to social) — emphasize draft-only, no auto-publish
- **sales-sdr**: `send_outreach` action → email channel send (approval-gated)
- **finance-ar**: `send_reminder` action → email channel send (approval-gated)
- **healthcare-intake**: `send_pre_visit_prep` action → SMS channel send (approval-gated, PHI-redacted body)
- **recruiter**: `send_screening_email` action → email channel send (approval-gated)
- **tax-prep**: `schedule_review` action → calendar MCP write (approval-gated)
- **product-manager**: `draft_prd` action → Linear/Notion MCP write (approval-gated)
- **real-estate-agent**: `schedule_viewing` action → calendar MCP write (approval-gated)
- **lead-research**: `draft_research_brief` action → document storage write (Dropbox/Box/OneDrive)
- **ops-on-call**: `draft_incident_update` action → Slack channel send (approval-gated)

Each wiring is a one-action surface — the rest stay in `dry_run` mode. Operator sees end-to-end demos for each vertical.

This is NOT in scope: wiring every action (one per vertical is the v1 ship gate); per-vertical demo recording (sibling brief); marketplace-seller-browser already lives in browser-executor pattern.

## Files you may touch

- `verticals/<vertical>/vertical.toml` per named vertical — add `[actions.<name>] executor = "..."` declaration
- `verticals/<vertical>/auto_approval.toml` per named vertical — set the action to `requires_approval = true` (most are already so via charter)
- Per-vertical eval fixture for the live-wired action — add `verticals/<vertical>/eval/live_executor_smoke.jsonl` (≤ 50 LOC each) gated by a feature flag so CI doesn't run live by default

## Files you must NOT touch

- The executor implementations themselves (assumed; if missing for a given action type, file a precursor brief and partial-deliver this brief)
- Per-vertical skills (sibling briefs)

## Acceptance criteria

- [ ] 11 verticals each have ≥1 live-wired action declared in vertical.toml
- [ ] Each live action is approval-gated by default
- [ ] Per-vertical live-executor smoke fixtures present (gated by env flag)
- [ ] `argo vertical lint <each>` passes
- [ ] BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-vertical-live-executor-wiring`
- PR title: `verticals: live-executor wiring per charter-stub (mission-vertical-live-executor-wiring)`

## Notes

- This brief is most valuable AS the v1 demo-recording prep — each vertical needs ONE thing to demo end-to-end. The wired action is that thing.
- Don't over-promise. v1 wires ONE action per vertical. The other actions stay dry-run and ship in follow-up briefs.
- Approval-gating per charter is non-negotiable. Even draft-only marketing-intern posts go through approval (operator copies to social manually); this is the safety posture.
