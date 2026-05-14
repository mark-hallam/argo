//! Environment parsing helpers for [`crate::loader`].
//!
//! These helpers keep the layered loader under the repo's 500-line
//! ceiling while preserving the legacy parse semantics of each env
//! knob.

use argo_types::{ArgoError, ArgoResult};

pub(crate) fn env_string(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.is_empty())
}

pub(crate) fn env_bool(key: &str) -> ArgoResult<Option<bool>> {
    let Some(raw) = env_string(key) else {
        return Ok(None);
    };
    match raw.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(Some(true)),
        "0" | "false" | "no" | "off" => Ok(Some(false)),
        _ => Err(ArgoError::Provider(format!(
            "{key}: expected bool (1/0/true/false), got '{raw}'"
        ))),
    }
}

pub(crate) fn env_u16(key: &str) -> ArgoResult<Option<u16>> {
    let Some(raw) = env_string(key) else {
        return Ok(None);
    };
    raw.parse::<u16>()
        .map(Some)
        .map_err(|e| ArgoError::Provider(format!("{key}: {e}")))
}

pub(crate) fn env_u64(key: &str) -> ArgoResult<Option<u64>> {
    let Some(raw) = env_string(key) else {
        return Ok(None);
    };
    raw.parse::<u64>()
        .map(Some)
        .map_err(|e| ArgoError::Provider(format!("{key}: {e}")))
}

pub(crate) fn env_bool_one(key: &str) -> Option<bool> {
    env_string(key).map(|raw| raw == "1")
}

pub(crate) fn env_bool_not_zero(key: &str) -> Option<bool> {
    env_string(key).map(|raw| raw != "0")
}

pub(crate) fn env_u32_lossy(key: &str) -> Option<u32> {
    env_string(key).and_then(|raw| raw.parse::<u32>().ok())
}

pub(crate) fn env_u32_positive_lossy(key: &str) -> Option<u32> {
    env_u32_lossy(key).filter(|n| *n > 0)
}

pub(crate) fn env_f64_positive_lossy(key: &str) -> Option<f64> {
    env_string(key)
        .and_then(|raw| raw.parse::<f64>().ok())
        .filter(|n| n.is_finite() && *n > 0.0)
}
