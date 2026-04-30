//! ACP "Permissions" → Argo capability mapping (§28.Y point 5).
//!
//! ACP harnesses ask the server "is the user allowed to do <X>?"
//! where `<X>` is an ACP-defined permission string. This module
//! translates those into Argo [`Capability`] values so the same
//! CLI/TUI/gateway approval flow handles them — and an unknown
//! permission resolves to a hard deny rather than a silent allow.
use argo_types::Capability;
use serde::{Deserialize, Serialize};

/// ACP permission category. The stringly-typed wire form
/// (`"file/read"`, `"net/egress"`, …) is parsed once at the
/// boundary into this enum, so the rest of the engine works with
/// typed values.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcpPermission {
    /// Read a file at `path` (workspace-relative).
    FileRead {
        /// Path glob.
        path: String,
    },
    /// Write a file at `path`.
    FileWrite {
        /// Path glob.
        path: String,
    },
    /// Network egress to `host`.
    NetEgress {
        /// Host glob.
        host: String,
    },
    /// Run a shell command.
    Terminal,
    /// Spawn a sub-agent.
    Subagent,
}

/// Translate an [`AcpPermission`] into the corresponding Argo
/// [`Capability`].
///
/// One-to-one for path/host-scoped permissions; `Terminal` and
/// `Subagent` map to [`argo_types::Capability::Spawn`] with the
/// appropriate spawn-kind. Unknown permissions are rejected at
/// parse time (the wire form fails to deserialize) so this
/// function is total over the ACP enum.
#[must_use]
pub fn map_permission(p: &AcpPermission) -> Capability {
    match p {
        AcpPermission::FileRead { path } => Capability::FsRead {
            path_glob: path.clone(),
        },
        AcpPermission::FileWrite { path } => Capability::FsWrite {
            path_glob: path.clone(),
        },
        AcpPermission::NetEgress { host } => Capability::NetEgress {
            host_glob: host.clone(),
            port: None,
        },
        AcpPermission::Terminal => Capability::Spawn {
            spawn_kind: argo_types::SpawnKind::Subprocess,
        },
        AcpPermission::Subagent => Capability::Spawn {
            spawn_kind: argo_types::SpawnKind::Subagent,
        },
    }
}
