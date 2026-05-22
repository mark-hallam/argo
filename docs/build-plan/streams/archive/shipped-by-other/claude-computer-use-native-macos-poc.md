# V1.5 — Native macOS computer-use PoC + design

**Owner**: claude. **Depends on**: B-computer-use-executor (#787)
merged.

## Scope (deliberately bounded)

The current `argo-executor-computer-use` runs Anthropic's CU
loop inside an E2B Linux microVM with browser. For "any task
a human can do on a PC/Mac" we need NATIVE desktop control:
drive Photoshop, Excel, native Slack/Mail, the Finder, etc.

This brief ships a **proof-of-concept + design doc**, NOT the
full integration. Scope is bounded to:

1. **Design doc** at
   `docs/skycot/COMPUTER_USE_NATIVE_MACOS.md` — architectural
   plan covering: Anthropic CU model with native macOS, route
   for screenshots (CGWindowList vs accessibility tree), input
   injection (CGEventCreate vs accessibility actions), permission
   model (privacy / accessibility prompts), sandboxing story,
   coexistence with the existing E2B-browser executor.
2. **One working executor** — `argo-executor-computer-use-macos`
   crate that drives ONE specific app via macOS accessibility
   APIs (pick: macOS Notes — small, scriptable, low-risk for the
   PoC). Implements `Executor` trait. Calls Anthropic CU model
   with screenshots + sends back keyboard/mouse via Core
   Graphics events.
3. **One smoke test** — gated on `ARGO_NATIVE_MACOS_INTEGRATION=1`
   and `[ "$(uname)" = "Darwin" ]`, drives Notes to create a
   note, verifies it appears via AppleScript readback.

Follow-up briefs (NOT in scope here, queued after this lands):
- macOS accessibility broader app coverage
- Windows native (separate brief)
- ExecutorKind::ComputerUseNative variant + dispatch wiring

## Files you may touch
- `docs/skycot/COMPUTER_USE_NATIVE_MACOS.md` (NEW design doc)
- `crates/argo-executor-computer-use-macos/` (NEW crate)
- `crates/argo-executor-computer-use-macos/Cargo.toml` (NEW)
- `crates/argo-executor-computer-use-macos/src/{lib,executor,a11y,events,screenshot}.rs` (NEW)
- `crates/argo-executor-computer-use-macos/tests/notes_smoke.rs` (NEW)
- `Cargo.toml` workspace — narrowly: add new member
- BLANKET CLI ALLOWLIST applies if dispatch wiring is added

## Acceptance criteria
- [ ] Design doc covers all 6 architectural questions in scope
- [ ] PoC drives macOS Notes via accessibility APIs
- [ ] Anthropic CU model integration works (screenshots → tool calls → events)
- [ ] Permission prompts handled gracefully (clear error if accessibility denied)
- [ ] Smoke test passes on macOS in CI (with proper env gating)
- [ ] No regression to existing argo-executor-computer-use (E2B)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-computer-use-native-macos-poc`
- PR title: `runtime: native macOS computer-use PoC + design (claude-computer-use-native-macos-poc)`
