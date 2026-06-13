//! Real-Windows screenshot verification for the native CU executor
//! (`win-cu-screenshot-real-png-encode`).
//!
//! Unlike `windows_smoke`, this is **passive** — it only captures the screen
//! (monitor enumeration → DC/BitBlt → GetDIBits → PNG encode). It injects NO
//! mouse/keyboard input, so it is safe to run on an active desktop session.
//! Still host-gated:
//!
//! 1. `#![cfg(target_os = "windows")]` — empty on unix/macOS.
//! 2. `ARGO_NATIVE_WINDOWS_INTEGRATION=1` — without it, early-returns with SKIP
//!    so routine `cargo test` on Windows does not capture the screen.
//!
//! ```text
//! set ARGO_NATIVE_WINDOWS_INTEGRATION=1
//! cargo test -p argo-executor-computer-use-windows --test windows_screenshot_real -- --nocapture
//! ```
//!
//! Asserts the deliverable of the encode brief: each monitor screenshot is a
//! decodable PNG of the reported dimensions (real pixels, not the old 16-byte
//! stub).

#![cfg(target_os = "windows")]

use argo_executor_computer_use_windows::{WindowsCuConfig, WindowsCuExecutor};

fn integration_enabled() -> bool {
    std::env::var("ARGO_NATIVE_WINDOWS_INTEGRATION").as_deref() == Ok("1")
}

#[test]
fn windows_screenshot_encodes_real_png() {
    if !integration_enabled() {
        eprintln!(
            "SKIP windows_screenshot_encodes_real_png — set \
             ARGO_NATIVE_WINDOWS_INTEGRATION=1 to run (captures the screen; no input)."
        );
        return;
    }

    let exec = WindowsCuExecutor::new(WindowsCuConfig::for_notepad_app())
        .expect("config must validate");
    exec.warm_up().expect("warm_up (COM + DPI) must succeed on a Windows host");

    let shot_dir = std::env::temp_dir().join("argo-cu-screenshot-real");
    let shots = exec
        .warm_up_screenshots(&shot_dir)
        .expect("warm_up_screenshots must capture on a Windows host");

    assert!(!shots.is_empty(), "at least one monitor must be captured");
    for s in &shots {
        let bytes = &s.image_bytes;
        println!(
            "monitor#{} {}x{} primary={} png_bytes={}",
            s.info.monitor_id, s.info.width, s.info.height, s.info.is_primary, bytes.len(),
        );
        assert_eq!(
            bytes.get(..8),
            Some(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A][..]),
            "monitor#{} must start with the PNG magic signature",
            s.info.monitor_id,
        );
        assert!(
            bytes.len() > 16,
            "monitor#{} must produce real pixels, not the 16-byte stub",
            s.info.monitor_id,
        );
        let decoded = image::load_from_memory(bytes)
            .unwrap_or_else(|e| panic!("monitor#{} PNG must decode: {e}", s.info.monitor_id))
            .to_rgba8();
        assert_eq!(
            decoded.dimensions(),
            (s.info.width, s.info.height),
            "monitor#{} PNG must decode to the reported dimensions",
            s.info.monitor_id,
        );
    }
    println!("OK — {} monitor screenshot(s) decode to reported dims as real PNGs", shots.len());
}
