//! Real-Windows smoke for the native computer-use executor
//! (`win-computer-use-executor-smoke`).
//!
//! This is a **manual, host-gated** smoke — NOT a CI test. It injects real
//! mouse/keyboard input and moves the cursor, so it is double-gated:
//!
//! 1. `#![cfg(target_os = "windows")]` — the file is empty on unix/macOS, so
//!    `cargo test` on those hosts never sees it.
//! 2. `ARGO_NATIVE_WINDOWS_INTEGRATION=1` — without it the test early-returns
//!    with a SKIP, so a routine `cargo test` on Windows does not fire input.
//!
//! Run it explicitly on a Windows desktop, with Notepad open + foregrounded
//! and maximized so the click/type primitives have a safe target:
//!
//! ```text
//! set ARGO_NATIVE_WINDOWS_INTEGRATION=1
//! cargo test -p argo-executor-computer-use-windows --test windows_smoke -- --nocapture
//! ```
//!
//! The test exercises each directly-drivable executor primitive
//! (warm_up → preflight → screenshot capture → mouse move/click → type) and
//! prints per-primitive evidence. It asserts only the deterministic
//! invariants (executor constructs, warm_up succeeds); the input + capture
//! primitives are recorded as evidence rather than asserted, because their
//! outcome depends on the live desktop session. NO THEATER: a partial smoke
//! with honest per-primitive results is the deliverable, not a forced pass.

#![cfg(target_os = "windows")]

use argo_executor_computer_use::e2b::CuAction;
use argo_executor_computer_use_windows::{WindowsCuConfig, WindowsCuExecutor};

fn integration_enabled() -> bool {
    std::env::var("ARGO_NATIVE_WINDOWS_INTEGRATION").as_deref() == Ok("1")
}

#[tokio::test]
async fn windows_cu_executor_smoke() {
    if !integration_enabled() {
        eprintln!(
            "SKIP windows_cu_executor_smoke — set ARGO_NATIVE_WINDOWS_INTEGRATION=1 to run \
             (it injects real input + moves the cursor). Open + foreground Notepad first."
        );
        return;
    }

    let exec = WindowsCuExecutor::new(WindowsCuConfig::for_notepad_app())
        .expect("for_notepad_app config must validate");
    println!("== win-cu smoke: executor constructed (target_app=notepad.exe) ==");

    // 1) warm_up — COM init + per-monitor-v2 DPI awareness. Must succeed on a
    // real Windows desktop; everything downstream depends on it.
    let warm = exec.warm_up();
    println!("[1/5 warm_up]      {warm:?}");
    assert!(warm.is_ok(), "warm_up must succeed on a Windows host: {warm:?}");

    // 2) preflight — six Windows probes (UAC / secure-desktop / DPI /
    // UIAutomation / monitor count / session). Recorded, not asserted: some
    // axes legitimately warn depending on the session.
    let report = exec.preflight().await;
    println!("[2/5 preflight]    all_passed={}", report.all_passed());
    println!("---- preflight summary ----\n{}\n---------------------------", report.actionable_summary());

    // 3) screenshot — monitor enumeration + DC/BitBlt capture path. NOTE: PNG
    // encoding is a documented stub (8-byte signature + 8 bytes of dims =
    // 16 bytes; real pixel encoding is a follow-up brief), so we report the
    // captured dimensions and flag whether real pixels were produced.
    let shot_dir = std::env::temp_dir().join("argo-cu-smoke-shots");
    match exec.warm_up_screenshots(&shot_dir) {
        Ok(shots) => {
            println!("[3/5 screenshot]   monitors={}", shots.len());
            for s in &shots {
                let real_pixels = s.image_bytes.len() > 16;
                println!(
                    "   monitor#{} {}x{} primary={} bytes={} real_pixels={}",
                    s.info.monitor_id,
                    s.info.width,
                    s.info.height,
                    s.info.is_primary,
                    s.image_bytes.len(),
                    real_pixels,
                );
            }
            println!("   (PNG encoder is a known stub — see encode_bitmap_as_png; dims are real, pixels are not)");
        }
        Err(e) => println!("[3/5 screenshot]   GAP: {e}"),
    }

    // 4) mouse move + click at a safe coordinate. Lands in the foreground
    // window (open Notepad maximized so this clicks its edit surface).
    let mv = exec.execute_step(&CuAction::MouseMove { x: 500, y: 400 }).await;
    println!("[4/5 mouse_move]   {mv:?}");
    let click = exec.execute_step(&CuAction::Click { x: 500, y: 400 }).await;
    println!("[4/5 click]        {click:?}");

    // 5) type a marker string into the foreground window (Notepad). Verify the
    // text landed by reading it back from the running app (screenshot via the
    // harness operator); `front_app_after` records which window received it.
    let marker = "skycot-win-cu-smoke";
    let typed = exec
        .execute_step(&CuAction::Type { text: marker.into() })
        .await;
    println!("[5/5 type]         marker=\"{marker}\" {typed:?}");

    // 6) read-back via the Key (modifier-combo) primitive: select-all + copy,
    // so the operator can confirm the typed text landed by reading the
    // clipboard (`Get-Clipboard`) — objective verification with no screen
    // capture. Also exercises the ctrl+<key> combo path.
    let sel = exec
        .execute_step(&CuAction::Key {
            keys: vec!["ctrl".into(), "a".into()],
        })
        .await;
    println!("[6/6 key ctrl+a]   {sel:?}");
    let copy = exec
        .execute_step(&CuAction::Key {
            keys: vec!["ctrl".into(), "c".into()],
        })
        .await;
    println!("[6/6 key ctrl+c]   {copy:?}");
    println!("== win-cu smoke complete — see per-primitive evidence above ==");
    println!("== read-back: run `Get-Clipboard` — it should contain \"{marker}\" if Type+Key landed in Notepad ==");
}
