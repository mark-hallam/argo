//! xdg-desktop-portal D-Bus client.
//!
//! Two portal interfaces are exercised in v1:
//! - `org.freedesktop.portal.Screenshot` — cross-compositor screen
//!   capture. Returns a `file://` URI that we copy bytes from.
//! - `org.freedesktop.portal.RemoteDesktop` — pointer/keyboard
//!   emulation. Multi-step session protocol; v1 implements
//!   reachability detection only and surfaces a clear
//!   `Unsupported` error for `click()`. Real RemoteDesktop wiring
//!   ships in the `mission-enterprise-wayland-libei` hardening
//!   brief — libei is the medium-term replacement.
//!
//! All zbus code lives behind `#[cfg(target_os = "linux")]`. On other
//! OSes the module compiles to stubs that return `false` /
//! `Unsupported`, so the crate cross-compiles cleanly.

use crate::{LinuxCuError, LinuxCuResult};
use std::path::Path;

/// Portal bus name + path + Screenshot interface name.
pub const PORTAL_BUS_NAME: &str = "org.freedesktop.portal.Desktop";
pub const PORTAL_OBJECT_PATH: &str = "/org/freedesktop/portal/desktop";
pub const SCREENSHOT_INTERFACE: &str = "org.freedesktop.portal.Screenshot";
pub const REMOTE_DESKTOP_INTERFACE: &str = "org.freedesktop.portal.RemoteDesktop";

/// Convert a `file://` URI from the portal Screenshot response into a
/// local filesystem path. Returns `None` for any non-`file://` scheme
/// or a malformed URI without a path component.
///
/// Pure function — pulled out so unit tests can verify URI parsing
/// without a live D-Bus connection.
#[must_use]
pub fn uri_to_path(uri: &str) -> Option<String> {
    let rest = uri.strip_prefix("file://")?;
    // After `file://` the path's leading `/` is the first slash in
    // `rest` (whether or not an authority like `localhost` precedes
    // it). Empty rest, or no slash at all → malformed.
    let slash = rest.find('/')?;
    let path = &rest[slash..];
    if path.is_empty() {
        return None;
    }
    Some(percent_decode(path))
}

/// Decode a small subset of percent-encoded characters that the
/// portal commonly emits (`%20` for space, `%2F` for slash).
/// Best-effort; unknown escapes pass through verbatim.
fn percent_decode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = &input[i + 1..i + 3];
            if let Ok(code) = u8::from_str_radix(hex, 16) {
                if code.is_ascii() {
                    out.push(code as char);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

// --- Linux: real zbus implementation ---------------------------------

#[cfg(target_os = "linux")]
pub fn is_reachable() -> bool {
    linux_impl::is_reachable()
}

#[cfg(target_os = "linux")]
pub fn screenshot_to_file(dest: &Path) -> LinuxCuResult<()> {
    linux_impl::screenshot_to_file(dest)
}

/// Mouse click via portal RemoteDesktop.
///
/// v1 fail-closed: requires a full RemoteDesktop session
/// (CreateSession + SelectDevices + Start + NotifyPointerMotion +
/// NotifyPointerButton) that is not yet wired in this crate.
/// Returns `Unsupported` with operator-facing guidance. The
/// `mission-enterprise-wayland-libei` hardening brief tracks the
/// real implementation.
#[cfg(target_os = "linux")]
pub fn click_via_portal(_x: i32, _y: i32, compositor: &str) -> LinuxCuResult<()> {
    Err(LinuxCuError::Unsupported {
        reason: super::probe::mouse_unsupported_hint(compositor),
    })
}

// --- Non-Linux: cross-compile stubs ----------------------------------

#[cfg(not(target_os = "linux"))]
#[must_use]
pub fn is_reachable() -> bool {
    false
}

#[cfg(not(target_os = "linux"))]
pub fn screenshot_to_file(_dest: &Path) -> LinuxCuResult<()> {
    Err(LinuxCuError::Unsupported {
        reason: "xdg-desktop-portal requires target_os=linux".into(),
    })
}

#[cfg(not(target_os = "linux"))]
pub fn click_via_portal(_x: i32, _y: i32, _compositor: &str) -> LinuxCuResult<()> {
    Err(LinuxCuError::Unsupported {
        reason: "xdg-desktop-portal requires target_os=linux".into(),
    })
}

#[cfg(target_os = "linux")]
mod linux_impl {
    use super::{
        PORTAL_BUS_NAME, PORTAL_OBJECT_PATH, SCREENSHOT_INTERFACE,
    };
    use crate::wayland::portal::uri_to_path;
    use crate::{LinuxCuError, LinuxCuResult};
    use std::collections::HashMap;
    use std::path::Path;
    use std::time::Duration;
    use zbus::blocking::{Connection, Proxy};
    use zbus::names::BusName;
    use zbus::zvariant::Value;

    /// Probe whether `org.freedesktop.portal.Desktop` is reachable
    /// on the session bus. Short, bounded D-Bus call.
    pub fn is_reachable() -> bool {
        let Ok(conn) = Connection::session() else {
            return false;
        };
        let Ok(bus_name) = BusName::try_from(PORTAL_BUS_NAME) else {
            return false;
        };
        let Ok(dbus) = Proxy::new(
            &conn,
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
        ) else {
            return false;
        };
        dbus.call::<_, _, bool>("NameHasOwner", &(bus_name)).unwrap_or(false)
    }

    /// Capture a screenshot via portal Screenshot and copy the bytes
    /// to `dest`. Blocks on the Response signal up to ~10s.
    pub fn screenshot_to_file(dest: &Path) -> LinuxCuResult<()> {
        let conn = Connection::session().map_err(zbus_to_err)?;
        let proxy = Proxy::new(
            &conn,
            PORTAL_BUS_NAME,
            PORTAL_OBJECT_PATH,
            SCREENSHOT_INTERFACE,
        )
        .map_err(zbus_to_err)?;
        let options: HashMap<&str, Value> = HashMap::from([
            ("interactive", Value::from(false)),
            ("modal", Value::from(false)),
        ]);
        // Call returns a request object path; the result arrives on
        // the Response signal of that path. v1 polls the standard
        // "Response" signal stream with a 10s timeout.
        let _request: zbus::zvariant::OwnedObjectPath = proxy
            .call("Screenshot", &("", options))
            .map_err(zbus_to_err)?;
        let response = await_screenshot_response(&conn, Duration::from_secs(10))?;
        let path = uri_to_path(&response).ok_or_else(|| {
            LinuxCuError::Protocol(format!(
                "portal Screenshot returned non-file URI `{response}`"
            ))
        })?;
        std::fs::copy(&path, dest)?;
        Ok(())
    }

    /// Wait for a portal `Response` signal carrying a `uri` key.
    /// Best-effort: subscribes to the Request interface signals and
    /// returns the first Response with a successful code (0).
    fn await_screenshot_response(
        conn: &Connection,
        timeout: Duration,
    ) -> LinuxCuResult<String> {
        let proxy = Proxy::new(
            conn,
            PORTAL_BUS_NAME,
            PORTAL_OBJECT_PATH,
            "org.freedesktop.portal.Request",
        )
        .map_err(zbus_to_err)?;
        let iter = proxy
            .receive_signal_with_args("Response", &[])
            .map_err(zbus_to_err)?;
        let start = std::time::Instant::now();
        for msg in iter {
            if start.elapsed() > timeout {
                break;
            }
            if let Ok((code, results)) =
                msg.body().deserialize::<(u32, HashMap<String, Value>)>()
            {
                if code != 0 {
                    return Err(LinuxCuError::Protocol(format!(
                        "portal Screenshot response code {code} (1=cancelled, 2=error)"
                    )));
                }
                if let Some(uri_val) = results.get("uri") {
                    if let Ok(uri) = String::try_from(uri_val.try_clone().map_err(|e| {
                        LinuxCuError::Protocol(format!("uri clone: {e}"))
                    })?) {
                        return Ok(uri);
                    }
                }
            }
        }
        Err(LinuxCuError::Protocol(
            "portal Screenshot timed out waiting for Response signal".into(),
        ))
    }

    fn zbus_to_err(e: zbus::Error) -> LinuxCuError {
        LinuxCuError::Protocol(format!("zbus: {e}"))
    }
}
