# V2.1 — Policy-as-code (declarative org policies)

**Owner**: claude. **Depends on**: H-auto-approval merged.

## Scope

Today's auto-approval policy is per-worker (TOML config,
imperative). Enterprise adoption needs **declarative org-wide
policies** that workers inherit + can't easily override.

This brief introduces a `Policy` primitive: a YAML/TOML
declarative file at `<workspace>/policies/<name>.yaml`
describing rules like:

```yaml
# /workspace/policies/finance-strict.yaml
applies_to:
  - role: finance-*
  - tenant: acme-*
rules:
  - action_kind: stripe_charge
    require_signatures: 2
    quiet_hours: { start: "18:00", end: "09:00", behavior: "block" }
    spend_cap_per_action_usd: 100
  - action_kind: gmail_send
    require_signatures: 1
    quiet_hours: { behavior: "queue_until_morning" }
  - action_kind: "*"
    audit_chain_required: true
```

Worker policy resolution order:
1. Action-specific rule
2. Worker explicit policy
3. Roster policy (if claude-worker-roster lands)
4. Workspace-wide policy file
5. Workspace default

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- EXHAUSTIVE-MATCH amendment applies
- `crates/argo-identity/src/policy.rs` (NEW or extend)
- `crates/argo-identity/src/lib.rs` — narrowly: re-export
- `crates/argo-store/migrations/<next>__policy_files.sql`
  (NEW — track policy files registered to a workspace)
- `crates/argo-cli/src/handlers/policy_*.rs` (4 NEW —
  validate/list/show/apply)
- `crates/argo-cli/src/cli_policy_cmds.rs` (NEW)
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly:
  register `Policy` subcommand

## Acceptance criteria
- [ ] YAML schema parser + validator
- [ ] Policy files persisted + registered to workspace
- [ ] Auto-approval gate consults policy chain in order
- [ ] `argo policy list/show/validate/apply <path>`
- [ ] `--json` universal output
- [ ] 12 unit tests + 1 integration test (full chain resolution)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-policy-as-code`
- PR title: `identity: declarative policy-as-code (claude-policy-as-code)`
