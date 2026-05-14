//! Multi-channel approval routing plans.
//!
//! Routing is transport-agnostic: the runtime selects which channel
//! should receive an approval request next, while channel adapters own
//! delivery and inbound replies.
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Skill IR annotation key for persisted approval routing plans.
pub const APPROVAL_ROUTING_ANNOTATION: &str = "skycot.approval_routing.v1";

/// Supported approval notification channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalChannelKind {
    /// Slack channel, user, or webhook alias.
    Slack,
    /// SMS destination such as an E.164 phone number or roster alias.
    Sms,
    /// Email address or group alias.
    Email,
    /// Mobile or desktop push channel.
    Push,
    /// Web inbox fallback.
    WebInbox,
    /// Voice-call approval channel.
    Voice,
    /// Generic chat webhook such as Teams or Mattermost.
    Chat,
    /// Server-sent event stream.
    Sse,
}

/// One approval delivery target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalChannel {
    /// Channel kind.
    pub kind: ApprovalChannelKind,
    /// Channel-specific address, alias, or route id.
    pub target: String,
}

impl ApprovalChannel {
    /// Build a channel target.
    #[must_use]
    pub fn new(kind: ApprovalChannelKind, target: impl Into<String>) -> Self {
        Self {
            kind,
            target: target.into(),
        }
    }
}

/// One node in a fallback routing tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRouteNode {
    /// Stable node id inside the plan.
    pub id: String,
    /// Destination selected when this node is active.
    pub channel: ApprovalChannel,
    /// Milliseconds to wait before falling back.
    pub timeout_ms: u64,
    /// Optional fallback node.
    pub fallback: Option<Box<ApprovalRouteNode>>,
}

impl ApprovalRouteNode {
    /// Build a route node.
    #[must_use]
    pub fn new(id: impl Into<String>, channel: ApprovalChannel, timeout_ms: u64) -> Self {
        Self {
            id: id.into(),
            channel,
            timeout_ms,
            fallback: None,
        }
    }

    /// Attach a fallback node.
    #[must_use]
    pub fn with_fallback(mut self, fallback: ApprovalRouteNode) -> Self {
        self.fallback = Some(Box::new(fallback));
        self
    }

    fn find(&self, node_id: &str) -> Option<&Self> {
        if self.id == node_id {
            return Some(self);
        }
        self.fallback.as_deref().and_then(|node| node.find(node_id))
    }

    fn validate(&self) -> Result<(), ApprovalRoutingError> {
        if self.id.trim().is_empty() {
            return Err(ApprovalRoutingError::EmptyNodeId);
        }
        if self.channel.target.trim().is_empty() {
            return Err(ApprovalRoutingError::EmptyChannelTarget {
                node_id: self.id.clone(),
            });
        }
        if self.timeout_ms == 0 && self.fallback.is_some() {
            return Err(ApprovalRoutingError::ZeroFallbackTimeout {
                node_id: self.id.clone(),
            });
        }
        if let Some(fallback) = self.fallback.as_deref() {
            fallback.validate()?;
        }
        Ok(())
    }
}

/// Serializable approval routing annotation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRoutingPlan {
    /// Stable plan id.
    pub id: String,
    /// Human-facing label.
    pub label: String,
    /// First channel in the routing tree.
    pub root: ApprovalRouteNode,
}

impl ApprovalRoutingPlan {
    /// Build a routing plan.
    #[must_use]
    pub fn new(id: impl Into<String>, label: impl Into<String>, root: ApprovalRouteNode) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            root,
        }
    }

    /// Validate basic plan invariants before storing in skill IR.
    pub fn validate(&self) -> Result<(), ApprovalRoutingError> {
        if self.id.trim().is_empty() {
            return Err(ApprovalRoutingError::EmptyPlanId);
        }
        self.root.validate()
    }

    fn node(&self, node_id: &str) -> Option<&ApprovalRouteNode> {
        self.root.find(node_id)
    }
}

/// Delivery status for one route node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalAttemptStatus {
    /// Sent and awaiting a response.
    Pending,
    /// Approved by the recipient.
    Approved,
    /// Rejected by the recipient.
    Rejected,
    /// Delivery failed; runtime may advance immediately.
    Failed,
}

/// Recorded delivery attempt for a route node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRouteAttempt {
    /// Node that was delivered.
    pub node_id: String,
    /// Channel copied from the plan at delivery time.
    pub channel: ApprovalChannel,
    /// Delivery timestamp.
    pub sent_at: DateTime<Utc>,
    /// Current attempt status.
    pub status: ApprovalAttemptStatus,
    /// Response timestamp, if any.
    pub resolved_at: Option<DateTime<Utc>>,
}

impl ApprovalRouteAttempt {
    /// Create a pending attempt for a node.
    #[must_use]
    pub fn pending(node: &ApprovalRouteNode, sent_at: DateTime<Utc>) -> Self {
        Self {
            node_id: node.id.clone(),
            channel: node.channel.clone(),
            sent_at,
            status: ApprovalAttemptStatus::Pending,
            resolved_at: None,
        }
    }
}

/// Runtime decision for the approval router.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalRoutingAction {
    /// Send an approval request to this route node now.
    Dispatch {
        /// Node id to deliver.
        node_id: String,
        /// Channel target to deliver.
        channel: ApprovalChannel,
    },
    /// Keep waiting for the active node.
    Wait {
        /// Active node id.
        node_id: String,
        /// Deadline when fallback becomes eligible.
        until: DateTime<Utc>,
    },
    /// The approval request resolved.
    Resolved {
        /// Node that resolved the request.
        node_id: String,
        /// Terminal status.
        status: ApprovalAttemptStatus,
    },
    /// No fallback remains.
    Exhausted {
        /// Last attempted node id.
        last_node_id: String,
    },
}

/// Approval routing validation/runtime error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ApprovalRoutingError {
    /// Plan id is empty.
    #[error("approval routing plan id is empty")]
    EmptyPlanId,
    /// Node id is empty.
    #[error("approval routing node id is empty")]
    EmptyNodeId,
    /// Channel target is empty.
    #[error("approval routing node {node_id} has an empty channel target")]
    EmptyChannelTarget {
        /// Invalid node id.
        node_id: String,
    },
    /// Fallback timeout must be positive.
    #[error("approval routing node {node_id} has fallback with zero timeout")]
    ZeroFallbackTimeout {
        /// Invalid node id.
        node_id: String,
    },
    /// Attempt references a node that does not exist in the plan.
    #[error("approval routing attempt references unknown node {node_id}")]
    UnknownNode {
        /// Missing node id.
        node_id: String,
    },
}

/// Pure runtime walker for approval routing plans.
pub struct ApprovalRoutingRuntime;

impl ApprovalRoutingRuntime {
    /// Decide whether to dispatch, wait, resolve, or exhaust.
    pub fn next_action(
        plan: &ApprovalRoutingPlan,
        attempts: &[ApprovalRouteAttempt],
        now: DateTime<Utc>,
    ) -> Result<ApprovalRoutingAction, ApprovalRoutingError> {
        plan.validate()?;
        let Some(last) = attempts.last() else {
            return Ok(dispatch(&plan.root));
        };
        let Some(node) = plan.node(&last.node_id) else {
            return Err(ApprovalRoutingError::UnknownNode {
                node_id: last.node_id.clone(),
            });
        };
        match last.status {
            ApprovalAttemptStatus::Approved | ApprovalAttemptStatus::Rejected => {
                Ok(ApprovalRoutingAction::Resolved {
                    node_id: last.node_id.clone(),
                    status: last.status,
                })
            }
            ApprovalAttemptStatus::Failed => Ok(next_or_exhaust(node, &last.node_id)),
            ApprovalAttemptStatus::Pending => {
                let timeout =
                    Duration::milliseconds(i64::try_from(node.timeout_ms).unwrap_or(i64::MAX));
                let deadline = last.sent_at + timeout;
                if now < deadline {
                    Ok(ApprovalRoutingAction::Wait {
                        node_id: last.node_id.clone(),
                        until: deadline,
                    })
                } else {
                    Ok(next_or_exhaust(node, &last.node_id))
                }
            }
        }
    }
}

fn dispatch(node: &ApprovalRouteNode) -> ApprovalRoutingAction {
    ApprovalRoutingAction::Dispatch {
        node_id: node.id.clone(),
        channel: node.channel.clone(),
    }
}

fn next_or_exhaust(node: &ApprovalRouteNode, last_node_id: &str) -> ApprovalRoutingAction {
    node.fallback
        .as_deref()
        .map(dispatch)
        .unwrap_or_else(|| ApprovalRoutingAction::Exhausted {
            last_node_id: last_node_id.to_string(),
        })
}
