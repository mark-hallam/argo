# IP Allowlist

The gateway can enforce a source-CIDR allowlist before bearer-token
authentication. This is a defence-in-depth gate: a leaked token alone
is not sufficient for access when the request source IP falls outside
the configured CIDRs.

Configure it in `argo.toml`:

```toml
[security.ip_allowlist]
enabled = true
cidrs = ["10.0.0.0/8", "192.168.1.0/24", "2001:db8::/32"]
trust_x_forwarded_for_hops = 1
```

## Proxy Hops

`trust_x_forwarded_for_hops` is the number of trusted proxy hops in
front of the gateway:

| Deployment | hops |
| --- | --- |
| Direct, no proxy | `0` |
| nginx in front | `1` |
| nginx plus AWS ELB, or two-layer L7 | `2` |
| nginx plus ELB plus CloudFront | `3` |

`0` ignores `X-Forwarded-For` entirely and uses the kernel-reported
peer IP. With `n > 0`, the gate takes the entry `n` positions from the
right of the `X-Forwarded-For` chain. Setting the value higher than the
real proxy chain lets a hostile client spoof a trusted IP by stuffing
the header; setting it lower trusts a proxy you did not deploy.

## Fail-Closed Mode

`enabled = true` with `cidrs = []` is deny-all, not allow-all. Empty
list with the gate on is treated as an operator misconfiguration; every
request is rejected with `403 Forbidden`. To disable the gate, set
`enabled = false`, which is the default.

## Observability

Deny events emit a `tracing::warn!` under target
`gateway::ip_allowlist` with structured `ip`, `method`, and `path`
fields. The path is query-stripped because query strings may carry
secrets. A later audit-chain brief can promote these to `audit_record`
rows once the audit kind is in scope.

## Non-Goals

This gate is CIDR-only. Hostname allowlists, GeoIP filtering, DDoS
mitigation, and per-source rate limiting are separate concerns.
