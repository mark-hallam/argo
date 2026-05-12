//! Screenshot capture via `scrot`.

use crate::error::truncate_stderr;
use crate::probe::ensure_supported;
use crate::{LinuxCuError, LinuxCuResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Result of one screenshot capture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Screenshot {
    /// Destination PNG path.
    pub path: PathBuf,
}

/// Capture an X11 screenshot to `dest` via `scrot --silent`.
pub fn capture(dest: &Path) -> LinuxCuResult<Screenshot> {
    ensure_supported()?;
    run_scrot(dest)?;
    Ok(Screenshot {
        path: dest.to_path_buf(),
    })
}

fn run_scrot(dest: &Path) -> LinuxCuResult<()> {
    let output = Command::new("scrot").args(scrot_args(dest)).output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(LinuxCuError::Subprocess {
            command: "scrot".into(),
            stderr: truncate_stderr(&output.stderr),
        })
    }
}

pub(crate) fn scrot_args(dest: &Path) -> Vec<String> {
    vec!["--silent".into(), dest.display().to_string()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_include_silent_and_destination() {
        let args = scrot_args(Path::new("/tmp/out.png"));
        assert_eq!(args, vec!["--silent", "/tmp/out.png"]);
    }

    #[test]
    fn capture_is_unsupported_off_linux() {
        if !cfg!(target_os = "linux") {
            let err = capture(Path::new("/tmp/argo-linux-cu.png")).unwrap_err();
            assert!(matches!(err, LinuxCuError::Unsupported { .. }));
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn capture_uses_scrot_from_path() {
        let _guard = crate::test_support::ENV_LOCK.lock().unwrap();
        let dir = crate::test_support::temp_dir("scrot");
        crate::test_support::write_executable(&dir, "xdotool", "#!/bin/sh\nexit 0\n");
        crate::test_support::write_executable(&dir, "scrot", "#!/bin/sh\ntouch \"$2\"\nexit 0\n");
        let old_path = std::env::var_os("PATH");
        let old_display = std::env::var_os("DISPLAY");
        std::env::set_var("PATH", &dir);
        std::env::set_var("DISPLAY", ":99");
        let dest = dir.join("shot.png");
        let shot = capture(&dest).unwrap();
        if let Some(path) = old_path {
            std::env::set_var("PATH", path);
        }
        if let Some(display) = old_display {
            std::env::set_var("DISPLAY", display);
        } else {
            std::env::remove_var("DISPLAY");
        }
        assert_eq!(shot.path, dest);
        assert!(shot.path.exists());
    }
}
