# mission-onboarding-audit-variants — add 2 `AuditKind` variants for onboarding flow

**Owner**: codex. **Depends on**: nothing (#1910 onboarding-batch is on `main` providing the endpoints that will emit these variants).

**Authored against**: origin/main `f13bb061` on 2026-05-21.

Suggest fast-mode in the worker header — pure mechanical clone-and-extend with no architectural decisions.

## Why

#1910 (`web-gateway-endpoints-onboarding-batch`) shipped 3 onboarding gateway endpoints (provider-key configure / vertical install / first-hire). The PR's brief explicitly required:

> "Add 2 new `AuditKind` variants: `ProviderKeyConfigured` and `VerticalInstalled`. Each onboarding endpoint emits an audit row with the appropriate kind."

The shipped PR deferred this — onboarding endpoints currently emit no audit rows OR emit them as the generic `AuditKind::HttpRequest`. This brief closes the gap.

Operator risk class: low. Defensive observability (lets `/audit` feed show named domain events for onboarding actions instead of opaque HTTP rows).

## Approach (concrete, mirrors `AuditKind::ApprovalExpiredByTimeout` pattern)

1. **Add 2 variants** to `crates/argo-audit/src/event.rs`'s `pub enum AuditKind`:
   ```rust
   ProviderKeyConfigured,
   VerticalInstalled,
   ```
   Insert alphabetically (or in whatever convention the existing enum follows — read first to confirm).

2. **Add 2 match arms** to `AuditKind::as_kind_str`:
   ```rust
   Self::ProviderKeyConfigured => "onboarding.provider_key_configured",
   Self::VerticalInstalled => "onboarding.vertical_installed",
   ```

3. **Add 2 match arms** to `AuditKind::from_kind_str`:
   ```rust
   "onboarding.provider_key_configured" => Self::ProviderKeyConfigured,
   "onboarding.vertical_installed" => Self::VerticalInstalled,
   ```

4. **Add 2 entries** to whatever `ALL_KINDS` slice / round-trip iterator exists (grep for it — look for `AuditKind::` references in a const array).

5. **Bump the variant-count assertion** in `event_tests.rs` if one exists (per the precedent `47→48 / 45→46` pattern from prior briefs).

6. **Emit the variants from the onboarding handlers** in `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs`:
   - `handle_provider_key_configure` (or similar — the function that handles `POST /api/v1/onboarding/provider-key`): after successful key persist, emit an audit row with `AuditKind::ProviderKeyConfigured` + payload `{"provider": <name>}` (don't include the key bytes — only the provider name).
   - `handle_vertical_install` (or similar): after successful vertical install, emit `AuditKind::VerticalInstalled` + payload `{"vertical": <id>}`.
   - Mirror the existing audit-emit pattern in adjacent handlers (e.g. `routes_workflows.rs` emits AuditKind on its mutations).

7. **Update 2-4 existing tests** in `crates/argo-cli/src/handlers/gateway/routes_onboarding_tests.rs` to assert: after calling the relevant endpoint, the audit table contains exactly 1 row with the expected `kind` string.

## Files you may touch (verified on `f13bb061`)

- `crates/argo-audit/src/event.rs` — MODIFY. 2 enum variants + 2 round-trip arms + 1 ALL_KINDS slice entry pair. ~10 LOC delta.
- `crates/argo-audit/src/event_tests.rs` — MODIFY. Variant-count bump + round-trip coverage for the 2 new kinds. ~10 LOC delta.
- `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs` — MODIFY. Add 2 audit-emit call sites in the relevant handlers. ~20 LOC delta.
- `crates/argo-cli/src/handlers/gateway/routes_onboarding_tests.rs` — MODIFY. Extend 2-4 existing tests to assert the audit-row write. ~30 LOC delta.

## Files you must NOT touch

- Any `crates/argo-store/migrations/*.sql` — no schema change.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` — dispatch wiring is correct; only the handlers themselves need to emit audit rows.
- The provider-key persistence logic in `workspace_keys.rs` — that's #1910's domain, unchanged here.
- The audit-sink chain emission logic in `argo-audit/src/sink_chain.rs` — chain logic is correct, this brief only adds new kinds.

## Acceptance

- [ ] `AuditKind::ProviderKeyConfigured` and `AuditKind::VerticalInstalled` exist and round-trip through `as_kind_str` ↔ `from_kind_str`.
- [ ] Onboarding's provider-key-configure endpoint emits exactly 1 audit row with `kind=onboarding.provider_key_configured`.
- [ ] Onboarding's vertical-install endpoint emits exactly 1 audit row with `kind=onboarding.vertical_installed`.
- [ ] The audit row's payload contains the provider/vertical identifier but NOT secret material (key bytes never written to audit).
- [ ] `cargo clippy -p argo-audit -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-cli routes_onboarding` green; `cargo test -p argo-audit event` green.
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.
- [ ] Every modified file < 500 LOC.
- [ ] No `unwrap`/`expect` outside `#[cfg(test)]`.

## Branch + PR

- Branch: `codex-onboarding-audit-variants`
- PR title: `argo-audit: add ProviderKeyConfigured + VerticalInstalled AuditKind variants (mission-onboarding-audit-variants)`

## Why this is a codex-sized brief

- Single architectural pattern (add enum variant; mirrors the `ServiceTokenBindingExpired` brief shipped as #1911).
- 4 files touched (2 in argo-audit, 2 in argo-cli), all narrow.
- No new pub types beyond the 2 trivial enum variants, no new crate, no migration.
- Worker can complete in 30-90 min including tests.
