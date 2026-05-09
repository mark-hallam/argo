# claude-config-loader-migrate-callsites — apply CONFIG_CALLSITE_AUDIT.md mechanically

**Owner**: claude. **Depends on**: `claude-config-loader-overhaul` MERGED (scaffold + audit report + CI warning step). **Phase A consolidation.**

**Authored against**: origin/main b3949a73 on 2026-05-09 (split-out from the original combined `claude-config-loader-overhaul` brief).

## Why

The scaffold phase produced `docs/skycot/CONFIG_CALLSITE_AUDIT.md` listing every direct env-var / TOML callsite outside `argo-config` with a recommended `ArgoConfig` field replacement. This brief mechanically migrates those callsites and flips the CI guard from warning to fail.

## Approach (concrete)

1. **Read `docs/skycot/CONFIG_CALLSITE_AUDIT.md`** — every row is `path:line + current pattern + recommended ArgoConfig field`.
2. **At each binary entry point** (gateway start, CLI main, etc.) that doesn't already have one: load `ArgoConfig::load()?` once and thread `&ArgoConfig` (or relevant slice) down to handlers.
3. **For each callsite row**: replace the direct `std::env::var("ARGO_*")` or `.skycot/argo.toml` read with the resolved `&ArgoConfig` field access. Preserve fallback/default behaviour byte-identically.
4. **Re-run `scripts/check-no-direct-env-reads.sh`** — should report 0 mismatches outside `argo-config`.
5. **Flip CI step from warning-only to fail-on-mismatch** in `.github/workflows/ci.yml`.

## Files you may touch

- Any `crates/argo-*/src/**/*.rs` file appearing in the audit report — narrowly: callsite migrations only.
- `crates/argo-cli/src/handlers/gateway/boot.rs` (or current gateway entry) — load `ArgoConfig` once, thread down.
- `crates/argo-cli/src/main.rs` — top-level entry; load config once.
- `.github/workflows/ci.yml` — narrowly: flip the audit step from warning to fail.

## Files you must NOT touch

- `crates/argo-config/src/**` — the scaffold is fixed by the prior brief.
- `scripts/check-no-direct-env-reads.sh` — already shipped.
- `docs/skycot/CONFIG_CALLSITE_AUDIT.md` — read-only input.
- TUI key bindings.
- Any test file other than `cargo test --workspace` validation runs.
- `argo-types` (no new types).

## Acceptance criteria

- [ ] Every callsite from `CONFIG_CALLSITE_AUDIT.md` migrated to the recommended `ArgoConfig` field access
- [ ] `scripts/check-no-direct-env-reads.sh` reports 0 mismatches outside `argo-config`
- [ ] CI step flipped from warning to fail
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` clean
- [ ] No behavioural regression on default values (gateway integration tests confirm)
- [ ] Every modified file ≤ 500 LOC
- [ ] `gh pr view --json mergeable` returns MERGEABLE post-push

## Branch + PR

- Branch: `claude-config-loader-migrate-callsites`
- PR title: `config: migrate callsites per CONFIG_CALLSITE_AUDIT.md + flip CI to fail (claude-config-loader-migrate-callsites)`

## Notes for the implementer

- This brief is **mechanical only**. If a row in the audit report doesn't fit cleanly into the flat `ArgoConfig` struct, leave it and add a comment to the report; do not invent semantics.
- Workspace-wide clippy + tests are in acceptance because migrations span multiple crates; budget for the cold workspace build.
- If the audit report has > 50 rows, consider splitting into per-crate sub-PRs (e.g. argo-cli/handlers, argo-vertical, argo-marketplace) — but the original estimate was ~30 callsites, manageable in one PR.
- Preserve byte-identical defaults: a field that reads `std::env::var("ARGO_GATEWAY_PORT").unwrap_or_else(|_| "8080".into())` becomes `cfg.gateway_port` where `cfg.gateway_port` already defaults to `"8080"`.
