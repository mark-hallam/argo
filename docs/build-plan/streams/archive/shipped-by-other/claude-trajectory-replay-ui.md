# V1.1 — Trajectory replay TUI overlay

**Owner**: claude. **Depends on**: H-action-replay (#826),
claude-tui-h-stack-panels (#822) merged.

## Scope

`argo action replay <id>` already walks an action's
trajectory step-by-step in the terminal. Operators
investigating worker behavior want a TUI overlay that
shows trajectory replay alongside live state — pause / step /
seek through screenshots + tool calls + assistant turns.

This brief adds a TUI overlay (key `R` to open) that loads
a session's trajectory and renders:
- Top bar: session id, worker, current step idx / total
- Left pane: action list (scrollable)
- Right pane: per-step content — for CU steps a screenshot
  preview (ASCII art / scaled thumbnail); for MCP steps the
  tool call + result; for Human steps the prompt + reply
- Bottom bar: `[Space] play  [←/→] step  [/] search  [q] quit`

- Reuse the existing `argo action replay` data path; the
  overlay is a renderer-only feature
- Screenshots: render via `viuer` or fall back to ASCII
  ratio art if the terminal doesn't support sixel/iTerm
  protocol
- Search: `/<text>` filters action list by step content

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-tui/src/overlays_replay.rs` (NEW —
  sibling of overlays_h_stack.rs / overlays_handoffs.rs /
  overlays_sessions.rs / overlays_budget.rs)
- `crates/argo-tui/src/lib.rs` — narrowly: register the
  overlay module + the `R` key route
- `crates/argo-tui/src/input_h_stack.rs` OR a new
  `input_replay.rs` — narrowly: input handlers (Space /
  arrows / search) for the replay overlay
- `crates/argo-tui/Cargo.toml` — narrowly: add `viuer` dep
  (optional, behind a feature flag if it pulls native
  image deps)

## Acceptance criteria
- [ ] `R` opens the replay overlay
- [ ] Up/Down navigates the action list
- [ ] Left/Right steps backward / forward
- [ ] Space toggles autoplay (1s/step)
- [ ] `/<query>` filters by content match
- [ ] Renders all 3 step kinds (CU / MCP / Human)
- [ ] Screenshot pane gracefully degrades to ASCII when
      terminal doesn't support image protocol
- [ ] 8 unit tests + 2 input-routing tests + 1 snapshot
      test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `claude-trajectory-replay-ui`
- PR title: `tui: trajectory replay overlay (claude-trajectory-replay-ui)`
