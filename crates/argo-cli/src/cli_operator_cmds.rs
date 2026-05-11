//! `argo operator …` subcommand tree.
//!
//! Initial surface (Stream `mission-operator-specific-risk-overrides`,
//! MISSION_BACKLOG Section 3 Claim 6 #122) is the `risk-overrides`
//! family — per-operator overrides of the manifest-driven default
//! risk classification. Additional operator-scoped surfaces land
//! through follow-up briefs (preferences, escalations, etc.).

use clap::{Subcommand, ValueEnum};

#[derive(Subcommand, Debug)]
pub enum OperatorCmd {
    /// Per-operator overrides of the manifest-driven default risk
    /// classification (Stream
    /// `mission-operator-specific-risk-overrides`). Upgrade-only by
    /// default — downgrades require `--allow-downgrade` so that a
    /// compromised account can't silently relax approval gates.
    #[command(name = "risk-overrides")]
    RiskOverrides {
        #[command(subcommand)]
        cmd: OperatorRiskOverridesCmd,
    },
}

#[derive(Subcommand, Debug)]
pub enum OperatorRiskOverridesCmd {
    /// Set or replace an operator's override for one
    /// `(action-kind, [--vertical <id>])` scope.
    Set {
        /// Operator id.
        #[arg(value_name = "OPERATOR_ID")]
        operator: String,
        /// Action kind the override applies to (e.g.
        /// `send_email_external`).
        #[arg(value_name = "ACTION_KIND")]
        action_kind: String,
        /// Replacement risk level.
        #[arg(value_name = "RISK_LEVEL", value_enum)]
        risk_level: RiskLevelArg,
        /// Restrict the override to a single vertical id. Omit to
        /// apply across every vertical this operator touches.
        #[arg(long)]
        vertical: Option<String>,
        /// Acknowledge that this override is allowed to lower the
        /// classification below the manifest default. Without this
        /// flag, downgrades are refused at resolution time.
        #[arg(long)]
        allow_downgrade: bool,
        /// Emit JSON instead of a one-line summary.
        #[arg(long)]
        json: bool,
    },
    /// Show every override on file for one operator.
    Get {
        /// Operator id.
        #[arg(value_name = "OPERATOR_ID")]
        operator: String,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
    /// List every override across every operator.
    List {
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
    /// Delete one override row keyed on
    /// `(operator-id, action-kind, [--vertical <id>])`.
    Delete {
        /// Operator id.
        #[arg(value_name = "OPERATOR_ID")]
        operator: String,
        /// Action kind whose override is being cleared.
        #[arg(value_name = "ACTION_KIND")]
        action_kind: String,
        /// Restrict the delete to a single vertical scope. Omit to
        /// clear the all-verticals row.
        #[arg(long)]
        vertical: Option<String>,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum RiskLevelArg {
    /// Read-only or draft-only work.
    Low,
    /// Reversible public or monetary side effects.
    Medium,
    /// Human-touching, irreversible, or high-cost side effects.
    High,
}

impl RiskLevelArg {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}
