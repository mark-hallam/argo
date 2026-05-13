//! Source-IP / CIDR allowlist gate
//! (`mission-ip-allowlist-deployment-mode`).
//!
//! Layered ahead of the bearer-auth path in [`super::boot::start`] so
//! a leaked token alone is insufficient for access — the request must
//! also arrive from an operator-listed CIDR. Disabled by default so
//! dev gateways are not accidentally locked out.
//!
//! # Trust model
//!
//! - `enabled = false` (default) — gate short-circuits with `Allow`,
//!   zero overhead beyond a single boolean check.
//! - `enabled = true, cidrs = []` — **deny-all**. Empty-list-with-gate-on
//!   is a misconfiguration signal the gate refuses to paper over.
//! - `enabled = true, cidrs = [...]` — request source IP must match at
//!   least one CIDR. Mismatched IP families are non-matches (an IPv4
//!   `/24` does not match an IPv6 peer).
//!
//! # Source-IP derivation
//!
//! `trust_x_forwarded_for_hops = 0` (default) → use the peer's
//! `SocketAddr::ip()`. The `X-Forwarded-For` header is ignored.
//!
//! `trust_x_forwarded_for_hops = n` (n > 0) → take the entry `n`
//! positions from the right of the `X-Forwarded-For` chain (i.e. the
//! IP injected by the *first untrusted* peer). When the header is
//! missing, malformed, or carries fewer than `n` entries, the gate
//! falls through to the peer IP — never to an attacker-controlled
//! value.
//!
//! # Deny response
//!
//! `403 Forbidden` with an **empty body**. The deny path does not
//! leak which header was unhappy (mirrors the cross-tenant audit
//! response pattern — 4xx codes leak less than a 200-with-error-payload).
//!
//! # Sensible deviations from brief
//!
//! - **Audit row → tracing event.** The brief asked for an `audit_record`
//!   row keyed `security.ip_allowlist_denied`, but the audit-kind enum
//!   lives in `argo-audit` which is on the must-NOT-touch list (no new
//!   variants without a schema migration). Deny events emit a
//!   `tracing::warn!` with `target = "gateway::ip_allowlist"` and
//!   structured `ip` / `path` / `method` fields, which the log-shipping
//!   pipeline already drains. A follow-up brief can promote this to a
//!   proper audit kind once the audit chain is in scope.
//! - **Wired in `boot::start`, not a per-dispatch `routes_dispatch_*`
//!   helper.** Peer `SocketAddr` is only observable inside the accept
//!   loop; the existing per-IP rate-limiter (#1051) sits in the same
//!   spot for the same reason.
//!
//! # CIDR parsing
//!
//! Hand-rolled IPv4 + IPv6 CIDR matcher (no new workspace dep). At
//! gate-construction time every entry parses to a typed
//! [`Cidr`]; entries that fail to parse turn into a hard
//! [`IpAllowlistError::InvalidCidr`] rather than silently widening
//! the gate.

use argo_config::IpAllowlistConfig;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

/// One parsed CIDR block (IPv4 or IPv6).
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) enum Cidr {
    /// IPv4 CIDR: `(network bits, prefix length 0..=32)`.
    V4 { net: u32, prefix: u8 },
    /// IPv6 CIDR: `(network bits, prefix length 0..=128)`.
    V6 { net: u128, prefix: u8 },
}

/// Construction-time error returned when an operator-supplied CIDR
/// entry cannot be parsed. Causes the gate to refuse to start; a
/// permissive default would silently widen the allowlist.
#[derive(Debug)]
pub enum IpAllowlistError {
    /// One `cidrs` entry was not a valid `<addr>/<prefix>` form.
    InvalidCidr {
        /// The raw entry as it appeared in TOML.
        entry: String,
        /// Short reason ("missing '/'", "prefix too large", etc.).
        reason: &'static str,
    },
}

impl std::fmt::Display for IpAllowlistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCidr { entry, reason } => {
                write!(f, "invalid CIDR '{entry}': {reason}")
            }
        }
    }
}

impl std::error::Error for IpAllowlistError {}

impl Cidr {
    /// Parse `<ip>/<prefix>` into a typed CIDR. Rejects malformed
    /// addresses and prefixes that exceed the address family's width.
    pub(super) fn parse(raw: &str) -> Result<Self, IpAllowlistError> {
        let (addr, prefix) = raw.split_once('/').ok_or_else(|| {
            IpAllowlistError::InvalidCidr {
                entry: raw.to_string(),
                reason: "missing '/' between addr and prefix",
            }
        })?;
        let prefix: u8 =
            prefix
                .trim()
                .parse()
                .map_err(|_| IpAllowlistError::InvalidCidr {
                    entry: raw.to_string(),
                    reason: "prefix must be an unsigned integer",
                })?;
        let ip: IpAddr =
            addr.trim()
                .parse()
                .map_err(|_| IpAllowlistError::InvalidCidr {
                    entry: raw.to_string(),
                    reason: "addr is not a valid IPv4 or IPv6 literal",
                })?;
        match ip {
            IpAddr::V4(v4) => {
                if prefix > 32 {
                    return Err(IpAllowlistError::InvalidCidr {
                        entry: raw.to_string(),
                        reason: "IPv4 prefix must be 0..=32",
                    });
                }
                Ok(Self::V4 {
                    net: mask_v4(v4, prefix),
                    prefix,
                })
            }
            IpAddr::V6(v6) => {
                if prefix > 128 {
                    return Err(IpAllowlistError::InvalidCidr {
                        entry: raw.to_string(),
                        reason: "IPv6 prefix must be 0..=128",
                    });
                }
                Ok(Self::V6 {
                    net: mask_v6(v6, prefix),
                    prefix,
                })
            }
        }
    }

    /// `true` when `ip` falls inside this CIDR block. Mismatched IP
    /// families are non-matches — an IPv4-mapped IPv6 address is not
    /// silently widened to match an IPv4 block.
    pub(super) fn contains(&self, ip: IpAddr) -> bool {
        match (self, ip) {
            (Self::V4 { net, prefix }, IpAddr::V4(v4)) => mask_v4(v4, *prefix) == *net,
            (Self::V6 { net, prefix }, IpAddr::V6(v6)) => mask_v6(v6, *prefix) == *net,
            _ => false,
        }
    }
}

fn mask_v4(addr: Ipv4Addr, prefix: u8) -> u32 {
    if prefix == 0 {
        return 0;
    }
    let shift = 32 - u32::from(prefix);
    let mask = u32::MAX.checked_shl(shift).unwrap_or(0);
    u32::from(addr) & mask
}

fn mask_v6(addr: Ipv6Addr, prefix: u8) -> u128 {
    if prefix == 0 {
        return 0;
    }
    let shift = 128 - u32::from(prefix);
    let mask = u128::MAX.checked_shl(shift).unwrap_or(0);
    u128::from(addr) & mask
}

/// Outcome of an allowlist check.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) enum AllowResult {
    /// Gate disabled or source IP matched a CIDR — request may proceed.
    Allow,
    /// Source IP did not match any CIDR. Caller should return
    /// `403 Forbidden` with an empty body.
    Deny,
}

/// CIDR allowlist gate. Cheap to clone (the CIDR list is small —
/// operators list ~1–10 entries in practice).
#[derive(Debug, Clone)]
pub struct IpAllowlist {
    enabled: bool,
    cidrs: Vec<Cidr>,
    trust_xff_hops: u8,
}

impl IpAllowlist {
    /// Build from the parsed `[security.ip_allowlist]` block. Returns
    /// an error if any entry is malformed — the gateway refuses to
    /// start on bad config rather than silently widening the gate.
    pub fn from_config(cfg: &IpAllowlistConfig) -> Result<Self, IpAllowlistError> {
        let cidrs = cfg
            .cidrs
            .iter()
            .map(|s| Cidr::parse(s))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            enabled: cfg.enabled,
            cidrs,
            trust_xff_hops: cfg.trust_x_forwarded_for_hops,
        })
    }

    /// `true` when the gate is enabled — operators want to know
    /// whether the gateway boot banner should mention the allowlist.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Number of configured CIDRs. Used by the boot banner.
    pub fn cidr_count(&self) -> usize {
        self.cidrs.len()
    }

    /// Check the request against the gate. `req` is the raw HTTP
    /// wire form; `peer` is the kernel-reported source [`SocketAddr`].
    /// Returns [`AllowResult::Allow`] when the gate is disabled or
    /// the source IP matches, otherwise [`AllowResult::Deny`].
    pub(super) fn check(&self, req: &str, peer: SocketAddr) -> AllowResult {
        if !self.enabled {
            return AllowResult::Allow;
        }
        let src_ip = self.source_ip(req, peer);
        if self.cidrs.iter().any(|c| c.contains(src_ip)) {
            AllowResult::Allow
        } else {
            AllowResult::Deny
        }
    }

    /// Pick the source IP to evaluate. `trust_xff_hops = 0` → peer
    /// IP. Otherwise read `X-Forwarded-For` and take the entry `n`
    /// positions from the right (the IP injected by the first
    /// untrusted peer). Falls through to the peer IP on missing
    /// header / parse failure / too-short chain.
    pub(super) fn source_ip(&self, req: &str, peer: SocketAddr) -> IpAddr {
        if self.trust_xff_hops == 0 {
            return peer.ip();
        }
        xff_after_hops(req, self.trust_xff_hops).unwrap_or_else(|| peer.ip())
    }
}

/// Walk `X-Forwarded-For` and return the entry `hops` positions from
/// the right of the chain. Header lookup is case-insensitive per RFC
/// 7230 §3.2. Returns `None` if the header is missing, the chain is
/// shorter than `hops`, or the picked entry does not parse as an IP.
pub(super) fn xff_after_hops(req: &str, hops: u8) -> Option<IpAddr> {
    if hops == 0 {
        return None;
    }
    let header_end = req.find("\r\n\r\n").unwrap_or(req.len());
    let headers = &req[..header_end];
    // Skip the request-line; only header lines carry a `:` pair.
    for line in headers.split("\r\n").skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if !name.trim().eq_ignore_ascii_case("x-forwarded-for") {
            continue;
        }
        let entries: Vec<&str> = value.split(',').map(str::trim).collect();
        // From-the-right indexing: hops=1 → the rightmost entry. The
        // rightmost entry is what the *first* trusted proxy added,
        // i.e. the IP of the peer that connected to that proxy.
        let idx = entries.len().checked_sub(usize::from(hops))?;
        return entries.get(idx)?.parse::<IpAddr>().ok();
    }
    None
}

/// Build the raw HTTP/1.1 403 response for a denied request. Empty
/// body — the deny path does not leak which CIDR or header was unhappy.
pub(super) fn deny_response_bytes() -> Vec<u8> {
    let resp = "HTTP/1.1 403 Forbidden\r\n\
                Content-Length: 0\r\n\
                Connection: close\r\n\r\n";
    resp.as_bytes().to_vec()
}

/// Emit the deny tracing event so operators see attempted-access
/// patterns via the existing log-shipping pipeline. Separated from
/// the deny-bytes helper so callers can sequence "emit log → write
/// bytes" without holding the formatted body across an `await`.
pub(super) fn log_deny(src_ip: IpAddr, req: &str) {
    let (method, path) = request_method_and_path(req);
    tracing::warn!(
        target: "gateway::ip_allowlist",
        ip = %src_ip,
        method = %method,
        path = %path,
        "ip allowlist deny"
    );
}

/// Parse `METHOD PATH` from the first line of an HTTP request.
/// Strips the query-string from the path (it may carry secrets the
/// log pipeline should not collect). Returns `("", "")` for malformed
/// input — the caller is logging on the deny path, not trusting it
/// for routing.
fn request_method_and_path(req: &str) -> (&str, &str) {
    let line = req.split("\r\n").next().unwrap_or("");
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let raw_path = parts.next().unwrap_or("");
    let path = raw_path.split('?').next().unwrap_or(raw_path);
    (method, path)
}

#[cfg(test)]
#[path = "ip_allowlist_tests.rs"]
mod tests;
