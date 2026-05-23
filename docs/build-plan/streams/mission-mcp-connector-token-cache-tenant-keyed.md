# mission-mcp-connector-token-cache-tenant-keyed

**Owner**: claude.
**Depends on**: `mission-cred-get-tenant-parameter` (informational — both close the same class of cross-tenant credential leak).
**Authored against**: origin/main `<latest>` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-mcp-connector-token-cache-tenant-keyed`.

🔴 **P0 — 3 MCP connectors cache OAuth tokens to cwd-shared paths.** Source: PR #2097 evaluator round-10 follow-up audit, finding 🚨 ESCALATION.

> "29-row matrix (round-10's '27' corrected to 29 real connector crates + 1 shared helper). Headline: **3 × P0 cross-tenant leaks** in gcal/gmail/salesforce — they cache OAuth access tokens to `<cwd>/.argo/connectors/<name>/{access_,token.json}`, cwd-shared across tenants in any multi-tenant gateway process."

Three connectors (gcal, gmail, salesforce) cache their OAuth access tokens to a process-cwd-shared file path. In a multi-tenant gateway running with one `cwd`, tenant_a's gmail token becomes readable to tenant_b — and tenant_b's refresh flow may even OVERWRITE tenant_a's cached token, mixing accounts.

This is the SAME root cause as #2080 finding §5 (round-9 credentials audit) but at the per-connector layer instead of the global path. Each connector built its own cache without tenant-keying.

## Scope

For each of 3 connectors (gcal, gmail, salesforce), rewrite the access-token cache to be tenant-keyed:

### Before (current — leak)

```rust
fn cache_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".argo/connectors/gmail/access_token.json")
}
```

### After

```rust
fn cache_path(tenant_id: &TenantId) -> PathBuf {
    argo_config::workspace_root()
        .join(".argo/connectors").join(tenant_id.as_str())
        .join("gmail").join("access_token.json")
}
```

For each `cache_path` call: thread `tenant_id`. For each `fs::read/write` of the cache: thread `tenant_id` to the path generation.

Migration: existing `<cwd>/.argo/connectors/<name>/access_token.json` files get moved to a `legacy/` subdirectory on first read; a one-shot `tracing::warn!` per (tenant, connector) reminds operator to re-authenticate. Don't auto-delete (operator might need to recover).

### Cross-tenant negative test

Per connector: bootstrap tenant_a + tenant_b, write each a different cached token, assert tenant_b's read returns its own token (not tenant_a's), and the file system has 2 separate paths.

## Parallelism

- Manager-worker allowed: yes (one child per connector — gcal/gmail/salesforce)
- Max children: 3
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-mcp-connector-gcal/src/**` — narrowly scoped: cache_path + read/write call sites.
- `crates/argo-mcp-connector-gmail/src/**` — same.
- `crates/argo-mcp-connector-salesforce/src/**` — same.
- 3 sibling test files for cross-tenant negative.
- `docs/skycot/operator/oauth-connector-cache.md` — NEW: ~30 LOC migration doc.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Other connector crates (they don't have this cache pattern per the round-10 matrix — only gcal/gmail/salesforce do).
- `argo-credentials` core — fixed via `mission-credentials-per-tenant-master-key` (in-flight).
- OAuth refresh logic itself.

## Acceptance criteria

- [ ] All 3 connectors' `cache_path` takes `tenant_id`.
- [ ] All read/write callsites thread it.
- [ ] Legacy `<cwd>/.argo/connectors/<name>/access_token.json` paths move to `legacy/` on first read with operator warning.
- [ ] 3 cross-tenant negative tests pass.
- [ ] Operator doc covers migration + how to clear `legacy/`.
- [ ] `cargo clippy -p argo-mcp-connector-gcal -p argo-mcp-connector-gmail -p argo-mcp-connector-salesforce --all-targets -- -D warnings` clean.
- [ ] Every modified file <500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `mission-mcp-connector-token-cache-tenant-keyed`
- PR title: `fix(mcp-connectors): tenant-key OAuth token caches (gcal/gmail/salesforce) (mission-mcp-connector-token-cache-tenant-keyed)`

## Notes for the implementer

- ~300-500 LOC across 3 connectors. Manager-worker eligible — 1 child per connector.
- **The migration on first read is load-bearing** — without it, operators upgrading lose their OAuth state until they re-authenticate, but at least don't get unauthorized cross-tenant access.
- Cite PR #2097's audit + round-9 #2080 §5 as the same-class precedent.
