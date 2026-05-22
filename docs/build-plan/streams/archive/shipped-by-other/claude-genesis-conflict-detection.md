# Genesis conflict detection — flag overlap with existing verticals

**Owner**: claude. **Depends on**: none (extends existing Genesis crate).

## Scope

Operators sometimes accidentally regenerate a vertical that already
exists ("a sales SDR" when `sales-sdr` is already on disk), or
generate two verticals so similar they should be merged. This brief
detects overlap **before** Genesis writes a new vertical to disk and
asks the operator to confirm or rename.

Overlap signals:
- Same role keywords (e.g. "sales sdr" matches existing sales-sdr/)
- ≥ 60% action vocabulary intersection with an existing vertical
- ≥ 50% memory namespace name overlap

## Approach (concrete)

1. **NEW** `crates/argo-genesis/src/conflict.rs` (~230 LOC):
   - `struct ConflictReport { matches: Vec<ConflictMatch> }`
   - `struct ConflictMatch { existing_vertical: String, score: f32, reasons: Vec<String> }`
   - `pub fn detect(generated: &GeneratedVertical, verticals_root: &Path) -> ArgoResult<ConflictReport>`
     — walks `verticals/*/vertical.toml`, scores each candidate, returns sorted matches
   - `pub fn highest_score(report: &ConflictReport) -> Option<f32>`
2. **Extend** `lib.rs` to `pub mod conflict;`
3. Wire into `pipeline.rs`: run `detect()` after generation; if
   `highest_score >= 0.7`, refuse to write unless `--allow-overlap`
   is set, and surface the report in the error message.

## Files you may touch

- `crates/argo-genesis/src/conflict.rs` (NEW, ≤ 260 LOC)
- `crates/argo-genesis/src/conflict_tests.rs` (NEW, ≤ 220 LOC) — 5 tests:
  - identical role to existing vertical → score ≥ 0.9
  - 60% action overlap, no memory overlap → score reflects both signals
  - empty verticals_root → empty matches, no error
  - missing verticals_root path → ArgoError::NotFound
  - score is in [0.0, 1.0] for every match
- `crates/argo-genesis/src/lib.rs` (narrowly: 1 pub mod line)
- `crates/argo-genesis/src/pipeline.rs` (narrowly: 1 call site + flag plumbing)

## Files you must NOT touch

- `verticals/<demo>/` — read-only inspection only
- `crates/argo-vertical/` schema — no schema change

## Acceptance criteria

- [ ] detect() walks verticals/*/vertical.toml without panicking on missing fields
- [ ] Score is in [0.0, 1.0] for every ConflictMatch
- [ ] reasons[] explains each contributing signal in plain English
- [ ] highest_score returns None for an empty report
- [ ] 5 unit tests pass
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-genesis` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-genesis-conflict-detection`
- PR title: `genesis: detect overlap with existing verticals (claude-genesis-conflict-detection)`

## Notes for the implementer

- Score is a weighted sum: 0.5 × role-keyword Jaccard + 0.3 × action
  Jaccard + 0.2 × namespace Jaccard. Tune later, lock thresholds in a const.
- For role-keyword overlap, lowercase + split-on-whitespace + filter
  stopwords (a tiny English stopword list constant is fine).
- The 0.7 cutoff for refusing to write is the conductor's call —
  document it in code comments, don't hardcode UX in this PR.
