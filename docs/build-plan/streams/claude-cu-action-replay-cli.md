# claude-cu-action-replay-cli — `argo cu replay <session-id>` for step-by-step CU action replay (markdown + GIF)

**Owner**: claude. **Depends on**: `claude-cu-screenshot-storage` (this brief consumes its `BlobRef` index) — soft dep; can land first if storage falls back to disk.

## Why

When a CU run goes sideways operators currently triage by tailing `browser.jsonl` plus opening PNGs by filename. Painful, slow, and the audit trail leaves the box. This brief ships a first-class CLI surface that reconstructs a CU session as a navigable artifact: per-step markdown for the action trajectory + a GIF rendering for the visual trace.

Mirrors the existing `argo session replay --stream` shape (NDJSON) but specialized for computer-use evidence.

## Scope

1. **NEW** `crates/argo-cli/src/handlers/cu_replay.rs` (~360 LOC):
   - `cmd_replay(session_id, opts) -> ArgoResult<()>` — top-level handler
   - Loads CU trajectory via existing `argo-store` queries (action rows + screenshot blob_refs)
   - Renders **markdown** to stdout (or `--out <path>`): one section per step with action description, screenshot inline (file:// path or data: URL with `--inline`), timing, captured signals
   - Renders **GIF** with `--gif <path>` — concatenates screenshots at 1fps default (configurable via `--gif-fps`); uses the existing `image` workspace dep + `gif` encoder
   - Supports `--from <step>` / `--to <step>` for slicing
   - Honors the markdown-format family (matches existing read-side surfaces — `argo session replay`, etc.)
2. **CLI plumbing** — add `cu replay` clap subcommand under the existing `cu` command tree (already exists per pre-swarm snapshot; if not, create `argo cu` parent under `argo-cli/src/commands.rs` keeping LOC under 500)
3. **Help-smoke test** — every clap variant must have a help-smoke row (per project conventions)

## Files you may touch

- `crates/argo-cli/src/handlers/cu_replay.rs` (NEW, ≤ 400 LOC)
- `crates/argo-cli/src/handlers/cu_replay_tests.rs` (NEW, ≤ 280 LOC sibling) — 8 unit tests:
  - markdown rendering for a 3-step trajectory
  - GIF rendering produces a valid GIF header
  - --from / --to slicing
  - --inline embeds screenshots as data URLs
  - empty trajectory → clean error
  - missing session → ArgoError::NotFound
  - --gif-fps validation (positive int)
  - tenant-isolation (cross-tenant session → NotFound)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `pub mod` line
- `crates/argo-cli/src/commands.rs` — narrowly: add `cu replay` clap variant + dispatch
- `crates/argo-cli/tests/cu_replay_smoke.rs` (NEW, ≤ 180 LOC) — 1 integration test seeding a fake session + asserting markdown + GIF outputs
- `crates/argo-cli/tests/help_smoke.rs` — narrowly: 1 row pinning `argo cu replay --help`

## Files you must NOT touch

- `argo-executor-computer-use` internals
- `argo-store` schema (consumes existing tables)
- `argo-store-attached` trait
- `argo-vertical/src/manifest.rs`

## Acceptance criteria

- [ ] `argo cu replay <session-id>` prints a markdown trajectory to stdout
- [ ] `argo cu replay <session-id> --out <path>` writes markdown to file
- [ ] `argo cu replay <session-id> --gif <path>` writes a valid GIF
- [ ] `--from / --to` slicing works
- [ ] `--inline` embeds screenshots as data URLs (no external file refs)
- [ ] `--format json` emits NDJSON one-step-per-line (parity with `session replay --stream`)
- [ ] Markdown output matches the existing markdown-format family
- [ ] Cross-tenant session → ArgoError::NotFound
- [ ] 8 unit tests + 1 integration smoke + 1 help-smoke row
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every file < 500 LOC

## Branch + PR

- Branch: `claude-cu-action-replay-cli`
- PR title: `cli: argo cu replay for step-by-step CU action replay (claude-cu-action-replay-cli)`

## Notes for the implementer

- Read `argo session replay --stream` (NDJSON) end-to-end before starting — your output shape should rhyme.
- Use the `gif` crate (workspace dep candidate) for GIF encoding; if it's not yet a dep, add it through `argo-cli/Cargo.toml` only.
- Default GIF fps = 1 (CU steps are slow; 1fps reads naturally). Bound `--gif-fps` to `1..=30` and reject otherwise.
- The CU trajectory may include explicit `done` and `failure` markers — surface them in the markdown header (status: success / failed / aborted).
- If `claude-cu-screenshot-storage` is not yet merged, fall back to disk paths under `.skycot/outputs/...` — the handler should accept either source via a `ScreenshotSource` enum (Blob or Disk). This keeps the brief unblocked.
