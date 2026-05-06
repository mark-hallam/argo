# V1.3 — Handoff routing rules

**Owner**: claude. **Depends on**: H-orchestrator-persist
(#806), claude-orchestrator-handoff-walker (#825) merged.

## Scope

Today, handoffs explicitly name a target Worker (or escalate
through a chain). Real teams want **declarative routing
rules** that pick the target based on context: "marketing
copy review → first available worker with `marketing-editor`
skill", "after-hours support escalation → on-call
worker for current rotation".

- New `HandoffRoutingRule` primitive in argo-orchestrator:
  `rule_id, name, condition (DSL), target_selector
  (skill_match / role_match / round_robin / specific_worker),
  priority, active`
- DSL is YAML / TOML, similar shape to claude-policy-as-code:
  ```yaml
  rules:
    - name: copy-review
      when: { handoff.expected_outcome: "copy review" }
      then: { skill_match: "marketing-editor", first_available: true }
    - name: after-hours-escalation
      when: { hour_of_day_utc_in: [22, 23, 0, 1, 2, 3, 4, 5] }
      then: { role_match: "on-call-sre" }
  ```
- New CLI: `argo handoff rules list/show/add/remove/test`
  (`test` simulates rule application against a hypothetical
  handoff)
- HandoffWalker (existing #825) consults rules in priority
  order before falling back to explicit target

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-orchestrator/src/routing.rs` (NEW)
- `crates/argo-orchestrator/src/lib.rs` — narrowly: re-export
- `crates/argo-orchestrator/src/handoff_walker.rs` —
  narrowly: insert routing-rule consult before existing path
- `crates/argo-store/migrations/<next>__handoff_routing.sql`
  (NEW)
- `crates/argo-store/src/handoff_routing.rs` (NEW)
- `crates/argo-cli/src/handlers/handoff_rules_*.rs` (5 NEW)
- `crates/argo-cli/src/cli_handoff_cmds.rs` — narrowly: add
  `Rules` sub-subcommand variants

## Acceptance criteria
- [ ] YAML/TOML rule DSL parses + validates
- [ ] HandoffWalker honors rules in priority order
- [ ] `argo handoff rules test` simulates without persisting
- [ ] All 5 subcommands + `--json` universal
- [ ] 12 unit tests (DSL + selector logic) + 1 integration
      test (rule fires and walker routes correctly)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-handoff-routing-rules`
- PR title: `orchestrator: declarative handoff routing rules (claude-handoff-routing-rules)`
