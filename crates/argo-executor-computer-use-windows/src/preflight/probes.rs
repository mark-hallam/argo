//! Windows-only probe helpers for each preflight axis.
//!
//! One function per entry in [`super::PreflightCheck::all`]. The
//! monitor-axis probes also return a captured value (count,
//! resolution) consumed by [`super::probe`].

#[cfg(target_os = "windows")]
use super::PreflightStatus;

#[cfg(target_os = "windows")]
pub(super) fn probe_uac_consent() -> PreflightStatus {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token: HANDLE = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return PreflightStatus::Unknown;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut ret_len: u32 = 0;
        let result = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        );
        if result.is_err() {
            return PreflightStatus::Unknown;
        }
        // We don't fail the check here — both elevated and
        // non-elevated argo can succeed depending on the target.
        // The fail mode is detected post-hoc via
        // IntegrityMismatch. Pass means "we know our IL".
        let _ = elevation.TokenIsElevated;
        PreflightStatus::Pass
    }
}

#[cfg(target_os = "windows")]
pub(super) fn probe_secure_desktop() -> PreflightStatus {
    use windows::Win32::System::RemoteDesktop::{
        WTSQuerySessionInformationW, WTSConnectState, WTS_CURRENT_SERVER_HANDLE,
        WTSActive,
    };
    use windows::Win32::System::RemoteDesktop::WTSGetActiveConsoleSessionId;

    unsafe {
        let session_id = WTSGetActiveConsoleSessionId();
        let mut buffer = windows::core::PWSTR::null();
        let mut bytes_returned: u32 = 0;
        let result = WTSQuerySessionInformationW(
            Some(WTS_CURRENT_SERVER_HANDLE),
            session_id,
            WTSConnectState,
            &mut buffer,
            &mut bytes_returned,
        );
        if result.is_err() || buffer.is_null() {
            return PreflightStatus::Unknown;
        }
        let state_ptr = buffer.0 as *const i32;
        let state = std::ptr::read(state_ptr);
        windows::Win32::System::RemoteDesktop::WTSFreeMemory(buffer.0 as *mut _);
        if state == WTSActive.0 {
            PreflightStatus::Pass
        } else {
            PreflightStatus::Fail
        }
    }
}

#[cfg(target_os = "windows")]
pub(super) fn probe_dpi_awareness() -> PreflightStatus {
    use windows::Win32::UI::HiDpi::{
        GetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        AreDpiAwarenessContextsEqual,
    };

    unsafe {
        let ctx = GetThreadDpiAwarenessContext();
        if AreDpiAwarenessContextsEqual(ctx, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2).as_bool() {
            PreflightStatus::Pass
        } else {
            PreflightStatus::Fail
        }
    }
}

#[cfg(target_os = "windows")]
pub(super) fn probe_a11y_service() -> PreflightStatus {
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
    use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation};

    unsafe {
        let res: Result<IUIAutomation, _> = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER);
        match res {
            Ok(_) => PreflightStatus::Pass,
            Err(_) => PreflightStatus::Fail,
        }
    }
}

#[cfg(target_os = "windows")]
pub(super) fn probe_monitor_count() -> (PreflightStatus, u32) {
    use crate::screenshot::enumerate_monitors;
    match enumerate_monitors() {
        Ok(monitors) => {
            let n = monitors.len() as u32;
            if n >= 1 {
                (PreflightStatus::Pass, n)
            } else {
                (PreflightStatus::Fail, 0)
            }
        }
        Err(_) => (PreflightStatus::Fail, 0),
    }
}

#[cfg(target_os = "windows")]
pub(super) fn probe_primary_monitor() -> (PreflightStatus, Option<(u32, u32)>) {
    use crate::screenshot::enumerate_monitors;
    match enumerate_monitors() {
        Ok(monitors) => {
            if let Some(primary) = monitors.iter().find(|m| m.is_primary) {
                (PreflightStatus::Pass, Some((primary.width, primary.height)))
            } else {
                (PreflightStatus::Fail, None)
            }
        }
        Err(_) => (PreflightStatus::Fail, None),
    }
}
