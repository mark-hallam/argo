# claude-store-schema-doc-generation — auto-generate SCHEMA.md from migrations + structs

**Owner**: claude. **Depends on**: nothing. **Phase B docs-as-code narrow.**

## Why

`docs/skycot/SCHEMA.md` (if it exists today) is hand-maintained and **inevitably drifts** from the actual store schema. The store has ~20+ tables across `crates/argo-store/migrations/*` and ~25+ Rust struct types in `crates/argo-store/src/types.rs` (or sibling modules). New columns, new tables, renamed indexes — none of these update the doc, and tenants reading the doc to write integrations get stale info.

The structured truth lives in two places:
1. SQL migrations under `crates/argo-store/migrations/` (canonical schema)
2. Rust struct annotations on the rows we read/write (canonical Rust shape)

A generator that reads both and produces SCHEMA.md as a build artifact eliminates drift entirely.

## Approach (concrete)

**Build a generator + a CI check.** Zero runtime change.

1. **NEW `scripts/gen-schema-doc.sh`** (~80 LOC):
   - Walks `crates/argo-store/migrations/*.sql` in lexicographic order
   - Parses `CREATE TABLE`, `ALTER TABLE`, `CREATE INDEX` statements via a simple regex pass (sufficient for our SQL; full SQL parsing is overkill)
   - Walks `crates/argo-store/src/**/*.rs` for `#[derive(... sqlx::FromRow ...)]` structs and extracts field names + types
   - Emits markdown to `docs/skycot/SCHEMA.md`:
     - One section per table
     - Column list (name, type, NOT NULL, default, FK references)
     - Indexes
     - Mapping to the corresponding Rust struct (if any)
2. **NEW `crates/argo-store/src/schema_doc_annotations.rs`** (~120 LOC, optional but recommended):
   - Lightweight attribute macro `#[schema_doc(table = "sessions", description = "Session-scoped state")]` (or just a doc-comment convention `/// schema_doc: table=sessions`) that the generator picks up to enrich the markdown.
3. **NEW `.github/workflows/schema-doc-check.yml`** (or extend existing CI workflow):
   - Runs `scripts/gen-schema-doc.sh > /tmp/SCHEMA.md`
   - Diffs against committed `docs/skycot/SCHEMA.md`
   - Fails if non-empty diff (i.e. doc is stale)
4. **Initial generated file**: run the script once and commit the output.

## Files you may touch

- `scripts/gen-schema-doc.sh` (NEW, ≤ 120 LOC)
- `docs/skycot/SCHEMA.md` (NEW or regenerated, size depends on tables)
- `crates/argo-store/src/schema_doc_annotations.rs` (NEW, ≤ 200 LOC, optional)
- `.github/workflows/*.yml` — narrowly: 1 new step (or 1 new workflow file)
- `crates/argo-store/src/lib.rs` — narrowly: `pub mod schema_doc_annotations;` if added

## Files you must NOT touch

- Any migration file (no schema change, only documentation)
- Any store query / handler logic
- `argo-types` — no new types

## Acceptance criteria

- [ ] `scripts/gen-schema-doc.sh` runs locally and produces `docs/skycot/SCHEMA.md`
- [ ] `docs/skycot/SCHEMA.md` is checked into the repo + committed
- [ ] CI fails if `gen-schema-doc.sh` output diverges from the committed file (drift guard)
- [ ] Every table in the migrations is represented in the doc
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` clean
- [ ] Every modified file ≤ 500 LOC
- [ ] `gh pr view --json mergeable` returns MERGEABLE post-push

## Branch + PR

- Branch: `claude-store-schema-doc-generation`
- PR title: `store: auto-generate SCHEMA.md from migrations + struct annotations (claude-store-schema-doc-generation)`

## Notes for the implementer

- Bash + `awk`/`grep`/`sed` is sufficient for the regex pass. Don't reach for a full SQL parser.
- For the Rust-side parse, you can use `cargo expand` output or just walk source with a simple state-machine looking for `#[derive(sqlx::FromRow)]` and the next `pub struct` block.
- If a column has a Rust struct mapping, link them in the markdown (anchor link to the struct's rustdoc page or section).
- The CI drift check is the safety net. Without it the doc still drifts.
- If a migration uses a SQL feature the regex doesn't handle (CTE in CREATE, etc.), document the limitation in the script's header and skip that statement — DO NOT widen scope to a full parser.
- Keep the markdown stable across runs (sort tables alphabetically) so diffs are reviewable.
