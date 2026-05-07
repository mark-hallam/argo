# claude-cu-native-linux-poc — native Linux computer-use companion crate (mirror of #941 macOS PoC)

**Owner**: claude. **Depends on**: nothing — standalone crate, mirrors `argo-executor-computer-use-macos` (#941). Dispatch wiring is a follow-up (cf. mission-cu-native-macos-executor-dispatch).

## Why

Mission claim 5 says "real-world execution = any task a human can do on a PC. Workers must have BOTH MCP AND computer use." #941 covers macOS via `screencapture` + `osascript`. Linux desktop workers (Wayland-or-X11 GNOME / KDE) have no native CU path today — they fall through to E2B. This brief ships the Linux-side companion so a Linux operator can run native CU without an E2B round-trip.

## Scope

NEW crate `crates/argo-executor-computer-use-linux` modeled exactly on `argo-executor-computer-use-macos` (~10 src files, ~1700 LOC). Subprocess-based primitives:

- **screenshot** → `scrot --silent <out.png>` (X11) with a `grim` fallback path stubbed for Wayland (Wayland support flagged TODO; X11 only in v1)
- **mouse / keyboard** → `xdotool mousemove <x> <y> click 1`, `xdotool key <keysym>`, `xdotool type <text>`
- **window enum / focus** → `xdotool search --name <pattern>` and `xdotool windowactivate`

Same public surface as the macOS crate: `LinuxComputerUse::screenshot()`, `::click(x, y)`, `::key(keysym)`, `::type(text)`, `::session(opts) -> Session`. The crate must compile cleanly on macOS (deps optional, runtime probes report unsupported) so the workspace stays cross-platform.

## Files you may touch

- `crates/argo-executor-computer-use-linux/**` (NEW crate, ≤ 1800 LOC across files)
  - `Cargo.toml` — minimal deps: `serde`, `tokio`, `argo-types`, `argo-error`, `tracing` (no new workspace deps)
  - `src/lib.rs` (≤ 80 LOC) — module decls + re-exports
  - `src/screenshot.rs` (≤ 200 LOC) — `scrot` subprocess wrapper, error path on missing binary
  - `src/input.rs` (≤ 250 LOC) — `xdotool` mouse + keyboard wrappers
  - `src/window.rs` (≤ 200 LOC) — window enumeration + focus
  - `src/session.rs` (≤ 300 LOC) — orchestration + step log
  - `src/probe.rs` (≤ 120 LOC) — runtime probe for `xdotool` + `scrot` presence; report `Unsupported` cleanly on mac
  - `src/error.rs` (≤ 100 LOC) — local error enum + `From` to `ArgoError`
  - sibling `_tests.rs` per non-trivial file (≤ 250 LOC each); skip on non-Linux via `#[cfg(target_os = "linux")]`
- `Cargo.toml` (workspace root) — narrowly: 1 `members =` line addition
- `README.md` inside the new crate — ~40 LOC, lists `xdotool`+`scrot` system deps and lists Wayland gap

## Files you must NOT touch

- `argo-executor-computer-use-macos` — separate crate; do NOT modify
- `argo-executor-computer-use` (E2B variant) — separate
- `worker_actions_executor.rs` / ExecutorKind — dispatch wiring is a sibling brief; this one is a standalone PoC crate
- `argo-vertical/src/manifest.rs` — no schema change

## Acceptance criteria

- [ ] New crate `argo-executor-computer-use-linux` builds clean on Linux + macOS (latter with runtime probe = Unsupported)
- [ ] Public API surface matches `argo-executor-computer-use-macos` 1:1 (screenshot, click, key, type, session)
- [ ] On a Linux dev box with `xdotool` + `scrot` installed, the `session` example takes a screenshot, clicks (100, 100), types "hello", and writes a JSON step log
- [ ] Unit tests for each subprocess wrapper with the binaries mocked via PATH stub
- [ ] Probe returns `Supported` only when both `xdotool` and `scrot` are on PATH; `Unsupported(reason)` otherwise
- [ ] `cargo build --workspace && cargo clippy -p argo-executor-computer-use-linux -- -D warnings && cargo test -p argo-executor-computer-use-linux` clean
- [ ] Every file < 500 LOC
- [ ] Wayland gap noted in README + an issue stub mentioned in the PR body

## Branch + PR

- Branch: `claude-cu-native-linux-poc`
- PR title: `executor(cu): native Linux computer-use PoC crate (claude-cu-native-linux-poc)`

## Notes for the implementer

- Read the macOS crate (#941) end-to-end before starting. Your file layout and trait shapes should mirror it exactly so a future generic `NativeComputerUse` trait can subsume both.
- `xdotool` keysym names differ from macOS osascript keycodes — keep the mapping in `input.rs` table-driven for testability.
- `scrot --silent` fails noisily without an X display; the runtime probe must catch this on headless CI and report `Unsupported`.
- Wayland coverage is explicit follow-up work — file the issue stub but do not block this PR on it. `grim` + `wtype` would be the eventual primitives.
- Dispatch wiring (ExecutorKind variant + adapter file) is a separate follow-up brief mirroring `mission-cu-native-macos-executor-dispatch`.
