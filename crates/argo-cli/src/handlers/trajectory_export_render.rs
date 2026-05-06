//! Format renderers + helpers for `argo trajectory export`. Split
//! out of the main handler to keep both files under the 500-LOC
//! project invariant.

use super::trajectory_export::{Format, Resolved, Trajectory};
use argo_types::{ArgoError, ArgoResult};
use sha2::{Digest, Sha256};

/// Render the trajectories per the requested format. Each format
/// is emitted as UTF-8 bytes so the caller can write them to disk
/// or stdout uniformly.
pub(super) fn render(trajectories: &[Trajectory], resolved: &Resolved) -> ArgoResult<Vec<u8>> {
    let mut value = match resolved.format {
        Format::Anthropic => render_anthropic(trajectories),
        Format::Openai => render_openai(trajectories),
        Format::Jsonl => return render_jsonl(trajectories, resolved.gdpr_redact_emails),
    };
    if resolved.gdpr_redact_emails {
        redact_value(&mut value);
    }
    let body = serde_json::to_string_pretty(&value)
        .map_err(|e| ArgoError::Provider(format!("serialize export: {e}")))?;
    Ok(body.into_bytes())
}

fn render_anthropic(trajectories: &[Trajectory]) -> serde_json::Value {
    let rows: Vec<_> = trajectories
        .iter()
        .map(|t| {
            let mut messages = Vec::new();
            for step in &t.steps {
                messages.push(serde_json::json!({
                    "role": "user",
                    "content": step.action,
                }));
                let mut assistant = serde_json::json!({
                    "role": "assistant",
                    "content": step.result,
                });
                if let Some(ref shot) = step.screenshot {
                    assistant["screenshot"] = serde_json::Value::String(shot.clone());
                }
                messages.push(assistant);
            }
            serde_json::json!({
                "action_id": t.action_id,
                "action_type": t.action_type,
                "state": t.state,
                "messages": messages,
            })
        })
        .collect();
    serde_json::Value::Array(rows)
}

fn render_openai(trajectories: &[Trajectory]) -> serde_json::Value {
    let rows: Vec<_> = trajectories
        .iter()
        .map(|t| {
            let mut messages = Vec::new();
            for step in &t.steps {
                messages.push(serde_json::json!({
                    "role": "user",
                    "content": serde_json::to_string(&step.action).unwrap_or_default(),
                }));
                let mut tool_call = serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": "argo_action",
                        "arguments": serde_json::to_string(&step.result).unwrap_or_default(),
                    },
                });
                if let Some(ref shot) = step.screenshot {
                    tool_call["screenshot"] = serde_json::Value::String(shot.clone());
                }
                messages.push(serde_json::json!({
                    "role": "assistant",
                    "tool_calls": [tool_call],
                }));
            }
            serde_json::json!({
                "action_id": t.action_id,
                "action_type": t.action_type,
                "state": t.state,
                "messages": messages,
            })
        })
        .collect();
    serde_json::Value::Array(rows)
}

fn render_jsonl(trajectories: &[Trajectory], redact: bool) -> ArgoResult<Vec<u8>> {
    let mut out = String::new();
    for t in trajectories {
        let mut value = serde_json::to_value(t)
            .map_err(|e| ArgoError::Provider(format!("serialize jsonl row: {e}")))?;
        if redact {
            redact_value(&mut value);
        }
        let line = serde_json::to_string(&value)
            .map_err(|e| ArgoError::Provider(format!("serialize jsonl row: {e}")))?;
        out.push_str(&line);
        out.push('\n');
    }
    Ok(out.into_bytes())
}

/// Walk an arbitrary serde_json value and apply
/// `redact_emails_in_str` to every string leaf. Mirrors the
/// `audit_export.rs` pattern so output is byte-equivalent for the
/// same input.
pub(super) fn redact_value(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(s) => *s = redact_emails_in_str(s),
        serde_json::Value::Array(items) => {
            for item in items {
                redact_value(item);
            }
        }
        serde_json::Value::Object(map) => {
            for item in map.values_mut() {
                redact_value(item);
            }
        }
        _ => {}
    }
}

/// Replace every `local@domain.tld` in `input` with `<redacted>`.
/// Lives here (rather than reusing `audit_export::redact_emails_in_str`)
/// because the audit_export helper is `pub(super)` and the brief's
/// may-touch list doesn't allow widening it.
pub(super) fn redact_emails_in_str(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut last = 0_usize;
    for (idx, byte) in bytes.iter().enumerate() {
        if *byte != b'@' {
            continue;
        }
        let start = scan_left(bytes, idx);
        let end = scan_right(bytes, idx + 1);
        if start == idx || end == idx + 1 {
            continue;
        }
        let candidate = &input[start..end];
        if candidate
            .rsplit_once('.')
            .is_some_and(|(_, tld)| !tld.is_empty() && tld.bytes().all(|b| b.is_ascii_alphanumeric()))
        {
            out.push_str(&input[last..start]);
            out.push_str("<redacted>");
            last = end;
        }
    }
    out.push_str(&input[last..]);
    out
}

fn scan_left(bytes: &[u8], at: usize) -> usize {
    let mut i = at;
    while i > 0 && is_email_char(bytes[i - 1]) {
        i -= 1;
    }
    i
}

fn scan_right(bytes: &[u8], from: usize) -> usize {
    let mut i = from;
    while i < bytes.len() && is_email_char(bytes[i]) {
        i += 1;
    }
    i
}

fn is_email_char(b: u8) -> bool {
    b.is_ascii_alphanumeric()
        || matches!(b, b'.' | b'_' | b'-' | b'+' | b'%')
}

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

pub(super) fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 3 <= input.len() {
        let b = &input[i..i + 3];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 6) & 0x3f) as usize] as char);
        out.push(TABLE[(n & 0x3f) as usize] as char);
        i += 3;
    }
    let rem = input.len() - i;
    if rem == 1 {
        let n = (input[i] as u32) << 16;
        out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8);
        out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 6) & 0x3f) as usize] as char);
        out.push('=');
    }
    out
}
