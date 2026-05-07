# Genesis export vertical bundle — portable .tar.gz of a vertical

**Owner**: claude. **Depends on**: none.

## Scope

Operators want to share a vertical with another Argo instance, version
it independently of the workspace, or hand it to a teammate. This
brief adds `argo genesis export` that bundles a vertical's TOML +
markdown + eval seed + memory schema into a single portable tarball
(`<vertical>.argo-vertical.tar.gz`) with a manifest of contents and
a SHA256 over the bundle.

A reciprocal `argo genesis import-bundle` is **out of scope here** —
it lives in a follow-up brief paired with verification logic.

## Approach (concrete)

1. **NEW** `crates/argo-cli/src/handlers/genesis_export.rs` (~280 LOC):
   - `pub async fn handle_export(args: GenesisExportArgs) -> ArgoResult<()>`
   - Args: `--vertical <name>`, `--output <path>` (default: `./<name>.argo-vertical.tar.gz`),
     `--include-skills` (default: true), `--manifest-only` (writes `BUNDLE.toml` only)
   - Bundle layout inside the tarball:
     - `BUNDLE.toml` — manifest: vertical name, version, sha256 of each file, argo version
     - `vertical.toml`
     - `system-prompt.md`
     - `eval-seed.toml`
     - `memory-schema.toml`
     - `skills/` (if present and `--include-skills`)
   - Use `tar` + `flate2` (already workspace deps from earlier audit work)
2. **Modify** `argo-cli/src/cli.rs`: add `Genesis::Export { ... }` subcommand
3. **Modify** `handlers/mod.rs`: 1 pub mod line

## Files you may touch

- `crates/argo-cli/src/handlers/genesis_export.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/genesis_export_tests.rs` (NEW, ≤ 220 LOC) — 6 tests:
  - export of a fixture vertical produces a tarball that opens via flate2 + tar
  - BUNDLE.toml inside the tarball has sha256 entries for every file
  - --manifest-only writes BUNDLE.toml only, no tarball
  - missing vertical → ArgoError::NotFound
  - existing output path → ArgoError::Validation (no overwrite without --force)
  - --include-skills=false omits skills/
- `crates/argo-cli/src/cli.rs` (narrowly: add Export subcommand)
- `crates/argo-cli/src/handlers/mod.rs` (narrowly: 1 pub mod line)
- `crates/argo-cli/tests/genesis_export_smoke.rs` (NEW, ≤ 180 LOC) —
  end-to-end: scaffold a fake vertical in a tempdir, run export,
  unpack the tarball, assert all expected files present + sha256 matches

## Files you must NOT touch

- `crates/argo-genesis/` — export is CLI-only
- The 5 demo verticals — tests use a tempdir scaffold

## Acceptance criteria

- [ ] `argo genesis export --vertical sales-sdr --output /tmp/x.tar.gz` produces a non-empty tarball
- [ ] Tarball opens via flate2 + tar and lists exactly the expected files
- [ ] BUNDLE.toml has sha256 + size for every file in the bundle
- [ ] --manifest-only writes only BUNDLE.toml
- [ ] No overwrite without --force
- [ ] 6 unit tests + 1 smoke test pass
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-genesis-export-vertical-bundle`
- PR title: `genesis: export vertical as portable .tar.gz bundle with sha256 manifest (claude-genesis-export-vertical-bundle)`

## Notes for the implementer

- BLANKET CLI ALLOWLIST applies — cli.rs / handlers/mod.rs / Cargo.toml
  edits are pre-approved.
- Verify `tar` and `flate2` are already in workspace deps (audit work
  added them for log archives) — reuse, don't add new deps.
- BUNDLE.toml `argo_version` field comes from the `CARGO_PKG_VERSION`
  env macro of `argo-cli` — easy stable choice.
- The reciprocal `import-bundle` lives in a follow-up brief; mention
  that in code comments so the next brief author has context.
