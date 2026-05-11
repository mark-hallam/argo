//! Screenshot capture via inline PowerShell + System.Drawing.

use crate::error::truncate_stderr;
use crate::probe::ensure_supported;
use crate::{WindowsCuError, WindowsCuResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Result of one screenshot capture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Screenshot {
    /// Destination PNG path.
    pub path: PathBuf,
}

/// Capture the primary screen to `dest` as PNG via PowerShell built-ins.
pub fn capture(dest: &Path) -> WindowsCuResult<Screenshot> {
    ensure_supported()?;
    run_powershell(&powershell_args(dest))?;
    Ok(Screenshot {
        path: dest.to_path_buf(),
    })
}

fn run_powershell(args: &[String]) -> WindowsCuResult<()> {
    let output = Command::new("powershell.exe").args(args).output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(WindowsCuError::Subprocess {
            command: "powershell.exe screenshot".into(),
            stderr: truncate_stderr(&output.stderr),
        })
    }
}

pub(crate) fn powershell_args(dest: &Path) -> Vec<String> {
    vec![
        "-NoProfile".into(),
        "-NonInteractive".into(),
        "-Command".into(),
        screenshot_script(dest),
    ]
}

pub(crate) fn screenshot_script(dest: &Path) -> String {
    let escaped = ps_quote(&dest.display().to_string());
    format!(
        "Add-Type -AssemblyName System.Drawing; \
Add-Type -AssemblyName System.Windows.Forms; \
$bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds; \
$bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height; \
$gfx = [System.Drawing.Graphics]::FromImage($bmp); \
$gfx.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size); \
$bmp.Save({escaped}, [System.Drawing.Imaging.ImageFormat]::Png); \
$gfx.Dispose(); $bmp.Dispose();"
    )
}

/// Single-quote a string for safe inclusion in a PowerShell `-Command`
/// expression. PowerShell single-quoted strings escape an embedded
/// single quote by doubling it.
pub(crate) fn ps_quote(raw: &str) -> String {
    let escaped = raw.replace('\'', "''");
    format!("'{escaped}'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_use_no_profile_and_non_interactive() {
        let args = powershell_args(Path::new("C:/temp/out.png"));
        assert_eq!(args[0], "-NoProfile");
        assert_eq!(args[1], "-NonInteractive");
        assert_eq!(args[2], "-Command");
        assert!(args[3].contains("System.Drawing"));
        assert!(args[3].contains("CopyFromScreen"));
    }

    #[test]
    fn script_quotes_destination_path() {
        let s = screenshot_script(Path::new("C:/Users/admin/out.png"));
        assert!(s.contains("'C:/Users/admin/out.png'"));
    }

    #[test]
    fn ps_quote_escapes_embedded_single_quote() {
        assert_eq!(ps_quote("o'brien"), "'o''brien'");
    }

    #[test]
    fn script_does_not_use_execution_policy_bypass() {
        let s = screenshot_script(Path::new("C:/x.png"));
        assert!(!s.to_lowercase().contains("set-executionpolicy"));
        assert!(!s.to_lowercase().contains("bypass"));
    }

    #[test]
    fn capture_is_unsupported_off_windows() {
        if !cfg!(target_os = "windows") {
            let err = capture(Path::new("C:/temp/argo-windows-cu.png")).unwrap_err();
            assert!(matches!(err, WindowsCuError::Unsupported { .. }));
        }
    }
}
