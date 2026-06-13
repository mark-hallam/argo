# Windows CU executor — first real-host smoke

**Brief**: `win-computer-use-executor-smoke` (P2 — first real Windows run).
**Depends on**: `win-computer-use-executor-first-build` (#2906, MERGED).
**Fleet**: WINDOWS. NO THEATER — this records what genuinely ran on a real
Windows desktop, with per-primitive pass/gap and an objective read-back.

## Environment (verified this run)

| Field | Value |
|---|---|
| Machine | `Latitude5440` — Windows 11 Pro 10.0.26200 |
| Host triple | `x86_64-pc-windows-msvc` |
| rustc / cargo | 1.96.0 |
| Built against | `origin/main` @ `3d5f5288d` (post #2906) |
| Harness | `crates/argo-executor-computer-use-windows/tests/windows_smoke.rs` (this PR) |
| Invocation | `set ARGO_NATIVE_WINDOWS_INTEGRATION=1` + `cargo test -p argo-executor-computer-use-windows --test windows_smoke -- --nocapture`, Notepad open + foreground |

The harness is **double-gated** so it never fires input in CI: `#![cfg(target_os = "windows")]`
plus the `ARGO_NATIVE_WINDOWS_INTEGRATION=1` env var (without it the test prints a
SKIP and returns). It drives the executor's directly-callable primitives — it does
not invoke the Anthropic CU loop (no API key needed).

## Result — `test result: ok. 1 passed; 0 failed` (0.61s)

Per-primitive evidence captured live:

| # | Primitive | Result | Evidence |
|---|---|---|---|
| 1 | `warm_up()` (COM init + per-monitor-v2 DPI) | ✅ PASS | `Ok(())` |
| 2 | `preflight()` (6 Windows probes) | ✅ PASS | `all_passed=true` — "all Windows preflight checks passed" |
| 3 | `warm_up_screenshots()` (monitor enum + BitBlt) | 🟡 PARTIAL | `monitors=1`, `monitor#57672349 1920x1080 primary=true` captured — but `bytes=16 real_pixels=false`. The enumeration + DC/BitBlt path runs and the **dimensions are real**; the PNG **encoder is a known stub** (`encode_bitmap_as_png` emits an 8-byte PNG signature + 8 bytes of dims = 16 bytes, not pixels). Real pixel encoding is a tracked follow-up. |
| 4 | `MouseMove { 500, 400 }` | ✅ PASS | `ok: true`, `front_app_after: "…- Notepad"` |
| 5 | `Click { 500, 400 }` | ✅ PASS | `ok: true`, `front_app_after: "…- Notepad"` |
| 6 | `Type { "skycot-win-cu-smoke" }` | ✅ PASS | `ok: true`, `front_app_after: "…- Notepad"` — **and** the marker appeared in Notepad (see read-back) |
| 7 | `Key { ctrl+a }` then `Key { ctrl+c }` | ✅ PASS | both `ok: true`; the select-all + copy produced clipboard content **ending in `skycot-win-cu-smoke`** |

### Objective read-back (no screen capture needed)
After the Type, the harness issued `ctrl+a` + `ctrl+c` via the executor's **Key**
primitive, then the operator read `Get-Clipboard`. The clipboard contained the
Notepad buffer **ending with `skycot-win-cu-smoke`** — proving the executor's
SendInput-driven `Type` and `Key` primitives delivered real keystrokes to the
foreground Notepad. This is a stronger, deterministic alternative to a screenshot
(which would only show the same text). The typed marker was then **undone
(`Ctrl+Z`)** so the operator's pre-existing Notepad draft was left unmodified.

## Honest gaps (NO THEATER)

1. **Screenshot PNG is a stub.** `encode_bitmap_as_png` returns a 16-byte
   signature+dimensions payload, not encoded pixels (see #2906's note). The
   capture path (monitor enumeration, DC creation, BitBlt) genuinely runs and
   reports real dimensions, but there is **no real image yet**. A real GDI+/`image`
   encoder is the documented follow-up — recommend a brief
   `win-cu-screenshot-real-png-encode`.
2. **No visual screenshot evidence in this run.** Driving Claude's own
   screen-capture (computer-use) needs an interactive approval dialog, which
   timed out in this autonomous worker session. Read-back was done via the
   clipboard instead (above), which is objective and self-contained.
3. **Anthropic CU loop (`run()`) not exercised.** This smoke drives the OS
   primitives directly (preflight / screenshot / input). The full model-in-the-loop
   `run()` path is out of scope here and is the dispatcher-wiring follow-up noted
   in `executor.rs` ("not yet wired into argo-cli's resolve_executor").
4. **The harness types into the foreground window.** Run it with a scratch
   Notepad foregrounded; in this run the operator's draft was present and the
   marker was undone afterward.

## Verdict

The native Windows CU executor's **input + preflight primitives work on a real
Windows host** — mouse move, click, Unicode type, and modifier-key combos all
deliver real input (clipboard-verified). The **only gap is screenshot pixel
encoding** (a known stub). First real Windows run: **substantially green, one
honest gap.**
