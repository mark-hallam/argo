//! Stream H-1 — Per-worker budget ledger.
//!
//! Tracks how much a worker has spent (in USD micros) over time
//! windows the auto-approval evaluator cares about: today and
//! this calendar month (UTC). Used to inform
//! [`crate::auto_approval::evaluate`] via
//! [`crate::auto_approval::SpendSnapshot`].
//!
//! v1 (this PR) ships the in-memory primitive + the serialised
//! row shape that backs the `worker_budget` table. Persistence
//! lands as part of H-2 (when the dispatch shim wires the gate +
//! starts charging). The module is split this way so tests for
//! the evaluator stay hermetic and don't need a database.

use crate::auto_approval::SpendSnapshot;
use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One charge entry — a single action's recorded spend.
/// Persisted in the `worker_budget` table (one row per charge).
/// The aggregator rolls these into daily / monthly totals at
/// snapshot time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BudgetCharge {
    /// Action that incurred the spend.
    pub action_id: String,
    /// USD micros charged.
    pub usd_micros: u64,
    /// When the charge fired (UTC).
    pub at: DateTime<Utc>,
}

/// In-memory budget ledger keyed by worker id (the
/// caller manages keying — the ledger itself is per-worker).
/// Use one [`WorkerBudget`] per `WorkerId`.
///
/// ```
/// use argo_identity::worker_budget::WorkerBudget;
/// use chrono::Utc;
/// let mut budget = WorkerBudget::new();
/// budget.charge("act-1", 500_000, Utc::now());
/// let snap = budget.snapshot(Utc::now());
/// assert_eq!(snap.today_usd_micros, 500_000);
/// ```
#[derive(Debug, Clone, Default)]
pub struct WorkerBudget {
    charges: Vec<BudgetCharge>,
}

impl WorkerBudget {
    /// Empty ledger.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Hydrate from persisted charges (e.g. on worker startup,
    /// load every row from the `worker_budget` table).
    #[must_use]
    pub fn from_charges(charges: Vec<BudgetCharge>) -> Self {
        Self { charges }
    }

    /// Record a charge. The caller is responsible for persisting
    /// the same row to the `worker_budget` table; this method
    /// updates the in-memory rollup so subsequent snapshots see
    /// the charge.
    pub fn charge(&mut self, action_id: &str, usd_micros: u64, at: DateTime<Utc>) {
        self.charges.push(BudgetCharge {
            action_id: action_id.to_string(),
            usd_micros,
            at,
        });
    }

    /// Build a [`SpendSnapshot`] for the auto-approval evaluator.
    /// Aggregates today's + this-month's spend (UTC calendar).
    #[must_use]
    pub fn snapshot(&self, now: DateTime<Utc>) -> SpendSnapshot {
        let today_key = (now.year(), now.month(), now.day());
        let month_key = (now.year(), now.month());
        let mut today = 0u64;
        let mut this_month = 0u64;
        for c in &self.charges {
            if (c.at.year(), c.at.month()) == month_key {
                this_month = this_month.saturating_add(c.usd_micros);
            }
            if (c.at.year(), c.at.month(), c.at.day()) == today_key {
                today = today.saturating_add(c.usd_micros);
            }
        }
        SpendSnapshot {
            today_usd_micros: today,
            this_month_usd_micros: this_month,
        }
    }

    /// Current charge count. Useful for tests + diagnostics.
    #[must_use]
    pub fn charge_count(&self) -> usize {
        self.charges.len()
    }

    /// Daily rollup map — `(yyyy, mm, dd) → total usd micros`.
    /// Used by `argo doctor` / observability surfaces; not on
    /// the dispatch hot path.
    #[must_use]
    pub fn daily_rollup(&self) -> BTreeMap<(i32, u32, u32), u64> {
        let mut out: BTreeMap<(i32, u32, u32), u64> = BTreeMap::new();
        for c in &self.charges {
            let key = (c.at.year(), c.at.month(), c.at.day());
            let entry = out.entry(key).or_insert(0);
            *entry = entry.saturating_add(c.usd_micros);
        }
        out
    }

    /// Monthly rollup map — `(yyyy, mm) → total usd micros`.
    #[must_use]
    pub fn monthly_rollup(&self) -> BTreeMap<(i32, u32), u64> {
        let mut out: BTreeMap<(i32, u32), u64> = BTreeMap::new();
        for c in &self.charges {
            let key = (c.at.year(), c.at.month());
            let entry = out.entry(key).or_insert(0);
            *entry = entry.saturating_add(c.usd_micros);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(year: i32, month: u32, day: u32, hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, hour, 0, 0).unwrap()
    }

    #[test]
    fn empty_ledger_returns_zero_snapshot() {
        let b = WorkerBudget::new();
        let snap = b.snapshot(at(2026, 5, 6, 12));
        assert_eq!(snap, SpendSnapshot::zero());
        assert_eq!(b.charge_count(), 0);
    }

    #[test]
    fn snapshot_aggregates_todays_charges_only() {
        let mut b = WorkerBudget::new();
        b.charge("act-1", 100_000, at(2026, 5, 6, 9));
        b.charge("act-2", 200_000, at(2026, 5, 6, 14));
        // Different day — counted in month total only.
        b.charge("act-3", 500_000, at(2026, 5, 5, 14));
        let snap = b.snapshot(at(2026, 5, 6, 18));
        assert_eq!(snap.today_usd_micros, 300_000);
        assert_eq!(snap.this_month_usd_micros, 800_000);
    }

    #[test]
    fn snapshot_isolates_months() {
        let mut b = WorkerBudget::new();
        b.charge("act-1", 100_000, at(2026, 4, 30, 23));
        b.charge("act-2", 200_000, at(2026, 5, 1, 1));
        let snap = b.snapshot(at(2026, 5, 1, 12));
        assert_eq!(snap.today_usd_micros, 200_000);
        // Only the May charge counts towards "this month".
        assert_eq!(snap.this_month_usd_micros, 200_000);
    }

    #[test]
    fn from_charges_round_trips_a_persisted_history() {
        let charges = vec![
            BudgetCharge {
                action_id: "act-x".into(),
                usd_micros: 50_000,
                at: at(2026, 5, 6, 10),
            },
            BudgetCharge {
                action_id: "act-y".into(),
                usd_micros: 75_000,
                at: at(2026, 5, 6, 11),
            },
        ];
        let b = WorkerBudget::from_charges(charges);
        assert_eq!(b.charge_count(), 2);
        let snap = b.snapshot(at(2026, 5, 6, 12));
        assert_eq!(snap.today_usd_micros, 125_000);
    }

    #[test]
    fn rollups_group_by_calendar_buckets() {
        let mut b = WorkerBudget::new();
        b.charge("act-1", 100_000, at(2026, 5, 1, 10));
        b.charge("act-2", 200_000, at(2026, 5, 1, 14));
        b.charge("act-3", 300_000, at(2026, 5, 2, 10));
        b.charge("act-4", 400_000, at(2026, 6, 1, 10));
        let daily = b.daily_rollup();
        assert_eq!(daily.get(&(2026, 5, 1)), Some(&300_000));
        assert_eq!(daily.get(&(2026, 5, 2)), Some(&300_000));
        assert_eq!(daily.get(&(2026, 6, 1)), Some(&400_000));
        let monthly = b.monthly_rollup();
        assert_eq!(monthly.get(&(2026, 5)), Some(&600_000));
        assert_eq!(monthly.get(&(2026, 6)), Some(&400_000));
    }

    #[test]
    fn budget_charge_serde_round_trips() {
        let c = BudgetCharge {
            action_id: "act-7".into(),
            usd_micros: 12_345,
            at: at(2026, 5, 6, 12),
        };
        let json = serde_json::to_string(&c).unwrap();
        let parsed: BudgetCharge = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, c);
    }
}
