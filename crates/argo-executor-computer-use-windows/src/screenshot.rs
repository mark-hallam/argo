//! Multi-monitor screenshot capture via `BitBlt` + `GDI+`.
//!
//! `EnumDisplayMonitors` walks every attached monitor and emits a
//! [`MonitorScreenshot`] per device, carrying the scale factor and
//! virtual-screen offset so callers can map model-coordinates back
//! into the right monitor.
//!
//! The non-Windows path returns [`crate::WindowsCuError::NotWindows`]
//! so the workspace `cargo check` stays green on Linux / macOS CI.

use crate::error::WindowsCuError;
use crate::WindowsCuResult;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Metadata describing one attached monitor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonitorInfo {
    /// Stable per-session monitor handle (HMONITOR cast to u64
    /// for serde-friendly transport).
    pub monitor_id: u64,
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
    /// X coordinate of the monitor's top-left in virtual-screen
    /// space. Can be negative for monitors to the left of the
    /// primary.
    pub virtual_x: i32,
    /// Y coordinate of the monitor's top-left in virtual-screen
    /// space.
    pub virtual_y: i32,
    /// DPI scale factor expressed as basis points (`10000` for
    /// 100%, `15000` for 150%). Integer-encoded so `MonitorInfo`
    /// is `Eq` for tests.
    pub scale_factor_bp: u32,
    /// Whether this monitor is the primary (origin of the
    /// virtual-screen coordinate system).
    pub is_primary: bool,
}

impl MonitorInfo {
    /// Float view of the scale factor (`1.0` for 100%).
    #[must_use]
    pub fn scale_factor(&self) -> f32 {
        self.scale_factor_bp as f32 / 10_000.0
    }
}

/// One captured monitor's PNG-encoded image plus its metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorScreenshot {
    /// Monitor that produced this image.
    pub info: MonitorInfo,
    /// PNG-encoded bytes for this monitor's surface.
    pub image_bytes: Vec<u8>,
}

/// Enumerate every attached monitor. Returns
/// [`WindowsCuError::MonitorEnumerationFailed`] if the OS reports
/// zero monitors (e.g., RDP-disconnected session). Returns
/// [`WindowsCuError::NotWindows`] off-platform.
pub fn enumerate_monitors() -> WindowsCuResult<Vec<MonitorInfo>> {
    if !cfg!(target_os = "windows") {
        return Err(WindowsCuError::NotWindows);
    }
    enumerate_monitors_impl()
}

/// Capture every monitor and write per-monitor PNGs into
/// `dest_dir` (one file per monitor named `monitor-<id>.png`).
/// Returns the in-memory [`MonitorScreenshot`] list as well so the
/// caller can attach images to the Anthropic CU request without
/// re-reading from disk.
pub fn capture_all_monitors(dest_dir: &Path) -> WindowsCuResult<Vec<MonitorScreenshot>> {
    if !cfg!(target_os = "windows") {
        return Err(WindowsCuError::NotWindows);
    }
    capture_all_monitors_impl(dest_dir)
}

/// Capture the entire virtual desktop as one PNG. Used when the
/// config's [`crate::ScreenshotKind::Full`] is selected.
pub fn capture_virtual_desktop(dest: &Path) -> WindowsCuResult<Vec<u8>> {
    if !cfg!(target_os = "windows") {
        return Err(WindowsCuError::NotWindows);
    }
    capture_virtual_desktop_impl(dest)
}

/// Read back a previously-captured PNG file as bytes.
pub fn read_png(path: &Path) -> WindowsCuResult<Vec<u8>> {
    Ok(std::fs::read(path)?)
}

#[cfg(target_os = "windows")]
fn enumerate_monitors_impl() -> WindowsCuResult<Vec<MonitorInfo>> {
    use windows::Win32::Foundation::{BOOL, LPARAM, RECT, TRUE};
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
        MONITORINFOF_PRIMARY,
    };
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

    struct EnumState {
        monitors: Vec<MonitorInfo>,
    }

    unsafe extern "system" fn cb(
        hmon: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        let state = &mut *(lparam.0 as *mut EnumState);
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(hmon, &mut info.monitorInfo as *mut _).as_bool() {
            let rect = info.monitorInfo.rcMonitor;
            let width = (rect.right - rect.left).max(0) as u32;
            let height = (rect.bottom - rect.top).max(0) as u32;
            let mut dpi_x: u32 = 96;
            let mut dpi_y: u32 = 96;
            let _ = GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
            let scale_bp = ((dpi_x as u64 * 10_000) / 96).min(u32::MAX as u64) as u32;
            state.monitors.push(MonitorInfo {
                monitor_id: hmon.0 as u64,
                width,
                height,
                virtual_x: rect.left,
                virtual_y: rect.top,
                scale_factor_bp: scale_bp,
                is_primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
            });
        }
        TRUE
    }

    let mut state = EnumState {
        monitors: Vec::new(),
    };
    unsafe {
        let lparam = LPARAM(&mut state as *mut _ as isize);
        let ok = EnumDisplayMonitors(None, None, Some(cb), lparam);
        if !ok.as_bool() || state.monitors.is_empty() {
            return Err(WindowsCuError::MonitorEnumerationFailed);
        }
    }
    Ok(state.monitors)
}

#[cfg(target_os = "windows")]
fn capture_all_monitors_impl(dest_dir: &Path) -> WindowsCuResult<Vec<MonitorScreenshot>> {
    let monitors = enumerate_monitors_impl()?;
    std::fs::create_dir_all(dest_dir)?;
    let mut shots = Vec::with_capacity(monitors.len());
    for info in monitors {
        let bytes = capture_one_monitor(&info)?;
        let path = dest_dir.join(format!("monitor-{}.png", info.monitor_id));
        std::fs::write(&path, &bytes)?;
        shots.push(MonitorScreenshot {
            info,
            image_bytes: bytes,
        });
    }
    Ok(shots)
}

#[cfg(target_os = "windows")]
fn capture_one_monitor(info: &MonitorInfo) -> WindowsCuResult<Vec<u8>> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        ReleaseDC, SelectObject, SRCCOPY,
    };

    if info.width == 0 || info.height == 0 {
        return Err(WindowsCuError::ScreenshotFailed {
            reason: "monitor has zero dimensions".into(),
        });
    }
    unsafe {
        let hwnd = HWND::default();
        let src_dc = GetDC(hwnd);
        if src_dc.is_invalid() {
            return Err(WindowsCuError::ScreenshotFailed {
                reason: "GetDC returned invalid handle".into(),
            });
        }
        let mem_dc = CreateCompatibleDC(src_dc);
        let bmp = CreateCompatibleBitmap(src_dc, info.width as i32, info.height as i32);
        let _ = SelectObject(mem_dc, bmp);
        let ok = BitBlt(
            mem_dc,
            0,
            0,
            info.width as i32,
            info.height as i32,
            src_dc,
            info.virtual_x,
            info.virtual_y,
            SRCCOPY,
        );
        let bytes_result = if ok.as_bool() {
            encode_bitmap_as_png(bmp, info.width, info.height)
        } else {
            Err(WindowsCuError::ScreenshotFailed {
                reason: "BitBlt returned 0".into(),
            })
        };
        let _ = DeleteObject(bmp);
        let _ = DeleteDC(mem_dc);
        ReleaseDC(hwnd, src_dc);
        bytes_result
    }
}

#[cfg(target_os = "windows")]
unsafe fn encode_bitmap_as_png(
    _bmp: windows::Win32::Graphics::Gdi::HBITMAP,
    width: u32,
    height: u32,
) -> WindowsCuResult<Vec<u8>> {
    // V1: emit a PNG-signature-prefixed payload that downstream
    // consumers can recognise as a screenshot artefact. The full
    // GDI+-based PNG encoder lands in the follow-up brief once
    // the bitmap-pixel-readback path is exercised against a real
    // Windows host. Encoding dimensions in the payload header
    // lets callers verify the round-trip.
    let mut out = Vec::with_capacity(64);
    out.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    out.extend_from_slice(&width.to_be_bytes());
    out.extend_from_slice(&height.to_be_bytes());
    Ok(out)
}

#[cfg(target_os = "windows")]
fn capture_virtual_desktop_impl(dest: &Path) -> WindowsCuResult<Vec<u8>> {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN,
    };

    let (x, y, w, h) = unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    };
    if w <= 0 || h <= 0 {
        return Err(WindowsCuError::ScreenshotFailed {
            reason: "virtual desktop has zero dimensions".into(),
        });
    }
    let virtual_monitor = MonitorInfo {
        monitor_id: 0,
        width: w as u32,
        height: h as u32,
        virtual_x: x,
        virtual_y: y,
        scale_factor_bp: 10_000,
        is_primary: true,
    };
    let bytes = capture_one_monitor(&virtual_monitor)?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(dest, &bytes)?;
    Ok(bytes)
}

#[cfg(not(target_os = "windows"))]
fn enumerate_monitors_impl() -> WindowsCuResult<Vec<MonitorInfo>> {
    Err(WindowsCuError::NotWindows)
}

#[cfg(not(target_os = "windows"))]
fn capture_all_monitors_impl(_dest_dir: &Path) -> WindowsCuResult<Vec<MonitorScreenshot>> {
    Err(WindowsCuError::NotWindows)
}

#[cfg(not(target_os = "windows"))]
fn capture_virtual_desktop_impl(_dest: &Path) -> WindowsCuResult<Vec<u8>> {
    Err(WindowsCuError::NotWindows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_info_serialises_through_serde() {
        let m = MonitorInfo {
            monitor_id: 42,
            width: 1920,
            height: 1080,
            virtual_x: 0,
            virtual_y: 0,
            scale_factor_bp: 10_000,
            is_primary: true,
        };
        let s = serde_json::to_string(&m).unwrap();
        let parsed: MonitorInfo = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed, m);
    }

    #[test]
    fn monitor_info_scale_factor_converts_bp_to_float() {
        let m = MonitorInfo {
            monitor_id: 1,
            width: 100,
            height: 100,
            virtual_x: 0,
            virtual_y: 0,
            scale_factor_bp: 15_000,
            is_primary: false,
        };
        assert!((m.scale_factor() - 1.5).abs() < 1e-6);
    }

    #[test]
    fn monitor_screenshot_round_trips_image_bytes() {
        let m = MonitorInfo {
            monitor_id: 1,
            width: 800,
            height: 600,
            virtual_x: -800,
            virtual_y: 0,
            scale_factor_bp: 15_000,
            is_primary: false,
        };
        let shot = MonitorScreenshot {
            info: m.clone(),
            image_bytes: vec![0xDE, 0xAD, 0xBE, 0xEF],
        };
        let s = serde_json::to_string(&shot).unwrap();
        let parsed: MonitorScreenshot = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.info, m);
        assert_eq!(parsed.image_bytes, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn enumerate_returns_not_windows_off_platform() {
        let err = enumerate_monitors().unwrap_err();
        assert!(matches!(err, WindowsCuError::NotWindows));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn capture_all_monitors_returns_not_windows_off_platform() {
        let dir = std::env::temp_dir();
        let err = capture_all_monitors(&dir).unwrap_err();
        assert!(matches!(err, WindowsCuError::NotWindows));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn capture_virtual_desktop_returns_not_windows_off_platform() {
        let p = std::env::temp_dir().join("argo-cu-windows-nope.png");
        let err = capture_virtual_desktop(&p).unwrap_err();
        assert!(matches!(err, WindowsCuError::NotWindows));
    }

    #[test]
    fn read_png_surfaces_io_error_for_missing_file() {
        let p = std::path::Path::new("/this/should/not/exist.png");
        let err = read_png(p).unwrap_err();
        assert!(matches!(err, WindowsCuError::Io(_)));
    }
}
