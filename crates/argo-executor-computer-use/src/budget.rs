//! Per-action budget tracking. Three caps run concurrently:
//! tokens, wallclock, and step count. Whichever hits first
//! returns [`BudgetVerdict::Exhausted`] from
//! [`BudgetTracker::charge_step`], and the executor turns that
//! into a `HumanHandoff` outcome with the budget reason.
//!
//! The token cap also honours the per-action `budget_usd` that
//! the manifest's `ExecutorTagged.budget_usd` field carries —
//! the tracker converts USD to a token-equivalent using the
//! profile's pinned model price (see
//! [`BudgetTracker::with_usd_cap`]).

use crate::profile::Profile;
use std::time::Instant;

/// Outcome of a budget check after one CU step.
#[derive(Debug, Clone, PartialEq)]
pub enum BudgetVerdict {
    /// Step is within budget. Carry on.
    Ok,
    /// At least one cap is exhausted. The executor turns this
    /// into a `HumanHandoff` outcome. Reason is the operator-
    /// facing message and includes which cap fired.
    Exhausted { reason: String },
}

/// Tracker for token + wallclock + step caps. Built once per
/// dispatch; charged once per CU step. Pure data — no async, no
/// I/O.
#[derive(Debug)]
pub struct BudgetTracker {
    max_tokens: u32,
    max_wallclock_seconds: u32,
    max_steps: u32,
    /// Optional per-action USD ceiling, converted into a token
    /// budget via [`Profile::tokens_per_dollar`] at construction
    /// time. `None` means the profile's `max_tokens_per_action`
    /// is the only token cap.
    usd_token_cap: Option<u32>,
    used_tokens: u32,
    used_steps: u32,
    started_at: Instant,
}

impl BudgetTracker {
    /// Build from a profile. The token cap is the profile's
    /// `max_tokens_per_action`; wallclock + step caps come from
    /// the same row.
    #[must_use]
    pub fn new(profile: &Profile) -> Self {
        Self {
            max_tokens: profile.max_tokens_per_action,
            max_wallclock_seconds: profile.max_wallclock_seconds,
            max_steps: profile.max_steps,
            usd_token_cap: None,
            used_tokens: 0,
            used_steps: 0,
            started_at: Instant::now(),
        }
    }

    /// Layer a per-action USD cap on top of the profile cap. The
    /// effective token budget becomes
    /// `min(profile_tokens, usd_to_tokens(usd))`. Caller passes
    /// in the USD figure from `ExecutorDeclaration::budget_usd`.
    #[must_use]
    pub fn with_usd_cap(mut self, usd: f64, tokens_per_dollar: u32) -> Self {
        // Defensive cast — we floor + clamp to u32.
        let cap = (usd * f64::from(tokens_per_dollar)).max(0.0) as u32;
        self.usd_token_cap = Some(cap);
        self
    }

    /// Effective per-action token cap, honouring both the
    /// profile and the optional USD cap.
    #[must_use]
    pub fn effective_token_cap(&self) -> u32 {
        match self.usd_token_cap {
            Some(usd_cap) => self.max_tokens.min(usd_cap),
            None => self.max_tokens,
        }
    }

    /// Current step count (1-indexed at the start of step N).
    #[must_use]
    pub fn step(&self) -> u32 {
        self.used_steps
    }

    /// Cumulative tokens charged so far.
    #[must_use]
    pub fn tokens_used(&self) -> u32 {
        self.used_tokens
    }

    /// Charge a CU step. Increments step + token counters and
    /// returns the verdict.
    pub fn charge_step(&mut self, tokens_used_this_step: u32) -> BudgetVerdict {
        self.used_steps = self.used_steps.saturating_add(1);
        self.used_tokens = self
            .used_tokens
            .saturating_add(tokens_used_this_step);
        if self.used_steps > self.max_steps {
            return BudgetVerdict::Exhausted {
                reason: format!(
                    "exceeded max_steps={} (step {})",
                    self.max_steps, self.used_steps
                ),
            };
        }
        let token_cap = self.effective_token_cap();
        if self.used_tokens > token_cap {
            return BudgetVerdict::Exhausted {
                reason: format!(
                    "exceeded token cap = {token_cap} (used {} tokens)",
                    self.used_tokens
                ),
            };
        }
        let elapsed = self.started_at.elapsed().as_secs() as u32;
        if elapsed > self.max_wallclock_seconds {
            return BudgetVerdict::Exhausted {
                reason: format!(
                    "exceeded wallclock cap = {}s (elapsed {elapsed}s)",
                    self.max_wallclock_seconds
                ),
            };
        }
        BudgetVerdict::Ok
    }

    /// Snapshot of budget state for inclusion in the audit log
    /// + result_json.
    #[must_use]
    pub fn snapshot(&self) -> BudgetSnapshot {
        BudgetSnapshot {
            tokens_used: self.used_tokens,
            tokens_cap: self.effective_token_cap(),
            steps_used: self.used_steps,
            steps_cap: self.max_steps,
            wallclock_seconds_used: self.started_at.elapsed().as_secs() as u32,
            wallclock_seconds_cap: self.max_wallclock_seconds,
        }
    }
}

/// Read-only view of the tracker. Safe to embed in the audit
/// log + result_json; serialisation order is stable.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BudgetSnapshot {
    pub tokens_used: u32,
    pub tokens_cap: u32,
    pub steps_used: u32,
    pub steps_cap: u32,
    pub wallclock_seconds_used: u32,
    pub wallclock_seconds_cap: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_profile() -> Profile {
        Profile {
            max_tokens_per_action: 100,
            max_wallclock_seconds: 60,
            max_steps: 3,
            ..Profile::default()
        }
    }

    #[test]
    fn charge_step_ok_within_caps() {
        let mut t = BudgetTracker::new(&small_profile());
        let v = t.charge_step(10);
        assert_eq!(v, BudgetVerdict::Ok);
        assert_eq!(t.step(), 1);
        assert_eq!(t.tokens_used(), 10);
    }

    #[test]
    fn charge_step_exhausts_on_token_overflow() {
        let mut t = BudgetTracker::new(&small_profile());
        let v = t.charge_step(101);
        match v {
            BudgetVerdict::Exhausted { reason } => assert!(reason.contains("token")),
            BudgetVerdict::Ok => panic!("should have exhausted"),
        }
    }

    #[test]
    fn charge_step_exhausts_on_step_overflow() {
        let mut t = BudgetTracker::new(&small_profile());
        // 3 steps allowed; the 4th charge fires the cap.
        let _ = t.charge_step(1);
        let _ = t.charge_step(1);
        let _ = t.charge_step(1);
        let v = t.charge_step(1);
        match v {
            BudgetVerdict::Exhausted { reason } => assert!(reason.contains("max_steps")),
            BudgetVerdict::Ok => panic!("should have exhausted"),
        }
    }

    #[test]
    fn usd_cap_clamps_token_budget_below_profile() {
        // Profile token cap = 100; USD cap = 0.10 * 1000 = 100
        // Equal — no clamp.
        let t = BudgetTracker::new(&small_profile()).with_usd_cap(0.10, 1000);
        assert_eq!(t.effective_token_cap(), 100);

        // USD cap = 0.05 * 1000 = 50, clamps below profile.
        let t = BudgetTracker::new(&small_profile()).with_usd_cap(0.05, 1000);
        assert_eq!(t.effective_token_cap(), 50);

        // USD cap larger than profile — profile wins.
        let t = BudgetTracker::new(&small_profile()).with_usd_cap(1.0, 1000);
        assert_eq!(t.effective_token_cap(), 100);
    }

    #[test]
    fn snapshot_reports_caps_and_used() {
        let mut t = BudgetTracker::new(&small_profile());
        let _ = t.charge_step(20);
        let snap = t.snapshot();
        assert_eq!(snap.tokens_used, 20);
        assert_eq!(snap.tokens_cap, 100);
        assert_eq!(snap.steps_used, 1);
        assert_eq!(snap.steps_cap, 3);
        assert!(snap.wallclock_seconds_cap > 0);
    }
}
