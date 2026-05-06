# V2.0 — Dreams plan-template learning (cross-pollination)

**Owner**: claude. **Depends on**: claude-runtime-planner-trait
(queued), claude-subtask-ledger-store (queued),
claude-dreams-skill-distillation (queued) merged. The most
novel proposal from the dreams investigation
(`DREAMS_AND_RUNTIME_SWARM.md` Section 5).

## Scope

Workers learn how to *plan better*, not just *act better*. Run
Dreams over PLANNER COHORTS — the subtask trees emitted by
the runtime Planner — and produce LearnedPlanTemplates that
the TemplatePlanner can use for future similar goals.

This is the cross-pollination between:
- claim 4 (skills shared and learned) — the learning loop
- claim 2 (delegation as the unit) — the planner-emitted
  decompositions

Mechanism:
- Nightly tick groups subtask_ledger rows by (parent_goal
  cosine-similarity, outcome=success)
- Dreams API distills cohort → polished plan template
- Template stored in `learned_plan_template` table; TrustGrade
  ladder applies (operator approves before promotion)
- TemplatePlanner consults `learned_plan_template` first;
  falls through to LlmPlanner only if no match

## Files you may touch
- `crates/argo-runtime/src/dreams_plan_templates.rs` (NEW)
- `crates/argo-runtime/src/planner.rs` — narrowly: hook
  TemplatePlanner to consult learned_plan_template before
  falling through
- `crates/argo-store/migrations/<next>__learned_plan_templates.sql`
  (NEW — pending-review-style table mirroring SkillCandidate)
- `crates/argo-store/src/learned_plan_template.rs` (NEW)
- `crates/argo-cli/src/handlers/plan_template_*.rs` (3 NEW —
  list/show/promote)
- `crates/argo-cli/src/cli_plan_cmds.rs` — narrowly: add
  `Template` sub-subcommand variants

## Acceptance criteria
- [ ] Cohort grouping over subtask_ledger by parent_goal
      similarity
- [ ] Dreams API call produces polished plan template
- [ ] Template persists with TrustGrade=Pending
- [ ] Operator promotion via `argo plan template promote <id>`
- [ ] TemplatePlanner uses promoted templates in priority
      order
- [ ] 10 unit tests + 1 integration smoke (full cohort →
      template → execute cycle)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-dreams-plan-templates`
- PR title: `runtime: Dreams plan-template learning — claim4×claim2 cross-pollination (claude-dreams-plan-templates)`
