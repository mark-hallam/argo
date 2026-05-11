//! `[store]` section — knobs for the SQLite-backed `SessionStore`.
//!
//! Today the only sub-block is `[store.wal_tuning]`, which selects
//! a [`WalTuningProfile`](argo_store::wal_tuning::WalTuningProfile)
//! applied at pool open. A missing `[store]` block (or a missing
//! `[store.wal_tuning]` sub-block) resolves to the `Default` profile
//! so a stripped `argo.toml` still ships with the opinionated
//! pragmas (Stream `mission-sqlite-wal-tuning`).
use serde::{Deserialize, Serialize};

/// `[store]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StoreConfig {
    /// `[store.wal_tuning]`.
    pub wal_tuning: WalTuningSection,
}

/// `[store.wal_tuning]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WalTuningSection {
    /// Profile name — `conservative`, `default`, or `aggressive`.
    /// Missing → `default`. Parsed via
    /// [`argo_store::wal_tuning::WalTuningProfile::from_str`].
    pub profile: String,
}

impl Default for WalTuningSection {
    fn default() -> Self {
        Self {
            profile: "default".into(),
        }
    }
}
