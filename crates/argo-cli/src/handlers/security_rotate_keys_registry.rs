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

/// How `argo security rotate-keys commit <key>` performs the
/// rotation for one entry.
///
/// `Subprocess` causes the coordinator to spawn the named `argo`
/// subcommand directly with `std::process::Command`, capture its
/// stdout/stderr into the audit-row payload, and propagate the exit
/// code. `OperatorPrompt` is the legacy print-and-suggest path —
/// the coordinator records rotation intent in the audit chain but
/// the operator is responsible for running the follow-up steps.
///
/// Categories whose rotation requires an env-var change followed by
/// a daemon restart (`gateway-bearer`, `channel-hmac`) stay on
/// `OperatorPrompt`: they can't self-rotate from inside the running
/// process. Categories whose subsystem CLI surface doesn't yet
/// accept a coordinator-driven new-key flag stay on
/// `OperatorPrompt` until the upstream flag lands.
#[derive(Clone, Copy, Debug)]
pub enum ExecutionKind {
    /// Spawn `argo <args...>` and append `[new_key_flag,
    /// <new-key-source>]` when `new_key_flag` is `Some`.
    Subprocess {
        args: &'static [&'static str],
        new_key_flag: Option<&'static str>,
    },
    /// Print rotation_command + operator_followups; no subprocess.
    OperatorPrompt,
}

impl serde::Serialize for ExecutionKind {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(match self {
            ExecutionKind::Subprocess { .. } => "subprocess",
            ExecutionKind::OperatorPrompt => "operator_prompt",
        })
    }
}

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
    /// How the coordinator runs this entry: spawn a subprocess or
    /// print operator follow-ups. Decoupled from `rotation_command`
    /// so the display string can stay stable while the dispatch
    /// shape evolves.
    pub execution: ExecutionKind,
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
        rotation_command: "argo db rekey --new-key-source <src>",
        execution: ExecutionKind::Subprocess {
            args: &["db", "rekey"],
            new_key_flag: Some("--new-key-source"),
        },
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
        // `argo cred encrypt-all` has no `--new-master-key` flag
        // today: it sources the master key from
        // `ARGO_CREDENTIALS_MASTER_KEY` / `~/.skycot/.master_key`.
        // Once that flag lands, swap execution to `Subprocess`.
        rotation_command: "argo cred encrypt-all",
        execution: ExecutionKind::OperatorPrompt,
        description: "Master key wrapping the workspace credentials.toml.",
        operator_followups: &[
            "Set ARGO_CREDENTIALS_MASTER_KEY (or rewrite ~/.skycot/.master_key) to the new key.",
            "Run `argo cred encrypt-all` to re-wrap every entry under the new key.",
            "Re-export with `argo cred export` if any shell rc files reference cached values.",
        ],
    },
    KeyEntry {
        name: "gateway-bearer",
        category: "Gateway auth",
        env_var: "ARGO_GATEWAY_BEARER_TOKEN",
        rotation_command: "",
        execution: ExecutionKind::OperatorPrompt,
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
        execution: ExecutionKind::OperatorPrompt,
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
        // No `argo connector reauth` subcommand today; closest path
        // is `argo connector authorize <kind>` which runs an
        // interactive OAuth flow. Subprocess-capture is a poor fit
        // for interactive flows, so this stays OperatorPrompt until
        // a non-interactive reauth lands.
        rotation_command: "argo connector authorize <name>",
        execution: ExecutionKind::OperatorPrompt,
        description: "OAuth refresh tokens stored per MCP connector.",
        operator_followups: &[
            "Run `argo connector list` to enumerate authorized connectors.",
            "For each connector: `argo connector authorize <name>` to mint a fresh refresh token.",
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

    #[test]
    fn db_encryption_dispatches_to_subprocess() {
        let entry = lookup("db-encryption").expect("registered");
        match entry.execution {
            ExecutionKind::Subprocess { args, new_key_flag } => {
                assert_eq!(args, &["db", "rekey"]);
                assert_eq!(new_key_flag, Some("--new-key-source"));
            }
            ExecutionKind::OperatorPrompt => {
                panic!("db-encryption must dispatch to a subprocess")
            }
        }
    }

    #[test]
    fn env_var_only_categories_stay_operator_prompt() {
        for name in ["gateway-bearer", "channel-hmac"] {
            let entry = lookup(name).expect("registered");
            assert!(
                matches!(entry.execution, ExecutionKind::OperatorPrompt),
                "{name} must stay OperatorPrompt (env-var rotation requires restart)"
            );
        }
    }
}
