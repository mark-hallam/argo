# Stream P1-8 — finish skill_invoke step kind in vertical-workflow runtime

**Owner**: claude.
**Depends on**: mission-pm-skill-overlay-load MERGED (so the agent runner reads per-worker overlays during the one-shot turn).
**Authored against**: origin/main `80ffa124` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pm-vertical-workflow-skill-invoke`.
**Source**: skycot-pm-worker REPORT.md §3.1 + §11 P1 #8.

## Scope

Today `crates/argo-cli/src/handlers/vertical_workflow_step.rs:67-77` returns `status="deferred_no_llm"` for `skill_invoke` and `prompt` steps, with the comment `"PR #659 ships tool_call only"`. This brief lights up `skill_invoke` end-to-end so vertical workflows can chain skill calls.

Mechanic: when a `skill_invoke` step fires, the runner builds a one-shot agent-runner invocation with the named skill attached for that turn only. The turn runs to completion (or `max_steps=8` hard cap), and the result lands in the step outcome with the assistant's final content text.

`prompt` step kind is left as-is — same shape but requires more design (user-message injection semantics, conversation continuity across steps). Out of scope for this brief.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-cli/src/handlers/vertical_workflow_step.rs` — replace the `skill_invoke` stub at lines 67-73 with a real call into a new `run_skill_invoke_step` helper.
- `crates/argo-cli/src/handlers/vertical_workflow_skill_step.rs` (NEW) — the helper. Builds an `argo-agent-runner` invocation with the skill attached for one turn, hard-caps step count, captures the final text into `StepOutcome`. ≤ 300 LOC.
- `crates/argo-cli/src/handlers/vertical_workflow_tests.rs` — extend with a `skill_invoke` happy-path test (the file already exists per `ls`).
- `crates/argo-cli/tests/vertical_workflow_skill_invoke_smoke.rs` (NEW) — end-to-end smoke: a 2-step vertical workflow `[tool_call → skill_invoke]` runs both steps to success.

## Files you must NOT touch

- `crates/argo-vertical/src/workflow.rs` — the schema is locked; this brief only touches the runner.
- `crates/argo-agent-runner/` — use the existing `run_turn_with_events` (or similar) entry point. No agent-runner API changes.
- `crates/argo-cli/src/handlers/vertical_workflow_step.rs` outside the `skill_invoke` arm — keep `tool_call` and `prompt` paths exactly as they are.

## Acceptance criteria

- [ ] `skill_invoke` step kind runs to completion and emits `status="success"` or `status="failed"` (never `"deferred_no_llm"`).
- [ ] The named skill is attached to the one-shot turn via the existing per-session `attach_to_session`-equivalent path; not via `worker_skill` (overlays come automatically once mission-pm-skill-overlay-load is merged).
- [ ] Hard cap of 8 model steps per `skill_invoke` step. If the cap is hit, status is `"failed"` with message `"skill_invoke step exceeded max_steps=8"`.
- [ ] If the named skill doesn't exist in the active set (workspace + library + overlay), status is `"failed"` with message `"skill_invoke '<skill>' not found"`.
- [ ] Capability check fires per the existing dispatch loop — a skill that calls a denied tool still hits the deny path; the step bubbles the deny up.
- [ ] Audit rows: each `skill_invoke` step emits `WorkflowStepStarted` + `WorkflowStepCompleted` (NEW kinds — coordinate with P1 #6 / P0 #4 if they land first). Payload includes `(workflow_name, step_name, skill_name, final_text_sha)`.
- [ ] Tests: (1) happy path — skill produces final text; (2) skill-not-found path; (3) max-steps-exceeded path; (4) skill-calls-denied-tool path.
- [ ] Smoke test: `tests/vertical_workflow_skill_invoke_smoke.rs` runs the real argo binary against a fixture vertical with one `skill_invoke` step.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] `cargo test -p argo-cli` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-pm-vertical-workflow-skill-invoke`
- PR title: `vertical-workflow: skill_invoke step runs end-to-end (mission-pm-vertical-workflow-skill-invoke)`

## Notes for the implementer

- The one-shot turn must reuse the existing `argo-agent-runner` codepath — do NOT spin up a separate executor. The point is for skill_invoke to inherit all the same audit + capability + redaction behaviour as a normal agent turn.
- "Final text" capture: the agent-runner emits assistant messages as it streams; capture the LAST assistant-content text block before the loop exits. If the turn ended with a tool result instead of assistant text, status is `"failed"` with message `"skill_invoke ended without assistant text"`.
- Coordinate `AuditKind` additions with the P0 #4 + P1 #6 briefs. If those have merged first, this brief picks up the latest `KNOWN_KINDS.len()` value. Add `WorkflowStepStarted` + `WorkflowStepCompleted` here.
- Do NOT touch the `prompt` step kind — its design needs a separate conversation about user-message injection across steps. Leave the `"deferred_no_llm"` stub.
- The `depends_on` field in `WorkflowStep` is metadata-only today (per workflow.rs:92-95). Continue to walk steps in declaration order; DAG ordering is a separate brief.

## Out of scope

- `prompt` step kind — separate brief.
- DAG (parallel) execution — sequential walk is fine.
- Conversation continuity across steps (turn N's messages visible to turn N+1) — each `skill_invoke` is a fresh one-shot turn.
- Streaming step output to the operator (the step records a single text result at the end).
