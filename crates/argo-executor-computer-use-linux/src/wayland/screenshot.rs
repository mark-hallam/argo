//! Wayland screenshot path: portal first, `grim` fallback.

use crate::error::truncate_stderr;
use crate::probe::{probe, DisplayMode};
use crate::wayland::portal;
use crate::x11::screenshot::Screenshot;
use crate::{LinuxCuError, LinuxCuResult};
use std::path::Path;
use std::process::Command;

/// Capture a Wayland screenshot to `dest`. Prefers
/// xdg-desktop-portal Screenshot, falls back to `grim`.
pub fn capture(dest: &Path) -> LinuxCuResult<Screenshot> {
    let caps = match probe().display_mode {
        DisplayMode::Wayland(c) => c,
        _ => {
            return Err(LinuxCuError::Unsupported {
                reason: "Wayland backend not active for screenshot".into(),
            })
        }
    };
    if caps.portal_reachable {
        match portal::screenshot_to_file(dest) {
            Ok(()) => {
                return Ok(Screenshot {
                    path: dest.to_path_buf(),
                })
            }
            Err(err) => {
                // Fall through to grim if the portal call fails and
                // grim is available. Otherwise propagate the portal
                // error so operators see the real reason.
                if !caps.grim {
                    return Err(err);
                }
                tracing::warn!(
                    error = %err,
                    "portal Screenshot failed; falling back to grim",
                );
            }
        }
    }
    if caps.grim {
        run_grim(dest)?;
        return Ok(Screenshot {
            path: dest.to_path_buf(),
        });
    }
    Err(LinuxCuError::Unsupported {
        reason: "Wayland screenshot requires xdg-desktop-portal or grim".into(),
    })
}

fn run_grim(dest: &Path) -> LinuxCuResult<()> {
    let output = Command::new("grim").args(grim_args(dest)).output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(LinuxCuError::Subprocess {
            command: "grim".into(),
            stderr: truncate_stderr(&output.stderr),
        })
    }
}

/// Argument vector for `grim <dest>`. Pure helper for unit testing.
#[must_use]
pub fn grim_args(dest: &Path) -> Vec<String> {
    vec![dest.display().to_string()]
}
