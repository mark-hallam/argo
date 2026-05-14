# M15-vertical-pack-loader - Track M (M-Tmpl), Phase 3, Tier 1

**Owner**: claude. **Depends on**: L1 MERGED; L2 MERGED.
**Authored against**: origin/main `5a5d3970` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M15-vertical-pack-loader`.

## Scope

Pack manifest format (TOML). Fetcher: pulls from github.com/mark-hallam/skycot-packs via release tarballs. Cached at ~/.argo/packs/<name>/<version>/. dev-link: argo packs dev-link <name> <path> points the runtime at a local checkout. CLI: argo packs install / list / update / dev-link / remove. Distribution decision locked per M0 section 1: separate skycot-packs repo from day one.

LOC: ~900. Branch: `claude-M15-vertical-pack-loader`. PR title: `feat(packs): vertical-pack loader + dev-link CLI (M15)`.

## Files you may touch

- NEW crate crates/argo-vertical-packs/
- NEW crates/argo-vertical-packs/src/lib.rs
- NEW crates/argo-vertical-packs/src/loader.rs
- NEW crates/argo-vertical-packs/src/manifest.rs
- NEW crates/argo-vertical-packs/src/fetcher.rs - GitHub release fetch
- NEW crates/argo-vertical-packs/src/dev_link.rs - local-path override
- NEW crates/argo-cli/src/handlers/packs.rs - CLI argo packs ...
- NEW crates/argo-vertical-packs/tests/loader.rs

## Files you must NOT touch

- L1/L2 schemas; existing skill marketplace.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Pack format documented + validated.
- [ ] Loader installs one sample pack from GitHub release.
- [ ] Cache hit avoids re-download.
- [ ] dev-link overrides cache; edits to local checkout reflect in next worker run.
- [ ] 8 unit tests.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-M15-vertical-pack-loader`
- PR title: `feat(packs): vertical-pack loader + dev-link CLI (M15)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for this brief.
- Mission-claim alignment: claim 3 (vertical packs as fast-start).


## Operator-provided context (2026-05-14)

The packs repo is `https://github.com/mark-hallam/skycot-packs` (PRIVATE while build phase). Pack URLs in `pack.toml` should use this path for now. Operator has not settled on long-term hosting — the URL may change pre-v1.0 release; treat as configuration, not a hardcoded constant. Add a `[fetcher] base_url = "..."` knob in the pack-loader config so swapping the URL is a TOML edit, not a code change.

The repo is private; the fetcher must auth via GitHub token (use existing `GH_TOKEN` / `GITHUB_TOKEN` env-var precedent the rest of the codebase follows).


## Amendment 2026-05-14 (may-touch widening)

Pre-empted from the same self-BLOCK pattern that hit M1/M5/M9: workers reasonably reported the original may-touch was too narrow to ship a real (non-scaffold) implementation. Conductor adds these load-bearing edits to the allowed surface:

- The containing crate's `lib.rs` — narrowly: add `pub mod <new_module>;` declaration. **≤10 modified lines.**
- The containing crate's `Cargo.toml` — narrowly: add any required `[dependencies]` line for sibling argo-* crates that this brief consumes. **≤5 modified lines.**
- If a gateway endpoint is in scope: register the route in the appropriate `routes_dispatch_*.rs` sibling (post-#1601 split). **≤10 modified lines.**

These edits are mechanical and required for the new code to compile + be reachable. Do NOT widen beyond the brief's original scope (no business logic in lib.rs, no new dependencies beyond what the brief names).


## Amendment 2026-05-15 (NEW-crate integration allowance — cascade-unblock for M16-M20)

The generic 2026-05-14 amendment above doesn't cover M15's specific scope of **creating a NEW workspace crate** (`crates/argo-vertical-packs/`) plus a top-level `argo packs` CLI subcommand. Codex correctly self-BLOCKED noting the may-touch list omits the integration files.

Conductor adds these load-bearing edits for new-crate-creation work:

- Workspace root `Cargo.toml` — narrowly: add `crates/argo-vertical-packs` to `[workspace.members]`. **≤3 modified lines.**
- `crates/argo-cli/src/cli_command_enum.rs` — narrowly: add the `Packs` variant + its argument struct ref. **≤15 modified lines.**
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: `pub mod packs;` declaration. **≤3 modified lines.**
- NEW `crates/argo-cli/src/handlers/packs.rs` — the CLI dispatch handler for `argo packs <subcommand>`. **≤200 LOC.**
- `crates/argo-cli/src/main_dispatch_*.rs` (sibling that owns the matching subcommand dispatch) — narrowly: add the dispatch arm. **≤15 modified lines.**
- `crates/argo-cli/Cargo.toml` — narrowly: add `argo-vertical-packs = { workspace = true }` dep. **≤5 modified lines.**

These are integration-glue edits required to make `argo packs` a real CLI surface, not an unreachable library. Without them M15 ships a library with no caller, and M16-M20 cannot be install-tested.

**Cascade-unblock**: once M15 ships with this amendment, M16-M20 (5 vertical pack briefs currently BLOCKED on M15) flip TODO automatically.
