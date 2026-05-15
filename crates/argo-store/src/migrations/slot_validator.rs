//! Slot-collision guardrails for the versioned migration runner.
//!
//! The production runner uses the Rust `Migration { version, up, down }`
//! registry, not filesystem ordering. Historical raw SQL files still
//! carry duplicate numeric prefixes, so Path A preserves those filenames
//! and makes the versioned registry the canonical order.

#[cfg(test)]
use std::collections::BTreeMap;
use std::collections::BTreeSet;

use argo_types::ArgoError;

#[cfg(test)]
#[path = "slot_validator_tests.rs"]
mod slot_validator_tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HistoricalDuplicateSlot {
    slot: u32,
    files: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg(test)]
struct SlotCollision {
    slot: u32,
    files: Vec<String>,
}

const CANONICAL_RAW_SQL_ORDER: &[(&str, u32)] = &[
    ("19__anomaly.sql", 19),
    ("19__worker_fs_root.sql", 20),
    ("22__roster.sql", 23),
    ("22__tenant_archived.sql", 24),
    ("23__handoff_routing.sql", 25),
    ("23__workflow_checkpoints.sql", 26),
    ("24__operator_feedback.sql", 27),
    ("24__skill_quality_score.sql", 28),
    ("24__vertical_archived.sql", 29),
    ("26__data_classification.sql", 31),
    ("26__spend_baseline.sql", 32),
    ("26__worker_budget_alert.sql", 33),
    ("27__handoff_auctions.sql", 34),
    ("27__spend_anomaly.sql", 35),
    ("28__recovery_cache.sql", 36),
    ("28__role_duties.sql", 37),
    ("28__tenant_quota.sql", 38),
    ("30__learned_plan_templates.sql", 0),
    ("30__worker_charter.sql", 0),
    ("45__cu_screenshot_index.sql", 0),
    ("45__worker_dependency.sql", 0),
    ("45__worker_level.sql", 0),
    ("51__audit_worker_deleted_at.sql", 0),
    ("51__worker_skill.sql", 47),
    ("75__scim_provisioned_tombstones.sql", 64),
    ("75__siem_dispatch_cursor.sql", 0),
];

const HISTORICAL_DUPLICATE_SLOTS: &[HistoricalDuplicateSlot] = &[
    HistoricalDuplicateSlot {
        slot: 19,
        files: &["19__anomaly.sql", "19__worker_fs_root.sql"],
    },
    HistoricalDuplicateSlot {
        slot: 22,
        files: &["22__roster.sql", "22__tenant_archived.sql"],
    },
    HistoricalDuplicateSlot {
        slot: 23,
        files: &["23__handoff_routing.sql", "23__workflow_checkpoints.sql"],
    },
    HistoricalDuplicateSlot {
        slot: 24,
        files: &[
            "24__operator_feedback.sql",
            "24__skill_quality_score.sql",
            "24__vertical_archived.sql",
        ],
    },
    HistoricalDuplicateSlot {
        slot: 26,
        files: &[
            "26__data_classification.sql",
            "26__spend_baseline.sql",
            "26__worker_budget_alert.sql",
        ],
    },
    HistoricalDuplicateSlot {
        slot: 27,
        files: &["27__handoff_auctions.sql", "27__spend_anomaly.sql"],
    },
    HistoricalDuplicateSlot {
        slot: 28,
        files: &[
            "28__recovery_cache.sql",
            "28__role_duties.sql",
            "28__tenant_quota.sql",
        ],
    },
    HistoricalDuplicateSlot {
        slot: 30,
        files: &["30__learned_plan_templates.sql", "30__worker_charter.sql"],
    },
    HistoricalDuplicateSlot {
        slot: 45,
        files: &[
            "45__cu_screenshot_index.sql",
            "45__worker_dependency.sql",
            "45__worker_level.sql",
        ],
    },
    HistoricalDuplicateSlot {
        slot: 51,
        files: &["51__audit_worker_deleted_at.sql", "51__worker_skill.sql"],
    },
    HistoricalDuplicateSlot {
        slot: 75,
        files: &[
            "75__scim_provisioned_tombstones.sql",
            "75__siem_dispatch_cursor.sql",
        ],
    },
];

/// Reject duplicate version numbers in the Rust runner registry.
pub(super) fn validate_registered_versions<I>(versions: I) -> Result<(), ArgoError>
where
    I: IntoIterator<Item = u32>,
{
    let mut seen = BTreeSet::new();
    let mut duplicates = Vec::new();
    for version in versions {
        if !seen.insert(version) {
            duplicates.push(version);
        }
    }
    if duplicates.is_empty() {
        return Ok(());
    }
    Err(ArgoError::Invariant(format!(
        "duplicate migration registry version(s): {}",
        duplicates
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    )))
}

/// Warn loudly that historical raw SQL filename collisions are intentional.
pub(super) fn warn_on_historical_raw_collisions() {
    let registered_mappings = CANONICAL_RAW_SQL_ORDER
        .iter()
        .filter(|(_, version)| *version > 0)
        .count();
    tracing::warn!(
        target: "argo_store::migrations",
        "MIGRATION SLOT COLLISION WARNING: historical raw SQL filename slots {} are preserved; \
         the Rust migration registry is the canonical order and no SQL files are renamed; \
         {registered_mappings} duplicate-prefix raw files map to schema_version entries",
        HISTORICAL_DUPLICATE_SLOTS
            .iter()
            .map(|slot| slot.slot.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

#[cfg(test)]
fn canonical_version_for_raw_file(file: &str) -> Option<u32> {
    CANONICAL_RAW_SQL_ORDER
        .iter()
        .find(|(entry, _)| *entry == file)
        .map(|(_, version)| *version)
}

#[cfg(test)]
fn unexpected_duplicate_slots<I>(files: I) -> Vec<SlotCollision>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let mut grouped: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for file in files {
        let file = file.as_ref();
        let Some(slot) = slot_prefix(file) else {
            continue;
        };
        grouped.entry(slot).or_default().push(file.to_string());
    }

    grouped
        .into_iter()
        .filter_map(|(slot, mut files)| {
            if files.len() <= 1 {
                return None;
            }
            files.sort();
            if is_known_historical_duplicate(slot, &files) {
                return None;
            }
            Some(SlotCollision { slot, files })
        })
        .collect()
}

#[cfg(test)]
fn slot_prefix(file: &str) -> Option<u32> {
    let (prefix, _) = file.split_once("__")?;
    prefix.parse().ok()
}

#[cfg(test)]
fn is_known_historical_duplicate(slot: u32, files: &[String]) -> bool {
    HISTORICAL_DUPLICATE_SLOTS.iter().any(|known| {
        known.slot == slot
            && known.files.len() == files.len()
            && known
                .files
                .iter()
                .zip(files)
                .all(|(left, right)| *left == right)
    })
}
