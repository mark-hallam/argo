//! Pluggable registry of workspace-managed keys for
//! `argo security rotate-keys` (Mission
//! `mission-automated-key-rotation`).
//!
//! Each entry pairs an operator-facing key name with the subsystem
//! that owns its rotation. The CLI is a coordinator only — actual
//! rotation logic lives in subsystem-specific commands referenced
//! by `rotation_command`. The defaults below cover the five
//! categories named in MISSION_BACKLOG #217.
//!
//! The catalog is intentionally hard-coded for v1; new key
//! categories should land via a follow-up brief that adds an entry
//! here. Avoids a config-file boot dependency and matches the
//! "manifests not adapters" invariant for the small, finite key
//! set we own today.
use serde::Serialize;

/// One registered key category.
#[derive(Clone, Debug, Serialize)]
pub struct KeyEntry {
    /// Operator-facing slug, e.g. `db-encryption`. Matches the
    /// `<key-name>` argument on `plan` / `commit`.
    pub name: &'static str,
    /// Human-readable category for the `list` output.
    pub category: &'static str,
    /// Environment variable (or credential-store key) holding the
    /// active key material. NEVER printed in cleartext by any
    /// surface in this brief.
    pub env_var: &'static str,
    /// Subsystem command this rotation delegates to. Empty means
    /// "operator follow-up only" (e.g. config edit + restart).
    pub rotation_command: &'static str,
    /// One-line summary printed in `list` and `plan`.
    pub description: &'static str,
    /// Free-text follow-ups the operator must perform after the
    /// subsystem command returns. May be empty.
    pub operator_followups: &'static [&'static str],
}

/// Default key catalog. Snapshot of MISSION_BACKLOG #217 categories
/// 1–5. Per-connector OAuth tokens are represented as a single
/// rolled-up entry; concrete connectors are listed under
/// `operator_followups` so `argo connector reauth <name>` is
/// discoverable from `plan` output.
pub const DEFAULT_KEYS: &[KeyEntry] = &[
    KeyEntry {
        name: "db-encryption",
        category: "Database (SQLCipher)",
        env_var: "ARGO_DB_ENCRYPTION_KEY",
        rotation_command: "argo db rekey",
        description: "SQLCipher database master key (workspace store).",
        operator_followups: &[
            "After rekey: confirm `argo doctor db` reports `encrypted=true`.",
            "Take a fresh `argo backup create` after rotation so backups use the new key.",
        ],
    },
    KeyEntry {
        name: "credentials-master",
        category: "Credentials (TOML store)",
        env_var: "ARGO_CREDENTIALS_MASTER_KEY",
        rotation_command: "argo credentials encrypt-all --new-master-key",
        description: "Master key wrapping the workspace credentials.toml.",
        operator_followups: &[
            "Re-export with `argo cred export` if any shell rc files reference cached values.",
        ],
    },
    KeyEntry {
        name: "gateway-bearer",
        category: "Gateway auth",
        env_var: "ARGO_GATEWAY_BEARER_TOKEN",
        rotation_command: "",
        description: "Bearer token enforced by the gateway HTTP surface.",
        operator_followups: &[
            "Edit argo.toml / credentials store to install the new token, then restart the gateway daemon.",
            "Update any external monitors that hit the gateway with the previous token.",
        ],
    },
    KeyEntry {
        name: "channel-hmac",
        category: "Voice / channel HMAC",
        env_var: "ARGO_CHANNEL_HMAC_SECRET",
        rotation_command: "",
        description: "HMAC secret signing voice DTMF + inbound channel approvals.",
        operator_followups: &[
            "Edit argo.toml / credentials store to install the new secret, then restart the gateway daemon.",
            "Re-broadcast the secret to every voice/IVR partner that uses it for inbound signing.",
        ],
    },
    KeyEntry {
        name: "connector-oauth",
        category: "Per-connector OAuth refresh tokens",
        env_var: "<per-connector>",
        rotation_command: "argo connector reauth <name>",
        description: "OAuth refresh tokens stored per MCP connector.",
        operator_followups: &[
            "Run `argo connector list` to enumerate authorized connectors.",
            "For each connector: `argo connector reauth <name>` to mint a fresh refresh token.",
        ],
    },
];

/// Look up an entry by `name`. Case-sensitive — slugs are stable.
pub fn lookup(name: &str) -> Option<&'static KeyEntry> {
    DEFAULT_KEYS.iter().find(|k| k.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_default_categories() {
        assert!(
            DEFAULT_KEYS.len() >= 5,
            "expected ≥5 key categories per MISSION_BACKLOG #217, got {}",
            DEFAULT_KEYS.len()
        );
    }

    #[test]
    fn names_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for entry in DEFAULT_KEYS {
            assert!(
                seen.insert(entry.name),
                "duplicate registered key name: {}",
                entry.name
            );
        }
    }

    #[test]
    fn lookup_round_trip() {
        for entry in DEFAULT_KEYS {
            let found = lookup(entry.name).expect("entry must round-trip");
            assert_eq!(found.name, entry.name);
        }
        assert!(lookup("no-such-key").is_none());
    }
}
