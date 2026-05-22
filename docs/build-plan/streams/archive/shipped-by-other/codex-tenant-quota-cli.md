# V1.4 — Tenant resource quota CLI

**Owner**: codex. **Depends on**: H-spend-persistence (#809)
merged.

## Scope

Today `argo budget show <worker>` covers per-worker spend.
Per-**tenant** budget enforcement is missing — useful when an
operator runs a multi-tenant deployment and wants hard caps
on each tenant's monthly USD spend / action count / worker count.

Add:
- `tenant_quota` table (migration 28): tenant_id, period (daily
  or monthly), max_usd, max_actions, max_workers, set_at.
- `argo tenant quota set <tenant> [--max-usd <N>] [--max-actions
  <N>] [--max-workers <N>] [--period daily|monthly] [--json]`
- `argo tenant quota show <tenant> [--json]` — current quota +
  period-to-date usage + remaining headroom
- `argo tenant quota list [--json]` — all tenant quotas
- `argo tenant quota clear <tenant> [--field <max-usd|max-actions|max-workers>] [--json]`
  — remove quota or specific field

Auto-approval gate consults tenant quota AFTER per-worker
budget check; tenant breach blocks regardless of worker
headroom. Mirrors the layered AutoApprovalPolicy resolution
pattern.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-store/migrations/28__tenant_quota.sql` (NEW)
- `crates/argo-store/src/tenant_quota.rs` (NEW) — CRUD +
  has_headroom_for(tenant, kind, amount)
- `crates/argo-store/src/lib.rs` — narrowly: re-export
- `crates/argo-cli/src/handlers/tenant_quota_set.rs` (NEW)
- `crates/argo-cli/src/handlers/tenant_quota_show.rs` (NEW)
- `crates/argo-cli/src/handlers/tenant_quota_list.rs` (NEW)
- `crates/argo-cli/src/handlers/tenant_quota_clear.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 4 pub mods
- `crates/argo-cli/src/cli_tenant_quota_cmds.rs` (NEW)
- `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs`
  — narrowly: tenant quota check before AutoApprove return
- `crates/argo-cli/tests/tenant_quota_smoke.rs` (NEW)

## Acceptance criteria
- [ ] `argo tenant quota set t-1 --max-usd 100 --period
      monthly` persists row
- [ ] `argo tenant quota show t-1` returns quota + used + remaining
- [ ] Auto-approval gate blocks when tenant breaches quota
      (verified via integration test that AutoApprove
      becomes Escalate when over-quota)
- [ ] `--json` envelope universal across all 4 subcommands
- [ ] 6 unit tests + 1 integration test (gate hookup)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-tenant-quota-cli`
- PR title: `tenant: argo tenant quota — per-tenant resource caps (codex-tenant-quota-cli)`
