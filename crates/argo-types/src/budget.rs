//! Budgets and the budget tree.
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

/// What is being charged against a budget.
///
/// ```
/// use argo_types::BudgetKind;
/// let json = serde_json::to_string(&BudgetKind::TokensIn).unwrap();
/// assert_eq!(json, "\"tokens_in\"");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetKind {
    /// Tool-calling iterations.
    Iterations,
    /// Input tokens.
    TokensIn,
    /// Output tokens.
    TokensOut,
    /// Wallclock milliseconds.
    WallclockMs,
    /// Outbound network bytes.
    NetEgressBytes,
    /// USD spent (in micros: 1 USD = 1_000_000).
    SpendUsdMicros,
    /// Tool calls.
    ToolCalls,
}

/// Declarable budget caps.
///
/// ```
/// use argo_types::Budget;
/// let budget = Budget { iterations: Some(3), spend_usd_micros: Some(1_000_000), ..Default::default() };
/// let json = serde_json::to_string(&budget).unwrap();
/// assert_eq!(serde_json::from_str::<Budget>(&json).unwrap().iterations, Some(3));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Budget {
    /// Iteration cap.
    pub iterations: Option<u32>,
    /// Input-token cap.
    pub tokens_in: Option<u64>,
    /// Output-token cap.
    pub tokens_out: Option<u64>,
    /// Wallclock cap (ms).
    pub wallclock_ms: Option<u64>,
    /// Network-egress cap (bytes).
    pub net_egress_bytes: Option<u64>,
    /// Spend cap (USD micros).
    pub spend_usd_micros: Option<u64>,
    /// Tool-call cap.
    pub tool_calls: Option<u32>,
    /// Hard cap per session (USD micros).
    pub spend_per_session_usd_micros: Option<u64>,
    /// Hard cap per day (USD micros).
    pub spend_per_day_usd_micros: Option<u64>,
    /// Hard cap per month (USD micros).
    pub spend_per_month_usd_micros: Option<u64>,
}

/// A node in the budget tree. Atomically charged; cascades to root.
///
/// ```
/// use argo_types::{Budget, BudgetKind, BudgetNode};
/// let node = BudgetNode::root(&Budget { iterations: Some(1), ..Default::default() });
/// assert!(node.charge(BudgetKind::Iterations, 1).is_ok());
/// assert!(node.charge(BudgetKind::Iterations, 1).is_err());
/// ```
pub struct BudgetNode {
    parent: Option<Arc<BudgetNode>>,
    iterations: AtomicU32,
    iter_max: u32,
    tokens_in: AtomicU64,
    tokens_in_max: u64,
    tokens_out: AtomicU64,
    tokens_out_max: u64,
    spend: AtomicU64,
    spend_max: u64,
    tool_calls: AtomicU32,
    tool_calls_max: u32,
}

impl BudgetNode {
    /// Build a root node from a `Budget`.
    #[must_use]
    pub fn root(b: &Budget) -> Arc<Self> {
        Arc::new(Self {
            parent: None,
            iterations: AtomicU32::new(0),
            iter_max: b.iterations.unwrap_or(u32::MAX),
            tokens_in: AtomicU64::new(0),
            tokens_in_max: b.tokens_in.unwrap_or(u64::MAX),
            tokens_out: AtomicU64::new(0),
            tokens_out_max: b.tokens_out.unwrap_or(u64::MAX),
            spend: AtomicU64::new(0),
            spend_max: b.spend_usd_micros.unwrap_or(u64::MAX),
            tool_calls: AtomicU32::new(0),
            tool_calls_max: b.tool_calls.unwrap_or(u32::MAX),
        })
    }

    /// Carve out a child node from `self`. Charges propagate upward.
    #[must_use]
    pub fn fork(self: &Arc<Self>, b: &Budget) -> Arc<Self> {
        Arc::new(Self {
            parent: Some(Arc::clone(self)),
            iterations: AtomicU32::new(0),
            iter_max: b.iterations.unwrap_or(u32::MAX),
            tokens_in: AtomicU64::new(0),
            tokens_in_max: b.tokens_in.unwrap_or(u64::MAX),
            tokens_out: AtomicU64::new(0),
            tokens_out_max: b.tokens_out.unwrap_or(u64::MAX),
            spend: AtomicU64::new(0),
            spend_max: b.spend_usd_micros.unwrap_or(u64::MAX),
            tool_calls: AtomicU32::new(0),
            tool_calls_max: b.tool_calls.unwrap_or(u32::MAX),
        })
    }

    /// Charge `amount` against `kind`. Returns `Err` if exhausted at any level.
    pub fn charge(&self, kind: BudgetKind, amount: u64) -> Result<(), BudgetExhausted> {
        match kind {
            BudgetKind::Iterations => {
                let n = u32::try_from(amount).unwrap_or(u32::MAX);
                let new = self
                    .iterations
                    .fetch_add(n, Ordering::SeqCst)
                    .saturating_add(n);
                if new > self.iter_max {
                    self.iterations.fetch_sub(n, Ordering::SeqCst);
                    return Err(BudgetExhausted(BudgetKind::Iterations));
                }
            }
            BudgetKind::TokensIn => {
                let new = self
                    .tokens_in
                    .fetch_add(amount, Ordering::SeqCst)
                    .saturating_add(amount);
                if new > self.tokens_in_max {
                    self.tokens_in.fetch_sub(amount, Ordering::SeqCst);
                    return Err(BudgetExhausted(BudgetKind::TokensIn));
                }
            }
            BudgetKind::TokensOut => {
                let new = self
                    .tokens_out
                    .fetch_add(amount, Ordering::SeqCst)
                    .saturating_add(amount);
                if new > self.tokens_out_max {
                    self.tokens_out.fetch_sub(amount, Ordering::SeqCst);
                    return Err(BudgetExhausted(BudgetKind::TokensOut));
                }
            }
            BudgetKind::SpendUsdMicros => {
                let new = self
                    .spend
                    .fetch_add(amount, Ordering::SeqCst)
                    .saturating_add(amount);
                if new > self.spend_max {
                    self.spend.fetch_sub(amount, Ordering::SeqCst);
                    return Err(BudgetExhausted(BudgetKind::SpendUsdMicros));
                }
            }
            BudgetKind::ToolCalls => {
                let n = u32::try_from(amount).unwrap_or(u32::MAX);
                let new = self
                    .tool_calls
                    .fetch_add(n, Ordering::SeqCst)
                    .saturating_add(n);
                if new > self.tool_calls_max {
                    self.tool_calls.fetch_sub(n, Ordering::SeqCst);
                    return Err(BudgetExhausted(BudgetKind::ToolCalls));
                }
            }
            BudgetKind::WallclockMs | BudgetKind::NetEgressBytes => {
                // Tracked elsewhere; no-op at this level.
            }
        }
        if let Some(parent) = &self.parent {
            parent.charge(kind, amount)?;
        }
        Ok(())
    }

    /// Iterations used at this node.
    #[must_use]
    pub fn iterations_used(&self) -> u32 {
        self.iterations.load(Ordering::SeqCst)
    }

    /// Spend used at this node (USD micros).
    #[must_use]
    pub fn spend_used(&self) -> u64 {
        self.spend.load(Ordering::SeqCst)
    }
}

/// Budget exhausted at some level of the tree.
///
/// ```
/// use argo_types::{BudgetExhausted, BudgetKind};
/// let err = BudgetExhausted(BudgetKind::ToolCalls);
/// assert!(err.to_string().contains("ToolCalls"));
/// ```
#[derive(Debug, Clone, thiserror::Error)]
#[error("budget exhausted: {0:?}")]
pub struct BudgetExhausted(pub BudgetKind);
