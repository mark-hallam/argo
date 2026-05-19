//! Audit emission for capability mutations.

use super::{Capability, CapabilityEngine, Principal};
use argo_audit::{AuditKind, AuditSink};
use argo_types::{ArgoError, ArgoResult};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

impl CapabilityEngine {
    /// Attach an audit sink used for role-binding and dynamic-grant
    /// mutation rows.
    ///
    /// The sink's tenant id supplies the tenant scope for emitted rows;
    /// callers are responsible for constructing it with the correct
    /// workspace tenant. Existing constructors leave auditing disabled.
    #[must_use]
    pub fn with_audit_sink(mut self, sink: Arc<AuditSink>) -> Self {
        self.audit_sink = Some(sink);
        self
    }

    /// Bind a principal to a role and stamp the operator or subsystem
    /// that made the change. `None` records `"system"` as the actor.
    pub fn bind_with_actor(
        &self,
        principal: Principal,
        role_name: String,
        actor: Option<&str>,
    ) -> ArgoResult<()> {
        if !self.roles.read().contains_key(&role_name) {
            return Err(ArgoError::Schema(format!("unknown role: {role_name}")));
        }
        self.bindings
            .write()
            .insert(principal.clone(), role_name.clone());
        self.emit_audit(
            AuditKind::RoleBindingChanged,
            &principal,
            serde_json::json!(role_name),
            actor,
            "bind",
        );
        Ok(())
    }

    /// Remove a principal's role binding and stamp the operator or
    /// subsystem that made the change. `None` records `"system"` as
    /// the actor.
    pub fn unbind(&self, principal: &Principal, actor: Option<&str>) {
        if let Some(role_name) = self.bindings.write().remove(principal) {
            self.emit_audit(
                AuditKind::RoleBindingChanged,
                principal,
                serde_json::json!(role_name),
                actor,
                "unbind",
            );
        }
    }

    /// Grant a dynamic capability and stamp the operator or subsystem
    /// that made the change. `None` records `"system"` as the actor.
    pub fn grant_dynamic_with_actor(
        &self,
        principal: Principal,
        cap: Capability,
        actor: Option<&str>,
    ) {
        self.dynamic
            .write()
            .entry(principal.clone())
            .or_default()
            .push(cap.clone());
        self.emit_audit(
            AuditKind::DynamicCapabilityGranted,
            &principal,
            serde_json::json!(cap),
            actor,
            "grant",
        );
    }

    /// Drop all dynamic grants for a principal.
    pub fn clear_dynamic(&self, principal: &Principal) {
        if let Some(caps) = self.dynamic.write().remove(principal) {
            for cap in caps {
                self.emit_audit(
                    AuditKind::DynamicCapabilityGranted,
                    principal,
                    serde_json::json!(cap),
                    None,
                    "revoke",
                );
            }
        }
    }

    /// Revoke one dynamic capability from a principal. Returns
    /// `true` when a matching grant was removed.
    pub fn revoke_dynamic(
        &self,
        principal: &Principal,
        cap: &Capability,
        actor: Option<&str>,
    ) -> bool {
        let removed = {
            let mut dynamic = self.dynamic.write();
            let mut remove_entry = false;
            let removed = if let Some(grants) = dynamic.get_mut(principal) {
                if let Some(pos) = grants.iter().position(|grant| grant == cap) {
                    let removed = grants.remove(pos);
                    remove_entry = grants.is_empty();
                    Some(removed)
                } else {
                    None
                }
            } else {
                None
            };
            if remove_entry {
                dynamic.remove(principal);
            }
            removed
        };
        if let Some(cap) = removed {
            self.emit_audit(
                AuditKind::DynamicCapabilityGranted,
                principal,
                serde_json::json!(cap),
                actor,
                "revoke",
            );
            true
        } else {
            false
        }
    }

    fn emit_audit(
        &self,
        kind: AuditKind,
        principal: &Principal,
        role_or_capability: serde_json::Value,
        actor: Option<&str>,
        action: &'static str,
    ) {
        let Some(sink) = self.audit_sink.clone() else {
            return;
        };
        let payload = serde_json::json!({
            "principal": principal,
            "role_or_capability": role_or_capability,
            "actor": actor.unwrap_or("system"),
            "at": audit_timestamp_ms(),
            "action": action,
        });
        spawn_audit_write(sink, kind, payload);
    }
}

fn audit_timestamp_ms() -> u64 {
    let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return 0;
    };
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn spawn_audit_write(sink: Arc<AuditSink>, kind: AuditKind, payload: serde_json::Value) {
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        tracing::warn!(
            audit_kind = kind.label(),
            "capability audit sink attached but no tokio runtime is available"
        );
        return;
    };
    drop(handle.spawn(async move {
        if let Err(err) = sink.write(None, kind, payload, None).await {
            tracing::warn!(
                error = ?err,
                audit_kind = kind.label(),
                "capability audit write failed"
            );
        }
    }));
}
