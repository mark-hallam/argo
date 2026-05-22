# claude-tui-theme-system — pluggable color themes via [tui.theme] config

**Owner**: claude. **Depends on**: none.

## Scope

TUI colors today are hard-coded across `render.rs`, `viewer.rs`, and
each overlay file. There is no way to switch palettes (light/dark/
solarized/dracula) or define custom themes via config. This brief adds
a `theme.rs` module: a `Theme` struct that centralises every styled
color used in the TUI, plus 4 built-in presets selectable via
`[tui.theme]` in the existing argo config.

## Approach (concrete)

1. **NEW** `crates/argo-tui/src/theme.rs` (~280 LOC):
   - `pub struct Theme { name: String, fg: Color, bg: Color, accent: Color, success: Color, warning: Color, error: Color, muted: Color, border: Color, selection_bg: Color, selection_fg: Color, status_idle: Color, status_busy: Color, status_error: Color, status_offline: Color }`
   - `impl Theme { pub fn dark() -> Self; pub fn light() -> Self; pub fn solarized() -> Self; pub fn dracula() -> Self; }`
   - `pub fn from_config(cfg: &TuiConfig) -> Theme` — looks up `cfg.theme.name`, falls back to `dark` if unknown, applies `cfg.theme.overrides` map
   - Thread `&Theme` through render functions via existing `RenderCtx`
     (or add one if not present); DO NOT use a global static
2. **Modify** `render.rs` minimally: replace each hard-coded
   `Style::default().fg(Color::X)` with `theme.x()`. This is a wide
   sweep — split across 2 PRs if it pushes any file over 500 LOC.
3. **Config**: new `[tui.theme]` section with `name = "dark"` (default)
   and optional `overrides = { accent = "#ff8800" }` map. Reuse the
   existing argo config loader; no new file format.

## Files you may touch

- `crates/argo-tui/src/theme.rs` (NEW, ≤ 300 LOC)
- `crates/argo-tui/src/theme_tests.rs` (NEW, ≤ 220 LOC) — 6 unit tests:
  - each preset (dark/light/solarized/dracula) constructs without panic
  - `from_config` returns dark for unknown name
  - `from_config` returns dracula when `name = "dracula"`
  - overrides map applies correctly
  - hex color parsing for overrides
  - invalid hex falls back to base color (no panic)
- `crates/argo-tui/src/lib.rs` — narrowly: 1 `pub mod theme;` line
- `crates/argo-tui/src/render.rs` — narrowly: replace hard-coded colors with theme lookups (must stay < 500 LOC)
- `crates/argo-config/src/lib.rs` — narrowly: add `[tui.theme]` section to TuiConfig struct + serde derive

## Files you must NOT touch

- `crates/argo-tui/src/input.rs` (at 500 LOC ceiling)
- `crates/argo-tui/src/app.rs` if pushing it over 500 LOC — coordinate with split
- Manifest schema — themes are TUI config, not manifest config
- argo-cli — no new flag, theme is config-only for v1

## Acceptance criteria

- [ ] 4 built-in presets (dark/light/solarized/dracula) compile + render
- [ ] `[tui.theme] name = "solarized"` in config switches palette
- [ ] `[tui.theme] overrides = { accent = "#ff00ff" }` applies override
- [ ] Unknown theme name falls back to dark with a warning log
- [ ] Invalid hex in overrides logs warning + falls back; no panic
- [ ] All overlays render correctly under each preset (manual smoke OK)
- [ ] 6 unit tests pass
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-tui` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-tui-theme-system`
- PR title: `tui: pluggable theme system with 4 presets (claude-tui-theme-system)`

## Notes

- This brief touches many files (every render call site). Land it as a
  stack: PR1 adds `theme.rs` + config wiring with a default Theme that
  matches today's colors exactly (zero visual diff). PR2 swaps render
  call sites to use the theme. PR3 adds the 4 presets.
- Default `dark` MUST exactly match current colors so PR1 is a no-op visually.
- Config field name: `[tui.theme] name = "..."` — match argo's existing kebab-or-snake convention (check `argo-config`).
