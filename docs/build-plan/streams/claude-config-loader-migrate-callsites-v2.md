# claude-config-loader-migrate-callsites-v2 — migrate env-var callsites per refreshed audit (scaffold-extension allowed)

**Owner**: claude. **Depends on**: `claude-config-callsite-audit-refresh` MERGED.
**Authored against**: origin/main `adeb3faf` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-config-loader-migrate-callsites-v2`.

> SPLIT 2 of 2 from the DEEP-DRIFT `claude-config-loader-migrate-callsites`. The v1 brief forbade scaffold extension; the refreshed audit confirms ~30 of 39 env vars need NEW `ArgoConfig` fields, so scaffold extension is mandatory.

## Scope

1. **Extend the scaffold** in `crates/argo-config/src/`: add the new `sections_<topic>.rs` siblings recommended in the refreshed `CONFIG_CALLSITE_AUDIT.md`. Expected new files (worker confirms against audit):
   - `sections_rate_limit.rs` (for `ARGO_IP_RATE_*` + `ARGO_RATE_LIMIT_TENANT_PER_MIN`)
   - `sections_pii.rs` (for `ARGO_PII_{AUTOREDACT,DETECT,LOG_FINDINGS}`)
   - `sections_mcp.rs` (for `ARGO_MCP_HEALTH_CHECK` + `ARGO_MCP_REGISTRY_BACKED`)
   - `sections_cu.rs` (for `ARGO_CU_SCREENSHOT_RETENTION_DAYS`)
   - `sections_lifecycle.rs` (for `ARGO_SHUTDOWN_DEADLINE_SECS`)
   - Plus integrations into existing `sections_security.rs` (for `ARGO_CSP_REPORT_ONLY`, `ARGO_TRUST_XFF`).

2. **Migrate the callsites** per the refreshed audit. Pattern: each binary entry point loads `ArgoConfig::load()` once and threads `&ArgoConfig` (or a slice) to handlers. Direct `std::env::var("ARGO_*")` reads replaced with `cfg.<topic>.<field>`.

3. **Per-crate sub-pulls allowed**: if the diff balloons past ~600 LOC in one PR, the worker may stop after a coherent crate-cluster (e.g. argo-gateway rate-limit cluster only) and BLOCK with a note pointing to a follow-up sub-brief. Document the split in PR body.

4. **CI flip is NOT in this brief**. After all migration sub-pulls land + 0 mismatches, a separate `claude-config-loader-flip-ci` brief flips the gate.

**Not in scope**: changing `ArgoConfig::load()` semantics; adding non-env-var config (TOML changes are out — only env-var consolidation); flipping CI.

## Files you may touch

- `crates/argo-config/src/sections_*.rs` (existing + NEW siblings per audit) — narrowly: add fields + `Default` impls + parse from env.
- `crates/argo-config/src/lib.rs` — narrowly: re-exports for new sections.
- Any `crates/argo-*/src/**/*.rs` file whose row in `CONFIG_CALLSITE_AUDIT.md` has `current = direct env var`. Replace with `cfg.<topic>.<field>`.
- `crates/argo-cli/src/main.rs` or `crates/argo-cli/src/handlers/gateway/boot.rs` — narrowly: load `ArgoConfig` once + thread down (likely already happening; verify).

## Files you must NOT touch

- `scripts/check-no-direct-env-reads.sh` — locked.
- `docs/skycot/CONFIG_CALLSITE_AUDIT.md` — read-only input.
- `.github/workflows/ci.yml` — final sub-brief.

## Acceptance criteria

- [ ] All env-var rows in the refreshed audit migrated (or the worker stopped at a coherent split and documented the remaining scope in PR body).
- [ ] New `sections_<topic>.rs` siblings exist with `Default` impls + env-binding.
- [ ] `bash scripts/check-no-direct-env-reads.sh` mismatch count drops by the migrated count.
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` clean.
- [ ] Every modified file ≤ 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-config-loader-migrate-callsites-v2`
- PR title: `config: migrate env-var callsites to ArgoConfig (claude-config-loader-migrate-callsites-v2)`

## Notes for the implementer

- This brief explicitly **allows** scaffold extension — the v1 brief's prohibition was based on a stale audit.
- If the scope grows beyond ~600 LOC, stop and BLOCK with a precise note. Each crate-cluster (rate-limit, PII, MCP) can be its own follow-up if the worker prefers a smaller PR.
- The migrating pattern: callsite `std::env::var("ARGO_X").ok().and_then(|v| v.parse().ok())` becomes `cfg.<topic>.x`. Defaults preserved byte-identically.
