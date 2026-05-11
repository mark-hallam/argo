//! Capability-based authorization model.
use serde::{Deserialize, Serialize};

/// A typed permission grant.
///
/// Capabilities are the unit of authorization in Argo. Every tool, skill,
/// plugin, and provider declares a `CapabilitySet`. The core enforces these
/// at call time.
///
/// ```
/// use argo_types::Capability;
/// let cap = Capability::NetEgress { host_glob: "*.example.com".into(), port: Some(443) };
/// assert!(serde_json::to_string(&cap).unwrap().contains("net_egress"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Capability {
    /// Read files matching `path_glob` (globset syntax).
    FsRead {
        /// Glob.
        path_glob: String,
    },
    /// Write files matching `path_glob`.
    FsWrite {
        /// Glob.
        path_glob: String,
    },
    /// Make outbound network connections to `host_glob` on optionally pinned `port`.
    NetEgress {
        /// Host glob.
        host_glob: String,
        /// Optional port.
        #[serde(default)]
        port: Option<u16>,
    },
    /// Read environment variables matching `var_glob`.
    EnvRead {
        /// Var-name glob.
        var_glob: String,
    },
    /// Read a named secret from the configured secret backend.
    SecretRead {
        /// Secret name.
        name: String,
    },
    /// Dispatch a tool whose name matches `tool_name_glob`.
    ToolDispatch {
        /// Tool-name glob.
        tool_name_glob: String,
    },
    /// Spawn a sub-process / sub-agent / container.
    Spawn {
        /// Kind of spawn.
        spawn_kind: SpawnKind,
    },
    /// Write directly to the audit log under one's own identity. Rare.
    AuditWrite,
    /// Mutate `MEMORY.md`/`USER.md`/`SOUL.md`/`AGENTS.md`.
    MemoryWrite,
    /// Write into the tenant-shared team memory namespace
    /// (`argo-memory::shared_team`). Strictly stronger than
    /// [`Capability::MemoryWrite`]: holding `MemoryWrite` does NOT
    /// confer `MemoryWriteTeam`. Reads from the shared-team
    /// namespace remain ungated; this gate exists to keep workers
    /// from polluting team-shared state without an explicit grant.
    MemoryWriteTeam,
    /// Create or modify a skill.
    SkillWrite,
    /// Emit judge verdicts (only the judge subsystem).
    SkillJudge,
    /// Run an Argo Forge optimization (only the Forge subsystem).
    ForgeRun,
}

/// What kind of process is being spawned.
///
/// ```
/// use argo_types::SpawnKind;
/// let json = serde_json::to_string(&SpawnKind::Subprocess).unwrap();
/// assert_eq!(json, "\"subprocess\"");
/// let json = serde_json::to_string(&SpawnKind::BackgroundProcess).unwrap();
/// assert_eq!(json, "\"background_process\"");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpawnKind {
    /// A child Argo agent (`delegate_task`).
    Subagent,
    /// An OS subprocess (`terminal`).
    Subprocess,
    /// A container (Docker/Modal/Daytona/...).
    Container,
    /// A long-lived background OS process tracked in the
    /// `ProcessRegistry` and exposed via the `process` tool.
    /// Strictly stronger than [`SpawnKind::Subprocess`]: holding
    /// `Spawn::Subprocess` does NOT confer
    /// `Spawn::BackgroundProcess` and vice versa (capability
    /// covers compares variants for equality).
    BackgroundProcess,
}

/// A bag of capabilities granted to a principal.
///
/// ```
/// use argo_types::{Capability, CapabilitySet};
/// let mut set = CapabilitySet::empty();
/// set.push(Capability::SkillWrite);
/// assert!(set.covers(&Capability::SkillWrite));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilitySet(pub Vec<Capability>);

impl CapabilitySet {
    /// Empty set (denies everything).
    #[must_use]
    pub fn empty() -> Self {
        Self(Vec::new())
    }

    /// Append a capability.
    pub fn push(&mut self, c: Capability) {
        self.0.push(c);
    }

    /// Does this set cover the requested capability?
    ///
    /// ```
    /// use argo_types::*;
    /// let granted = CapabilitySet(vec![Capability::FsRead {
    ///     path_glob: "/tmp/**".into(),
    /// }]);
    /// let want = Capability::FsRead {
    ///     path_glob: "/tmp/foo/bar".into(),
    /// };
    /// assert!(granted.covers(&want));
    /// ```
    #[must_use]
    pub fn covers(&self, requested: &Capability) -> bool {
        self.0.iter().any(|g| capability_covers(g, requested))
    }
}

fn capability_covers(grant: &Capability, want: &Capability) -> bool {
    use Capability::{
        AuditWrite, EnvRead, ForgeRun, FsRead, FsWrite, MemoryWrite, MemoryWriteTeam, NetEgress,
        SecretRead, SkillJudge, SkillWrite, Spawn, ToolDispatch,
    };
    match (grant, want) {
        (FsRead { path_glob: g }, FsRead { path_glob: w })
        | (FsWrite { path_glob: g }, FsWrite { path_glob: w }) => glob_covers(g, w),
        (
            NetEgress {
                host_glob: gh,
                port: gp,
            },
            NetEgress {
                host_glob: wh,
                port: wp,
            },
        ) => {
            glob_covers(gh, wh)
                && match (gp, wp) {
                    (None, _) => true,
                    (Some(_), None) => false,
                    (Some(g), Some(w)) => g == w,
                }
        }
        (EnvRead { var_glob: g }, EnvRead { var_glob: w }) => glob_covers(g, w),
        (SecretRead { name: g }, SecretRead { name: w }) => g == w || g == "*",
        (ToolDispatch { tool_name_glob: g }, ToolDispatch { tool_name_glob: w }) => {
            glob_covers(g, w)
        }
        (Spawn { spawn_kind: g }, Spawn { spawn_kind: w }) => g == w,
        (AuditWrite, AuditWrite) => true,
        (MemoryWrite, MemoryWrite) => true,
        (MemoryWriteTeam, MemoryWriteTeam) => true,
        (SkillWrite, SkillWrite) => true,
        (SkillJudge, SkillJudge) => true,
        (ForgeRun, ForgeRun) => true,
        _ => false,
    }
}

fn glob_covers(grant_glob: &str, requested: &str) -> bool {
    if grant_glob == requested {
        return true;
    }
    let Ok(glob) = globset::Glob::new(grant_glob) else {
        return false;
    };
    glob.compile_matcher().is_match(requested)
}

/// A named role: a `CapabilitySet` plus optional single-inheritance.
///
/// ```
/// use argo_types::{Capability, CapabilitySet, Role};
/// let role = Role {
///     name: "operator".into(), capabilities: CapabilitySet(vec![Capability::AuditWrite]),
///     inherits_from: Some("viewer".into()),
/// };
/// assert_eq!(role.inherits_from.as_deref(), Some("viewer"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Role name (e.g. `"owner"`).
    pub name: String,
    /// Capabilities granted by the role.
    pub capabilities: CapabilitySet,
    /// Single-inheritance parent role name.
    #[serde(default)]
    pub inherits_from: Option<String>,
}

/// Who or what is acting.
///
/// ```
/// use argo_types::Principal;
/// let principal = Principal::Local { os_user: "mark".into() };
/// assert!(serde_json::to_string(&principal).unwrap().contains("local"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Principal {
    /// A local CLI/TUI user identified by OS user.
    Local {
        /// OS user name.
        os_user: String,
    },
    /// A messaging-platform identity.
    Gateway {
        /// Adapter name (e.g. `"slack"`).
        adapter: String,
        /// Platform-specific identity (e.g. Slack user ID).
        identity: String,
    },
    /// An ACP harness (VS Code, Zed, JetBrains, …).
    Acp {
        /// Harness name.
        harness: String,
        /// Harness identity.
        identity: String,
    },
    /// The Forge subsystem itself (special principal).
    Forge,
}

/// A binding of a principal to a role.
///
/// ```
/// use argo_types::{Principal, RoleBinding};
/// let binding = RoleBinding {
///     principal: Principal::Forge,
///     role_name: "builder".into(),
/// };
/// assert_eq!(binding.role_name, "builder");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleBinding {
    /// Bound principal.
    pub principal: Principal,
    /// Role name (resolved via the role table).
    pub role_name: String,
}
