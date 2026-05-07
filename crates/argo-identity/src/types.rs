use crate::CircuitState;
use argo_types::TenantId;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

#[path = "memory_compose.rs"]
mod memory_compose;

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

/// Public-facing voice profile for a persona.
///
/// ```
/// use argo_identity::VoiceProfile;
/// let voice = VoiceProfile { preamble: "Be concise.".into() };
/// assert!(voice.preamble.contains("concise"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceProfile {
    /// System-prompt preamble that defines the worker's voice.
    pub preamble: String,
}

/// Persona presented to humans.
///
/// ```
/// use argo_identity::Persona;
/// let persona = Persona::named("Maya", "operator@example.com");
/// assert_eq!(persona.name, "Maya");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Persona {
    /// Human-facing name.
    pub name: String,
    /// Voice and tone instructions.
    pub voice_profile: VoiceProfile,
    /// Optional avatar URL or asset id.
    pub avatar: Option<String>,
    /// Optional public biography.
    pub bio: Option<String>,
    /// Operator email, handle, or id that built the persona.
    pub builder_attribution: String,
    /// Skill ids that contributed to this persona over time.
    pub provenance_chain: Vec<String>,
}

impl Persona {
    /// Create a minimal named persona.
    #[must_use]
    pub fn named(name: impl Into<String>, builder_attribution: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            voice_profile: VoiceProfile {
                preamble: format!("You are {name}, an autonomous SKYCOT worker."),
            },
            name,
            avatar: None,
            bio: None,
            builder_attribution: builder_attribution.into(),
            provenance_chain: Vec::new(),
        }
    }
}

/// A scheduled or callable duty assigned to a role
/// (Stream `claude-worker-duty-scheduler`). `id`/`enabled`/
/// `paused_until`/`last_fired_at` are `#[serde(default)]` so role
/// JSON written before V1.3 still deserialises unchanged.
///
/// ```
/// use argo_identity::Duty;
/// let duty = Duty::new("0 9 * * 1-5", "daily_check", "Check the inbox.");
/// assert_eq!(duty.entry_action, "daily_check");
/// assert!(duty.enabled && duty.paused_until.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Duty {
    /// Stable id (UUIDv7-stringified for sort order).
    #[serde(default = "Duty::default_id")]
    pub id: String,
    /// Cron schedule that drives the duty.
    pub cron: String,
    /// Entry action invoked when the duty runs.
    pub entry_action: String,
    /// Human-readable description.
    pub description: String,
    /// Scheduler considers this duty when `true`.
    #[serde(default = "Duty::default_enabled")]
    pub enabled: bool,
    /// Scheduler skips this duty until the `DateTime` passes.
    #[serde(default)]
    pub paused_until: Option<chrono::DateTime<chrono::Utc>>,
    /// Last successful fire (powers the per-window idempotency).
    #[serde(default)]
    pub last_fired_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Duty {
    /// Build an enabled, never-fired duty with a fresh UUIDv7 id.
    #[must_use]
    pub fn new(
        cron: impl Into<String>,
        entry_action: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: Self::default_id(),
            cron: cron.into(),
            entry_action: entry_action.into(),
            description: description.into(),
            enabled: true,
            paused_until: None,
            last_fired_at: None,
        }
    }

    fn default_id() -> String { Uuid::now_v7().to_string() }
    fn default_enabled() -> bool { true }
}

/// Delegated role: charter, duties, KPIs, and escalation path.
///
/// ```
/// use argo_identity::Role;
/// let role = Role::new("Follow up with approved leads.");
/// assert!(role.duties.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    /// Free-text operating charter.
    pub charter: String,
    /// Duties that can be scheduled or invoked.
    pub duties: Vec<Duty>,
    /// Key performance indicators.
    pub kpis: Vec<String>,
    /// Escalation path for blocked work.
    pub escalation: Option<String>,
}

impl Role {
    /// Create a role with no duties yet.
    ///
    /// ```
    /// use argo_identity::Role;
    /// let role = Role::new("Own qualified support triage.");
    /// assert!(role.charter.starts_with("Own"));
    /// ```
    #[must_use]
    pub fn new(charter: impl Into<String>) -> Self {
        Self {
            charter: charter.into(),
            duties: Vec::new(),
            kpis: Vec::new(),
            escalation: None,
        }
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
