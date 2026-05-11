//! Opinionated SQLite WAL tuning for `SessionStore`.
//!
//! Argo's session store sees a write-heavy, bursty workload (audit
//! rows + session inserts) interleaved with occasional long reads.
//! The shipped defaults below are tuned for that pattern; they are
//! starting points and may be re-evaluated against production
//! telemetry.
//!
//! # Profiles
//!
//! - [`WalTuningProfile::Conservative`] — apply nothing beyond
//!   sqlx's default `journal_mode = WAL`. Matches operators on
//!   flaky hardware who want SQLite's `synchronous = FULL` safety.
//! - [`WalTuningProfile::Default`] — the seven-pragma set below.
//!   Safe with WAL on commodity hardware.
//! - [`WalTuningProfile::Aggressive`] — `Default` plus
//!   `synchronous = OFF` and a 10× larger autocheckpoint window.
//!   **Dev-only.** Emits `tracing::warn!` on open.
//!
//! # Pragma rationale (Default profile)
//!
//! | Pragma | Value | Why |
//! |---|---|---|
//! | `journal_mode` | `WAL` | Concurrent readers + a single writer. |
//! | `synchronous` | `NORMAL` | Safe with WAL; halves fsyncs per commit. |
//! | `wal_autocheckpoint` | `1000` (pages, ~4 MB @ 4 KiB page) | Caps WAL growth in bursty writes. |
//! | `cache_size` | `-64000` (negative = KiB) | 64 MiB page cache. |
//! | `mmap_size` | `268435456` (256 MiB) | Read-side mmap window; capped silently on 32-bit hosts. |
//! | `temp_store` | `MEMORY` | Sort/aggregate scratch stays off disk. |
//! | `busy_timeout` | `5000` ms | Multi-worker contention floor. |
//!
//! For `:memory:` pools, SQLite ignores `journal_mode = WAL` and
//! reports `memory` back; the other pragmas apply unchanged. The
//! `apply_pragmas` call is safe — and a no-op for the WAL pragma —
//! against memory DBs, so tests can exercise the same code path.
use argo_types::ArgoError;
use sqlx::{Pool, Row, Sqlite};
use std::str::FromStr;

/// Tuning profile chosen at [`SessionStore::open_with_profile`]
/// time. Defaults to [`WalTuningProfile::Default`].
///
/// [`SessionStore::open_with_profile`]: crate::SessionStore::open_with_profile
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WalTuningProfile {
    /// SQLite defaults (apart from sqlx's existing `journal_mode = WAL`).
    Conservative,
    /// Seven opinionated pragmas tuned for the v1 workload.
    #[default]
    Default,
    /// `Default` + `synchronous = OFF` + `wal_autocheckpoint = 10000`.
    /// **Dev-only.** Logs a `WARN` on open.
    Aggressive,
}

impl FromStr for WalTuningProfile {
    type Err = ArgoError;

    /// Parse a profile name from `argo.toml`. Empty string maps to
    /// [`WalTuningProfile::Default`] so a missing config block
    /// preserves the v1 default.
    ///
    /// ```
    /// use argo_store::wal_tuning::WalTuningProfile;
    /// use std::str::FromStr;
    /// assert_eq!(WalTuningProfile::from_str("default").unwrap(), WalTuningProfile::Default);
    /// assert_eq!(WalTuningProfile::from_str("").unwrap(), WalTuningProfile::Default);
    /// assert!(WalTuningProfile::from_str("bogus").is_err());
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "conservative" => Ok(Self::Conservative),
            "" | "default" => Ok(Self::Default),
            "aggressive" => Ok(Self::Aggressive),
            other => Err(ArgoError::Provider(format!(
                "unknown wal_tuning profile '{other}' (expected conservative|default|aggressive)"
            ))),
        }
    }
}

/// The seven pragmas that the [`WalTuningProfile::Default`] profile
/// applies, in the order they are issued.
///
/// Exposed for tests so the round-trip assertions can iterate the
/// same list the production path applies.
#[must_use]
pub fn default_pragmas() -> [(&'static str, &'static str); 7] {
    [
        ("journal_mode", "WAL"),
        ("synchronous", "NORMAL"),
        ("wal_autocheckpoint", "1000"),
        ("cache_size", "-64000"),
        ("mmap_size", "268435456"),
        ("temp_store", "MEMORY"),
        ("busy_timeout", "5000"),
    ]
}

/// Apply the profile's pragma set to an already-connected pool.
/// Called from [`SessionStore::open_with_profile`].
///
/// [`SessionStore::open_with_profile`]: crate::SessionStore::open_with_profile
///
/// # Errors
///
/// Returns [`ArgoError::Provider`] if any pragma statement fails to
/// execute on the pool.
pub async fn apply_pragmas(
    pool: &Pool<Sqlite>,
    profile: WalTuningProfile,
) -> Result<(), ArgoError> {
    if profile == WalTuningProfile::Conservative {
        return Ok(());
    }

    let synchronous = if profile == WalTuningProfile::Aggressive {
        "OFF"
    } else {
        "NORMAL"
    };
    let autocheckpoint = if profile == WalTuningProfile::Aggressive {
        "10000"
    } else {
        "1000"
    };

    let pragmas: [(&str, &str); 7] = [
        ("journal_mode", "WAL"),
        ("synchronous", synchronous),
        ("wal_autocheckpoint", autocheckpoint),
        ("cache_size", "-64000"),
        ("mmap_size", "268435456"),
        ("temp_store", "MEMORY"),
        ("busy_timeout", "5000"),
    ];

    for (name, value) in pragmas {
        let sql = format!("PRAGMA {name} = {value}");
        sqlx::query(&sql)
            .execute(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("pragma {name}: {e}")))?;
    }

    if profile == WalTuningProfile::Aggressive {
        tracing::warn!(
            "wal_tuning profile=Aggressive applied: synchronous=OFF risks data loss on \
             crash, dev-only — switch to Default for production"
        );
    }

    Ok(())
}

/// Read the current value of `PRAGMA <name>` against `pool`.
/// Returns the value as a string so callers can render any pragma's
/// readout uniformly (`journal_mode` → `wal`, `cache_size` → `-64000`).
///
/// # Errors
///
/// Returns [`ArgoError::Provider`] if the pragma query fails or the
/// result row cannot be decoded.
pub async fn read_pragma(pool: &Pool<Sqlite>, name: &str) -> Result<String, ArgoError> {
    let sql = format!("PRAGMA {name}");
    let row = sqlx::query(&sql)
        .fetch_optional(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("read pragma {name}: {e}")))?;
    let Some(row) = row else {
        return Ok(String::new());
    };
    if let Ok(s) = row.try_get::<String, _>(0) {
        return Ok(s);
    }
    if let Ok(n) = row.try_get::<i64, _>(0) {
        return Ok(n.to_string());
    }
    Ok(String::new())
}

#[cfg(test)]
#[path = "wal_tuning_tests.rs"]
mod tests;
