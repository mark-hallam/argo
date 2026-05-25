# mission-skill-library-bootstrap-on-fresh-install — bootstrap ~/.argo/packs/ on first run

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-library-bootstrap-on-fresh-install`.

## Why this is P2

Mission gap manifest row `argo-vertical-packs` Finding F-1: `~/.argo/packs/` is empty on a fresh install — `PackLoader::list` returns empty, no `argo packs search` is wired, no default catalog. Mission claim 4 (skills shared + learned) and Genesis demo flow both assume a baseline catalog exists. New users (and the demo MP4) hit a "no packs available" state.

## Scope

Add a bootstrap path that ensures `~/.argo/packs/` contains a sensible default catalogue on first binary run.

1. Decide the default catalogue source: (a) bundled-in-binary minimum-viable set (~5 packs: project-coordinator, status-reporter, sales-rep, support-tier-1, default), (b) lazy-fetch from `github.com/mark-hallam/skycot-packs` on first run (private repo — needs auth), or (c) both: bundled minimum + opt-in remote fetch.
2. Implement option (a) as minimum-viable: extend `argo-vertical-packs::PackLoader` boot path so on `list()` if the cache is empty, write the bundled set into `~/.argo/packs/` first.
3. Use the existing `packs/` repo subdirectories as the bundled source (read at compile time via `include_str!` or `include_bytes!` per file).
4. Update `argo packs list` so the first-run user sees ≥5 entries.
5. Tests at `crates/argo-vertical-packs/tests/bootstrap_on_first_run_test.rs` (~150-250 LOC): tempdir cache, call `PackLoader::new + list`, assert ≥5 entries; second call doesn't re-bootstrap.

## Files you may touch

- `crates/argo-vertical-packs/src/loader.rs` — bootstrap path.
- `crates/argo-vertical-packs/build.rs` — if needed, to inline `packs/` at compile time.
- `crates/argo-vertical-packs/tests/bootstrap_on_first_run_test.rs` — new.

## Files you must NOT touch

- `packs/**` — the source catalogue (these get read by the build, not modified).
- `crates/argo-vertical/**`.

## Acceptance criteria

- [ ] Fresh `tempdir` cache + `PackLoader::new + list` returns ≥5 entries.
- [ ] Second call on the same cache does NOT re-bootstrap (idempotent).
- [ ] `argo packs list` on a fresh `~/.argo/packs/` shows the bundled set.
- [ ] No new file > 500 LOC; no fn > 80 LOC.
- [ ] `cargo test -p argo-vertical-packs` passes.

## Branch + PR

- Branch: `claude-mission-skill-library-bootstrap-on-fresh-install` or `codex-…`.
- PR title: `mission(packs): bootstrap ~/.argo/packs/ on fresh install (mission-skill-library-bootstrap-on-fresh-install)`.

## Notes

- ~200-350 LOC Rust + tests.
- Option (a) is operator-recommended starting point per CLAUDE.md "enterprise focus, not consumer" — bundled is more predictable than remote.
