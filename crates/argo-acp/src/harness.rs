//! Harness types — what kind of ACP client is connected, and what
//! capabilities we advertise to each.
//!
//! Multi-harness from day 1 (§28.Y point 2): Copilot CLI, Cursor
//! CLI, generic ACP. Configuration drives which harnesses to
//! advertise capabilities for; nothing is hardcoded to a single
//! harness.
use serde::{Deserialize, Serialize};

/// Connected harness kind. Drives which methods we advertise + how
/// we map ACP permission requests onto Argo capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Harness {
    /// GitHub Copilot CLI.
    CopilotCli,
    /// Cursor agent CLI.
    CursorCli,
    /// VS Code via the generic ACP extension.
    VsCode,
    /// Zed editor (built-in ACP support).
    Zed,
    /// JetBrains IDE family (IntelliJ, GoLand, …).
    JetBrains,
    /// Generic ACP client (catch-all).
    Generic,
}

impl Harness {
    /// Stable string identifier (used in [`argo_types::Principal::Acp::harness`]).
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::CopilotCli => "copilot_cli",
            Self::CursorCli => "cursor_cli",
            Self::VsCode => "vscode",
            Self::Zed => "zed",
            Self::JetBrains => "jetbrains",
            Self::Generic => "generic",
        }
    }
}

/// Capabilities advertised to a harness in the initial handshake.
///
/// §28.Y point 4: "Capabilities exposed: send message, receive
/// stream, list sessions, fork session." Each harness gets a
/// possibly-different subset based on what its UI can drive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HarnessCapabilities {
    /// `session/send_message` is supported.
    pub send_message: bool,
    /// Streaming notifications (`session/stream_chunk`) are supported.
    pub stream: bool,
    /// `session/list` is supported.
    pub list_sessions: bool,
    /// `session/fork` is supported.
    pub fork_session: bool,
}

impl HarnessCapabilities {
    /// Default capabilities advertised to `harness`. Every harness
    /// in v0.2 ships with all four; configuration can opt out per
    /// deployment.
    #[must_use]
    pub fn default_for(_harness: Harness) -> Self {
        Self {
            send_message: true,
            stream: true,
            list_sessions: true,
            fork_session: true,
        }
    }

    /// Empty capability set (deny-by-default for tests).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            send_message: false,
            stream: false,
            list_sessions: false,
            fork_session: false,
        }
    }
}
