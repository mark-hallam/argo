# verticals-demo-stubs-promote-or-archive

**Owner**: claude or codex.
**Depends on**: nothing.
**Authored against**: origin/main `ca02058c9` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh verticals-demo-stubs-promote-or-archive`.

🟢 **P3 — clean up 4 demo-only vertical stubs that `argo_vertical::load_dir` silently skips.** Source: `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` §56 F-3 + `verticals/README.md:55-60` ("4 demo-only stubs silently skipped").

`verticals/` contains 36 subdirs. `argo_vertical::load_dir` parses 32 cleanly. 4 directories carry `vertical.toml` files that fail the loader's schema check and are silently skipped:

- `verticals/ap-specialist/`
- `verticals/deal-desk/`
- `verticals/support-tier1/`
- `verticals/vendor-manager/`

Each has incomplete sections — usually missing the required `[workflow]` block or a `worker.charter` field. They were authored as design-time placeholders during the original gap-manifest authoring round but were never finished. Carrying them as silent skips creates two failure modes:

1. **Inventory-drift confusion** — a developer running `argo verticals list` sees 36 subdirs and 32 results; the discrepancy is opaque.
2. **Gateway promise-mismatch** — if any sales surface (marketing, demo decks, internal docs) references these names, the operator believes they exist; the runtime silently disagrees.

Operator decision per row: promote to a full vertical OR archive cleanly.

## Scope

For each of the 4 stub directories:

1. Read its current `vertical.toml` and any sibling files.
2. Run `cargo run -p argo-cli -- verticals load-dir verticals/<name>/` to capture the exact parse error.
3. Two paths:
   - **Promote**: if the stub is salvageable (≥80% of required schema fields present, charter writeable in <50 lines), fill in the missing fields, add a minimal `[workflow]` block with one stub step, add an eval-seed file with at least one example, and confirm `argo_vertical::load_dir` parses cleanly.
   - **Archive**: if the stub is too sparse, move the directory tree to `docs/skycot/verticals-archive/<name>/` and remove from `verticals/`. Add a README at the archive root naming the move date, the original commit SHA, and a one-line "why archived" note.
4. After all 4 are resolved, update `verticals/README.md:55-60` to drop the "silently skipped" paragraph and re-render the loadable-vertical count.
5. Add a new test under `crates/argo-vertical/tests/load_dir_inventory_test.rs` (~80 LOC) that loops over every directory under `verticals/` and asserts `argo_vertical::load_dir` returns `Ok` for each — flips the silent-skip behaviour into a hard error.

## Files you may touch

- `verticals/ap-specialist/` (+ sub-tree) — either complete or remove.
- `verticals/deal-desk/` (+ sub-tree) — either complete or remove.
- `verticals/support-tier1/` (+ sub-tree) — either complete or remove.
- `verticals/vendor-manager/` (+ sub-tree) — either complete or remove.
- `docs/skycot/verticals-archive/` — NEW root for archived stubs (if any).
- `verticals/README.md` — drop the "skipped" paragraph, re-render counts.
- `crates/argo-vertical/tests/load_dir_inventory_test.rs` — NEW.

## Files you must NOT touch

- The 32 loadable verticals. This brief is scoped to the 4 stubs only.
- `crates/argo-vertical/src/` — loader changes belong in a separate brief.
- `web/lib/connectors/registry.ts` or any frontend catalog — the vertical archive is a backend-only cleanup.

## Acceptance criteria

- [ ] All 4 stub directories are either complete (parses via `load_dir`) or archived under `docs/skycot/verticals-archive/`.
- [ ] `cargo test -p argo-vertical load_dir_inventory_test` passes — no silent-skip directories under `verticals/`.
- [ ] `verticals/README.md` no longer mentions silent-skip behaviour and reports the correct loadable count.
- [ ] If any directory is archived, `docs/skycot/verticals-archive/README.md` documents the move date + reason per directory.
- [ ] BUILD_STATUS.md updated with PR number.
- [ ] Eval gap-manifest §56 F-3 row flipped 🔴 → 🟢 with PR back-cite.

## Test plan

- Crate-scoped `cargo test -p argo-vertical`.
- The new `load_dir_inventory_test.rs` walks `verticals/` and asserts every subdir parses; this is the contract pin that catches future drift.
- Manual: `cargo run -p argo-cli -- verticals list` shows the new count (matches the new test's expectation).

## Parallelism

- Manager-worker allowed: no.
- Max children: 1.

## Out of scope

- Promoting any of the 4 stubs into a "production vertical" (eval-seed + workflow + prompts + memory) — even when "promote" is chosen, the deliverable is "parses cleanly via load_dir", not "shipped on marketing".
- Refactoring the loader to expose silent-skip as a structured error — separate brief.
- Adding the archived verticals to the marketplace listing — separate brief.

~50-200 LOC of TOML edits / file moves + ~80 LOC new test. No production Rust code changes.
