//! Runtime instrumentation hook for Skill IR node execution.
//!
//! Track L Phase 1 — L5 (`skill-canvas-live-trace`). Surfaces three
//! lifecycle moments per executing IR node — `entered`, `completed`,
//! `failed` — so the canvas live-trace overlay can pulse the current
//! node, mark completed nodes green, and mark failed nodes red.
//!
//! ## Architecture
//!
//! [`TraceEvent`] is the typed event vocabulary. A process-global
//! hook ([`set_trace_hook`]) routes emitted events to whichever
//! subscriber needs them. The gateway WS handler
//! (`argo-cli::handlers::gateway::ws_skill_trace`) lazily installs
//! the hook on its first subscribe and fans events out to
//! per-`(tenant_id, skill_id, run_id)` broadcast channels.
//!
//! No IR runtime executor exists in this crate yet (Track L Phase
//! 2+); this module is the future call-site contract. Any subsystem
//! executing a Skill IR node calls [`emit_node_entered`] /
//! [`emit_node_completed`] / [`emit_node_failed`] with the node's
//! identity and the (tenant, skill, run) coordinates. With no hook
//! installed the calls are silent no-ops, so an IR-runtime caller
//! can wire emits unconditionally without worrying whether a UI is
//! currently subscribed.
//!
//! Example
//! ```
//! use argo_agent_runner::instrumentation::ir_trace::{
//!     emit_node_entered, emit_node_completed, TraceEventKind,
//! };
//! emit_node_entered("tenant-1", "skill-A", "run-7", "node-fetch");
//! emit_node_completed("tenant-1", "skill-A", "run-7", "node-fetch", Some(42));
//! # let _ = TraceEventKind::NodeEntered;
//! ```

use serde_json::{json, Value};
use std::sync::OnceLock;

/// Per-event lifecycle discriminant. The brief locks three values —
/// `node.entered`, `node.completed`, `node.failed` — which surface
/// verbatim in the wire JSON via [`TraceEvent::to_wire_json`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceEventKind {
    /// Worker just began executing this node.
    NodeEntered,
    /// Worker finished this node successfully. `duration_ms` is
    /// optional — runtimes that don't track per-node timing pass
    /// `None` and the field is omitted from the wire frame.
    NodeCompleted {
        /// Optional execution time in milliseconds.
        duration_ms: Option<u64>,
    },
    /// Worker failed inside this node. `error` is a short human-
    /// readable message; the IR engine decides whether to retry,
    /// branch, or propagate.
    NodeFailed {
        /// Failure reason — shown in the canvas overlay tooltip.
        error: String,
    },
}

/// One trace event emitted by the IR runtime as a node moves through
/// its lifecycle. The `(tenant_id, skill_id, run_id)` triple is the
/// fan-out key the gateway WS layer uses to scope events to a single
/// canvas viewer's subscription.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceEvent {
    /// Tenant that owns the run (always set; enforces per-tenant
    /// isolation downstream).
    pub tenant_id: String,
    /// Skill being executed (matches `SkillIrStore`'s skill id).
    pub skill_id: String,
    /// Specific run / invocation — uniquely identifies one execution
    /// of `skill_id` so multiple concurrent runs don't collide on the
    /// canvas overlay.
    pub run_id: String,
    /// Node id from the `SkillGraph` whose lifecycle this event
    /// describes.
    pub node_id: String,
    /// Lifecycle moment + optional payload (timing / error).
    pub kind: TraceEventKind,
}

impl TraceEvent {
    /// Render the inner kind + node id as the wire-format payload the
    /// gateway WS envelope wraps with `seq` / `ts`. The `type` field
    /// matches the brief vocabulary verbatim; `duration_ms` and
    /// `error` appear only on the relevant variants.
    pub fn to_wire_json(&self) -> Value {
        let mut obj = serde_json::Map::new();
        let kind_tag = match &self.kind {
            TraceEventKind::NodeEntered => "node.entered",
            TraceEventKind::NodeCompleted { .. } => "node.completed",
            TraceEventKind::NodeFailed { .. } => "node.failed",
        };
        obj.insert("type".into(), json!(kind_tag));
        obj.insert("node_id".into(), json!(self.node_id));
        match &self.kind {
            TraceEventKind::NodeCompleted {
                duration_ms: Some(ms),
            } => {
                obj.insert("duration_ms".into(), json!(ms));
            }
            TraceEventKind::NodeFailed { error } => {
                obj.insert("error".into(), json!(error));
            }
            _ => {}
        }
        Value::Object(obj)
    }
}

/// Process-global hook signature. The gateway WS handler installs
/// the hook on its first subscribe to fan events out to per-topic
/// broadcast channels.
pub type TraceHook = Box<dyn Fn(TraceEvent) + Send + Sync + 'static>;

static TRACE_HOOK: OnceLock<TraceHook> = OnceLock::new();

/// Register the process-global trace hook. Idempotent — only the
/// first caller wins; subsequent calls are silently ignored. This
/// matches the `argo_channels::webinbox::set_event_hook` pattern so
/// tests re-using process state don't double-register.
pub fn set_trace_hook(hook: TraceHook) {
    let _ = TRACE_HOOK.set(hook);
}

/// Fire a trace event through the installed hook. No-op when no
/// hook is set, so IR-runtime callers can emit unconditionally.
pub fn dispatch_event(event: TraceEvent) {
    if let Some(hook) = TRACE_HOOK.get() {
        hook(event);
    }
}

/// Convenience: emit a `node.entered` event.
pub fn emit_node_entered(tenant_id: &str, skill_id: &str, run_id: &str, node_id: &str) {
    dispatch_event(TraceEvent {
        tenant_id: tenant_id.to_string(),
        skill_id: skill_id.to_string(),
        run_id: run_id.to_string(),
        node_id: node_id.to_string(),
        kind: TraceEventKind::NodeEntered,
    });
}

/// Convenience: emit a `node.completed` event with optional timing.
pub fn emit_node_completed(
    tenant_id: &str,
    skill_id: &str,
    run_id: &str,
    node_id: &str,
    duration_ms: Option<u64>,
) {
    dispatch_event(TraceEvent {
        tenant_id: tenant_id.to_string(),
        skill_id: skill_id.to_string(),
        run_id: run_id.to_string(),
        node_id: node_id.to_string(),
        kind: TraceEventKind::NodeCompleted { duration_ms },
    });
}

/// Convenience: emit a `node.failed` event with a short error string.
pub fn emit_node_failed(
    tenant_id: &str,
    skill_id: &str,
    run_id: &str,
    node_id: &str,
    error: impl Into<String>,
) {
    dispatch_event(TraceEvent {
        tenant_id: tenant_id.to_string(),
        skill_id: skill_id.to_string(),
        run_id: run_id.to_string(),
        node_id: node_id.to_string(),
        kind: TraceEventKind::NodeFailed {
            error: error.into(),
        },
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entered_wire_shape_carries_type_and_node_id() {
        let evt = TraceEvent {
            tenant_id: "t".into(),
            skill_id: "s".into(),
            run_id: "r".into(),
            node_id: "n1".into(),
            kind: TraceEventKind::NodeEntered,
        };
        let v = evt.to_wire_json();
        assert_eq!(v["type"], "node.entered");
        assert_eq!(v["node_id"], "n1");
        assert!(v.get("duration_ms").is_none());
        assert!(v.get("error").is_none());
    }

    #[test]
    fn completed_wire_shape_includes_duration_when_present() {
        let with = TraceEvent {
            tenant_id: "t".into(),
            skill_id: "s".into(),
            run_id: "r".into(),
            node_id: "n2".into(),
            kind: TraceEventKind::NodeCompleted {
                duration_ms: Some(123),
            },
        }
        .to_wire_json();
        assert_eq!(with["type"], "node.completed");
        assert_eq!(with["duration_ms"], 123);

        let without = TraceEvent {
            tenant_id: "t".into(),
            skill_id: "s".into(),
            run_id: "r".into(),
            node_id: "n2".into(),
            kind: TraceEventKind::NodeCompleted { duration_ms: None },
        }
        .to_wire_json();
        assert_eq!(without["type"], "node.completed");
        assert!(without.get("duration_ms").is_none());
    }

    #[test]
    fn failed_wire_shape_includes_error() {
        let evt = TraceEvent {
            tenant_id: "t".into(),
            skill_id: "s".into(),
            run_id: "r".into(),
            node_id: "n3".into(),
            kind: TraceEventKind::NodeFailed {
                error: "tool dispatch refused".into(),
            },
        };
        let v = evt.to_wire_json();
        assert_eq!(v["type"], "node.failed");
        assert_eq!(v["error"], "tool dispatch refused");
    }
}
