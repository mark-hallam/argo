# Zero-Trust Networking for Multi-Host SKYCOT

**Status**: Design doc (no code). Blueprint for the multi-host security
hardening that lands in follow-up briefs.
**Audience**: operators planning a >1-host SKYCOT deployment;
contributors implementing the follow-up briefs named in §9.
**Scope**: gateway↔gateway, gateway↔worker, gateway↔store, and
worker↔external-API trust. **Out of scope**: end-user authentication
(covered by `mission-sso-saml-oidc-gateway`), tenant data-at-rest
encryption (`mission-encryption-at-rest-credentials-toml`), and the
choice of service mesh (operator concern, see §7).

Authored against `origin/main` `c0fcf689` on 2026-05-12. The "current
state" inventory in §1 is a point-in-time snapshot — re-grep the cited
paths before quoting any specific line.

## 0. TL;DR

Single-host SKYCOT today implicitly trusts that every component shares
the same loopback. The first multi-host deployment will expose that
assumption in at least five places — bearer-token-over-cleartext on the
peer mesh, an `ARGO_GATEWAY_AUTH_BYPASS=1` escape hatch, a single
HMAC secret with no per-peer rotation, principal headers parsed
without cryptographic proof, and a SQLite-on-disk store assumed
co-located with the gateway. The design recommended below adds:

1. **Identity = per-host x509, mTLS-only inter-service.** One trust
   root per SKYCOT deployment; one leaf certificate per host;
   short-lived (24h) leaves issued by an in-binary CA fronted by the
   conductor or, for larger deployments, an operator-supplied
   cert-manager / step-ca. SPIFFE remains the natural V3 upgrade;
   PSK is rejected as a primary.
2. **Authorization = capability tokens scoped to caller × resource ×
   action × expiry.** Capability tokens piggyback on the mTLS channel
   (the channel proves identity; the token proves intent). Tokens are
   short-lived (≤5min) and issued by `argo-capability` on the
   authoritative host.
3. **Opt-in migration via `[security.zero_trust].enabled = true`.**
   Single-host operators see no change. Multi-host operators turn
   the switch on; the gateway then refuses to boot if the trust
   bundle is absent or any peer connection is plaintext.

The doc below justifies each pick and names ≥5 follow-up briefs that
implement it.

## 1. Current trust assumptions — audit

Today's code treats "another SKYCOT process" as trustworthy if it can
either (a) bind to the same loopback or (b) present a single shared
bearer secret. Concrete instances:

| Boundary | Today's assumption | Code reference |
|---|---|---|
| Gateway bind | Loopback-only by default; operator can override to `0.0.0.0` with no warning | `crates/argo-config/src/defaults.rs:163` (`DEFAULT_GATEWAY_HOST = "127.0.0.1"`); `crates/argo-cli/src/handlers/gateway/boot.rs:78-99` |
| Gateway URL | Hardcoded `http://` (no TLS) in defaults | `crates/argo-config/src/defaults.rs:170` (`DEFAULT_GATEWAY_URL = "http://127.0.0.1:8787"`) |
| Principal assertion | `X-Argo-Principal` header parsed and trusted; anonymous fallback | `crates/argo-cli/src/handlers/gateway/routes_auth_principal.rs:10-41` |
| Capability binding | `CapabilityEngine` binds principal → role without proving how the principal arrived | `crates/argo-capability/src/lib.rs:42-146` |
| Approval-token signing | Single HMAC secret, environment-wide | `crates/argo-channels/src/token.rs:54-74` (sign), `:120-131` (verify) |
| Peer mesh auth | Single shared bearer (`ARGO_PEER_SECRET`); plaintext HTTP; bypass via `ARGO_GATEWAY_AUTH_BYPASS=1` | `crates/argo-cli/src/handlers/gateway/peer.rs:86-89,190,272`; `routes_peer.rs:66-87` |
| Store connection | Local SQLite file path; no network DB driver | `crates/argo-store/src/lib.rs:236-277`; `crates/argo-cli/src/home.rs:23-25` |
| Inter-process TLS | None. No `rustls` / `native-tls` / `x509` deps in store, gateway, or CLI | `crates/argo-store/Cargo.toml`, `crates/argo-cli/Cargo.toml` (gateway lives under `argo-cli/src/handlers/gateway/`) |
| Config schema | No `[security]` / `[tls]` / `[trust]` blocks exist to extend | `crates/argo-config/src/sections.rs:1-68` |

The pattern: every inter-service hop either trusts the network
(loopback assumption) or trusts a single static credential (peer
bearer, HMAC secret). Neither survives an adversary on the LAN.

## 2. Threat model

Three adversaries motivate the design. For each: capability, what
the current code does, what the design must defend.

### 2.1 Adversary on the network (inside the perimeter)

**Capability**: passive sniff + active TCP injection between any
two SKYCOT hosts. Realistic in any colo, shared k8s cluster, or
office LAN.

**Today**: peer mesh bearer + principal header travel in plaintext.
Sniff once, replay forever (bearer is long-lived by design — see
`peer.rs:86`).

**Design must**: encrypt every inter-service hop, authenticate both
ends of the hop (not just the client), and bind credentials to a
short-lived channel so replay is bounded by token TTL.

### 2.2 Adversary on a compromised worker host

**Capability**: full code-execution on one SKYCOT host. Goal:
escalate to other tenants' data or to other hosts' workers.

**Today**: worker can read `ARGO_PEER_SECRET` from its own env →
authenticate as any peer → call `/peer/sync` against any host → pull
every tenant's session headers + audit records up to the limit
(`routes_peer.rs:130-160`).

**Design must**: scope credentials to (host-identity × tenant ×
resource-class × action). A compromised worker on host A must not
be able to read tenant B's data on host B even if A is in B's peer
list. Capability tokens enforce the per-call narrowing; mTLS host
identity enforces the per-peer narrowing.

### 2.3 Supply-chain compromise of the SKYCOT binary

**Capability**: adversary controls the published `argo` binary (or
a dependency that ends up linked into it). Detection-focused, not
prevention-focused — once the trusted code lies, no in-process
defense holds.

**Today**: no reproducibility story; no signing; no SBOM gate.

**Design must** (modest scope): emit cryptographic identity hashes
in audit records so operators can detect drift between expected and
running binary on a peer. Full reproducibility is out of scope for
this brief (separate `mission-supply-chain-attestation` follow-up).

## 3. Identity primitives

Three serious options. Recommendation up front, then the
side-by-side and the trade-offs of the alternatives.

### 3.1 Recommendation: per-host x509 + mTLS

Every SKYCOT host gets exactly one x509 leaf certificate, identifying
the host by a stable `host_id` (UUID) in the certificate SAN. All
inter-service connections (gateway↔gateway, gateway↔worker,
gateway↔store) use mTLS with both ends presenting certificates from
the same trust root.

**Why this and not the others**:

- mTLS is the well-trodden path. `rustls` is workspace-quality,
  audited, and already a transitive dep through `reqwest` for
  outbound provider HTTPS.
- Cert-manager / step-ca / Vault PKI all speak x509 natively, so
  operators with existing PKI infra don't have to learn a new
  primitive.
- The migration story is incremental: today's bearer-over-HTTP peer
  mesh becomes bearer-over-mTLS in V1, and the bearer drops out in
  V2 once capability tokens land.
- It composes with capability tokens cleanly: TLS proves "who you
  are at the channel level"; tokens prove "what you may do on this
  call." The two concerns stay separate.

**Trust root**: in-binary CA managed by the conductor host (the
"primary" in `MULTI_HOST_ORCHESTRATOR.md`'s V2.x topology). For
larger deployments, operators substitute an external issuer
(cert-manager, step-ca, Vault PKI) by pointing
`[security.zero_trust].ca_mode = "external"` and supplying the bundle.

**Leaf lifetime**: 24h. Long enough that a routine re-issue isn't
hot-path; short enough that a stolen leaf burns itself out before
the next on-call rotation. Rotation is automated and on by default
(see `mission-automated-key-rotation`).

### 3.2 Alternative: SPIFFE workload identity

**Pros**: stronger workload-identity story (SPIFFE IDs are namespaced
URIs, not bare hostnames); attestation hooks for cloud workload
APIs (AWS IMDS, GCP metadata, k8s service accounts); battle-tested
at large scale (Bloomberg, Uber).

**Cons**: requires a SPIRE server (or equivalent) as additional
infra; operationally heavier; the workload-attestation step is the
hardest part and we'd be implementing it from cold. The
`spiffe-bundle` Rust crate is one-maintainer at the time of writing.

**Defer to V3**, gated on either (a) ≥10-host deployments in the
field where bespoke cert rotation becomes painful, or (b) SKYCOT
Cloud needing federation across customer-controlled boundaries.
The mTLS choice is forward-compatible — SPIFFE IDs are themselves
x509-based.

### 3.3 Rejected: per-host PSK

**Pros**: one shared secret per pair; trivial to bootstrap; survives
the no-PKI-infra case.

**Cons**: rotation is O(N²) — every host needs every other host's
PSK, and rotating one means a coordinated re-distribution. Doesn't
survive a host compromise (an attacker with one host's PSK can
impersonate it to every peer). No revocation primitive short of
"reissue everything."

This is the shape of today's `ARGO_PEER_SECRET`. We are leaving it,
not enshrining it.

## 4. Per-RPC authorization

Identity (§3) answers "who is calling." Authorization answers "may
this caller do this thing right now."

### 4.1 Capability tokens

Scoped tokens, issued by `argo-capability` on the authoritative host,
present on every RPC. Each token binds:

- `caller_identity` — the `host_id` from the mTLS leaf SAN, OR an
  end-user principal when the call is operator-initiated.
- `target_resource` — typed handle: `session:<uuid>`,
  `tenant:<uuid>`, `audit:tenant:<uuid>`, `worker:<uuid>`. Bare
  wildcards (`session:*`) are forbidden on the wire; the issuer may
  expand to specific IDs but tokens always carry concrete targets.
- `action` — enum: `Read`, `Write`, `Run`, `Cancel`, `Admin`. Mirrors
  the existing `argo-capability` `Action` enum.
- `expiry` — absolute Unix-micros, ≤5 minutes from issue.
- `issuer_host_id` — the host that minted the token, for revocation.
- `nonce` — 16 random bytes, used by replay-detection cache.

Token format: COSE_Sign1 over the above CBOR-encoded claims, signed
with the issuer's leaf private key. CBOR over JWT because (a) we
already CBOR-encode audit payloads in places, (b) COSE is the IETF
counterpart to JWT for non-JSON contexts, (c) shorter on the wire.

### 4.2 Validation pipeline

Receivers run a fixed pipeline on every authenticated RPC:

1. mTLS handshake completes; pin caller `host_id` from leaf SAN.
2. Extract capability token from `Authorization: Capability <b64>`.
3. Verify COSE signature against the trust bundle.
4. Reject if `caller_identity` ≠ pinned `host_id` (channel/token
   identity mismatch = active MITM attempt or stale token).
5. Reject if `expiry` < now (with 30s skew tolerance).
6. Check `nonce` against the per-tenant replay cache (LRU,
   2× expiry window). Reject if present; insert otherwise.
7. Defer to `argo-capability` for the (principal, resource, action)
   policy check.

Steps 1–5 are stateless per receiver. Step 6 is the only stateful
piece and the natural place to instrument with metrics
(`zero_trust.replay_rejected`).

### 4.3 Why two layers

A common objection: "mTLS already authenticates; why a separate
token?" Two reasons:

1. **Granularity.** A leaf certificate proves "I am host B." It
   does not prove "host B should be allowed to read this specific
   tenant's session right now." Capability tokens carry that
   per-call narrowing without re-issuing leaves.
2. **Delegation.** Operator-initiated actions ("user U on host A
   asks host B to cancel session S") need a credential that carries
   U's authority, not A's. Tokens do this; bare mTLS does not.

## 5. Migration sequence

Three numbered phases. Each phase is a separate follow-up brief; the
named briefs in §9 below map onto them.

### Phase M1 — Trust bundle plumbing (no behavior change)

- Add `[security.zero_trust]` config section, default `enabled = false`.
- Add `rustls` + `tokio-rustls` deps to gateway, store, CLI.
- Implement `argo-identity::TrustBundle` type that loads CA + leaf
  cert + leaf key from filesystem paths (no issuance yet).
- Add health-check endpoint `/healthz/zero-trust` that reports
  bundle presence + leaf expiry; useful for operators wiring up
  external cert-manager before flipping the switch.

Single-host operators (`enabled = false`) are unaffected.

### Phase M2 — mTLS inter-service (opt-in)

- When `enabled = true`, gateway listener requires client certs on
  every non-end-user route (`/peer/*`, future `/store/*`,
  `/worker/*`). Public routes (operator browser → gateway) stay on
  the existing TLS termination path (see
  `mission-gateway-tls-termination-or-proxy-doc`).
- When `enabled = true`, all outbound peer client connections use
  mTLS; cleartext peer URLs are refused at config load.
- Boot-time refusal: if `enabled = true` AND the bundle is absent
  or any leaf is past `expiry - 1h`, refuse to boot with a clear
  error pointing at this doc.

### Phase M3 — Capability tokens on every RPC

- `argo-capability` gains COSE issuance + verification.
- All `/peer/*` handlers require a valid capability token alongside
  the mTLS channel. `ARGO_PEER_SECRET` is deprecated and ignored
  when `enabled = true`; logged-warn-and-honored when `enabled =
  false` for backwards compat.
- `ARGO_GATEWAY_AUTH_BYPASS` is removed entirely (not gated): it is
  a development convenience and has no place in any production
  shape.

### Per-tenant rollout

`[security.zero_trust]` is global-only in V1; per-tenant policy is
deferred. The operator who turns this on does so for the whole
deployment. Per-tenant variation is an interesting future feature
but adds matrix complexity that isn't justified until we have
real multi-tenant multi-host deployments in the wild.

## 6. What this brief does NOT decide

- **Service mesh.** Operators may front SKYCOT with Consul Connect,
  Linkerd, Istio, or nothing at all. The mTLS design here works
  whether the mesh terminates TLS at a sidecar or whether SKYCOT
  terminates it natively. We do not bless a mesh.
- **End-user authentication.** SSO / SAML / OIDC for the
  operator-facing gateway is covered by
  `mission-sso-saml-oidc-gateway`. Zero-trust here is
  inter-service, not end-user.
- **Data-at-rest encryption.** The SQLite store on disk is covered
  by `mission-encryption-at-rest-credentials-toml`. This doc only
  addresses the wire.
- **Network egress policy.** Worker → external provider API trust
  is real (a compromised provider can poison responses) but the
  defense (response signing, provider attestation) is a separate
  problem from inter-service zero-trust. Tracked but not designed
  here.
- **Reproducible builds + supply-chain attestation.** Threat 2.3
  is acknowledged but the mitigation is a separate brief.

## 7. Cross-cutting concerns

### 7.1 Single-host operators

`enabled = false` is the default and the single-host SKYCOT story
is unchanged: localhost bind, no certs, no tokens. The doc states
this explicitly because it is the load-bearing operational
guarantee.

### 7.2 Cert issuance UX

V1: operator-supplied paths to PEM files. Most operators with
multi-host appetite already run cert-manager or step-ca; this is
the path of least surprise. The conductor-as-CA mode is the
"laptop-mesh" fallback; it ships with a `argo trust init` CLI
subcommand (see `mission-trust-init-cli`) that materializes the CA
on first run and issues leaves on `argo trust enroll <host>`.

### 7.3 Observability

Every accept/reject decision emits an `argo_audit` record with
fields: `caller_host_id`, `target_resource`, `action`, `decision`,
`reason_code`. Reason codes are an enum (`OK`,
`ExpiredToken`, `NonceReplay`, `IdentityMismatch`,
`PolicyDenied`, ...) so dashboards can count without parsing
strings. This is load-bearing — operators must be able to tell a
misconfigured peer from an active attack.

### 7.4 Failure modes

- **Clock skew kills tokens.** 5-minute lifetime + 30s skew
  tolerance is tight on purpose; operators are expected to run NTP.
  The boot check (Phase M2) emits a warn when `chrony` /
  `timesyncd` aren't running.
- **Cert near-expiry on a peer.** Each host gossips its own
  `leaf_expiry` over the peer state endpoint so a stale peer can
  be detected from any other peer's `argo trust status` output.
- **Bundle missing on boot.** Fail closed: refuse to start. Do not
  fall back to plaintext.

## 8. Code locations needing rework

Each of these gets a follow-up brief in §9. Counts and line numbers
are snapshots from the audit in §1 — re-grep before quoting.

- `crates/argo-cli/src/handlers/gateway/peer.rs` — bearer-token
  client to be wrapped in mTLS + capability tokens.
- `crates/argo-cli/src/handlers/gateway/routes_peer.rs:66-87` —
  `check_peer_auth` to become a 2-stage check (mTLS + token), and
  `ARGO_GATEWAY_AUTH_BYPASS` to be removed.
- `crates/argo-cli/src/handlers/gateway/boot.rs:78-99` — listener
  to grow a TLS acceptor, with bind-non-loopback refusing to start
  in `enabled = false` mode.
- `crates/argo-channels/src/token.rs:54-131` — HMAC primitive to
  become COSE_Sign1 with per-issuer keys, keeping HMAC for
  webhook-signature back-compat only.
- `crates/argo-capability/src/lib.rs:42-146` — `CapabilityEngine`
  to gain a `verify_token` entrypoint that consumes a wire-format
  capability token before reaching the existing
  (principal, resource, action) check.
- `crates/argo-identity/src/lib.rs` — new `TrustBundle` type and
  loader; `host_id` extraction from a presented client cert.
- `crates/argo-config/src/sections.rs:1-68` —
  `[security.zero_trust]` section with `enabled`, `ca_mode`,
  `ca_path`, `leaf_path`, `key_path`, `ca_mode = "internal" |
  "external"`, `token_lifetime_secs` (default 300).
- `crates/argo-config/src/defaults.rs:163-170` — defaults stay as
  today; mTLS only kicks in when `enabled = true`.

## 9. Follow-up briefs

The implementation lands as the following briefs. Each is a
separate row on `BUILD_STATUS.md`; the conductor queues them in
this order.

1. **`mission-zero-trust-config-section`** (Phase M1, small).
   Adds `[security.zero_trust]` config block + defaults +
   `health/zero-trust` endpoint. No runtime behavior change.
2. **`mission-trust-bundle-loader`** (Phase M1, small).
   `argo-identity::TrustBundle` PEM loader; refuses malformed
   bundles; emits `leaf_expiry` metric.
3. **`mission-trust-init-cli`** (Phase M1, medium). `argo trust
   init` (materialize in-binary CA), `argo trust enroll <host>`
   (issue a leaf), `argo trust status` (list peers + expiries).
4. **`mission-mtls-gateway-listener`** (Phase M2, medium). Wrap
   the gateway accept loop with `tokio-rustls`; enforce client
   certs on `/peer/*`; bind-non-loopback refuses when
   `zero_trust.enabled = false`.
5. **`mission-mtls-peer-client`** (Phase M2, small). Outbound
   peer client uses mTLS; cleartext peer URLs refused at config
   load when `enabled = true`.
6. **`mission-cose-capability-tokens`** (Phase M3, medium).
   `argo-capability` gains COSE issuance + verification; the
   five-step validation pipeline lands.
7. **`mission-capability-token-on-peer-rpcs`** (Phase M3,
   medium). All `/peer/*` handlers require a valid capability
   token; `ARGO_PEER_SECRET` deprecated; `ARGO_GATEWAY_AUTH_BYPASS`
   removed.
8. **`mission-zero-trust-audit-records`** (Phase M3, small).
   Reason-coded audit records for every accept/reject decision;
   `zero_trust.*` metrics surface in the operator dashboard.
9. **`mission-automated-key-rotation`** (already claimed) —
   24h leaf rotation lands here; the design assumed in §3.1 is
   already in flight.

Total: 8 new briefs + 1 already in flight. Phase M1 ships first
and is safe to merge with `enabled = false` left as the default;
Phase M2 and M3 are gated behind the flag and roll out per-operator.

## 10. References

- `docs/skycot/MULTI_HOST_ORCHESTRATOR.md` — V2.x peer-mesh design
  (the trust gaps audited in §1 originate here).
- `docs/skycot/SECURITY.md` — current high-level security posture
  (single-host).
- `docs/build-plan/streams/mission-gateway-tls-termination-or-proxy-doc.md`
  — the operator-facing TLS termination story (Path A native or
  Path B reverse-proxy). The zero-trust mTLS design here is
  inter-service; that brief handles end-user-facing TLS.
- `docs/build-plan/streams/mission-sso-saml-oidc-gateway.md` —
  end-user auth (orthogonal).
- `docs/build-plan/streams/mission-automated-key-rotation.md` —
  rotation of the leaves §3.1 assumes.
- RFC 8392 (CWT) and RFC 9052 (COSE) — wire formats for §4.1.
- SPIFFE spec (`spiffe.io`) — the deferred V3 alternative.
