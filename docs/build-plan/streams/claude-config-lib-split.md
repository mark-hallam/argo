# claude-config-lib-split — split argo-config lib.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: claude-config-loader-overhaul (#1078, MERGED). **Phase A architectural prep — defensive split.**

**Authored against**: origin/main 828e5e1b on 2026-05-10.

## Why

`crates/argo-config/src/lib.rs` is at 473 LOC after the #1078 scaffold landed. The follow-up `claude-config-loader-migrate-callsites` brief will need headroom here. Pure refactor mirrors proven pattern.

## Approach

1. Read `lib.rs` end-to-end. Identify a cohesive group (likely a section of the `ArgoConfig` API surface — getters vs. validators vs. helpers).
2. Move to `crates/argo-config/src/<topic>.rs`.
3. Re-export from `lib.rs` to preserve the public API.

Target: `lib.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-config/src/lib.rs` — narrowly: extract sibling, re-export
- `crates/argo-config/src/<topic>.rs` (NEW, ≤ 250 LOC)

## Files you must NOT touch

- `crates/argo-config/src/loader.rs` / `defaults.rs` / `sections.rs` — these are sibling modules, separate concern
- Test files — refactor only the source

## Acceptance criteria

- [ ] `lib.rs` ≤ 350 LOC
- [ ] Sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-config --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-config` clean
- [ ] No public API changes (re-exports preserve consumer-visible names)

## Branch + PR

- Branch: `claude-config-lib-split`
- PR title: `config: split lib.rs below 500-LOC ceiling (claude-config-lib-split)`
