//! Canonical JSON + canonical hash for [`SkillGraph`].
//!
//! The canonical form is the one input that participates in the L2
//! hash chain: every IR write stores both the value and its
//! `sha256(canonical_json(graph))`. Two semantically-equal graphs
//! must produce byte-identical canonical JSON, regardless of map
//! iteration order.
//!
//! Rules:
//! - Object keys are sorted lexicographically (UTF-8 codepoint order,
//!   matching `BTreeMap<String, _>`).
//! - No whitespace anywhere — `","`, `":"` are bare separators.
//! - Strings are JSON-escaped via [`serde_json`]'s string writer
//!   (RFC 8785-compatible at the surface we care about for ASCII
//!   keys; non-ASCII content roundtrips through serde_json's escape
//!   tables).
//! - Numbers are emitted via [`serde_json::Number::to_string`] which
//!   uses the minimal lossless representation for the underlying
//!   type — sufficient for the integer + lossy-f64 fields in the
//!   schema. Operators relying on full RFC 8785 number canonicalisation
//!   should normalise upstream.

use crate::ir::SkillGraph;
use argo_types::{ArgoError, ArgoResult};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Render a [`SkillGraph`] to its canonical JSON form — sorted keys,
/// no whitespace. Stable across runs; suitable for hashing.
///
/// # Example
/// ```
/// use argo_skill::ir::{SkillGraph, canonical_json};
/// let g = SkillGraph::default();
/// let canon = canonical_json(&g).unwrap();
/// assert!(canon.starts_with('{'));
/// assert!(!canon.contains(' '));
/// ```
pub fn canonical_json(graph: &SkillGraph) -> ArgoResult<String> {
    let value = serde_json::to_value(graph)
        .map_err(|e| ArgoError::Provider(format!("ir canonical_json serialize: {e}")))?;
    let mut buf = Vec::with_capacity(256);
    write_canonical(&mut buf, &value)?;
    String::from_utf8(buf).map_err(|e| ArgoError::Provider(format!("ir canonical_json utf8: {e}")))
}

/// Compute `sha256(canonical_json(graph))` and return the 32-byte
/// digest.
///
/// # Example
/// ```
/// use argo_skill::ir::{SkillGraph, canonical_hash};
/// let g = SkillGraph::default();
/// let digest = canonical_hash(&g).unwrap();
/// assert_eq!(digest.len(), 32);
/// ```
pub fn canonical_hash(graph: &SkillGraph) -> ArgoResult<[u8; 32]> {
    let canon = canonical_json(graph)?;
    let mut hasher = Sha256::new();
    hasher.update(canon.as_bytes());
    Ok(hasher.finalize().into())
}

/// Compute `sha256(canonical_json(graph))` and return it as a 64-char
/// lowercase hex string.
///
/// # Example
/// ```
/// use argo_skill::ir::{SkillGraph, canonical_hash_hex};
/// let g = SkillGraph::default();
/// let hex = canonical_hash_hex(&g).unwrap();
/// assert_eq!(hex.len(), 64);
/// ```
pub fn canonical_hash_hex(graph: &SkillGraph) -> ArgoResult<String> {
    Ok(hex::encode(canonical_hash(graph)?))
}

fn write_canonical(out: &mut Vec<u8>, v: &Value) -> ArgoResult<()> {
    match v {
        Value::Null => {
            out.extend_from_slice(b"null");
        }
        Value::Bool(b) => {
            out.extend_from_slice(if *b { b"true" } else { b"false" });
        }
        Value::Number(n) => {
            out.extend_from_slice(n.to_string().as_bytes());
        }
        Value::String(s) => {
            write_json_string(out, s)?;
        }
        Value::Array(arr) => {
            out.push(b'[');
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_canonical(out, item)?;
            }
            out.push(b']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push(b'{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_json_string(out, key)?;
                out.push(b':');
                if let Some(child) = map.get(*key) {
                    write_canonical(out, child)?;
                } else {
                    return Err(ArgoError::Provider(format!(
                        "ir canonical_json: missing key {key} in object"
                    )));
                }
            }
            out.push(b'}');
        }
    }
    Ok(())
}

fn write_json_string(out: &mut Vec<u8>, s: &str) -> ArgoResult<()> {
    let encoded = serde_json::to_string(s)
        .map_err(|e| ArgoError::Provider(format!("ir canonical_json string-escape: {e}")))?;
    out.extend_from_slice(encoded.as_bytes());
    Ok(())
}
