//! Resource-policy config sections: `[budget]`, `[delegate]`,
//! `[parallel]`. Extracted from [`crate::sections`] to keep that
//! module under the 500-LOC ceiling. Re-exported through `sections`
//! so the public API is unchanged.
use crate::defaults;
use argo_types::CostCeilings;
use serde::{Deserialize, Serialize};

/// `[budget]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BudgetConfig {
    /// Default iteration cap.
    pub default_iterations: u32,
    /// Default token-in cap.
    pub default_tokens_in: u64,
    /// Default token-out cap.
    pub default_tokens_out: u64,
    /// Default wallclock cap (ms).
    pub default_wallclock_ms: u64,
    /// Default spend cap.
    pub default_spend_usd_micros: u64,
    /// Per-session hard cap.
    pub default_spend_per_session_usd_micros: u64,
    /// Per-day hard cap.
    pub default_spend_per_day_usd_micros: u64,
    /// Per-month hard cap.
    pub default_spend_per_month_usd_micros: u64,
    /// Notification thresholds.
    pub notify_at_pct: Vec<u8>,
    /// `halt` | `prompt_user` | `auto_continue`.
    pub on_max_iterations: String,
    /// Hard cap on auto-continue extensions.
    pub auto_continue_max_extensions: u32,
    /// How many iterations each extension grants.
    pub auto_continue_extension_size: u32,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            default_iterations: defaults::DEFAULT_BUDGET_ITERATIONS,
            default_tokens_in: 0,
            default_tokens_out: 0,
            default_wallclock_ms: 0,
            default_spend_usd_micros: 0,
            default_spend_per_session_usd_micros: defaults::DEFAULT_SPEND_PER_SESSION_USD_MICROS,
            default_spend_per_day_usd_micros: defaults::DEFAULT_SPEND_PER_DAY_USD_MICROS,
            default_spend_per_month_usd_micros: defaults::DEFAULT_SPEND_PER_MONTH_USD_MICROS,
            notify_at_pct: defaults::DEFAULT_NOTIFY_AT_PCT.to_vec(),
            on_max_iterations: defaults::DEFAULT_ON_MAX_ITERATIONS.into(),
            auto_continue_max_extensions: defaults::DEFAULT_AUTO_CONTINUE_MAX_EXTENSIONS,
            auto_continue_extension_size: defaults::DEFAULT_AUTO_CONTINUE_EXTENSION_SIZE,
        }
    }
}

impl BudgetConfig {
    /// Bind the `[budget]` block to a [`CostCeilings`] (item C).
    /// `0` for any of the per-scope caps disables that scope.
    #[must_use]
    pub fn to_cost_ceilings(&self) -> CostCeilings {
        CostCeilings {
            per_session_usd_micros: nonzero(self.default_spend_per_session_usd_micros),
            per_day_usd_micros: nonzero(self.default_spend_per_day_usd_micros),
            per_month_usd_micros: nonzero(self.default_spend_per_month_usd_micros),
            notify_at_pct: self.notify_at_pct.clone(),
        }
    }
}

fn nonzero(v: u64) -> Option<u64> {
    if v == 0 {
        None
    } else {
        Some(v)
    }
}

/// `[delegate]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DelegateConfig {
    /// Max nesting depth.
    pub max_spawn_depth: u32,
    /// Cap on concurrent siblings.
    pub max_concurrent_children: u32,
    /// Default child iteration cap.
    pub child_default_iterations: u32,
    /// `explicit` | `share`.
    pub budget_inheritance: String,
    /// Inherit fallback chain (Amendment 7).
    pub fallback_chain_inheritance: bool,
}

impl Default for DelegateConfig {
    fn default() -> Self {
        Self {
            max_spawn_depth: defaults::DEFAULT_MAX_SPAWN_DEPTH,
            max_concurrent_children: defaults::DEFAULT_MAX_CONCURRENT_CHILDREN,
            child_default_iterations: defaults::DEFAULT_CHILD_ITERATIONS,
            budget_inheritance: "explicit".into(),
            fallback_chain_inheritance: true,
        }
    }
}

/// `[parallel]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ParallelConfig {
    /// Max parallel tool dispatches.
    pub max_parallel_tools: u32,
    /// Force serial for destructive commands.
    pub destructive_command_serial: bool,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            max_parallel_tools: defaults::DEFAULT_MAX_PARALLEL_TOOLS,
            destructive_command_serial: true,
        }
    }
}
