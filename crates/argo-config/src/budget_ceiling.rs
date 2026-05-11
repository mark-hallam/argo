//! `[budget.ceiling]` TOML section — hard per-session / per-day /
//! per-month USD ceilings consumed by
//! `argo-provider::spend_gate::SpendGate`. Operators write USD;
//! [`BudgetCeilingSection::to_micros`] converts to the workspace
//! canonical USD-micros and rejects negative / non-finite values.
//! Zero is permitted (explicit kill-switch); omit the line for
//! "uncapped". Pair with `argo-store::spend_ceiling::CeilingConfig`
//! at the use site (the wire-up brief
//! `mission-budget-gate-route-callers` does the conversion).
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};

const USD_TO_MICROS: f64 = 1_000_000.0;

/// Parsed `[budget.ceiling]` section — three optional USD caps.
///
/// ```
/// use argo_config::budget_ceiling::BudgetCeilingSection;
/// let toml = "per_session_usd = 5.0\nper_month_usd = 100.0\n";
/// let s: BudgetCeilingSection = toml::from_str(toml).unwrap();
/// assert_eq!(s.per_session_usd, Some(5.0));
/// assert_eq!(s.per_day_usd, None);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BudgetCeilingSection {
    /// Per-session cap in USD. Absent ⇒ uncapped.
    pub per_session_usd: Option<f64>,
    /// Per-day cap in USD (UTC day boundary). Absent ⇒ uncapped.
    pub per_day_usd: Option<f64>,
    /// Per-month cap in USD (UTC month boundary). Absent ⇒ uncapped.
    pub per_month_usd: Option<f64>,
}

/// Triple of `(session, day, month)` caps in USD micros — the
/// wire-format that `argo-store::spend_ceiling::CeilingConfig`
/// reads. `argo-config` stays free of an `argo-store` dep.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetCeilingMicros {
    /// Per-session cap in USD micros, or `None` when unset.
    pub per_session_usd_micros: Option<u64>,
    /// Per-day cap in USD micros, or `None` when unset.
    pub per_day_usd_micros: Option<u64>,
    /// Per-month cap in USD micros, or `None` when unset.
    pub per_month_usd_micros: Option<u64>,
}

impl BudgetCeilingSection {
    /// Validate the section and convert each set field to USD micros.
    ///
    /// # Errors
    /// [`ArgoError::Schema`] when any field is negative or non-finite.
    ///
    /// ```
    /// use argo_config::budget_ceiling::BudgetCeilingSection;
    /// let s = BudgetCeilingSection { per_session_usd: Some(5.0), ..Default::default() };
    /// assert_eq!(s.to_micros().unwrap().per_session_usd_micros, Some(5_000_000));
    /// ```
    pub fn to_micros(&self) -> ArgoResult<BudgetCeilingMicros> {
        Ok(BudgetCeilingMicros {
            per_session_usd_micros: usd_to_micros("per_session_usd", self.per_session_usd)?,
            per_day_usd_micros: usd_to_micros("per_day_usd", self.per_day_usd)?,
            per_month_usd_micros: usd_to_micros("per_month_usd", self.per_month_usd)?,
        })
    }
}

fn usd_to_micros(field: &str, v: Option<f64>) -> ArgoResult<Option<u64>> {
    let Some(usd) = v else {
        return Ok(None);
    };
    if !usd.is_finite() || usd < 0.0 {
        return Err(ArgoError::Schema(format!(
            "[budget.ceiling].{field} must be a finite non-negative \
             USD value (got {usd}); omit the line for 'uncapped'"
        )));
    }
    let micros = (usd * USD_TO_MICROS).round().clamp(0.0, u64::MAX as f64);
    Ok(Some(micros as u64))
}

#[cfg(test)]
#[path = "budget_ceiling_tests.rs"]
mod tests;
