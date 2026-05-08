# Workspace Version Pinning

Cargo dependency versions are canonical in the root `Cargo.toml`
`[workspace.dependencies]` table. Crates under `crates/*` must inherit
those pins with `workspace = true` whenever they depend on a workspace-
pinned crate.

Use this shape for workspace-pinned dependencies:

```toml
tokio = { workspace = true }
serde_json = { workspace = true }
```

Do not repeat the version in a crate manifest:

```toml
# Wrong when tokio is already in [workspace.dependencies]
tokio = "1"
```

Per-crate literal versions are allowed only when the dependency is not
present in `[workspace.dependencies]`. If two or more crates need the
same new external dependency, promote it to the workspace table first
and inherit it from each crate.

Path dependencies are unchanged by this policy unless the root
workspace starts pinning them explicitly. Keep dependency upgrades in
their own PRs; this policy is about making version ownership obvious,
not changing versions opportunistically.

Run the local guard before opening dependency PRs:

```bash
scripts/check-version-pinning.sh
```
