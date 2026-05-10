use crate::CircuitState;
use argo_types::TenantId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

#[path = "memory_compose.rs"]
mod memory_compose;

#[path = "types_role.rs"]
mod types_role;

pub use types_role::{Duty, Persona, Role, VoiceProfile};

/// Stable worker identity.
///
/// ```
/// use argo_identity::WorkerId;
/// let id = WorkerId::new();
/// assert_eq!(id.0.get_version_num(), 7);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkerId(pub Uuid);

impl WorkerId {
    /// Generate a fresh UUIDv7 worker id.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for WorkerId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for WorkerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl FromStr for WorkerId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self)
    }
}

/// Memory namespace scope for a worker.
///
/// ```
/// use argo_identity::MemoryScope;
/// assert_eq!(format!("{:?}", MemoryScope::WorkerSelf), "WorkerSelf");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryScope {
    /// Private memory owned by the worker instance.
    WorkerSelf,
    /// Memory shared by workers executing the same role.
    Role,
    /// Team-wide shared memory for handoffs.
    SharedTeam,
}

impl MemoryScope {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::WorkerSelf => "worker_self",
            Self::Role => "role",
            Self::SharedTeam => "shared_team",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "worker_self" => Some(Self::WorkerSelf),
            "role" => Some(Self::Role),
            "shared_team" => Some(Self::SharedTeam),
            _ => None,
        }
    }
}

/// Tenant-scoped memory namespace for every identity-aware query.
///
/// ```
/// use argo_identity::{MemoryNs, MemoryScope, WorkerId};
/// use argo_types::TenantId;
///
/// let ns = MemoryNs::new(TenantId::new(), WorkerId::new(), MemoryScope::WorkerSelf);
/// assert_eq!(ns.scope, MemoryScope::WorkerSelf);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryNs {
    /// Tenant boundary for the memory query.
    pub tenant_id: TenantId,
    /// Worker instance whose memory is being queried.
    pub worker_id: WorkerId,
    /// Scope inside the worker's memory namespace.
    pub scope: MemoryScope,
}

impl MemoryNs {
    /// Build a namespace from tenant, worker id, and scope.
    #[must_use]
    pub fn new(tenant_id: TenantId, worker_id: WorkerId, scope: MemoryScope) -> Self {
        Self {
            tenant_id,
            worker_id,
            scope,
        }
    }

    /// Compose memory rows into a `## Memory` markdown block (top-N=8,
    /// override via `ARGO_MEMORY_COMPOSE_TOP_N`). Empty input → empty.
    ///
    /// ```
    /// use argo_identity::{MemoryNs, MemoryScope, WorkerId};
    /// use argo_types::TenantId;
    /// let ns = MemoryNs::new(TenantId::new(), WorkerId::new(), MemoryScope::SharedTeam);
    /// assert_eq!(ns.compose_context(&[]), "");
    /// ```
    #[must_use]
    pub fn compose_context(&self, rows: &[(chrono::DateTime<chrono::Utc>, String)]) -> String {
        memory_compose::compose(rows, memory_compose::resolve_top_n())
    }
}

/// Auto-approval policy attached to a worker.
///
/// The Stream H-1 evaluator in [`crate::auto_approval::evaluate`]
/// consumes this + a [`crate::auto_approval::SpendSnapshot`] + a
/// `PendingAction` and returns a decision for the dispatch gate.
///
/// All fields beyond `enabled` are optional and additive — older
/// persisted values still deserialise (serde `default`s on every
/// new field).
///
/// ```
/// use argo_identity::AutoApprovalPolicy;
/// assert!(!AutoApprovalPolicy::default().enabled);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoApprovalPolicy {
    /// Whether auto-approval is enabled.
    pub enabled: bool,
    /// Optional per-action spend ceiling in USD micros.
    #[serde(default)]
    pub max_usd_micros_per_action: Option<u64>,
    /// Optional daily spend ceiling in USD micros.
    #[serde(default)]
    pub daily_budget_usd_micros: Option<u64>,
    /// Allowed time windows. Empty = no time-based restriction.
    /// Outside the windows the evaluator returns
    /// [`crate::auto_approval::Decision::Ask`].
    #[serde(default)]
    pub time_windows: Vec<crate::auto_approval::TimeWindow>,
    /// Wake-rules: action types that bypass the time-window
    /// guard because they're urgent (e.g. `page_oncall`,
    /// `escalate_ticket`). The evaluator skips the window check
    /// when the pending action_type matches.
    #[serde(default)]
    pub wake_action_types: Vec<String>,
}

/// Notification channel preference for approvals.
///
/// ```
/// use argo_identity::NotifyChannelRef;
/// let channel = NotifyChannelRef::new("email", "operator@example.com");
/// assert_eq!(channel.channel, "email");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotifyChannelRef {
    /// Channel adapter id, such as `email` or `slack`.
    pub channel: String,
    /// Adapter-specific destination.
    pub target: String,
    /// Optional risk level this channel handles.
    pub risk_level: Option<String>,
}

impl NotifyChannelRef {
    /// Create a channel target with no risk filter.
    ///
    /// ```
    /// use argo_identity::NotifyChannelRef;
    /// let channel = NotifyChannelRef::new("slack", "C123");
    /// assert_eq!(channel.target, "C123");
    /// ```
    #[must_use]
    pub fn new(channel: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            channel: channel.into(),
            target: target.into(),
            risk_level: None,
        }
    }
}

/// Maximum byte length of `Worker::charter` (mission claim 2).
pub const MAX_CHARTER_BYTES: usize = 16 * 1024;

/// Maximum byte length of the serialised `Worker::kpis` JSON.
pub const MAX_KPIS_BYTES: usize = 8 * 1024;

/// Worker tenure level (Stream `claude-worker-promotion-workflow`,
/// mission claim 2). Workers start at [`WorkerLevel::Entry`] and can
/// be promoted up the ladder via `argo worker promote`. Auto-approval
/// downstream consumption (gating riskier policies behind seniority)
/// is a follow-up brief.
///
/// ```
/// use argo_identity::WorkerLevel;
/// use std::str::FromStr;
/// assert_eq!(WorkerLevel::Entry.next(), Some(WorkerLevel::Junior));
/// assert_eq!(WorkerLevel::Lead.next(), None);
/// assert_eq!(WorkerLevel::from_str("junior").unwrap(), WorkerLevel::Junior);
/// assert_eq!(WorkerLevel::Senior.to_string(), "senior");
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerLevel {
    /// Newly-created worker. Default for every fresh row.
    #[default]
    Entry,
    /// Promoted from `Entry` once the tenure / sessions / success-rate
    /// gate is cleared.
    Junior,
    /// Promoted from `Junior`. Usually the threshold for unattended
    /// auto-approval scenarios.
    Senior,
    /// Highest level. Promotion to `Lead` always requires explicit
    /// operator confirmation (`--force`).
    Lead,
}

impl WorkerLevel {
    /// Stable wire label persisted in `worker.level`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Entry => "entry",
            Self::Junior => "junior",
            Self::Senior => "senior",
            Self::Lead => "lead",
        }
    }

    /// Next level on the promotion ladder, or `None` if already at the top.
    #[must_use]
    pub fn next(self) -> Option<Self> {
        match self {
            Self::Entry => Some(Self::Junior),
            Self::Junior => Some(Self::Senior),
            Self::Senior => Some(Self::Lead),
            Self::Lead => None,
        }
    }

    /// Ordinal rank — higher means more senior. Useful for guarding
    /// against demotion or skip-level promotions.
    #[must_use]
    pub fn rank(self) -> u8 {
        match self {
            Self::Entry => 0,
            Self::Junior => 1,
            Self::Senior => 2,
            Self::Lead => 3,
        }
    }
}

impl fmt::Display for WorkerLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for WorkerLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "entry" => Ok(Self::Entry),
            "junior" => Ok(Self::Junior),
            "senior" => Ok(Self::Senior),
            "lead" => Ok(Self::Lead),
            other => Err(format!(
                "invalid worker level '{other}' (expected entry|junior|senior|lead)"
            )),
        }
    }
}

/// Runtime worker instance.
///
/// `charter` (markdown body, ≤ 16 KB) and `kpis` (free-form JSON,
/// ≤ 8 KB serialised) back mission claim 2 (delegation contract);
/// both fields are optional so older worker rows without the columns
/// load cleanly.
///
/// ```
/// use argo_identity::{Persona, Role, Worker};
/// use argo_types::TenantId;
/// let worker = Worker::draft(TenantId::new(), "support", Persona::named("Ari", "op"), Role::new("Triage."));
/// assert_eq!(worker.vertical_id, "support");
/// assert!(worker.charter.is_none() && worker.kpis.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Worker {
    /// Stable worker id.
    pub id: WorkerId,
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Vertical template id this worker instantiates.
    pub vertical_id: String,
    /// Public persona.
    pub persona: Persona,
    /// Delegated role.
    pub role: Role,
    /// Worker memory namespace.
    pub memory_namespace: MemoryNs,
    /// Auto-approval policy.
    pub auto_approval: AutoApprovalPolicy,
    /// Dispatch circuit state. Defaults closed for older rows.
    #[serde(default)]
    pub circuit_state: CircuitState,
    /// Approval notification channels.
    pub channels: Vec<NotifyChannelRef>,
    /// Per-instance operating charter (markdown, ≤ `MAX_CHARTER_BYTES`).
    #[serde(default)]
    pub charter: Option<String>,
    /// Per-instance KPIs (free-form JSON, ≤ `MAX_KPIS_BYTES` serialised).
    #[serde(default)]
    pub kpis: Option<serde_json::Value>,
    /// Tenure level (Stream `claude-worker-promotion-workflow`).
    /// Defaults to [`WorkerLevel::Entry`] for older rows lacking the
    /// column / serialised field.
    #[serde(default)]
    pub level: WorkerLevel,
    /// Worker-side trust grade on the same numeric ladder as
    /// [`argo_skill::trust::TrustGrade`] — `0` = unknown / fresh row.
    /// Stream `mvp-worker-trust-grade-column`; the routing engine
    /// consumes this to rank candidates for `TargetSelector::SkillMatch`
    /// / `RoleMatch` resolution. Defaults to `0` for older rows.
    #[serde(default)]
    pub trust_grade: u8,
    /// Most-recent assignment timestamp (UTC). `None` when the
    /// worker has never been used. Stream
    /// `mvp-worker-trust-grade-column`. Defaults to `None` for older
    /// rows lacking the column.
    #[serde(default)]
    pub last_used_at: Option<DateTime<Utc>>,
}

impl Worker {
    /// Draft a worker with private memory and default approval settings.
    ///
    /// ```
    /// use argo_identity::{Persona, Role, Worker};
    /// use argo_types::TenantId;
    /// let worker = Worker::draft(TenantId::new(), "gtm", Persona::named("Maya", "op"), Role::new("Sell."));
    /// assert_eq!(worker.memory_namespace.worker_id, worker.id);
    /// ```
    #[must_use]
    pub fn draft(
        tenant_id: TenantId,
        vertical_id: impl Into<String>,
        persona: Persona,
        role: Role,
    ) -> Self {
        let id = WorkerId::new();
        Self {
            id,
            tenant_id,
            vertical_id: vertical_id.into(),
            persona,
            role,
            memory_namespace: MemoryNs::new(tenant_id, id, MemoryScope::WorkerSelf),
            auto_approval: AutoApprovalPolicy::default(),
            circuit_state: CircuitState::Closed,
            channels: Vec::new(),
            charter: None,
            kpis: None,
            level: WorkerLevel::Entry,
            trust_grade: 0,
            last_used_at: None,
        }
    }

    /// Stream `claude-worker-instance-policy` — resolve the
    /// per-action [`AutoApprovalPolicy`] this worker should be
    /// gated against.
    ///
    /// Resolution order:
    ///
    /// 1. Worker's own `auto_approval` if `enabled`.
    /// 2. Workspace default passed in.
    /// 3. Disabled default (Ask everything; the H-1 evaluator
    ///    handles the fallback semantics).
    ///
    /// "Empty / default policy" = treat as not-configured. The
    /// brief calls this out: a worker with `enabled = false`
    /// falls through to the workspace default.
    ///
    /// ```
    /// use argo_identity::{AutoApprovalPolicy, Persona, Role, Worker};
    /// use argo_types::TenantId;
    /// let mut worker = Worker::draft(TenantId::new(), "gtm",
    ///     Persona::named("Maya", "op"), Role::new("Sell."));
    /// // Disabled worker → workspace default applies.
    /// let workspace_enabled = AutoApprovalPolicy { enabled: true, ..Default::default() };
    /// assert!(worker.resolved_policy(&workspace_enabled).enabled);
    /// // Enable on the worker → worker policy overrides.
    /// worker.auto_approval.enabled = true;
    /// worker.auto_approval.daily_budget_usd_micros = Some(50_000);
    /// let workspace_disabled = AutoApprovalPolicy::default();
    /// let resolved = worker.resolved_policy(&workspace_disabled);
    /// assert_eq!(resolved.daily_budget_usd_micros, Some(50_000));
    /// ```
    #[must_use]
    pub fn resolved_policy(
        &self,
        workspace_default: &AutoApprovalPolicy,
    ) -> AutoApprovalPolicy {
        self.resolved_policy_with_rosters(&[], workspace_default)
    }

    /// Stream `claude-worker-roster` — resolution chain with the
    /// roster layer wedged between worker and workspace defaults.
    ///
    /// 1. Worker's own `auto_approval` if `enabled`.
    /// 2. Most-restrictive merge of the enabled `roster_policies`.
    /// 3. `workspace_default`.
    #[must_use]
    pub fn resolved_policy_with_rosters(
        &self,
        roster_policies: &[AutoApprovalPolicy],
        workspace_default: &AutoApprovalPolicy,
    ) -> AutoApprovalPolicy {
        if self.auto_approval.enabled {
            return self.auto_approval.clone();
        }
        let enabled: Vec<&AutoApprovalPolicy> =
            roster_policies.iter().filter(|p| p.enabled).collect();
        if enabled.is_empty() {
            workspace_default.clone()
        } else {
            AutoApprovalPolicy::most_restrictive(&enabled)
        }
    }
}
