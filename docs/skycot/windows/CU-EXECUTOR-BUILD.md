# argo-executor-computer-use-windows — first Windows build

**Brief**: `win-computer-use-executor-first-build` (P1 CROWN JEWEL).
**Machine**: `Latitude5440` — Windows 11 Pro 10.0.26200, `x86_64-pc-windows-msvc`,
rustc/cargo 1.96.0, `windows` crate 0.58.

This crate (2482 LOC: UIAutomation a11y + SendInput events + BitBlt/monitor
screenshot + WTS/UAC/DPI preflight probes) was authored on macOS/Linux, where
its `cfg(target_os = "windows")` body — and the real `windows` 0.58 deps it
calls — **never compiled**. This is its first real Windows compile.

## Result (verified this run)

```
cargo build  -p argo-executor-computer-use-windows            : Finished, BUILD_EXIT=0
cargo clippy -p argo-executor-computer-use-windows -- -D warnings : Finished, CLIPPY_EXIT=0
```

**The crate compiles and is clippy-clean (`-D warnings`) on Windows.** 10 compile
errors fixed; no warnings remain.

## What was fixed (10 errors, 5 files)

| # | File | Error | Fix |
|---|---|---|---|
| 1–2 | `Cargo.toml` | `windows::Win32::Security` + `OpenProcessToken` unresolved (gated behind the `Win32_Security` feature) | Added `"Win32_Security"` to the `windows` feature list. |
| 3 | `src/screenshot.rs:100` | `MONITORINFOF_PRIMARY` not generated in `windows` 0.58 `Graphics::Gdi` | Defined a local `const MONITORINFOF_PRIMARY: u32 = 0x0000_0001` (documented winuser.h value); dropped the bad import. |
| 4–5 | `src/a11y.rs:90,127` | `IUIAutomationElement::is_invalid()` does not exist (COM interfaces here are non-null) | `FindFirst` already returns `Err` on a NULL/not-found element (windows-rs runs `Type::from_abi`, which errors on null). Replaced the `is_invalid()` guard with `FindFirst(...).map_err(|_| A11yLookupFailed { .. })?` — preserves the intended typed error. |
| 6 | `src/events.rs:346` | `hwnd.0 == 0` — `HWND.0` is `*mut c_void`, not an integer | `hwnd.0.is_null()`. |
| 7–9 | `src/preflight/probes.rs:56` | `WTSQuerySessionInformationW(Some(WTS_CURRENT_SERVER_HANDLE), …)` — `Option<HANDLE>` is not `Param<HANDLE>` (3 cascading trait errors) | `WTS_CURRENT_SERVER_HANDLE` is itself a `HANDLE`; passed directly (no `Some(..)`). |
| 10 | `src/screenshot.rs:203` | `as_bool()` on `Result<(), windows::core::Error>` — `BitBlt` returns `Result<()>` in 0.58, not `BOOL` | `if ok.is_ok()`. |

All fixes are confined to the `cfg(target_os = "windows")` body or the
Windows-only `[target.'cfg(target_os = "windows")'.dependencies]`; the
non-Windows placeholder paths and other crates are untouched.

## What is still STUBBED (NO THEATER)

The crate **compiles and links**, but it is NOT yet a working screen-capture
executor. The following is a **pre-existing stub** (authored on macOS, not
introduced by this build-fix) that this build did not change:

- **`screenshot.rs::encode_bitmap_as_png` does not read pixels.** It emits a
  placeholder payload = the 8-byte PNG magic signature + the width/height as
  big-endian bytes — **not** the actual screen content. The real GDI+
  pixel-readback + PNG encoder is explicitly deferred (its own comment: "the
  full GDI+-based PNG encoder lands in the follow-up brief once the
  bitmap-pixel-readback path is exercised against a real Windows host"). So
  `capture_one_monitor` / `capture_virtual_desktop` run end-to-end (real
  `GetDC` → `CreateCompatibleDC`/`Bitmap` → `BitBlt`) but return a placeholder
  blob, not a usable screenshot.

Everything else (UIAutomation element lookup, SendInput key/mouse events,
monitor enumeration, the UAC/secure-desktop/DPI preflight probes) is real
Windows-API code that now compiles — but **none of it has been RUN** here. This
brief is compile + clippy only; runtime behavior is the sibling brief
`win-computer-use-executor-smoke`. Do not read "compiles" as "works".

## Honest gaps / follow-ups

1. **Runtime is unverified** — first execution against a live Windows desktop is
   `win-computer-use-executor-smoke`. Expect the screenshot stub and any
   untested API assumptions to surface there.
2. **PNG encoder** — `encode_bitmap_as_png` needs the real GDI(+) pixel readback
   before screenshots are usable (own brief / part of the smoke follow-up).
3. **`argo-cli` linkage** — argo-cli depends on this crate on Windows
   (`[target.'cfg(target_os = "windows")']`), so the full `argo` binary still
   can't link on Windows until the separate kryptering/PKCS#11 blocker
   (`win-kryptering-cfg-gate-hsm-signing-backend`) lands. This crate building
   standalone is independent of that.
