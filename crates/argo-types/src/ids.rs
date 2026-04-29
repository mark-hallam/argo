//! UUIDv7-based identifier types and the human-readable `AgentName`.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub Uuid);

        impl $name {
            /// Generate a fresh ID.
            ///
            /// ```
            /// use argo_types::*;
            #[doc = concat!("let id = ", stringify!($name), "::new();")]
            /// assert_ne!(id.0.get_version_num(), 0);
            /// ```
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0, f)
            }
        }
    };
}

id_type!(
    TenantId,
    "Tenant identity. Plumbed through every primitive."
);
id_type!(
    RequestId,
    "Outbound provider-call request identifier. Used as the HTTP \
    `Idempotency-Key` so retries against the same logical call don't \
    duplicate the side-effect on the provider side."
);
id_type!(
    AgentId,
    "An agent instance ID (one per tenant + named-agent run)."
);
id_type!(SessionId, "A conversation session.");
id_type!(MessageId, "A single message in a session.");
id_type!(ToolCallId, "A tool call request/result correlation ID.");
id_type!(WorkspaceId, "A workspace bundle.");
id_type!(SkillId, "A skill (named + versioned).");
id_type!(JudgeRecordId, "A judge verdict record.");
id_type!(RoutingDecisionId, "A routing decision record.");
id_type!(ForgeRunId, "An Argo Forge optimization run.");

/// A human-readable named agent within a tenant (Amendment 5).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentName(pub String);
