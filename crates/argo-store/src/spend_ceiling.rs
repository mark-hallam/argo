//! Per-tenant hard `[budget.ceiling]` enforcement — read side.
//!
//! Sits on top of `spend_ledger` (item C — see `spend.rs`) and
//! provides the **read** primitive consumed by the
//! `argo-provider::spend_gate::SpendGate` enforcement-time wrapper.
//!
//! The write side (`SessionStore::record_spend`) is unchanged: this
//! module only reads existing rows. There is no migration; the
//! `spend_ledger` table is already populated by every provider
//! callsite that records a charge.
//!
//! # Windows
//!
//! - `session` — lifetime of the [`SessionId`]; no calendar
//!   boundary, only `(tenant_id, session_id)` filtering.
//! - `day` — UTC calendar day, `[day_start_utc, now)`.
//! - `month` — UTC calendar month, `[month_start_utc, now)`.
//!
//! Document the UTC choice loudly so operators in non-UTC time zones
//! aren't surprised by a "ceiling reset" at 17:00 local time.
//!
//! # Projected-cost contract
//!
//! Enforcement compares `projected_cost_micros` (typically `4¢` —
//! see [`FALLBACK_PROJECTED_COST_MICROS`]) against `remaining` on
//! each window. The contract:
//!
//! - `projected == remaining` is **allowed** (boundary inclusive).
//! - `projected > remaining` is **rejected** (boundary exclusive).
//! - `remaining = None` (unset ceiling) is always **allowed**.
//!
//! This matches the brief's acceptance contract under
//! `mission-llm-budget-hard-ceiling`.
use crate::SessionStore;
use argo_types::{
    ArgoError, ArgoResult, BudgetWindow, SessionId, SpendCounters, TenantId,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Conservative ceiling-side projected-cost fallback (USD micros).
///
/// `4¢` per request — used by callers (the `SpendGate` wrapper) when
/// the request doesn't carry a per-call cost estimate. Deliberately
/// pessimistic so the ceiling fires *before* a runaway request is
/// dispatched, not after.
pub const FALLBACK_PROJECTED_COST_MICROS: u64 = 40_000;

/// Per-window hard ceilings in USD micros. `None` ⇒ that window is
/// **uncapped** and never blocks.
///
/// Mirrors the shape of `[budget.ceiling]` in `argo.toml`:
///
/// ```toml
/// [budget.ceiling]
/// per_session_usd = 5.00
/// per_day_usd     = 100.00
/// per_month_usd   = 2000.00
/// ```
///
/// Use [`CeilingConfig::uncapped`] for the test/dev default that
/// never blocks.
///
/// ```
/// use argo_store::spend_ceiling::CeilingConfig;
/// let cfg = CeilingConfig {
///     per_session_usd_micros: Some(5_000_000),
///     per_day_usd_micros:     Some(100_000_000),
///     per_month_usd_micros:   None,
/// };
/// assert!(cfg.per_month_usd_micros.is_none());
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CeilingConfig {
    /// Session-lifetime cap. `None` ⇒ uncapped.
    pub per_session_usd_micros: Option<u64>,
    /// UTC-day cap. `None` ⇒ uncapped.
    pub per_day_usd_micros: Option<u64>,
    /// UTC-month cap. `None` ⇒ uncapped.
    pub per_month_usd_micros: Option<u64>,
}

impl CeilingConfig {
    /// Config where every window is uncapped. Used by tests and by
    /// production callers that haven't configured `[budget.ceiling]`.
    ///
    /// ```
    /// use argo_store::spend_ceiling::CeilingConfig;
    /// let cfg = CeilingConfig::uncapped();
    /// assert!(cfg.per_session_usd_micros.is_none());
    /// ```
    #[must_use]
    pub fn uncapped() -> Self {
        Self::default()
    }

    /// `true` when every window is `None` — the gate is a no-op.
    #[must_use]
    pub fn all_uncapped(&self) -> bool {
        self.per_session_usd_micros.is_none()
            && self.per_day_usd_micros.is_none()
            && self.per_month_usd_micros.is_none()
    }
}

/// A snapshot of the per-`(tenant, session)` spend windows alongside
/// the remaining headroom under each configured ceiling.
///
/// `remaining` is `None` whenever the corresponding ceiling is unset.
/// When the ceiling is set, `remaining = max(0, ceiling - used)` —
/// always non-negative.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CeilingCheck {
    /// Spend on this session so far (USD micros).
    pub session_used_micros: u64,
    /// Spend today (UTC) for this tenant (USD micros).
    pub day_used_micros: u64,
    /// Spend this month (UTC) for this tenant (USD micros).
    pub month_used_micros: u64,
    /// Headroom under the session ceiling, or `None` if uncapped.
    pub session_remaining: Option<u64>,
    /// Headroom under the day ceiling, or `None` if uncapped.
    pub day_remaining: Option<u64>,
    /// Headroom under the month ceiling, or `None` if uncapped.
    pub month_remaining: Option<u64>,
}

impl CeilingCheck {
    /// Project a charge of `projected_cost_micros` against this
    /// snapshot. Returns `Ok(())` when every set window has
    /// `remaining >= projected`. Returns
    /// `Err(ArgoError::BudgetExceeded)` for the **first** window
    /// that would be exceeded (Session → Day → Month order).
    ///
    /// The boundary is **inclusive**: a projected charge exactly
    /// equal to remaining is allowed; exceeding by one micro is
    /// rejected.
    ///
    /// # Errors
    /// [`ArgoError::BudgetExceeded`] with the first tripped window.
    ///
    /// ```
    /// use argo_store::spend_ceiling::CeilingCheck;
    /// let check = CeilingCheck {
    ///     session_used_micros: 0,
    ///     day_used_micros: 0,
    ///     month_used_micros: 0,
    ///     session_remaining: Some(100),
    ///     day_remaining: Some(100),
    ///     month_remaining: Some(100),
    /// };
    /// assert!(check.project(100).is_ok());   // boundary inclusive
    /// assert!(check.project(101).is_err());  // boundary exclusive
    /// ```
    pub fn project(&self, projected_cost_micros: u64) -> ArgoResult<()> {
        project_window(
            BudgetWindow::Session,
            self.session_used_micros,
            self.session_remaining,
            projected_cost_micros,
        )?;
        project_window(
            BudgetWindow::Day,
            self.day_used_micros,
            self.day_remaining,
            projected_cost_micros,
        )?;
        project_window(
            BudgetWindow::Month,
            self.month_used_micros,
            self.month_remaining,
            projected_cost_micros,
        )?;
        Ok(())
    }
}

fn project_window(
    window: BudgetWindow,
    used_micros: u64,
    remaining: Option<u64>,
    projected_cost_micros: u64,
) -> ArgoResult<()> {
    // No remaining = window is uncapped = always allowed.
    let Some(rem) = remaining else {
        return Ok(());
    };
    if projected_cost_micros > rem {
        // `ceiling = used + remaining` reconstructs the configured
        // cap without rounding — `remaining` was clamped non-negative
        // at construction, so the sum is unambiguous.
        return Err(ArgoError::BudgetExceeded {
            window,
            used_micros,
            ceiling_micros: used_micros.saturating_add(rem),
            projected_cost_micros,
        });
    }
    Ok(())
}

impl SessionStore {
    /// Read the current `(session, day, month)` spend totals for a
    /// `(tenant, session)` pair and combine them with `config` into a
    /// [`CeilingCheck`] that callers can project a proposed charge
    /// against.
    ///
    /// `now` is the wallclock used to define the day/month window
    /// boundaries; tests pass a fixed `DateTime<Utc>` so window math
    /// is deterministic.
    ///
    /// # Errors
    /// Surfaces any [`ArgoError`] returned by the underlying spend
    /// query (`load_spend_counters`).
    ///
    /// ```no_run
    /// # async fn doc() -> argo_types::ArgoResult<()> {
    /// use argo_store::{spend_ceiling::CeilingConfig, SessionStore};
    /// use argo_types::{SessionId, TenantId};
    /// let store = SessionStore::open(":memory:").await?;
    /// store.migrate().await?;
    /// let check = store
    ///     .check_ceiling(
    ///         TenantId::new(),
    ///         SessionId::new(),
    ///         &CeilingConfig::uncapped(),
    ///         chrono::Utc::now(),
    ///     )
    ///     .await?;
    /// assert_eq!(check.session_used_micros, 0);
    /// # Ok(()) }
    /// ```
    pub async fn check_ceiling(
        &self,
        tenant_id: TenantId,
        session_id: SessionId,
        config: &CeilingConfig,
        now: DateTime<Utc>,
    ) -> ArgoResult<CeilingCheck> {
        let counters = self.load_spend_counters(tenant_id, session_id, now).await?;
        Ok(check_from_counters(counters, config))
    }
}

/// Build a [`CeilingCheck`] from raw counters + a ceiling config.
/// Pure function; exposed for tests that want to skip the store
/// round-trip.
#[must_use]
pub fn check_from_counters(
    counters: SpendCounters,
    config: &CeilingConfig,
) -> CeilingCheck {
    CeilingCheck {
        session_used_micros: counters.session_used,
        day_used_micros: counters.day_used,
        month_used_micros: counters.month_used,
        session_remaining: remaining_micros(
            counters.session_used,
            config.per_session_usd_micros,
        ),
        day_remaining: remaining_micros(counters.day_used, config.per_day_usd_micros),
        month_remaining: remaining_micros(
            counters.month_used,
            config.per_month_usd_micros,
        ),
    }
}

fn remaining_micros(used: u64, ceiling: Option<u64>) -> Option<u64> {
    ceiling.map(|c| c.saturating_sub(used))
}

#[cfg(test)]
#[path = "spend_ceiling_tests.rs"]
mod tests;
