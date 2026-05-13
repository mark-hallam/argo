//! Tenant/worker/session log-context for structured logging
//! (Stream `mission-structured-logging-tenant-worker-session`).
//!
//! Three primitives:
//!
//! 1. [`LogContext`] — the three-tuple every log line should carry
//!    when emitted under a request/work context.
//! 2. [`enter_context`] — enter a `LogContext`-bearing tracing span.
//!    Returns a `tracing::span::EnteredSpan` guard whose `drop` exits
//!    the span. The three call-sites this brief installs
//!    ([`crate`] gateway boot, worker action handler, session start)
//!    are the *only* sites that should call this directly. All other
//!    code relies on the layer-injection working automatically.
//! 3. [`ContextInjectorLayer`] — a `tracing-subscriber` layer that, on
//!    every event emitted within a `LogContext`-bearing span, walks
//!    the current span and its ancestors to recover the three fields
//!    and writes a structured log line containing them — regardless
//!    of whether the event-author explicitly named them.
//!
//! Author-wins semantics: when an event-author explicitly names
//! `tenant_id` / `worker_id` / `session_id`, that value wins; the
//! Layer fills in only when absent. This preserves more-specific
//! tenant_id overrides at the call-site.
//!
//! The Layer is opt-in via `[telemetry.context]`. When disabled the
//! Layer is not constructed and there is zero overhead.
//!
//! ```
//! use argo_telemetry::span_context::{enter_context, LogContext};
//! let _g = enter_context(LogContext {
//!     tenant_id: Some("tenant-x".into()),
//!     worker_id: Some("wkr-1".into()),
//!     session_id: Some("sess-42".into()),
//! });
//! tracing::info!("anything emitted here carries the three-tuple");
//! ```

use parking_lot::Mutex;
use std::io::{self, Write as IoWrite};

use tracing::field::{Field, Visit};
use tracing::span::{Attributes, EnteredSpan, Id};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

/// The three-tuple every log line should carry when emitted under a
/// request/work context.
///
/// All fields are `Option<String>` rather than typed newtypes
/// ([`argo_types::TenantId`] etc.) so this crate does not have to
/// take a dep on every id-owning crate (the `WorkerId` type lives in
/// `argo-identity`, the `Session` lives in `argo-orchestrator`).
/// Call-sites convert their typed ids via `.to_string()`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogContext {
    /// Tenant the work is being done for. `None` for boot-time / anon
    /// requests; the [`ContextInjectorLayer`] emits the operator-
    /// configured anon marker rather than omitting the field.
    pub tenant_id: Option<String>,
    /// Worker carrying the work. `None` outside a worker context
    /// (e.g. inside a gateway-only request).
    pub worker_id: Option<String>,
    /// Session this work is grouped under. `None` outside a session.
    pub session_id: Option<String>,
}

/// Field-name constants — kept here so both the entry function and
/// the layer's visitor agree on spelling.
const FIELD_TENANT: &str = "tenant_id";
const FIELD_WORKER: &str = "worker_id";
const FIELD_SESSION: &str = "session_id";

/// Enter a [`LogContext`]-bearing tracing span. The returned
/// `EnteredSpan` exits the span when dropped — bind it to a local
/// variable for the scope of the work, e.g.
/// `let _ctx = enter_context(ctx);`.
///
/// The span emitted here carries the three fields as span attributes,
/// so any `tracing` subscriber that walks span ancestors
/// automatically (the default [`tracing_subscriber::fmt::layer`])
/// will surface them. The [`ContextInjectorLayer`] in this module
/// additionally writes its own structured line per event so
/// non-fmt subscribers (e.g. the workspace's
/// [`SecretsScrubberLayer`](../../../argo-cli/src/secrets_scrubber.rs))
/// don't drop the three-tuple.
#[must_use = "drop the returned guard at end of context scope"]
pub fn enter_context(ctx: LogContext) -> EnteredSpan {
    let tenant = ctx.tenant_id.unwrap_or_default();
    let worker = ctx.worker_id.unwrap_or_default();
    let session = ctx.session_id.unwrap_or_default();
    tracing::info_span!(
        "log_context",
        tenant_id = tenant,
        worker_id = worker,
        session_id = session,
    )
    .entered()
}

/// `tracing-subscriber` layer that injects `tenant_id` / `worker_id`
/// / `session_id` from the current span (or any ancestor) into every
/// emitted event, regardless of whether the event-author explicitly
/// named them.
///
/// On `on_new_span` the layer extracts whatever the new span carries
/// for the three fields, inherits anything missing from its parent's
/// cached [`LogContext`], and stores the merged context in the new
/// span's extensions — so per-event lookups are O(1) after the first
/// event under that span.
///
/// On `on_event` the layer walks to the nearest ancestor with a
/// cached [`LogContext`], formats a structured line carrying the
/// three fields plus the event's own fields, and writes it to the
/// layer's writer. Event-author values for any of the three fields
/// WIN over span-ancestor values (per brief recommendation, line 98).
///
/// The layer's writer defaults to `stderr`; tests can construct one
/// with a buffer via [`Self::with_writer`].
pub struct ContextInjectorLayer {
    emit_anon_tenant_as: String,
    writer: Mutex<Box<dyn IoWrite + Send>>,
}

impl ContextInjectorLayer {
    /// Build a layer writing to `stderr`. `emit_anon_tenant_as` is
    /// the string emitted in place of an absent `tenant_id` (typical
    /// anon GETs to `/health`). Worker / session absences render as
    /// the literal `-` so observability tools can differentiate
    /// "field absent" from "intentional anon".
    #[must_use]
    pub fn new(emit_anon_tenant_as: impl Into<String>) -> Self {
        Self::with_writer(emit_anon_tenant_as, Box::new(io::stderr()))
    }

    /// Build a layer with a caller-provided writer (tests).
    #[must_use]
    pub fn with_writer(
        emit_anon_tenant_as: impl Into<String>,
        writer: Box<dyn IoWrite + Send>,
    ) -> Self {
        Self {
            emit_anon_tenant_as: emit_anon_tenant_as.into(),
            writer: Mutex::new(writer),
        }
    }

    fn render_anon_tenant(&self) -> &str {
        &self.emit_anon_tenant_as
    }
}

/// Visitor that grabs `tenant_id` / `worker_id` / `session_id` from a
/// span's attributes or an event's fields. Other field values are
/// captured into `other_fields` so the event-formatter half can echo
/// them.
struct ContextVisitor {
    tenant: Option<String>,
    worker: Option<String>,
    session: Option<String>,
    message: String,
    other_fields: Vec<(String, String)>,
}

impl ContextVisitor {
    fn new() -> Self {
        Self {
            tenant: None,
            worker: None,
            session: None,
            message: String::new(),
            other_fields: Vec::new(),
        }
    }

    fn store(&mut self, name: &str, value: String) {
        match name {
            FIELD_TENANT => {
                if !value.is_empty() {
                    self.tenant = Some(value);
                }
            }
            FIELD_WORKER => {
                if !value.is_empty() {
                    self.worker = Some(value);
                }
            }
            FIELD_SESSION => {
                if !value.is_empty() {
                    self.session = Some(value);
                }
            }
            "message" => self.message = value,
            _ => self.other_fields.push((name.to_string(), value)),
        }
    }
}

impl Visit for ContextVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.store(field.name(), value.to_string());
    }
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.store(field.name(), format!("{value:?}"));
    }
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.store(field.name(), value.to_string());
    }
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.store(field.name(), value.to_string());
    }
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.store(field.name(), value.to_string());
    }
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.store(field.name(), value.to_string());
    }
}

impl<S> Layer<S> for ContextInjectorLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else {
            return;
        };
        let mut visitor = ContextVisitor::new();
        attrs.record(&mut visitor);
        // Inherit anything missing from the parent's cached context.
        let parent_ctx = span
            .parent()
            .and_then(|p| p.extensions().get::<LogContext>().cloned());
        let merged = LogContext {
            tenant_id: visitor
                .tenant
                .or_else(|| parent_ctx.as_ref().and_then(|c| c.tenant_id.clone())),
            worker_id: visitor
                .worker
                .or_else(|| parent_ctx.as_ref().and_then(|c| c.worker_id.clone())),
            session_id: visitor
                .session
                .or_else(|| parent_ctx.as_ref().and_then(|c| c.session_id.clone())),
        };
        span.extensions_mut().insert(merged);
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut visitor = ContextVisitor::new();
        event.record(&mut visitor);

        // Walk to nearest ancestor with a cached LogContext for any
        // of the three fields the event-author did not name.
        let mut tenant = visitor.tenant.clone();
        let mut worker = visitor.worker.clone();
        let mut session = visitor.session.clone();
        if tenant.is_none() || worker.is_none() || session.is_none() {
            if let Some(span) = ctx.event_span(event) {
                for ancestor in span.scope() {
                    if let Some(c) = ancestor.extensions().get::<LogContext>() {
                        if tenant.is_none() {
                            tenant = c.tenant_id.clone();
                        }
                        if worker.is_none() {
                            worker = c.worker_id.clone();
                        }
                        if session.is_none() {
                            session = c.session_id.clone();
                        }
                        if tenant.is_some() && worker.is_some() && session.is_some() {
                            break;
                        }
                    }
                }
            }
        }

        let tenant_out = tenant.unwrap_or_else(|| self.render_anon_tenant().to_string());
        let worker_out = worker.unwrap_or_else(|| "-".to_string());
        let session_out = session.unwrap_or_else(|| "-".to_string());

        let metadata = event.metadata();
        let mut line = String::with_capacity(160);
        let _ = std::fmt::Write::write_fmt(
            &mut line,
            format_args!(
                "ctx tenant_id={tenant_out} worker_id={worker_out} session_id={session_out} \
                 {level} {target}",
                level = metadata.level(),
                target = metadata.target(),
            ),
        );
        if !visitor.message.is_empty() {
            let _ = std::fmt::Write::write_fmt(
                &mut line,
                format_args!(" {}", visitor.message),
            );
        }
        for (k, v) in &visitor.other_fields {
            let _ = std::fmt::Write::write_fmt(&mut line, format_args!(" {k}={v}"));
        }
        line.push('\n');

        let mut w = self.writer.lock();
        let _ = w.write_all(line.as_bytes());
    }
}

#[cfg(test)]
#[path = "span_context_tests.rs"]
mod tests;
