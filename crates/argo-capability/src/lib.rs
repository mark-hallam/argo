//! `argo-capability` — capability check + RBAC role engine.
//!
//! See `docs/plan/argo-build-plan.md` §10.
//!
//! v0.2 invariant: capability-based security with first-class RBAC. Every tool
//! dispatch checks declared capabilities. Default policies deny.
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

use argo_types::{ArgoError, ArgoResult, Capability, CapabilitySet, Principal, Role};
use parking_lot::RwLock;
use std::collections::HashMap;

/// The capability + RBAC engine.
///
/// Roles are named capability sets with optional single-inheritance.
/// Principals are bound to roles; capability checks resolve a principal's
/// effective set by walking inheritance and union-ing capabilities.
///
/// ```
/// use argo_capability::CapabilityEngine;
/// use argo_types::{Capability, CapabilitySet, Principal, Role};
///
/// let engine = CapabilityEngine::new();
/// engine.add_role(Role {
///     name: "user".into(),
///     inherits_from: None,
///     capabilities: CapabilitySet(vec![Capability::ToolDispatch {
///         tool_name_glob: "web_*".into(),
///     }]),
/// }).unwrap();
/// engine.bind(Principal::Local { os_user: "alice".into() }, "user".into()).unwrap();
///
/// let alice = Principal::Local { os_user: "alice".into() };
/// assert!(engine.check(&alice, &Capability::ToolDispatch {
///     tool_name_glob: "web_search".into()
/// }).is_ok());
/// assert!(engine.check(&alice, &Capability::ToolDispatch {
///     tool_name_glob: "terminal".into()
/// }).is_err());
/// ```
pub struct CapabilityEngine {
    roles: RwLock<HashMap<String, Role>>,
    bindings: RwLock<HashMap<Principal, String>>,
    /// Dynamic per-principal grants (e.g., user-approved one-shot grants).
    dynamic: RwLock<HashMap<Principal, Vec<Capability>>>,
}

impl CapabilityEngine {
    /// Build an empty engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            roles: RwLock::new(HashMap::new()),
            bindings: RwLock::new(HashMap::new()),
            dynamic: RwLock::new(HashMap::new()),
        }
    }

    /// Build an engine pre-populated with the four default role presets
    /// (owner / admin / user / guest) per build plan §10.2.
    #[must_use]
    pub fn with_defaults() -> Self {
        let e = Self::new();
        for r in default_roles() {
            // safe: default_roles produces unique names by construction
            let _ = e.add_role(r);
        }
        e
    }

    /// Define or replace a role.
    pub fn add_role(&self, role: Role) -> ArgoResult<()> {
        if let Some(parent) = &role.inherits_from {
            if !self.roles.read().contains_key(parent) {
                return Err(ArgoError::Schema(format!(
                    "role '{}' inherits from unknown role '{parent}'",
                    role.name
                )));
            }
            if would_create_cycle(&self.roles.read(), &role) {
                return Err(ArgoError::Schema(format!(
                    "adding role '{}' would create an inheritance cycle",
                    role.name
                )));
            }
        }
        self.roles.write().insert(role.name.clone(), role);
        Ok(())
    }

    /// Bind a principal to a role.
    pub fn bind(&self, principal: Principal, role_name: String) -> ArgoResult<()> {
        if !self.roles.read().contains_key(&role_name) {
            return Err(ArgoError::Schema(format!("unknown role: {role_name}")));
        }
        self.bindings.write().insert(principal, role_name);
        Ok(())
    }

    /// Remove a principal's role binding.
    pub fn revoke_binding(&self, principal: &Principal) {
        self.bindings.write().remove(principal);
    }

    /// Resolve a principal's effective capabilities (role + dynamic grants).
    pub fn effective(&self, principal: &Principal) -> CapabilitySet {
        let mut all: Vec<Capability> = Vec::new();
        if let Some(role_name) = self.bindings.read().get(principal).cloned() {
            let roles = self.roles.read();
            walk_role(&roles, &role_name, &mut all, 0);
        }
        if let Some(dyn_caps) = self.dynamic.read().get(principal) {
            all.extend(dyn_caps.iter().cloned());
        }
        CapabilitySet(all)
    }

    /// Check that `principal` has a capability covering `requested`.
    pub fn check(&self, principal: &Principal, requested: &Capability) -> ArgoResult<()> {
        let eff = self.effective(principal);
        if eff.covers(requested) {
            Ok(())
        } else {
            // Distinguish "no role bound" vs "role lacks capability" for better errors.
            if let Some(role_name) = self.bindings.read().get(principal).cloned() {
                Err(ArgoError::RbacDenied {
                    role: role_name,
                    cap: requested.clone(),
                })
            } else {
                Err(ArgoError::CapabilityDenied(requested.clone()))
            }
        }
    }

    /// Grant a dynamic capability to a principal (e.g., from user approval).
    pub fn grant_dynamic(&self, principal: Principal, cap: Capability) {
        self.dynamic.write().entry(principal).or_default().push(cap);
    }

    /// Drop all dynamic grants for a principal.
    pub fn clear_dynamic(&self, principal: &Principal) {
        self.dynamic.write().remove(principal);
    }
}

impl Default for CapabilityEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn walk_role(roles: &HashMap<String, Role>, name: &str, out: &mut Vec<Capability>, depth: u32) {
    if depth > 16 {
        return;
    }
    let Some(role) = roles.get(name) else {
        return;
    };
    out.extend(role.capabilities.0.iter().cloned());
    if let Some(parent) = &role.inherits_from {
        walk_role(roles, parent, out, depth + 1);
    }
}

fn would_create_cycle(roles: &HashMap<String, Role>, new_role: &Role) -> bool {
    let Some(start) = &new_role.inherits_from else {
        return false;
    };
    let mut current = start.clone();
    let mut seen = vec![new_role.name.clone()];
    for _ in 0..32 {
        if seen.iter().any(|s| s == &current) {
            return true;
        }
        seen.push(current.clone());
        let Some(role) = roles.get(&current) else {
            return false;
        };
        let Some(parent) = role.inherits_from.clone() else {
            return false;
        };
        current = parent;
    }
    true
}

/// Default role presets shipped by `argo init` (build plan §10.2).
#[must_use]
pub fn default_roles() -> Vec<Role> {
    use argo_types::SpawnKind;
    vec![
        Role {
            name: "owner".into(),
            inherits_from: None,
            capabilities: CapabilitySet(vec![
                Capability::FsRead {
                    path_glob: "**".into(),
                },
                Capability::FsWrite {
                    path_glob: "**".into(),
                },
                Capability::ToolDispatch {
                    tool_name_glob: "*".into(),
                },
                Capability::Spawn {
                    spawn_kind: SpawnKind::Subagent,
                },
                Capability::Spawn {
                    spawn_kind: SpawnKind::Subprocess,
                },
                Capability::Spawn {
                    spawn_kind: SpawnKind::Container,
                },
                Capability::MemoryWrite,
                Capability::SkillWrite,
                Capability::AuditWrite,
                Capability::SecretRead { name: "*".into() },
                Capability::NetEgress {
                    host_glob: "**".into(),
                    port: None,
                },
            ]),
        },
        Role {
            name: "admin".into(),
            inherits_from: None,
            capabilities: CapabilitySet(vec![
                Capability::FsRead {
                    path_glob: "**".into(),
                },
                Capability::FsWrite {
                    path_glob: "**".into(),
                },
                Capability::ToolDispatch {
                    tool_name_glob: "*".into(),
                },
                Capability::Spawn {
                    spawn_kind: SpawnKind::Subagent,
                },
                Capability::Spawn {
                    spawn_kind: SpawnKind::Subprocess,
                },
                Capability::MemoryWrite,
                Capability::SkillWrite,
                Capability::NetEgress {
                    host_glob: "**".into(),
                    port: None,
                },
            ]),
        },
        Role {
            name: "user".into(),
            inherits_from: None,
            capabilities: CapabilitySet(vec![
                // FsRead/Write to workspace are interpolated by the config layer
                // ({workspace}/**); we ship a permissive **placeholder** here.
                Capability::FsRead {
                    path_glob: "**".into(),
                },
                Capability::FsWrite {
                    path_glob: "**".into(),
                },
                Capability::ToolDispatch {
                    tool_name_glob: "web_*".into(),
                },
                Capability::ToolDispatch {
                    tool_name_glob: "memory".into(),
                },
                Capability::ToolDispatch {
                    tool_name_glob: "skill_view".into(),
                },
                Capability::ToolDispatch {
                    tool_name_glob: "skills_list".into(),
                },
                Capability::ToolDispatch {
                    tool_name_glob: "clarify".into(),
                },
                Capability::MemoryWrite,
            ]),
        },
        Role {
            name: "guest".into(),
            inherits_from: None,
            capabilities: CapabilitySet(vec![
                Capability::ToolDispatch {
                    tool_name_glob: "web_search".into(),
                },
                Capability::ToolDispatch {
                    tool_name_glob: "web_extract".into(),
                },
                Capability::ToolDispatch {
                    tool_name_glob: "clarify".into(),
                },
            ]),
        },
    ]
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
