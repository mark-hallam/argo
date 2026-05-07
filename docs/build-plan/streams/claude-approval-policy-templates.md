# V1.4 — Approval policy templates

**Owner**: claude. **Depends on**: H-auto-approval merged,
worker_actions_auto_approval.rs stable.

## Why

Today every operator hand-rolls auto-approval rules in
`.skycot/auto-approval.toml`. Common patterns (auto-approve
read-only file ops, require human for finance/payment kinds,
auto-deny network calls outside allowlist) get rewritten on
every project. Ship a small set of vetted policy templates
that operators can opt into by name, plus a `argo approval
policy show <name>` to print the resolved rules.

## Scope

Add a directory `.skycot/approval-policies/` (per-repo) and a
bundled set of starter templates shipped inside the binary
(via `include_str!`). Operators reference a template in their
auto-approval config:

```toml
[auto_approval]
template = "auto-approve-low-risk"
# or
extends = ["require-human-for-finance", "deny-network-egress"]
```

CLI surface:
- `argo approval policy list` — show bundled + repo-local
- `argo approval policy show <name>` — print resolved TOML
- `argo approval policy init <name>` — copy bundled template
  into `.skycot/approval-policies/<name>.toml` for editing

Policy resolution merges templates in `extends` order, then
overlays the repo's own rules. Conflict resolution: later
entries win (well-documented in inline comments).

NOT in scope: dynamic policy evaluation engine — the bundled
templates compile down to existing `AutoApprovalRule` structs
in `worker_actions_auto_approval.rs`.

## Approach

1. NEW `crates/argo-cli/src/handlers/approval_policy.rs`
   (~280 LOC):
   - `BUNDLED_TEMPLATES: &[(&str, &str)]` — name → embedded
     TOML body via `include_str!("../../assets/approval_policies/*.toml")`
   - `pub async fn run(args: ApprovalPolicyArgs) -> ArgoResult<()>`
   - `fn resolve(template_name: &str) -> ArgoResult<AutoApprovalConfig>`
2. NEW `crates/argo-cli/assets/approval_policies/` with starter
   templates (~5 files, each ≤ 60 LOC):
   - `auto-approve-low-risk.toml`
   - `require-human-for-finance.toml`
   - `deny-network-egress.toml`
   - `read-only-developer.toml`
   - `strict-production.toml`
3. Hook the resolver into `worker_actions_auto_approval.rs` —
   when config has `template = "..."`, load + merge before
   evaluating rules. (Narrow change; do not refactor existing
   evaluator.)
4. `cli.rs` adds `Policy { #[command(subcommand)] cmd: PolicyCmd }`
   under `ApprovalCmd`.

## Files you may touch

- `crates/argo-cli/src/handlers/approval_policy.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/approval_policy_tests.rs` (NEW, ≤ 280 LOC)
- `crates/argo-cli/assets/approval_policies/*.toml` (NEW; 5 files ≤ 60 LOC each)
- `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs` —
  narrowly: add `template` / `extends` field handling at config
  load only (do not change rule evaluation)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: pub mod line
- `crates/argo-cli/src/cli.rs` / `main.rs` — narrowly: subcommand
  wiring (BLANKET CLI ALLOWLIST)

## Files you must NOT touch

- `crates/argo-types/` — no new types unless absolutely needed;
  reuse existing `AutoApprovalRule`
- Storage layer — templates live on disk, not in the DB
- Other auto-approval logic (transitions, persistence)

## Acceptance criteria

- [ ] `argo approval policy list` shows 5 bundled + any local
- [ ] `argo approval policy show require-human-for-finance`
      prints the resolved TOML to stdout
- [ ] `argo approval policy init strict-production` writes
      `.skycot/approval-policies/strict-production.toml`
- [ ] Auto-approval config with `template = "auto-approve-low-risk"`
      behaves as if the bundled rules were inlined
- [ ] `extends = ["a", "b"]` merges in declared order
- [ ] Round-trip test: bundled TOML → parse → re-serialise
      produces equivalent `AutoApprovalConfig`
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every file < 500 LOC

## Branch + PR

- Branch: `claude-approval-policy-templates`
- PR title: `approval: policy templates (claude-approval-policy-templates)`

## Notes for the implementer

- Templates are READ-ONLY at runtime. Operators clone via
  `init` and edit the local copy — never modify bundled
  strings.
- Keep each template under 60 LOC of TOML, with `# WHY:` inline
  comments. They double as documentation.
- Add a help-smoke entry for each new subcommand variant.
