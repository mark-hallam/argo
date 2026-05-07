# V1.1 — Worker handoff policy `.skycot/handoff-policy.toml`

**Owner**: claude. **Depends on**: handoff_routing migration 23 + 27
`handoff_auctions` already present on main.

## Why

Workers handing off too eagerly is a real failure mode — the AR clerk
escalates every aging invoice to the senior accountant, defeating the
point of delegation. Workers handing off too rarely is the opposite
failure — the entry-level worker churns retries instead of asking
for help.

There's no single workspace-level knob today. Routing weights exist
per-worker in `handoff_routing` but nothing answers "should we hand
off **at all** for this kind of situation?"

## Scope

- File: `.skycot/handoff-policy.toml` at workspace root.
  ```toml
  [defaults]
  inline_max_retries = 2
  inline_max_latency_seconds = 30
  escalate_on_low_confidence = 0.6

  [[rules]]
  match = { action_kind = "approval_request" }
  policy = "always_escalate"

  [[rules]]
  match = { action_kind = "send_email", confidence_lt = 0.8 }
  policy = "escalate_to_senior"

  [[rules]]
  match = { action_kind = "*", retry_count_gte = 3 }
  policy = "escalate"
  ```
- New module `crates/argo-cli/src/handlers/handoff_policy.rs` (NEW) —
  loader + match engine.
- Wired into `argo-orchestrator` handoff entry point — narrowly: a
  hook that consults the policy before falling back to existing
  routing weights.
- CLI:
  - `argo handoff policy show [--explain <action-id>]` — print
    parsed policy + (with `--explain`) trace match decision
  - `argo handoff policy validate [<file>]` — schema check

## Approach

1. `handoff_policy.rs` (NEW, ≤ 420 LOC) — TOML parser, rule struct,
   `Decision::{ Inline, Escalate, EscalateTo(WorkerId) }` enum,
   first-match-wins evaluator.
2. Orchestrator hook: 1-line addition where handoff currently
   evaluates routing weights — call policy first, fall back if
   decision is `Inline` or no rules match.
3. Glob `*` for action_kind = match-all wildcard.

## Files you may touch

- BLANKET CLI ALLOWLIST applies
- EXHAUSTIVE-MATCH amendment applies
- `crates/argo-cli/src/handlers/handoff_policy.rs` (NEW)
- `crates/argo-cli/src/cli.rs` — narrowly: `Handoff { Policy { Show,
  Validate } }` (or extend existing handoff section)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 pub mod line
- `crates/argo-orchestrator/src/handoff.rs` — narrowly: 1 hook call,
  ≤ 15 LOC delta. Document the addition with a `// policy hook`
  comment.
- `docs/skycot/handoff-policy.example.toml` (NEW) — annotated example

## Files you must NOT touch

- Migrations — policy is a file, not a table (intentional: operator
  edits + version-controls it)
- handoff_routing / handoff_auctions tables — those stay as fallback
- TUI — out of scope

## Acceptance criteria

- [ ] Parser accepts the example file; rejects malformed `policy =`
      with clear error
- [ ] First-match-wins: order matters
- [ ] `*` wildcard for action_kind
- [ ] `confidence_lt`, `retry_count_gte` numeric matchers work
- [ ] `argo handoff policy show --explain <action>` prints which rule
      (or "no match — using routing weights") fired
- [ ] No-policy-file case = current routing-weights behaviour
      (graceful absence)
- [ ] 10 unit tests (parse, match precedence, wildcard, numeric
      matchers, missing file fallback) + 2 CLI smoke + 1 orchestrator
      integration
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-worker-handoff-policy`
- PR title: `handoff: workspace .skycot/handoff-policy.toml with match engine (claude-worker-handoff-policy)`

## Notes

- The orchestrator hook is the riskiest part — keep the patch tiny
  and prove with the integration test that no-policy = current path.
- Reading `.skycot/handoff-policy.toml` from workspace root: reuse
  existing workspace-root resolution helpers in `argo-cli` (search
  for `find_workspace_root` or similar).
