# claude-tui-search-everywhere — universal Ctrl-K search across sessions/messages/audit/skills

**Owner**: claude. **Depends on**: none (composes existing read endpoints).

## Scope

The TUI has no global search. To find a session by name, a message
substring, or an audit entry, the user must drill into each surface
individually. This brief adds a Ctrl-K command palette that searches
across 4 data domains in parallel and routes the user to the chosen result.

## Approach (concrete)

1. **NEW** `crates/argo-tui/src/search.rs` (~330 LOC):
   - `pub struct SearchOverlay { query: String, results: Vec<SearchResult>, selected: usize, scope: ScopeFilter }`
   - `enum SearchResult { Session{id,title,score}, Message{session_id,snippet,score}, Audit{id,summary,score}, Skill{name,description,score} }`
   - `enum ScopeFilter { All, Sessions, Messages, Audit, Skills }` — toggleable via Tab
   - `pub fn render(&self, frame: &mut Frame, area: Rect)` — modal overlay 70% width, top input + result list below, scope chip row at bottom
   - `pub fn handle_key(&mut self, key: KeyEvent) -> SearchOutcome` — typing updates query, j/k navigates, Enter routes to selected result, Tab cycles scope, Esc closes
   - `pub async fn refresh(&mut self, gateway: &GatewayClient)` — debounced 150ms, fans out 4 parallel gateway queries
2. **Hook** into `app.rs` global key dispatch: Ctrl-K opens overlay
   from any surface.
3. **Routing**: on Enter, return a `SearchOutcome::Navigate` variant
   that the app loop translates into the appropriate overlay open
   (e.g. `SessionResult` → open session detail pane).
4. **Scoring**: use `fuzzy-matcher` if already a workspace dep,
   otherwise a simple substring + ranked match (do NOT add new deps).

## Data sources (no new wire)

- Sessions: `argo sessions list --json`
- Messages: `argo session messages --json --grep <q>` if grep flag exists
- Audit: `argo session audit --json --grep <q>` if grep flag exists
- Skills: `argo skills list --json`

If `--grep` filter flags do not exist on any of these, this brief does
client-side substring filter on the full lists (acceptable for v1).

## Files you may touch

- `crates/argo-tui/src/search.rs` (NEW, ≤ 350 LOC)
- `crates/argo-tui/src/search_tests.rs` (NEW, ≤ 240 LOC) — 7 unit tests:
  - empty query → empty results
  - typing populates query
  - results scored + ordered correctly
  - Tab cycles scope filter
  - j/k navigation wraps
  - Enter on each result kind returns correct Navigate variant
  - Esc returns Close outcome
- `crates/argo-tui/src/lib.rs` — narrowly: 1 `pub mod search;` line
- `crates/argo-tui/src/app.rs` — narrowly: 1 global key arm + 1 dispatch arm only

## Files you must NOT touch

- `crates/argo-tui/src/input.rs` (at 500 LOC ceiling)
- Any gateway / CLI / manifest file — TUI-only brief
- Do NOT add new deps; reuse existing `Cargo.toml`

## Acceptance criteria

- [ ] Ctrl-K opens search overlay from any TUI surface
- [ ] Typing updates query with 150ms debounce
- [ ] Results show across 4 domains, sorted by score
- [ ] Tab cycles scope filter (All/Sessions/Messages/Audit/Skills)
- [ ] Enter routes to chosen result; Esc closes overlay
- [ ] 7 unit tests pass
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-tui` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-tui-search-everywhere`
- PR title: `tui: universal Ctrl-K search across sessions/messages/audit/skills (claude-tui-search-everywhere)`

## Notes

- Debounce is critical — without it, every keystroke fans out 4 gateway calls.
- Modal: while open, eats all keys except Ctrl-K-defined ones.
- For v1, client-side substring is fine. Server-side grep is a follow-up.


## Amendment 2026-05-15 (input_globals extension point — supersedes old may-touch)

Worker self-BLOCKED 2026-05-08 noting the original may-touch forbade `crates/argo-tui/src/input.rs` (where global-key dispatch lived). PR #1670 (`claude-tui-input-globals-split`) MERGED 2026-05-15 extracts the global-key dispatcher into a sibling `input_globals.rs` with a registry-builder API: `default_global_keys() -> Vec<GlobalKeyBinding>` and `lookup_global_key(registry, key) -> Option<GlobalKeyAction>`.

This brief is now claimable using the new extension point. Updated may-touch:

- `crates/argo-tui/src/search_everywhere.rs` — NEW. The feature module itself (≤300 LOC).
- `crates/argo-tui/src/search_everywhere_tests.rs` — NEW. Unit tests (≤200 LOC, 5+ cases).
- `crates/argo-tui/src/input_globals.rs` — narrowly: add ONE new variant `GlobalKeyAction::SearchEverywhereOverlay` + ONE new entry in `default_global_keys()` mapping key `Ctrl-K` to that variant. **≤15 modified lines total.**
- `crates/argo-tui/src/input.rs` — narrowly: add the dispatch arm that matches `GlobalKeyAction::SearchEverywhereOverlay` and invokes the new feature module. **≤15 modified lines.** (Previously forbidden — now in scope because the file is no longer the centralized dispatcher; it delegates to `input_globals` per #1670.)
- `crates/argo-tui/src/lib.rs` — narrowly: `pub mod search_everywhere;` declaration. **≤3 modified lines.**

Files still forbidden:
- `crates/argo-tui/src/render.rs` — overlay rendering is separate from input dispatch.
- `crates/argo-tui/src/overlays.rs` — same.
- Any other crate.

Acceptance criteria (in addition to the brief's original list):
- New variant added to `GlobalKeyAction` enum.
- New key binding registered in `default_global_keys()`.
- `cargo test -p argo-tui input_globals` includes a coverage case for the new key→variant mapping.
- No regression: existing global keys still dispatch correctly.

**Drift-prevention:** brief adds ONE new pub variant + ONE new pub module — no parallel-name surfaces.


## Amendment 2026-05-15 #2 (rendering scope widening — codex worker self-BLOCK feedback)

Codex worker correctly self-BLOCKED noting that input-side scope alone is insufficient: the overlay renders through `state.<overlay_field>` + `render.rs`/`overlays.rs::draw_<overlay>`. The previous Amendment only opened input dispatch; visible overlay content also needs scope.

Further widening:

- `crates/argo-tui/src/render.rs` — narrowly: add ONE call site in the render loop that invokes the new overlay's render function when the overlay is active. **≤15 modified lines.**
- `crates/argo-tui/src/overlays.rs` — narrowly: add ONE `pub fn draw_<feature>(state, frame, area)` function for the new overlay's visible content. **≤200 LOC for the new function** (may include sub-state struct + helpers in the same file if cohesive).
- `crates/argo-tui/src/state.rs` (or wherever app state lives) — narrowly: add ONE field for the overlay's open/closed state + any cursor/selection sub-state. **≤30 modified lines.**

These are the same load-bearing edits the original brief implied. With both amendments in place, the brief is fully claimable:
- Input layer: `input_globals.rs` (variant + key) + `input.rs` (dispatch arm)
- Rendering layer: `state.rs` (overlay field) + `overlays.rs` (`draw_*` fn) + `render.rs` (call site)
- Module: NEW feature module + tests + `lib.rs` declaration

No file > 500 LOC after; behavior of existing overlays preserved.
