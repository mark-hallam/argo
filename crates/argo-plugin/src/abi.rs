//! Plugin ABI — pinned wire-contract constants and version-compat helpers.
//!
//! The Argo plugin ABI (Application Binary Interface) is the wire
//! contract that every plugin subprocess implements. It is the
//! polyglot-core invariant (MISSION.md invariant #1): a stable wire
//! shape lets plugins exist in any language (Python, TypeScript,
//! Go, …).
//!
//! This module pins three things so the surface cannot drift silently:
//!
//! 1. The JSON-RPC envelope version ([`JSONRPC_VERSION`]).
//! 2. The subset of JSON-RPC 2.0 error codes Argo's host emits or
//!    surfaces ([`RPC_PARSE_ERROR`] … [`RPC_INTERNAL_ERROR`]).
//! 3. Semver-style [`AbiCompatibility`] classification for handshake
//!    version negotiation.
//!
//! Wire-shape and forward-compat assertions live in
//! `tests/abi_contract.rs`. If a silent rename, removed field, or
//! tightened decoder regresses the ABI, those contract tests fail and
//! CI rejects the PR. Bumping [`crate::PROTOCOL_VERSION`] in lockstep
//! with the failing snapshot is the documented intentional path.
//!
//! ## Forward-compat policy
//!
//! - **Additive** changes are non-breaking: new optional fields, new
//!   methods, new error-code data payloads. Plugins running an older
//!   SDK MUST ignore unknown fields on decode (`serde` ignores by
//!   default — `tests/abi_contract.rs::ignores_unknown_fields_…`
//!   pins this).
//! - **Removing** a field, **renaming** a field, or **narrowing** an
//!   enum variant is breaking and requires a major-version bump.
//! - The current host enforces **exact-string** version match at
//!   handshake (see `host::handshake`). The semver helpers below
//!   exist for the planned wire-up that loosens that to
//!   `is_wire_compatible` (allowing the plugin to be at a newer
//!   minor/patch on the same major); the contract tests pin the
//!   current strict behavior so the loosening is a deliberate code
//!   change, not a regression.

/// JSON-RPC envelope version. Argo speaks 2.0 only.
pub const JSONRPC_VERSION: &str = "2.0";

/// JSON-RPC 2.0 standard: parse error (the plugin sent malformed JSON).
pub const RPC_PARSE_ERROR: i64 = -32700;

/// JSON-RPC 2.0 standard: invalid request envelope.
pub const RPC_INVALID_REQUEST: i64 = -32600;

/// JSON-RPC 2.0 standard: method name unknown to the plugin.
pub const RPC_METHOD_NOT_FOUND: i64 = -32601;

/// JSON-RPC 2.0 standard: params payload failed plugin-side validation.
pub const RPC_INVALID_PARAMS: i64 = -32602;

/// JSON-RPC 2.0 standard: internal plugin error.
pub const RPC_INTERNAL_ERROR: i64 = -32603;

/// Classification of a plugin's reported protocol version against the
/// host's own [`crate::PROTOCOL_VERSION`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbiCompatibility {
    /// Exact match.
    Identical,
    /// Plugin is at a newer minor or patch on the SAME major. Wire is
    /// forward-compatible per the additive-only policy.
    PluginNewerMinor,
    /// Plugin is at a newer major. Wire is NOT compatible — host must
    /// reject the handshake.
    PluginNewerMajor,
    /// Plugin is older. Host's wire shape may include fields the
    /// plugin doesn't know about; safe by serde-defaults, but the
    /// host MAY still reject if it depends on newer plugin behavior.
    PluginOlder,
    /// One of the version strings did not parse as `MAJOR.MINOR.PATCH`
    /// with all-numeric components.
    Unparseable,
}

/// Parse a `MAJOR.MINOR.PATCH` triple. Returns `None` on any
/// non-numeric segment, missing segment, extra segment, or
/// pre-release / build suffix (e.g. `1.0.0-rc1`, `1.0.0+meta`). Strict
/// by design — pre-release semantics are out of scope for v0.1; a
/// future bump that adopts them will update this parser in lockstep
/// with `tests/abi_contract.rs::parse_protocol_version_*`.
#[must_use]
pub fn parse_protocol_version(version: &str) -> Option<(u32, u32, u32)> {
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// Classify a plugin-reported version against a host-pinned version.
#[must_use]
pub fn classify_compatibility(host: &str, plugin: &str) -> AbiCompatibility {
    let Some(host) = parse_protocol_version(host) else {
        return AbiCompatibility::Unparseable;
    };
    let Some(plugin) = parse_protocol_version(plugin) else {
        return AbiCompatibility::Unparseable;
    };
    if host == plugin {
        return AbiCompatibility::Identical;
    }
    match plugin.0.cmp(&host.0) {
        std::cmp::Ordering::Greater => AbiCompatibility::PluginNewerMajor,
        std::cmp::Ordering::Less => AbiCompatibility::PluginOlder,
        std::cmp::Ordering::Equal => {
            if plugin > host {
                AbiCompatibility::PluginNewerMinor
            } else {
                AbiCompatibility::PluginOlder
            }
        }
    }
}

/// Convenience: returns `true` when the plugin's reported version is
/// wire-compatible with the host's (same major, plugin >= host
/// minor/patch). Same-version match is always compatible.
///
/// NOT YET WIRED. `host::handshake` currently enforces exact-string
/// equality. The semver helper exists so the planned loosening is a
/// one-line swap with known-good behavior pinned by the contract tests.
#[must_use]
pub fn is_wire_compatible(host: &str, plugin: &str) -> bool {
    matches!(
        classify_compatibility(host, plugin),
        AbiCompatibility::Identical | AbiCompatibility::PluginNewerMinor
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsonrpc_envelope_pinned_at_2_0() {
        assert_eq!(JSONRPC_VERSION, "2.0");
    }

    #[test]
    fn standard_error_codes_match_jsonrpc_spec() {
        assert_eq!(RPC_PARSE_ERROR, -32700);
        assert_eq!(RPC_INVALID_REQUEST, -32600);
        assert_eq!(RPC_METHOD_NOT_FOUND, -32601);
        assert_eq!(RPC_INVALID_PARAMS, -32602);
        assert_eq!(RPC_INTERNAL_ERROR, -32603);
    }

    #[test]
    fn parse_protocol_version_accepts_clean_triple() {
        assert_eq!(parse_protocol_version("0.1.0"), Some((0, 1, 0)));
        assert_eq!(parse_protocol_version("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_protocol_version("12.34.567"), Some((12, 34, 567)));
    }

    #[test]
    fn parse_protocol_version_rejects_pre_release_and_garbage() {
        assert_eq!(parse_protocol_version("9.9.9-mismatch"), None);
        assert_eq!(parse_protocol_version("1.0.0-rc1"), None);
        assert_eq!(parse_protocol_version("1.0.0+meta"), None);
        assert_eq!(parse_protocol_version("1.0"), None);
        assert_eq!(parse_protocol_version("1.0.0.0"), None);
        assert_eq!(parse_protocol_version(""), None);
        assert_eq!(parse_protocol_version("not.a.version"), None);
    }

    #[test]
    fn classify_compatibility_covers_every_axis() {
        assert_eq!(
            classify_compatibility("0.1.0", "0.1.0"),
            AbiCompatibility::Identical
        );
        assert_eq!(
            classify_compatibility("0.1.0", "0.2.0"),
            AbiCompatibility::PluginNewerMinor
        );
        assert_eq!(
            classify_compatibility("0.1.0", "0.1.5"),
            AbiCompatibility::PluginNewerMinor
        );
        assert_eq!(
            classify_compatibility("0.1.0", "1.0.0"),
            AbiCompatibility::PluginNewerMajor
        );
        assert_eq!(
            classify_compatibility("1.0.0", "0.9.9"),
            AbiCompatibility::PluginOlder
        );
        assert_eq!(
            classify_compatibility("0.2.0", "0.1.5"),
            AbiCompatibility::PluginOlder
        );
        assert_eq!(
            classify_compatibility("0.1.0", "9.9.9-mismatch"),
            AbiCompatibility::Unparseable
        );
    }

    #[test]
    fn is_wire_compatible_accepts_only_same_major_at_or_above_host() {
        assert!(is_wire_compatible("0.1.0", "0.1.0"));
        assert!(is_wire_compatible("0.1.0", "0.1.5"));
        assert!(is_wire_compatible("0.1.0", "0.2.0"));
        assert!(!is_wire_compatible("0.1.0", "1.0.0"));
        assert!(!is_wire_compatible("1.0.0", "0.9.9"));
        assert!(!is_wire_compatible("0.1.0", "9.9.9-mismatch"));
    }
}
