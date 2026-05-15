# codex-rbac-workspace-scope — Workspace-scoped FsRead/FsWrite for default user role

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `25821ea8` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-rbac-workspace-scope`.
**Source**: F-006 in `docs/skycot/audit/2026-05-15-codex-full-architecture-audit.md`.

## Scope

P1 RBAC. The default `user` role in `crates/argo-capability/src/lib.rs:285-295` ships with `FsRead { path_glob: "**" }` and `FsWrite { path_glob: "**" }` as "permissive placeholders". The comment at `:284` claims "FsRead/Write to workspace are interpolated by the config layer ({workspace}/**)" — but in current code, no interpolation actually happens on this role's caps, so the placeholder leaks into production as workspace-unscoped FS access.

`crates/argo-config/src/lib.rs` already exposes `workspace_tenant_id` and the related workspace resolution path. This brief wires that resolution into the user role's FsRead/FsWrite globs at role-creation time, so the "**" placeholder becomes `{workspace_root}/**` for any caller that uses the standard `default_roles()` factory.

Scope:
- Add a `default_roles_for_workspace(workspace_root: &Path) -> Vec<CapabilityRole>` constructor in argo-capability that returns roles with `{workspace_root}/**` substituted for the `"**"` placeholder.
- Mark the bare `default_roles()` either `#[deprecated]` or `#[cfg(test)]`-only — production code must use the workspace-scoped variant.
- Find every production caller of `default_roles()` and migrate to `default_roles_for_workspace()`.
- Tests prove the placeholder no longer leaks: `default_roles_for_workspace("/tmp/ws")` returns roles with `/tmp/ws/**` not `**`.

LOC: ~120.

## Files you may touch

- `crates/argo-capability/src/lib.rs` — add `default_roles_for_workspace`; gate or deprecate bare `default_roles`. ≤80 modified lines.
- Production callers of `default_roles()` (run `rg 'default_roles\(\)' crates --type rust -l` to identify; expect 3-5 sites — gateway boot, role-config-load, fixture-setup). For each: ≤30 modified lines.
- Test sibling: existing tests in `argo-capability/src/lib.rs` extend or new `tests.rs` sibling. ≤60 LOC.

## Files you must NOT touch

- `crates/argo-config/src/lib.rs` — `workspace_tenant_id` is a consumed primitive, not redefined here.
- The `Capability` enum itself in `argo-types` — no new variants; this brief just adjusts glob values.
- Anything outside the may-touch list above.

## Acceptance criteria

- [ ] `default_roles_for_workspace(workspace_root)` exists and returns roles with workspace-scoped globs.
- [ ] No production caller uses bare `default_roles()` (verify: `rg 'default_roles\(\)' crates --type rust` returns only `#[cfg(test)]` or `tests/` hits).
- [ ] Test proves the `"**"` placeholder is replaced with `{workspace_root}/**` in the returned roles.
- [ ] `cargo test -p argo-capability` passes.
- [ ] `cargo clippy -p argo-capability --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-rbac-workspace-scope`
- PR title: `feat(capability): workspace-scope default user-role FsRead/FsWrite (codex-rbac-workspace-scope)`

## Notes for the implementer

- The original comment at `crates/argo-capability/src/lib.rs:284` claims the interpolation happens at config-layer; if you find an EXISTING interpolation site, prefer to fix it in place rather than adding a parallel function. Document which path is canonical.
- If `default_roles()` callers exist in tests with workspace_root unavailable, a test-only constant like `TEST_WORKSPACE_ROOT` works.
- Operator decision context: this is enterprise-grade defaults. Strict workspace scope is the target; v1 may still ship the permissive default for ad-hoc CLI use behind a tagged seam.
