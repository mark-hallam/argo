# Claude — SKYCOT operator runbook (v1 customer-launch doc)

**Owner**: claude. **Depends on**: claude-e2e-integration-test
merged (so the runbook references a green test path).

## Scope

A single Markdown document at `docs/skycot/RUNBOOK.md` that
walks a brand-new operator from install → first worker →
first approved action → audit / observability.

Audience: a developer or technical operator who's never
seen SKYCOT, evaluating whether to deploy it for their team.

Sections (each ~200-400 lines):
1. **What you're getting** — restate the mission +
   outcomes (link to MISSION.md).
2. **Install** — `cargo install argo-cli` (or the OCI
   pull command), required env vars, recommended
   directory layout.
3. **Bootstrap** — `argo bootstrap` (or its current
   equivalent — codex's H-doctor-extensions might rename
   this). Confirms providers, channels, vertical
   loadability.
4. **Generate your first worker** —
   `argo genesis create --charter "..."` with concrete
   examples (one for sales SDR, one for marketing intern,
   one for customer support). Show the generated
   tree + walk through what each file does.
5. **Configure approvals** — explain the workspace
   `auto_approval.toml`, the per-worker override (per
   claude-worker-instance-policy), and which actions
   require approval by default.
6. **Drive the first action** — emit, observe gate
   decision, observe channel notify, decide via web inbox
   /  email / slack / SMS / push, see the action
   transition through executing → completed.
7. **Skill learning loop** — when the worker completes a
   novel sequence successfully, see the proposal land in
   `pending_review`. Walk through `argo skill review
   approve`.
8. **Multi-worker handoff** — start a session, second
   worker joins, handoff with deadline, demonstrate the
   walker auto-escalating.
9. **Audit + observability** — `argo audit query`,
   `argo doctor`, the Prometheus dashboard suggestions.
10. **Where to next** — link to MISSION.md for the v1.x
    roadmap, contributing guide.

## Files you may touch
- `docs/skycot/RUNBOOK.md` (NEW)
- `docs/skycot/runbook-screenshots/*.png` — ONLY if
  trivially captured from existing TUI snapshot tests; do
  NOT take real screenshots from a running install (env
  setup is too brittle for v1 docs)

## Files you must NOT touch
- Any source crate
- `docs/skycot/MISSION.md` (link to it; don't restate)
- `BUILD_STATUS.md` / `streams/*.md`

## Acceptance criteria
- [ ] Document opens with the 4-stat headline (briefs,
      PRs, time, test status) just like the PROTOCOL.md
      style
- [ ] Each of the 10 sections is present + filled
- [ ] Every command shown actually exists in the v1 main
      (run `argo --help` for each subcommand to verify)
- [ ] Every config snippet is parseable (TOML / JSON
      validates)
- [ ] Errors are explicitly handled — when something
      could go wrong (missing env var, mis-spelled
      vertical id), document the symptom + fix
- [ ] No new Rust code — pure markdown + TOML / JSON
      examples

## Branch + PR
- Branch: `claude-skycot-runbook`
- PR title: `docs: SKYCOT operator runbook for v1 customer launch (claude-skycot-runbook)`

## Notes
- This is the document that goes to the FIRST CUSTOMER.
  Tone matters: confident, concrete, no marketing fluff.
- 1500-3000 lines of markdown is in range. Don't pad.
- If a feature mentioned in MISSION.md isn't yet on main,
  document it with a "v1.x" tag rather than fake it.
- Cross-reference PROTOCOL.md once — readers who want to
  understand the multi-agent coordination story can dive
  there.
