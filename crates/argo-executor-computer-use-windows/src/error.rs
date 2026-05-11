//! Error types for the native Windows computer-use PoC.

use argo_types::ArgoError;
use std::fmt;

/// Top-level error for Windows computer-use primitives.
#[derive(Debug)]
pub enum WindowsCuError {
    /// Host is not Windows or PowerShell is unavailable.
    Unsupported {
        /// Operator-facing reason.
        reason: String,
    },
    /// Required subprocess is not available on PATH.
    MissingBinary {
        /// Binary name.
        binary: &'static str,
    },
    /// A subprocess exited with a non-zero status.
    Subprocess {
        /// Command label.
        command: String,
        /// Captured stderr, truncated for logs.
        stderr: String,
    },
    /// I/O failure while spawning, reading, or writing.
    Io(std::io::Error),
    /// Output did not match the expected protocol.
    Protocol(String),
}

impl fmt::Display for WindowsCuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported { reason } => write!(f, "windows CU unsupported: {reason}"),
            Self::MissingBinary { binary } => {
                write!(f, "windows CU missing required binary `{binary}`")
            }
            Self::Subprocess { command, stderr } => write!(f, "`{command}` failed: {stderr}"),
            Self::Io(e) => write!(f, "windows CU io: {e}"),
            Self::Protocol(msg) => write!(f, "windows CU protocol: {msg}"),
        }
    }
}

impl std::error::Error for WindowsCuError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for WindowsCuError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<WindowsCuError> for ArgoError {
    fn from(e: WindowsCuError) -> Self {
        match e {
            WindowsCuError::Io(e) => ArgoError::Io(e.to_string()),
            other => ArgoError::Provider(other.to_string()),
        }
    }
}

pub(crate) fn truncate_stderr(raw: &[u8]) -> String {
    let s = String::from_utf8_lossy(raw);
    if s.len() <= 512 {
        s.into_owned()
    } else {
        let mut out = s.chars().take(508).collect::<String>();
        out.push_str(" ...");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_long_stderr() {
        let msg = "x".repeat(900);
        let got = truncate_stderr(msg.as_bytes());
        assert!(got.len() <= 512);
        assert!(got.ends_with(" ..."));
    }

    #[test]
    fn converts_to_argo_error() {
        let err = WindowsCuError::MissingBinary {
            binary: "powershell.exe",
        };
        let argo: ArgoError = err.into();
        assert!(argo.to_string().contains("powershell"));
    }

    #[test]
    fn unsupported_message_carries_reason() {
        let err = WindowsCuError::Unsupported {
            reason: "no interactive session".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("unsupported"));
        assert!(msg.contains("no interactive session"));
    }
}
