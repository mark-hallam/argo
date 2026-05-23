# mission-cred-get-tenant-parameter

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `<latest>` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cred-get-tenant-parameter`.

🔴 **P0 CROSS-TENANT CREDENTIAL LEAK.** Source: evaluator round-9 PR #2080 finding §4 P0.

> "`argo_config::cred::get(name)` is tenant-blind across 40+ channel + connector callsites reading the gateway's shared `cwd/.argo/credentials.toml`. By-construction cross-tenant credential leak in any multi-tenant gateway."

The crate-level `cred::get(name)` API takes a credential name but NOT a tenant_id. Every channel + connector callsite passes a bare string. The gateway resolves all credentials from the SAME `cwd/.argo/credentials.toml` regardless of which tenant's worker is asking. **Tenant_a's worker can ask for tenant_b's Slack token and get it.**

## Scope

Extend `argo_config::cred::get(name)` → `cred::get(tenant_id, name)` and sweep all 40+ callsites.

### API change

```rust
// Before (current)
pub fn get(name: &str) -> Result<String, CredError>;

// After
pub fn get(tenant_id: &TenantId, name: &str) -> Result<String, CredError>;
```

Plus an additive `cred::get_unscoped(name)` for the genuine single-tenant CLI use cases (mark each callsite explicitly). Default to the scoped form.

### Storage shape

Credentials file becomes `cwd/.argo/credentials.toml` with per-tenant sections:

```toml
[tenant.<tenant_id>.slack]
bot_token = "xoxb-..."

[tenant.<tenant_id>.gmail]
oauth_refresh = "1//..."
```

Or per-tenant files: `cwd/.argo/credentials/<tenant_id>/<service>.toml`. Pick whichever fits the existing read path's shape.

Migration: existing root-level keys (no tenant prefix) get loaded under a special `legacy` tenant on first read, with a `tracing::warn!` on every access. Operator manually migrates to per-tenant via a `argo creds migrate-to-tenant <tid>` CLI helper (separate brief).

### Callsite sweep

Find every callsite via `rg 'cred::get\(' crates/`. ~40 sites. For each: thread `tenant_id` from the caller's context (usually available via the request principal or worker.tenant_id).

If a callsite legitimately has no tenant context (e.g. the gateway startup probe), use `get_unscoped(name)` + add a `# tenant-unscoped: <reason>` doc comment.

### Cross-tenant negative test

Sibling tests: bootstrap tenant_a with `slack/bot_token` + tenant_b with a different `slack/bot_token` → call `cred::get(tenant_b, "slack/bot_token")` → assert returns tenant_b's value, NOT tenant_a's. Try `cred::get(tenant_a, "<tenant_b's key>")` → assert returns Err(NotFound), NOT the cross-tenant value.

## Parallelism

- Manager-worker allowed: yes (40+ callsites — split by crate)
- Max children: 4
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: argo-config
- Shared hotspots: [`crates/argo-config/src/cred.rs`]

## Files you may touch

- `crates/argo-config/src/cred.rs` — extend API.
- `crates/argo-config/src/cred_tests.rs` — sibling tests.
- 40+ callsites across `crates/argo-channels-*`, `crates/argo-mcp-*`, `crates/argo-cli/`.
- `docs/skycot/operator/credentials-config.md` — NEW: migration guide.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- The credentials file itself in `~/.argo/` — operator data, only the LOADER changes.
- Storage encryption (separate brief if needed).

## Acceptance criteria

- [ ] `cred::get(tenant_id, name)` signature live.
- [ ] `cred::get_unscoped(name)` available for genuine single-tenant cases (marked explicitly).
- [ ] All 40+ callsites swept.
- [ ] Cross-tenant negative test pinned (tenant_a key NOT visible to tenant_b).
- [ ] Legacy file shape readable with warning (backward-compat).
- [ ] `cargo clippy -p argo-config --all-targets -- -D warnings` clean.
- [ ] Workspace clippy clean.
- [ ] Every modified file <500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `mission-cred-get-tenant-parameter`
- PR title: `fix(cred): tenant-parameterize cred::get across 40+ callsites (mission-cred-get-tenant-parameter)`

## Notes for the implementer

- ~500-800 LOC expected. Big sweep brief; consider manager-worker (1 child per crate-family).
- **This is the highest-leverage P0 from round-9 credentials audit**. Operator authorization implicit in "every customer's third-party tokens" framing.
- **Don't change the storage encryption** — that's a separate concern. This brief is the TENANCY layer.
- Cite round-9 evaluator audit + #2080 §4 P0 verbatim in PR body.
