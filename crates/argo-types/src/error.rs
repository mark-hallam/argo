//! Argo-wide error and result types.
//!
//! # Canonical variant contract
//!
//! [`ArgoError`] variants encode *what kind of failure* happened, not just
//! *where* the failure originated. The gateway layer maps each variant to a
//! stable HTTP status; downstream services and operators rely on that
//! mapping to triage. Pick the variant that names the **failure mode**, not
//! the call site.
//!
//! See `docs/skycot/ERROR_VARIANT_AUDIT.md` for the live audit of
//! mismatched call sites and the recommended replacement variant.
//!
//! ## Quick reference
//!
//! | Variant                  | When to use                                                       | HTTP |
//! |--------------------------|-------------------------------------------------------------------|------|
//! | [`ArgoError::Provider`]  | Downstream service failure (HTTP/MCP/model API/action endpoint).  | 502  |
//! | [`ArgoError::Tool`]      | Structured tool-execution failure (carries tool name + reason).   | 502  |
//! | [`ArgoError::Io`]        | Local file-system or process I/O failure.                         | 500  |
//! | [`ArgoError::Schema`]    | Caller-supplied data malformed / missing field / bad manifest.    | 400  |
//! | [`ArgoError::Invariant`] | Internal invariant violated; this is a bug. Should be rare.       | 500  |
//! | [`ArgoError::Interrupted`] | Operation cancelled (SIGINT / explicit cancel).                 | 499  |
//! | [`ArgoError::UserProtected`] | Caller tried to mutate a user-protected entity.               | 403  |
//! | [`ArgoError::JudgeRejected`] | Judge layer rejected the candidate.                           | 422  |
//! | [`ArgoError::RoutingFailed`] | No viable routing target.                                     | 503  |
//! | [`ArgoError::RbacDenied`] | RBAC role lacks the required capability.                         | 403  |
//! | [`ArgoError::CapabilityDenied`] | Capability gate denied access.                             | 403  |
//! | [`ArgoError::BudgetExhausted`] | Budget exhausted (iterations / tokens / latency).           | 429  |
//! | [`ArgoError::Oci`]       | OCI registry / packaging error.                                   | 502  |
//! | [`ArgoError::NotFound`]  | Addressed entity does not exist.                                  | 404  |
//! | [`ArgoError::StreamingRetryExhausted`] | Streaming retry exhausted with partial text.      | 502  |
//! | [`ArgoError::ContextWindowExhausted`] | Adaptive truncation cannot fit even the minimum.     | 500  |
//!
//! ## Common mismatches (audit fodder)
//!
//! - `ArgoError::Provider(format!("read {path}: {e}"))` — local FS read; use
//!   [`ArgoError::Io`] (or rely on the `From<std::io::Error>` impl).
//! - `ArgoError::Provider(format!("audit insert: {e}"))` — sqlx persistence
//!   error; this is **not** a downstream service. Today there is no dedicated
//!   storage variant; see `ERROR_VARIANT_AUDIT.md` for guidance.
//! - `ArgoError::Provider(format!("invalid tenant id '{raw}': {e}"))` — bad
//!   caller input; use [`ArgoError::Schema`].
//! - `ArgoError::Provider("missing field foo".into())` — bad caller input;
//!   use [`ArgoError::Schema`].
//! - `ArgoError::Provider(format!("uuid blob: {e}"))` — internal data
//!   corruption; use [`ArgoError::Invariant`].
use crate::budget::BudgetKind;
use crate::capability::Capability;
use crate::cost_ceilings::CapScope;

/// Argo-wide result.
pub type ArgoResult<T> = Result<T, ArgoError>;

/// Which spend-window tripped a hard `[budget.ceiling]` cap.
///
/// Type alias of [`CapScope`] so the `BudgetExceeded` error and the
/// existing pre-spend notification surface (`enforce_pre_spend`)
/// speak the same enum. The two names refer to the same variants
/// (`Session` / `Day` / `Month`); `CapScope` is the canonical type,
/// `BudgetWindow` is the operator-facing name used by the
/// enforcement-time error variant.
///
/// ```
/// use argo_types::BudgetWindow;
/// let json = serde_json::to_string(&BudgetWindow::Day).unwrap();
/// assert_eq!(json, "\"day\"");
/// ```
pub type BudgetWindow = CapScope;

/// Argo-wide error.
///
/// See the module-level docs for the canonical contract that maps each
/// variant to a failure mode and HTTP status. Picking the right variant is
/// load-bearing: gateway clients use it to triage, and the variant audit
/// (`scripts/check-error-variant-usage.sh`) flags common mismatches.
///
/// # Examples
///
/// Capability gate denied:
///
/// ```
/// use argo_types::{ArgoError, Capability};
/// let err = ArgoError::CapabilityDenied(Capability::FsRead {
///     path_glob: "/etc/passwd".into(),
/// });
/// assert!(format!("{err}").starts_with("capability denied"));
/// ```
///
/// Budget exhausted:
///
/// ```
/// use argo_types::{ArgoError, BudgetKind};
/// let err = ArgoError::BudgetExhausted { kind: BudgetKind::Iterations };
/// assert!(format!("{err}").starts_with("budget exhausted"));
/// ```
///
/// Downstream provider failure (HTTP/MCP/model API):
///
/// ```
/// use argo_types::ArgoError;
/// let err = ArgoError::Provider("openai: 502 bad gateway".into());
/// assert!(format!("{err}").contains("openai"));
/// ```
///
/// Tool execution failure with structured context:
///
/// ```
/// use argo_types::ArgoError;
/// let err = ArgoError::Tool {
///     tool: "fs.read".into(),
///     msg: "permission denied".into(),
/// };
/// assert!(format!("{err}").contains("fs.read"));
/// ```
///
/// Local I/O — prefer the `From<std::io::Error>` impl, but explicit form is:
///
/// ```
/// use argo_types::ArgoError;
/// let err = ArgoError::Io("read /tmp/foo: permission denied".into());
/// assert!(format!("{err}").starts_with("io error"));
/// ```
///
/// Caller-supplied data was malformed (manifest, JSON, query param):
///
/// ```
/// use argo_types::ArgoError;
/// let err = ArgoError::Schema("vertical manifest: missing field `name`".into());
/// assert!(format!("{err}").contains("missing field"));
/// ```
///
/// Internal invariant violated (a bug — prefer to fix the bug):
///
/// ```
/// use argo_types::ArgoError;
/// let err = ArgoError::Invariant("session status transition not allowed".into());
/// assert!(format!("{err}").contains("invariant"));
/// ```
///
/// User cancellation:
///
/// ```
/// use argo_types::ArgoError;
/// let err = ArgoError::Interrupted;
/// assert_eq!(format!("{err}"), "interrupted");
/// ```
///
/// Caller attempted to mutate a user-protected entity:
///
/// ```
/// use argo_types::ArgoError;
/// let err = ArgoError::UserProtected { entity: "skill:human-review".into() };
/// assert!(format!("{err}").contains("user-protected"));
/// ```
///
/// Judge layer rejected the candidate output:
///
/// ```
/// use argo_types::ArgoError;
/// let err = ArgoError::JudgeRejected { reason: "low confidence".into() };
/// assert!(format!("{err}").contains("judge"));
/// ```
///
/// Routing layer found no viable target:
///
/// ```
/// use argo_types::ArgoError;
/// let err = ArgoError::RoutingFailed { reason: "no provider quoted".into() };
/// assert!(format!("{err}").contains("routing"));
/// ```
///
/// RBAC role lacks the required capability:
///
/// ```
/// use argo_types::{ArgoError, Capability};
/// let err = ArgoError::RbacDenied {
///     role: "viewer".into(),
///     cap: Capability::FsWrite { path_glob: "/var/**".into() },
/// };
/// assert!(format!("{err}").starts_with("rbac denied"));
/// ```
///
/// OCI registry / packaging error:
///
/// ```
/// use argo_types::ArgoError;
/// let err = ArgoError::Oci("manifest digest mismatch".into());
/// assert!(format!("{err}").starts_with("oci error"));
/// ```
///
/// Addressed entity does not exist:
///
/// ```
/// use argo_types::ArgoError;
/// let err = ArgoError::NotFound("session: 1234".into());
/// assert!(format!("{err}").starts_with("not found"));
/// ```
#[derive(Debug, thiserror::Error)]
pub enum ArgoError {
    /// A capability check failed. Maps to HTTP 403.
    ///
    /// Use when a [`Capability`] gate denies access to a resource. Distinct
    /// from [`ArgoError::RbacDenied`], which carries the role name.
    #[error("capability denied: {0:?}")]
    CapabilityDenied(Capability),
    /// A budget was exhausted. Maps to HTTP 429.
    ///
    /// Use when a per-session, per-day, or per-tenant budget (iterations,
    /// tokens, latency) is hit. The caller may retry after backoff.
    #[error("budget exhausted: {kind:?}")]
    BudgetExhausted {
        /// Which budget.
        kind: BudgetKind,
    },
    /// A provider (downstream service) call failed. Maps to HTTP 502.
    ///
    /// Use ONLY for failures originating in a downstream service: model API
    /// (OpenAI / Anthropic), MCP transport, third-party HTTP, action
    /// endpoint. Do NOT use for local I/O, caller input validation, or
    /// internal invariant violations — those are [`ArgoError::Io`],
    /// [`ArgoError::Schema`], and [`ArgoError::Invariant`] respectively.
    /// This is the workspace's most-overloaded variant; see the audit
    /// (`docs/skycot/ERROR_VARIANT_AUDIT.md`).
    #[error("provider error: {0}")]
    Provider(String),
    /// A tool call failed. Maps to HTTP 502.
    ///
    /// Use for structured tool-execution failures where the caller benefits
    /// from carrying the tool name separately from the message.
    #[error("tool error: {tool}: {msg}")]
    Tool {
        /// Tool name.
        tool: String,
        /// Reason.
        msg: String,
    },
    /// I/O error (local file system, process, OS). Maps to HTTP 500.
    ///
    /// The `From<std::io::Error>` impl auto-converts; prefer that over
    /// explicit construction.
    #[error("io error: {0}")]
    Io(String),
    /// Schema validation failed. Maps to HTTP 400.
    ///
    /// Use when caller-supplied data is malformed: bad JSON, missing
    /// required field, invalid manifest, unparseable query param. This is
    /// the canonical "client error" variant.
    #[error("schema validation failed: {0}")]
    Schema(String),
    /// Internal invariant violated. Maps to HTTP 500.
    ///
    /// Use when an internal assumption breaks (impossible state machine
    /// transition, missing migration, corrupt persisted blob). This signals
    /// a bug — should be rare.
    ///
    /// Carries a `String` so call sites can interpolate the offending value
    /// (decoded blob, mis-typed column, parse error) with `format!()`. Aligns
    /// with the [`Provider`](Self::Provider) / [`Schema`](Self::Schema) /
    /// [`Io`](Self::Io) payload shape.
    #[error("invariant violated: {0}")]
    Invariant(String),
    /// Operation was interrupted (e.g. SIGINT). Maps to HTTP 499.
    #[error("interrupted")]
    Interrupted,
    /// Attempted to edit a user-protected entity. Maps to HTTP 403.
    #[error("user-protected: {entity}")]
    UserProtected {
        /// What's protected.
        entity: String,
    },
    /// The judge rejected the candidate. Maps to HTTP 422.
    #[error("judge rejected: {reason}")]
    JudgeRejected {
        /// Why.
        reason: String,
    },
    /// Routing failed (no viable target / auction exhausted). Maps to HTTP 503.
    #[error("routing failed: {reason}")]
    RoutingFailed {
        /// Why.
        reason: String,
    },
    /// RBAC role lacks the required capability. Maps to HTTP 403.
    ///
    /// Distinct from [`ArgoError::CapabilityDenied`] in that it carries the
    /// role name, supporting better error messages and audit logs.
    #[error("rbac denied: {role} lacks {cap:?}")]
    RbacDenied {
        /// Role name.
        role: String,
        /// Missing capability.
        cap: Capability,
    },
    /// OCI registry / packaging error. Maps to HTTP 502.
    #[error("oci error: {0}")]
    Oci(String),
    /// The addressed entity does not exist. Maps to HTTP 404 at the
    /// gateway boundary.
    #[error("not found: {0}")]
    NotFound(String),
    /// A hard `[budget.ceiling]` cap would be exceeded by the
    /// projected cost of an LLM call. Maps to HTTP 429.
    ///
    /// Returned by the `SpendGate` transport wrapper at
    /// provider-call time, BEFORE the wrapped transport is invoked —
    /// so no spend is incurred. Distinct from
    /// [`ArgoError::BudgetExhausted`] in that the latter signals an
    /// in-flight budget tree (iterations / tokens / per-session
    /// `Budget`) was exceeded *during* the agent loop, whereas
    /// `BudgetExceeded` is a pre-spend ceiling enforcement against
    /// the persisted `spend_ledger` (per-tenant session / day /
    /// month windows).
    #[error(
        "budget ceiling exceeded: {window:?} window used \
         {used_micros} usd-micros, ceiling {ceiling_micros}, \
         projected charge {projected_cost_micros}"
    )]
    BudgetExceeded {
        /// Which window (Session / Day / Month) tripped.
        window: BudgetWindow,
        /// Spend already recorded in the window (USD micros).
        used_micros: u64,
        /// Configured ceiling for the window (USD micros).
        ceiling_micros: u64,
        /// Projected cost of the call that was blocked (USD micros).
        projected_cost_micros: u64,
    },
    /// A streaming LLM call exhausted its retry budget while attempting
    /// to recover from one or more transient mid-stream failures.
    /// Maps to HTTP 502 at the gateway boundary.
    ///
    /// Emitted by the `argo_provider::streaming_retry` wrapper after
    /// `max_attempts` retries with continuation hints all failed. The
    /// `partial_content` payload carries whatever assistant text was
    /// buffered across attempts so the caller can surface "best-effort"
    /// output to the user rather than discarding everything. Distinct
    /// from [`ArgoError::Provider`] in that the latter signals a fatal
    /// non-retryable wire failure (4xx, malformed body, etc.) without
    /// recovered content.
    ///
    /// ```
    /// use argo_types::ArgoError;
    /// let err = ArgoError::StreamingRetryExhausted {
    ///     partial_content: "Once upon a ti".into(),
    /// };
    /// assert!(format!("{err}").contains("streaming retry exhausted"));
    /// ```
    #[error(
        "streaming retry exhausted: {} bytes of partial content recovered",
        partial_content.len()
    )]
    StreamingRetryExhausted {
        /// Whatever assistant text the wrapper buffered before the last
        /// retry attempt failed. May be empty if every attempt failed
        /// before yielding any chunks.
        partial_content: String,
    },
    /// Adaptive context truncation could not fit the message history
    /// within the model's context window even after dropping older
    /// turns and intra-turn truncating giant tool outputs. Maps to
    /// HTTP 500.
    ///
    /// Returned by `argo_runtime::context_truncate::truncate_to_fit`
    /// when even the minimum-must-keep set (system prompt + last user
    /// turn) exceeds the window. The caller MUST handle this — a
    /// silent malformed call is worse than failing loudly.
    /// Distinct from [`ArgoError::BudgetExhausted`] (an in-flight
    /// per-session budget tree) and from [`ArgoError::BudgetExceeded`]
    /// (a pre-spend USD-micros ceiling) in that this is a *structural*
    /// failure of the message envelope, not a cost-based gate.
    #[error(
        "context window exhausted: minimum {minimum_tokens} tokens \
         exceeds window {window_tokens}"
    )]
    ContextWindowExhausted {
        /// Token count of the minimum-must-keep set (system prompt +
        /// last user turn).
        minimum_tokens: usize,
        /// Configured model context window in tokens.
        window_tokens: usize,
    },
}

impl From<std::io::Error> for ArgoError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}
