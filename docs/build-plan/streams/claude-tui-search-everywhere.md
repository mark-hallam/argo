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
