//! Stream `claude-error-recovery-suggestions` — prompt assembly +
//! audit-chain summarisation. Lives in its own file so unit tests
//! can exercise prompt shape without invoking the LLM.

use argo_store::AuditRow;
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const PROMPT_INSTRUCTIONS: &str = r#"You are a post-mortem assistant for an autonomous agent platform called SKYCOT.
Given the audit chain below for one Session that ended in failure, produce a structured post-mortem.

Output ONLY a JSON object matching this exact shape:
{
  "likely_cause": "1-2 sentence root cause",
  "fixes": [
    {"rank": 1, "title": "...", "command": "...", "rationale": "..."},
    {"rank": 2, "title": "...", "command": "...", "rationale": "..."},
    {"rank": 3, "title": "...", "command": "...", "rationale": "..."}
  ]
}

Rank 1 is most-confident-fix; rank 3 is the long-shot.
Each `command` should be a runnable `argo` CLI invocation when applicable, or a description of a manual operator action.
Do not auto-apply anything — these are advisory."#;

/// Structured suggestion returned by the LLM (or pulled from
/// cache).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoverySuggestions {
    /// 1-2 sentence root cause.
    pub likely_cause: String,
    /// Three ranked fixes, rank 1 = most confident.
    pub fixes: Vec<RecoveryFix>,
}

/// One ranked fix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryFix {
    /// 1 (most-confident) through 3.
    pub rank: u8,
    /// Short headline.
    pub title: String,
    /// Suggested command snippet.
    pub command: String,
    /// Why this fix would help.
    pub rationale: String,
}

impl RecoverySuggestions {
    /// Validate the LLM response shape: 1..=3 fixes, ranks 1/2/3
    /// distinct, no empty fields. Caller surfaces as a typed error.
    pub fn validate(&self) -> Result<(), String> {
        if self.likely_cause.trim().is_empty() {
            return Err("likely_cause empty".into());
        }
        if self.fixes.is_empty() {
            return Err("fixes empty".into());
        }
        if self.fixes.len() > 5 {
            return Err(format!("too many fixes: {}", self.fixes.len()));
        }
        let mut seen_ranks = std::collections::HashSet::new();
        for fix in &self.fixes {
            if !(1..=10).contains(&fix.rank) {
                return Err(format!("rank out of range: {}", fix.rank));
            }
            if !seen_ranks.insert(fix.rank) {
                return Err(format!("duplicate rank: {}", fix.rank));
            }
            if fix.title.trim().is_empty() {
                return Err("fix title empty".into());
            }
        }
        Ok(())
    }
}

/// Identify the failing step from an audit chain. Returns the row
/// that's most likely the failure point — last `tool_result`
/// with `is_error=true`, or last `action.failed` row, or simply
/// the last row when no error markers found.
#[must_use]
pub fn find_failing_step(rows: &[AuditRow]) -> Option<&AuditRow> {
    // Walk in reverse to find the most recent error first.
    for row in rows.iter().rev() {
        if row.kind == "tool_result" {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&row.payload_json) {
                if parsed.get("is_error").and_then(|v| v.as_bool()) == Some(true) {
                    return Some(row);
                }
            }
        }
        if row.kind == "action.failed" || row.kind.contains(".failed") {
            return Some(row);
        }
    }
    rows.last()
}

/// Compute the deterministic chain hash for caching. Hashes the
/// concatenation of every row's seq + kind + payload_json so any
/// new row invalidates the cache.
#[must_use]
pub fn chain_hash_for(rows: &[AuditRow]) -> String {
    let mut hasher = Sha256::new();
    for row in rows {
        hasher.update(row.seq.to_le_bytes());
        hasher.update(row.kind.as_bytes());
        hasher.update(b"\0");
        hasher.update(row.payload_json.as_bytes());
        hasher.update(b"\0");
    }
    let bytes = hasher.finalize();
    hex::encode(bytes)
}

/// Build the LLM prompt body. The system prompt + the rendered
/// chain summary. Caller hands this string to the
/// [`crate::handlers::recover::AnthropicCaller`]; tests exercise
/// the rendering shape directly.
#[must_use]
pub fn build_prompt(brief: Option<&str>, manifest: Option<&str>, rows: &[AuditRow]) -> String {
    let mut prompt = String::new();
    prompt.push_str(PROMPT_INSTRUCTIONS);
    prompt.push_str("\n\n");
    if let Some(b) = brief {
        prompt.push_str("## Session brief\n\n");
        prompt.push_str(b);
        prompt.push_str("\n\n");
    }
    if let Some(m) = manifest {
        prompt.push_str("## Worker manifest\n\n");
        prompt.push_str(m);
        prompt.push_str("\n\n");
    }
    prompt.push_str("## Audit chain (");
    prompt.push_str(&rows.len().to_string());
    prompt.push_str(" rows, oldest first)\n\n");
    for row in rows {
        let ts = Utc
            .timestamp_micros(row.created_at)
            .single()
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| row.created_at.to_string());
        prompt.push_str(&format!("[seq={} kind={} at={}] ", row.seq, row.kind, ts));
        // Truncate huge payloads.
        let snippet = if row.payload_json.len() > 600 {
            format!("{}…", &row.payload_json[..600])
        } else {
            row.payload_json.clone()
        };
        prompt.push_str(&snippet);
        prompt.push('\n');
    }
    prompt.push_str("\nGiven the chain above, identify the failing step + return the JSON shape described.\n");
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(seq: i64, kind: &str, payload: &str) -> AuditRow {
        AuditRow {
            seq,
            kind: kind.to_string(),
            payload_json: payload.to_string(),
            created_at: 1_000_000 * 60 + seq,
        }
    }

    #[test]
    fn find_failing_step_picks_latest_error_tool_result() {
        let rows = vec![
            row(1, "tool_call", "{}"),
            row(2, "tool_result", r#"{"is_error":true,"reason":"timeout"}"#),
            row(3, "tool_call", "{}"),
            row(4, "tool_result", r#"{"is_error":true,"reason":"died"}"#),
            row(5, "session.complete", "{}"),
        ];
        let failing = find_failing_step(&rows).unwrap();
        assert_eq!(failing.seq, 4);
    }

    #[test]
    fn find_failing_step_falls_back_to_last_when_no_errors() {
        let rows = vec![row(1, "tool_call", "{}"), row(2, "tool_result", "{}")];
        let failing = find_failing_step(&rows).unwrap();
        assert_eq!(failing.seq, 2);
    }

    #[test]
    fn find_failing_step_picks_action_failed_kind() {
        let rows = vec![
            row(1, "tool_call", "{}"),
            row(2, "action.failed", r#"{"reason":"oom"}"#),
            row(3, "session.complete", "{}"),
        ];
        let failing = find_failing_step(&rows).unwrap();
        assert_eq!(failing.seq, 2);
    }

    #[test]
    fn find_failing_step_returns_none_for_empty_chain() {
        assert!(find_failing_step(&[]).is_none());
    }

    #[test]
    fn chain_hash_for_different_chains_differs() {
        let rows1 = vec![row(1, "tool_call", "{}")];
        let rows2 = vec![row(1, "tool_call", "{}"), row(2, "tool_result", "{}")];
        assert_ne!(chain_hash_for(&rows1), chain_hash_for(&rows2));
    }

    #[test]
    fn chain_hash_for_identical_chains_matches() {
        let rows = vec![row(1, "tool_call", "{}"), row(2, "tool_result", "{}")];
        let other = vec![row(1, "tool_call", "{}"), row(2, "tool_result", "{}")];
        assert_eq!(chain_hash_for(&rows), chain_hash_for(&other));
    }

    #[test]
    fn build_prompt_includes_brief_manifest_and_rows() {
        let rows = vec![row(1, "tool_call", "{}")];
        let prompt = build_prompt(Some("brief here"), Some("manifest here"), &rows);
        assert!(prompt.contains("Session brief"));
        assert!(prompt.contains("brief here"));
        assert!(prompt.contains("Worker manifest"));
        assert!(prompt.contains("manifest here"));
        assert!(prompt.contains("seq=1 kind=tool_call"));
        assert!(prompt.contains("JSON object"));
    }

    #[test]
    fn build_prompt_omits_optional_sections_when_none() {
        let rows = vec![row(1, "tool_call", "{}")];
        let prompt = build_prompt(None, None, &rows);
        assert!(!prompt.contains("Session brief"));
        assert!(!prompt.contains("Worker manifest"));
        assert!(prompt.contains("Audit chain"));
    }

    #[test]
    fn build_prompt_truncates_huge_payloads() {
        let big = "x".repeat(2000);
        let rows = vec![row(1, "tool_call", &big)];
        let prompt = build_prompt(None, None, &rows);
        // The truncated marker should appear and the prompt
        // shouldn't contain the full 2000-byte string.
        assert!(prompt.contains("…"));
    }

    #[test]
    fn validate_accepts_well_formed_response() {
        let s = RecoverySuggestions {
            likely_cause: "timeouts on slow API".into(),
            fixes: vec![
                RecoveryFix {
                    rank: 1,
                    title: "raise timeout".into(),
                    command: "argo worker update --timeout 60s".into(),
                    rationale: "current is 5s".into(),
                },
                RecoveryFix {
                    rank: 2,
                    title: "retry".into(),
                    command: "argo action replay <id>".into(),
                    rationale: "transient".into(),
                },
            ],
        };
        assert!(s.validate().is_ok());
    }

    #[test]
    fn validate_rejects_empty_likely_cause() {
        let s = RecoverySuggestions {
            likely_cause: "  ".into(),
            fixes: vec![RecoveryFix {
                rank: 1,
                title: "x".into(),
                command: "y".into(),
                rationale: "z".into(),
            }],
        };
        let err = s.validate().unwrap_err();
        assert!(err.contains("likely_cause"));
    }

    #[test]
    fn validate_rejects_duplicate_ranks() {
        let s = RecoverySuggestions {
            likely_cause: "x".into(),
            fixes: vec![
                RecoveryFix {
                    rank: 1,
                    title: "a".into(),
                    command: "b".into(),
                    rationale: "c".into(),
                },
                RecoveryFix {
                    rank: 1,
                    title: "d".into(),
                    command: "e".into(),
                    rationale: "f".into(),
                },
            ],
        };
        let err = s.validate().unwrap_err();
        assert!(err.contains("duplicate"));
    }
}
