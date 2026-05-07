---
title: Computer-use — native macOS
status: design — V1.5 PoC stage (claude-computer-use-native-macos-poc)
audience: argo runtime engineers
---

# Computer-use — native macOS

## Why a second executor

Today `argo-executor-computer-use` runs Anthropic's CU loop inside an
E2B Linux microVM and a bundled browser. That's enough for browser
work — checking a marketplace, filling a form, scraping a price — but
the mission's "any task a human can do on a Mac" promise stretches
past the browser. Photoshop, Excel, native Slack, Mail, Finder,
Reaper, Logic Pro, Xcode — none of those run inside the E2B image,
and even the ones with a web build (Slack, Notion) lose deep
keyboard-shortcut control once you leave the native binary.

The native macOS executor closes that gap. It runs side-by-side with
the E2B browser executor — both implement the same `Executor` trait,
both speak the Anthropic CU protocol, both feed the same audit log —
but each chooses its sandbox strategy from the action manifest.

This doc covers the V1.5 PoC: design plus a working executor that
drives macOS Notes. Notes is the smallest, lowest-risk target —
scriptable, sandboxed by Apple, no destructive operations under
default permissions. The patterns established here generalize to
larger apps as follow-up briefs.

## Architecture at a glance

```
┌────────────────────┐         ┌─────────────────────────┐
│  Action manifest   │         │  Anthropic /v1/messages │
│  executor=cu_macos │◀──────▶ │  computer-use-2024-10-22│
└────────┬───────────┘         └────────▲────────────────┘
         │                              │
         ▼                              │
┌──────────────────────────────┐        │
│  argo-executor-computer-use- │        │
│  macos::MacosCuExecutor      │────────┘ system_prompt + screenshot
│                              │◀──────── tool_use(CuAction) | stop
│  ┌────────┐ ┌────────┐ ┌───┐ │
│  │screen  │ │ events │ │a11│ │
│  │  shot  │ │        │ │ y │ │
│  └────────┘ └────────┘ └───┘ │
└────────┬───────────────┬─────┘
         │               │
         ▼               ▼
   CGWindowList    CGEventCreate    AXUIElement / AppleScript
   (PNG bytes)    (mouse + key)     (Notes app control + readback)
```

The executor is structurally identical to the E2B variant: both wrap
an `AnthropicClient`, both run a step loop, both budget tokens +
wallclock, both write a `cu_macos.jsonl` audit row per step. What
changes is the sandbox driver — instead of HTTP calls to E2B, the
macOS variant calls into Core Graphics + the macOS accessibility
framework directly. That keeps the public `Executor` trait
unchanged.

## Six architectural questions

### 1. Anthropic CU model integration

The `computer-use-2024-10-22` tool variant (already used by the E2B
executor) is desktop-shaped: it emits `click(x, y)`, `type(text)`,
`key(keys)`, `scroll(x, y, dy)`, `mouse_move(x, y)`. Every primitive
maps directly onto Core Graphics events — no protocol translation
needed. The model gets the same `system_prompt + screenshot`
shape; the response shape is the same `tool_use | stop_reason`.

Concretely the macOS executor reuses the existing `anthropic.rs`
machinery (`CuStepRequest`, `CuStepResponse`, `parse_response`) by
linking against `argo-executor-computer-use` as a `path` dependency.
Only the screen dimensions in `CuStepRequest` change — instead of
the E2B sandbox's fixed 1920×1080, we report the live macOS display
geometry.

The Anthropic API surface is unchanged: same `x-api-key`, same
`anthropic-version: 2023-06-01`, same `anthropic-beta:
computer-use-2024-10-22`. Per-action token + wallclock budget is
shared with the E2B executor; either executor can saturate the
worker's budget and trigger handoff.

### 2. Screenshots — CGWindowList vs accessibility tree

Two paths exist on macOS:

- **`CGWindowListCreateImage`** (Core Graphics) returns a CGImage of
  the entire display, a window, or a bounded rect. PNG-encoded
  output is ~1-3 MB at Retina resolution. Permission required:
  Screen Recording (`NSScreenRecordingUsageDescription`). Refresh
  rate: synchronous, ~30-80 ms.
- **`AXUIElementCopyAttributeValue` with `kAXValueAttribute`**
  returns DOM-shaped accessibility trees as text. Smaller payload
  (~5-50 KB), but: (1) Anthropic's CU model expects pixel input, not
  text; (2) many native apps publish minimal a11y trees that miss
  visual context (e.g., Photoshop renders custom widgets that the
  a11y tree describes as opaque rectangles).

**Decision**: PNG via `CGWindowListCreateImage`. The CU model is
trained on pixels; trying to substitute text would lose information
the model was trained to use. The a11y tree is supplementary — used
inside `a11y.rs` to discover semantic targets (e.g., "find the New
Note button by AXRole=AXButton AXTitle=New Note") which is a
post-screenshot guard, not a primary input.

The PoC captures the front window only by default
(`kCGWindowListOptionIncludingWindow`) to keep payloads small. A
`MacosCuConfig::full_display: bool` switch will land in a follow-up
brief once we hit cases where window-only loses critical context
(e.g., dock-based interactions).

### 3. Input injection — CGEventCreate vs accessibility actions

Two paths again:

- **`CGEventCreateMouseEvent` + `CGEventCreateKeyboardEvent`**
  (Core Graphics, in the
  `core_graphics::event::CGEvent` Rust binding). Posts global
  events at the OS level — every app sees them as if from a real
  mouse/keyboard. Permission required: Input Monitoring +
  Accessibility (`NSAccessibilityUsageDescription`).
- **`AXUIElementPerformAction` with `kAXPressAction`** (HIServices /
  ApplicationServices). Targets a specific element by AXUIElementRef;
  doesn't need pointer coordinates. Cleaner for buttons + text
  fields, but doesn't cover everything (no scroll, no drag, no
  arbitrary keystrokes).

**Decision**: Hybrid. Use `AXUIElementPerformAction` for any action
the model can express in semantic terms (click button by name,
focus text field, set value). Fall back to `CGEvent` for everything
else (mouse moves, scrolls, modifier-heavy keystrokes,
drag-and-drop). The PoC ships CGEvent only — the a11y action path
lands as a follow-up once the AXUIElement discovery layer is
mature.

The split matters for reliability: a11y actions survive the target
window losing focus; CGEvent posting requires the target window to
be focused. The `MacosCuExecutor` brings the target app to front
via `NSWorkspace::activateApplication` before any CGEvent post —
this is one of the few places we hold an Objective-C reference for
more than a step.

### 4. Permission model

macOS gates all four required permissions behind explicit
user-granted prompts:

| Permission | Used for | Granted via | Failure mode |
|---|---|---|---|
| Screen Recording | `CGWindowListCreateImage` | System Settings → Privacy & Security → Screen Recording | `CGWindowListCreateImage` returns NULL; we surface `MacosCuError::PermissionDenied { kind: "screen-recording" }` |
| Accessibility | `AXUIElement*`, posting `CGEvent` | System Settings → Privacy & Security → Accessibility | `AXUIElementCreateApplication` returns `kAXErrorAPIDisabled` |
| Input Monitoring | Some `CGEvent` keystroke flavors | System Settings → Privacy & Security → Input Monitoring | Posted events silently dropped (no error) |
| Automation | AppleScript control of target apps | First osascript call triggers a "argo wants to control Notes" dialog | osascript exits 1; we capture stderr |

The PoC checks all four at startup via `MacosCuExecutor::preflight()`
and returns a structured `PermissionReport` listing each missing
permission with the exact System Settings deep-link
(`x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility`).
The operator-facing error message is actionable — it tells you
exactly which checkbox to flip.

CI environments don't grant any of these. The PoC's smoke test
gates on `ARGO_NATIVE_MACOS_INTEGRATION=1` AND `[ "$(uname)" =
"Darwin" ]` AND a successful preflight; if preflight fails the test
exits 0 with a clear `t.skip("missing macOS permissions")` line.
This keeps the test green on default-permissioned CI runners while
still allowing local validation by the operator.

### 5. Sandboxing

The E2B executor inherits microVM isolation for free — every action
is contained inside a Linux VM that's torn down between sessions.
macOS native has no equivalent. We instead rely on three weaker
guards:

1. **App allowlist.** `MacosCuConfig::allowed_apps` is a
   `HashSet<String>` of bundle ids the executor may activate or
   send events to. Default: `[com.apple.Notes]`. Any
   `bring_to_front` request to a non-allowed bundle id returns
   `MacosCuError::AppNotAllowed`. The model can't escape via
   `key(cmd+space, "Terminal", return)` because the resulting
   front-app check fails.
2. **Action allowlist.** `MacosCuConfig::allowed_actions` filters
   the CuAction set per-app. For Notes the PoC allows
   `Click | Type | Key{Cmd+N, Cmd+S, Tab, Return} | Scroll`. Drag,
   `Cmd+Q`, `Cmd+Shift+3` (screenshot to clipboard — could exfiltrate)
   are denied.
3. **Per-step dwell.** Each CGEvent post is followed by a
   ≤200 ms `tokio::time::sleep` so a runaway loop can be operator-
   killed mid-step. The Anthropic loop respects the same per-action
   wallclock budget the E2B executor uses.

This is weaker than VM isolation. Strong isolation requires either
running inside a macOS VM (Tart, UTM, or Apple Virtualization
framework) or using the macOS app sandbox + sandbox-exec to wrap
the executor process. Both are out of scope for the V1.5 PoC. The
right scaling story is **VM-per-tenant via Apple Virtualization
framework** in a V2.x brief — it would re-establish E2B-equivalent
isolation while keeping the native macOS toolchain.

### 6. Coexistence with the existing E2B browser executor

Both executors implement `argo_executor_trait::Executor` (placeholder
name — the unification trait lives in `argo-types` with the
`ExecutorKind` enum). The action manifest's `executor` field
selects which one runs:

```toml
[action.create_marketing_brief]
executor = { kind = "computer_use", profile = "marketplace_browser" }
# vs
[action.draft_meeting_notes]
executor = { kind = "computer_use_native_macos", target_app = "com.apple.Notes" }
```

The dispatcher in `argo-cli/src/handlers/worker_actions_executor.rs`
already has an `ExecutorKind::ComputerUse` arm. The follow-up brief
adds `ExecutorKind::ComputerUseNativeMacos` and routes to the new
crate. The PoC does NOT wire the dispatch arm — the new crate is a
buildable, testable library that the next brief plugs into the
manifest schema + dispatcher.

Sharing across executors:

- `argo-executor-computer-use::anthropic` is reused as a `path`
  dependency. Both executors hit Anthropic the same way; only the
  sandbox driver differs.
- `argo-executor-computer-use::budget` is reused for token +
  wallclock accounting.
- `argo-executor-computer-use::audit` writes to the same
  `<workspace>/.skycot/outputs/<job>/cu/<step>.png` layout. The
  macOS executor adds a `cu_macos.jsonl` sidecar so the audit
  walker can tell which executor produced each step.
- `argo-executor-computer-use::success_metric` is reused for
  skill-learning hooks. A successful native-macOS trajectory feeds
  the same `SkillStore::propose_for_review` path as a successful
  browser trajectory.

What is NOT shared: pool reuse. The E2B executor pools sandboxes
across `(tenant, persona, profile)` keys with a 10-min idle TTL. The
native macOS executor has nothing to pool — there's only one host
desktop, and "warmth" comes from leaving the target app open. A
follow-up brief might pool target-app launches to avoid cold-start
cost (the first `osascript` call to a freshly-opened Notes takes
~800 ms).

## What ships in the V1.5 PoC

- `crates/argo-executor-computer-use-macos/` — new crate
  - `lib.rs` — module decls + re-exports + the `MacosCuExecutor`
    public surface
  - `error.rs` — `MacosCuError` variants (one per failure mode
    above)
  - `screenshot.rs` — `Screenshotter` wrapping
    `CGWindowListCreateImage` via the `core-graphics` crate
  - `events.rs` — `EventPoster` wrapping `CGEvent` for the 5 CU
    primitives (click / type / key / scroll / move)
  - `a11y.rs` — `Accessibility` discovery layer using AppleScript
    as the V1.5 backbone (osascript invocations); the AXUIElement
    Rust path is sketched in `a11y::axui` but ships as a stub —
    follow-up brief integrates `accessibility-sys` properly
  - `notes.rs` — Notes-specific helpers (open Notes, ensure a fresh
    note, read note body via osascript)
  - `executor.rs` — main loop wiring: anthropic → events →
    screenshot → audit
  - `config.rs` — `MacosCuConfig` (allowed_apps, allowed_actions,
    target_app, dwell_ms, anthropic_base_url, anthropic_model)
  - `preflight.rs` — `preflight()` permission check returning
    `PermissionReport`
- `crates/argo-executor-computer-use-macos/tests/notes_smoke.rs` —
  end-to-end test gated on
  `ARGO_NATIVE_MACOS_INTEGRATION=1` AND `cfg(target_os = "macos")`
  AND a successful preflight. Drives Notes through the full loop
  (open → new note → type → save) and verifies via osascript
  readback.
- `Cargo.toml` workspace member added.

What is **out of scope** for the V1.5 PoC (queued as follow-ups):

- `ExecutorKind::ComputerUseNativeMacos` arm in the dispatcher
- AXUIElement-based a11y discovery (uses AppleScript stub)
- Multi-app coverage (Photoshop, Excel, etc.)
- VM-per-tenant isolation via Apple Virtualization framework
- Windows-native executor (separate brief)
- Linux-native executor (separate brief; X11/Wayland choice point)

## Open questions

1. **Permission UX during onboarding.** Should `argo onboard`
   trigger preflight on macOS and walk the operator through the
   permission prompts? Or save that for a separate `argo native
   macos preflight` subcommand the operator runs explicitly?
2. **Multi-display.** The PoC assumes a single primary display.
   `CGWindowListCreateImage` can capture per-display, but the
   Anthropic CU model has no notion of multiple coordinate spaces.
   Pick one display per session and document.
3. **Audit storage cost.** Native macOS screenshots are 5-10× larger
   than E2B browser ones (Retina resolution, real desktop). At
   ~1500-step trajectories this is GB-per-day per worker. The
   follow-up brief should add a downscale + JPEG re-encode option
   in `MacosCuConfig`.
4. **Apple Vision / Spotlight overlays.** When Spotlight or
   Notification Center is open, the front-app check fails. The
   executor surfaces this as `MacosCuError::FrontAppMismatch` for
   now; a richer policy (auto-dismiss before each step) is a
   follow-up.

## References

- [Anthropic computer-use docs](https://docs.anthropic.com/en/docs/build-with-claude/computer-use)
- [Apple — Quartz Display Services (`CGWindowListCreateImage`)](https://developer.apple.com/documentation/coregraphics/cgwindowlistcreateimage)
- [Apple — Accessibility framework](https://developer.apple.com/documentation/applicationservices/axuielement_h)
- `crates/argo-executor-computer-use/src/anthropic.rs` (existing
  Anthropic CU client; reused as `path` dep)
- `docs/skycot/RUNBOOK.md` §"Operator-permission preflight" (queued
  for v1.5)
