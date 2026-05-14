# claude-toml-inherit-workspace-versions — Replace 236 workspace-dep version literals with inheritance

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `c928d8fe` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-toml-inherit-workspace-versions`.

## Scope

Phase D audit found 236 lines across `crates/*/Cargo.toml` files that hardcode `version = "0.0.1"` on internal workspace dependencies instead of inheriting from `[workspace.package]`. Single regex pass to fix.

**Pattern to find:**
```
argo-X = { path = "../argo-X", version = "0.0.1" }
```

**Replace with:**
```
argo-X = { path = "../argo-X" }
```

(version inheritance is implicit when path-deps live in the workspace and `[workspace.package] version` is set, which it is.)

LOC: ~50 (one Python or sed script + manual verification on edge cases).

## Files you may touch

- All `crates/*/Cargo.toml` files with matching pattern.
- NEW `scripts/fix-toml-version-inheritance.py` — one-shot script that runs the replacement + emits a summary; idempotent.

## Files you must NOT touch

- External-crate version specs (those that pin a real crates.io version, e.g., `tokio = { workspace = true, features = [...] }` — those are already workspace-correct).
- The workspace-root `Cargo.toml` `[workspace.package]` section.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `rg '"\.\.\/argo-[^"]+", version = ' crates/*/Cargo.toml` returns zero matches.
- [ ] `cargo build --workspace` clean (no resolver errors).
- [ ] `cargo metadata --format-version 1 | jq '.packages[] | .name' | sort -u | wc -l` returns the same count before/after (no accidental crate removal).
- [ ] One-shot script is idempotent: running it twice produces zero further changes.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-toml-inherit-workspace-versions`
- PR title: `chore(cargo): inherit workspace versions on internal deps (claude-toml-inherit-workspace-versions)`

## Notes for the implementer

- Mission-claim alignment: production hardening (TOML hygiene; eliminates 236 redundant lines; enables future single-source version bumps).
- Phase D audit report: `docs/skycot/audit/2026-05-14-phase-D-cross-cutting-drift.md` §4 + §6.
- Verify in a fresh worktree first; the change is mechanical but workspace-resolver edge cases exist (some crates with dev-deps may need explicit version specs).
- Drift-prevention: brief touches only Cargo.toml files; no Rust pub surfaces.
