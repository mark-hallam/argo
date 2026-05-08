//! Hard $$$ cost ceilings (item C of the v0.2 amendments).
//!
//! Per-session, per-day, per-month enforced caps from `[budget]` —
//! **block before spend, not after**. Notification thresholds
//! (default 80% / 95%) emit one event per crossing per period so
//! UIs can surface "approaching cap" warnings without resending the
//! same warning on every charge.
//!
//! This module is a pure primitive — it doesn't talk to a database
//! or a clock. Callers feed it the current spend totals (loaded from
//! a `SpendLedger`), the proposed charge, and `now`; it returns
//! either a list of newly-crossed thresholds (success path) or a
//! `CapBlocked` describing which ceiling was hit (error path).
use crate::ids::{SessionId, TenantId};
use serde::{Deserialize, Serialize};

/// Spend caps, in USD micros. `None` disables that scope.
///
/// ```
/// use argo_types::CostCeilings;
/// let ceilings = CostCeilings::defaults();
/// assert_eq!(ceilings.per_session_usd_micros, Some(1_000_000));
/// assert!(CostCeilings::unlimited().per_day_usd_micros.is_none());
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostCeilings {
    /// Per-session hard cap.
    pub per_session_usd_micros: Option<u64>,
    /// Per-day hard cap (UTC day boundaries).
    pub per_day_usd_micros: Option<u64>,
    /// Per-month hard cap (UTC ISO-month boundaries).
    pub per_month_usd_micros: Option<u64>,
    /// Notification thresholds, in percent. `[80, 95]` is the
    /// shipped default; `[]` disables notifications.
    pub notify_at_pct: Vec<u8>,
}

impl CostCeilings {
    /// Default config: $1/session, $10/day, $100/month, notify at
    /// 80% / 95%. Matches the spec's `[budget]` defaults.
    #[must_use]
    pub fn defaults() -> Self {
        Self {
            per_session_usd_micros: Some(1_000_000),
            per_day_usd_micros: Some(10_000_000),
            per_month_usd_micros: Some(100_000_000),
            notify_at_pct: vec![80, 95],
        }
    }

    /// All-disabled: never blocks, never notifies.
    #[must_use]
    pub fn unlimited() -> Self {
        Self::default()
    }
}

/// Current spend totals for a `(tenant, session)` pair.
///
/// ```
/// use argo_types::SpendCounters;
/// let counters = SpendCounters { session_used: 10, day_used: 20, month_used: 30 };
/// assert_eq!(serde_json::to_string(&counters).unwrap(), "{\"session_used\":10,\"day_used\":20,\"month_used\":30}");
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpendCounters {
    /// Spend on this session (USD micros).
    pub session_used: u64,
    /// Spend so far today for this tenant (USD micros).
    pub day_used: u64,
    /// Spend so far this month for this tenant (USD micros).
    pub month_used: u64,
}

/// Which scope a notification fired in.
///
/// ```
/// use argo_types::CapScope;
/// let json = serde_json::to_string(&CapScope::Month).unwrap();
/// assert_eq!(json, "\"month\"");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapScope {
    /// Per-session cap.
    Session,
    /// Per-day cap.
    Day,
    /// Per-month cap.
    Month,
}

/// One notification event emitted by [`enforce_pre_spend`].
///
/// ```
/// use argo_types::{CapScope, NotifyEvent};
/// let event = NotifyEvent {
///     scope: CapScope::Session, threshold_pct: 80,
///     new_used_usd_micros: 850_000, cap_usd_micros: 1_000_000,
/// };
/// assert_eq!(event.threshold_pct, 80);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotifyEvent {
    /// Scope.
    pub scope: CapScope,
    /// Threshold percent that was first crossed by this charge.
    pub threshold_pct: u8,
    /// New spend in this scope after the charge.
    pub new_used_usd_micros: u64,
    /// Cap value (USD micros).
    pub cap_usd_micros: u64,
}

/// Hard cap blocked the charge. Pre-spend, so no money was spent.
///
/// ```
/// use argo_types::{CapBlocked, CapScope, SessionId, TenantId};
/// let blocked = CapBlocked {
///     scope: CapScope::Day, cap_usd_micros: 10, attempted_usd_micros: 11,
///     tenant: TenantId::new(), session: SessionId::new(),
/// };
/// assert_eq!(blocked.scope, CapScope::Day);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, ::thiserror::Error)]
#[error("cost ceiling blocked: {scope:?} cap = {cap_usd_micros} usd-micros, attempted = {attempted_usd_micros}")]
pub struct CapBlocked {
    /// Which scope blocked.
    pub scope: CapScope,
    /// Cap value (USD micros).
    pub cap_usd_micros: u64,
    /// Spend that would have brought scope past the cap (USD micros).
    pub attempted_usd_micros: u64,
    /// Tenant scoped to.
    pub tenant: TenantId,
    /// Session scoped to (only meaningful for `Session`-scope blocks).
    pub session: SessionId,
}

/// Caller context: who is being charged, on which session.
///
/// ```
/// use argo_types::{EnforceContext, SessionId, TenantId};
/// let ctx = EnforceContext { tenant: TenantId::new(), session: SessionId::new() };
/// assert_ne!(ctx.tenant, TenantId::new());
/// ```
#[derive(Debug, Clone, Copy)]
pub struct EnforceContext {
    /// Tenant.
    pub tenant: TenantId,
    /// Session.
    pub session: SessionId,
}

/// Pre-spend cap enforcement.
///
/// Returns the notifications that would fire as a result of this
/// charge (one per `(scope, threshold)` first crossed). Returns
/// `Err(CapBlocked)` if any of the three caps would be exceeded —
/// in which case **no spend should be recorded** by the caller.
///
/// # Errors
/// Returns [`CapBlocked`] when any of `per_session_usd_micros`,
/// `per_day_usd_micros`, or `per_month_usd_micros` would be exceeded
/// after adding `attempted_usd_micros` to the matching counter.
///
/// ```
/// use argo_types::{enforce_pre_spend, CostCeilings, EnforceContext, SessionId, SpendCounters, TenantId};
/// let ctx = EnforceContext { tenant: TenantId::new(), session: SessionId::new() };
/// let events = enforce_pre_spend(&CostCeilings::defaults(), SpendCounters::default(), 850_000, ctx).unwrap();
/// assert_eq!(events[0].threshold_pct, 80);
/// ```
pub fn enforce_pre_spend(
    ceilings: &CostCeilings,
    counters: SpendCounters,
    attempted_usd_micros: u64,
    ctx: EnforceContext,
) -> Result<Vec<NotifyEvent>, CapBlocked> {
    // Block path runs first; spend == 0 means "no charge, but check
    // notifications" — used by callers that want to test the state.
    check_cap(
        ceilings.per_session_usd_micros,
        counters.session_used,
        attempted_usd_micros,
        CapScope::Session,
        ctx,
    )?;
    check_cap(
        ceilings.per_day_usd_micros,
        counters.day_used,
        attempted_usd_micros,
        CapScope::Day,
        ctx,
    )?;
    check_cap(
        ceilings.per_month_usd_micros,
        counters.month_used,
        attempted_usd_micros,
        CapScope::Month,
        ctx,
    )?;
    let mut events = Vec::new();
    push_events(
        &mut events,
        ceilings,
        ceilings.per_session_usd_micros,
        counters.session_used,
        attempted_usd_micros,
        CapScope::Session,
    );
    push_events(
        &mut events,
        ceilings,
        ceilings.per_day_usd_micros,
        counters.day_used,
        attempted_usd_micros,
        CapScope::Day,
    );
    push_events(
        &mut events,
        ceilings,
        ceilings.per_month_usd_micros,
        counters.month_used,
        attempted_usd_micros,
        CapScope::Month,
    );
    Ok(events)
}

fn check_cap(
    cap: Option<u64>,
    used: u64,
    attempted: u64,
    scope: CapScope,
    ctx: EnforceContext,
) -> Result<(), CapBlocked> {
    let Some(cap_v) = cap else {
        return Ok(());
    };
    let after = used.saturating_add(attempted);
    if after > cap_v {
        return Err(CapBlocked {
            scope,
            cap_usd_micros: cap_v,
            attempted_usd_micros: attempted,
            tenant: ctx.tenant,
            session: ctx.session,
        });
    }
    Ok(())
}

fn push_events(
    out: &mut Vec<NotifyEvent>,
    ceilings: &CostCeilings,
    cap: Option<u64>,
    used: u64,
    attempted: u64,
    scope: CapScope,
) {
    let Some(cap_v) = cap else {
        return;
    };
    if cap_v == 0 {
        return;
    }
    let after = used.saturating_add(attempted);
    for &pct in &ceilings.notify_at_pct {
        let bound = bound_at_pct(cap_v, pct);
        // First crossing only: was strictly below before, at or
        // above now.
        if used < bound && after >= bound {
            out.push(NotifyEvent {
                scope,
                threshold_pct: pct,
                new_used_usd_micros: after,
                cap_usd_micros: cap_v,
            });
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn bound_at_pct(cap_v: u64, pct: u8) -> u64 {
    // cap * pct / 100, saturating to avoid overflow on huge caps.
    let p = u128::from(pct);
    let c = u128::from(cap_v);
    let prod = c.saturating_mul(p) / 100;
    if prod > u128::from(u64::MAX) {
        u64::MAX
    } else {
        prod as u64
    }
}
