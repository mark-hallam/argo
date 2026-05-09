# Plugin System — V3.x Design Doc

**Status:** design (no code lands in this doc).
**Audience:** anyone preparing to extract `argo-plugin-api`,
add a new executor kind, ship a NotifyChannel variant, or
distribute a Genesis vertical pack out-of-band.
**Spec id:** `claude-plugin-system-design`. Successor brief:
`claude-plugin-api-crate`.

## Why this doc

V3.0 is closing in on feature-completeness for the in-tree
verticals (gtm-engineer, ai-receptionist, sales-sdr, …).
The next axis of growth is **third-party extensibility**:
customers want to ship verticals without forking the
workspace, partners want executors that hit SaaS APIs we
have no business knowing about, and the OSS ecosystem
wants to publish NotifyChannel adapters (Teams, Discord,
RocketChat) on its own cadence.

This doc fixes the *shape* of the plugin system so the
next brief — extracting `argo-plugin-api` — can derive
crate boundaries, ABI rules, and semver policy without
revisiting first principles. It makes one load-model
recommendation (subprocess + JSON-RPC, already partially
implemented in `crates/argo-plugin`), pins the V3.0
minimum security bar, and names the dogfood candidates.

The single most important decision is the loading model;
that is §2. Everything else is mechanism.

---

## 1. Extension points

A plugin can extend Argo through three surfaces. Anything
not listed here stays in-tree for V3.x.

### 1.1 `Executor` kinds (third-party action handlers)

Each Worker v3 action carries a typed `kind` (e.g.
`post_listing`, `send_email`, `boost_listing`). Today the
`Executor` impls live as separate crates
(`argo-executor-document`, `argo-executor-filesystem`,
`argo-executor-computer-use`). Plugins extend this set.

A plugin executor declares:

- `kind: &str` — wire id matched against the manifest
  `action_type`.
- `permissions: &[Permission]` — see §4.1.
- `async fn execute(&self, action: Action, ctx:
  ExecutorCtx) -> ExecutorResult`.

The host loads exactly one executor per `kind`. Conflicts
(two plugins claiming the same kind) fail load with a
`PluginConflict` error. Tenant-scoped overrides are out of
scope for V3.0.

### 1.2 `NotifyChannel` variants (approval routing)

`crates/argo-channels` already exposes a transport-neutral
`NotifyChannel` trait (`send`, `poll`, `id`). In-tree
implementations cover Slack, Email, SMS, iMessage, Discord,
and push. Plugins add channels that ship out-of-tree —
Microsoft Teams, RocketChat, PagerDuty pages, customer
in-product notifications.

A plugin channel declares:

- `id: ChannelId` — globally unique.
- `permissions` (typically `network_egress` for the
  provider's API plus a credential reference).
- `async fn send(...)` and `async fn poll(...)` matching
  the in-tree trait.

### 1.3 Genesis templates (vertical packs)

`crates/argo-genesis` ships 12 archetypes today
(receptionist, sales-sdr, …). Plugins distribute additional
*vertical packs* — a manifest, a prompt pack, memory schemas,
tool permissions, escalation policy, and seed eval cases —
without compiling any Rust. A vertical pack plugin is
declarative: the plugin host loads its TOML/JSON manifest
and prompt files; Argo's existing vertical layer handles
the rest. No new trait surface required beyond a single
`vertical_pack_root()` accessor.

### 1.4 Explicitly out of scope for V3.x

The following stay in-tree. They are too load-bearing,
security-sensitive, or invariant-critical to pluginize on
this release:

- **LLM providers.** Provider routing/manifests are
  Invariant 4 (TOML manifests, not Rust adapters); a
  plugin trait would be redundant and would let a plugin
  intercept every model call.
- **Storage backends.** SQLite/Postgres choice is a
  deployment knob (`argo.toml`), not a plugin axis.
- **Audit sinks.** Audit must be append-only and tamper-
  evident; a plugin in the audit path is a regression on
  Priority 4 (security).
- **Identity / RBAC providers.** Authentication is
  Invariant 3 territory.

Re-opening any of these requires a new design doc.

---

## 2. Loading model

### 2.1 Trade-off table

| Option | Pros | Cons | ABI burden | Crash blast radius |
|---|---|---|---|---|
| **Subprocess + JSON-RPC** | Crash isolation; OS-level sandbox available (seccomp, sandbox-exec, job objects); language-agnostic (Python, TS, Go); easy to version on the wire | One process per plugin; ~5ms RPC overhead per call; serialization tax | **Wire format only** — JSON schemas + a `protocol_version` constant; no Rust-ABI fragility | Plugin OOM/panic confined to its subprocess; gateway lives |
| **Dynamic load (libloading / `cdylib`)** | Zero-copy data; ~ns-scale call overhead; one process | Rust has **no stable ABI** — must export `extern "C"` and hand-roll FFI; plugin compiled against exact rustc + crate versions; one panic in the plugin unwinds into the gateway and corrupts allocator state | Severe — every internal type the plugin sees has to be ABI-stable; semver bump = recompile world | Whole gateway dies; SIGSEGV is non-recoverable |
| **Static cargo feature** | Simplest; full type system; zero wire overhead | Requires **forking the workspace** to add a vertical-specific plugin — defeats the whole point of an extension system; no third-party distribution | None | Same as core code |
| **WASM (wasmtime / component model)** | Strong sandbox; deterministic execution; cross-language | Async story still maturing in 2026; no native sockets without WASI-Sockets; binary size; debuggability worse than subprocess | Wire format + WASM component schema | Confined to wasmtime instance |

### 2.2 Recommendation: **Subprocess + JSON-RPC**

This matches **Invariant 1** ("Polyglot core: Rust core +
JSON-RPC plugin ABI; plugins are subprocesses") and is
already partly implemented in `crates/argo-plugin`
(`PluginHost`, line-delimited JSON-RPC 2.0 over
stdin/stdout, `PROTOCOL_VERSION = "0.1.0"`).

Reasons, ranked by Argo's priority list:

1. **Reliability (P1).** Crash isolation at the process
   boundary is non-negotiable for a multi-tenant gateway.
   A `cdylib` plugin segfault kills every tenant's
   workers. A subprocess plugin loses one job.
2. **Observability (P2).** Plugin stderr is captured per-
   process and tagged with the plugin id; resource usage
   (RSS, CPU) is visible to the host without instrumentation.
3. **Security (P4).** OS-level isolation primitives
   (`sandbox-exec` on macOS, `seccomp` + namespaces on
   Linux, job objects on Windows) attach to subprocesses.
   In-process plugins inherit the gateway's ambient
   authority.
4. **Polyglot.** Receptionist verticals built by
   integrators in Python or TypeScript should not need a
   Rust toolchain.

The two real costs — RPC overhead and serialization — are
bounded. Plugin calls are off the hot path: they happen
once per action emission, once per approval notification,
and once per Genesis template load. A 5ms RPC budget is
two orders of magnitude under any user-visible threshold.

WASM is a credible *future* upgrade for sandboxing without
forking processes, but the component-model async story is
still rough enough in 2026 that committing to it now would
trade a working subprocess host for a half-finished WASM
host. We can layer WASM in later behind the same wire
protocol without breaking plugin authors. Subprocess +
JSON-RPC is the V3.x default.

### 2.3 ABI implications of the recommendation

The "ABI" is the **wire protocol**, not Rust types:

- JSON-RPC 2.0 line-delimited framing (one object per
  newline; the framing is already pinned).
- A pinned `protocol_version` string sent in the
  handshake (currently `"0.1.0"`).
- Versioned method schemas: `pre_tool_call`, `execute`,
  `notify_send`, `notify_poll`, `vertical_pack_load`,
  `shutdown`. New methods are additive; removed or
  reshaped methods bump the major version.

There is no Rust-ABI stability surface. A Rust-based
plugin links its own private copy of `serde` + tokio; the
host only sees bytes on a pipe.

---

## 3. Versioning + ABI

### 3.1 Crates

Two new crates fall out of this design (lands in
`claude-plugin-api-crate`):

- **`argo-plugin-api`** — pure types: `Action`,
  `ExecutorCtx`, `Notification`, `VerticalPackManifest`,
  the `protocol_version` constant, JSON schemas, error
  enums. **No tokio**. Zero runtime dependencies. SemVer
  governed.
- **`argo-plugin`** (already exists) — host-side
  subprocess runner (`PluginHost`, framing, timeouts,
  crash detection). Argo-internal; does not constrain
  plugin authors.

Plugin authors depend only on `argo-plugin-api` (or its
JSON schemas, if they're in another language).

### 3.2 SemVer policy

Standard Cargo semver with two amplifications:

- **`protocol_version` is the contract.** A SemVer-major
  bump of `argo-plugin-api` MUST bump `protocol_version`.
  A minor bump MAY add methods/fields; old plugins keep
  working. A patch bump is wire-identical.
- **Wire schemas are the source of truth.** Generated
  Rust types are a convenience for Rust authors. The JSON
  schemas under `crates/argo-plugin-api/schemas/` ship in
  the package and are the artifact a TypeScript or Python
  SDK consumes.

### 3.3 Breaking-change policy

- Removing a method, removing a request field, or
  changing the type of an existing field is a major bump.
- Adding a method, adding an optional field with a
  default, or widening an enum is a minor bump.
- Plugins MAY pin to a `^0.x` range during V3.x; once V4
  ships, plugins SHOULD pin to `^N.0` and the host
  refuses load on cross-major mismatch.

### 3.4 Compatibility matrix

The host advertises a `supported_protocol_versions: &[&str]`
slice. A plugin's handshake declares its
`protocol_version`. The host accepts any plugin whose
version is in the supported slice; it logs a deprecation
warning when the plugin's version is one minor below the
host's preferred version, and refuses load otherwise.

Concretely for V3.0:

| Host version | Supported plugin versions |
|---|---|
| Argo 0.3.x | 0.1.x |
| Argo 0.4.x (planned) | 0.1.x, 0.2.x |
| Argo 1.0.x | 0.2.x, 1.0.x |

The matrix is shipped in `argo plugin doctor` output and
in the marketplace metadata so operators can audit before
install.

---

## 4. Security model

Argo runs multi-tenant workloads against real customer
data. A plugin gets the **least authority** it asks for.

### 4.1 Permissions (declared at load time)

A plugin's manifest declares a permissions block. The host
refuses load if the operator hasn't granted the requested
set in `argo.toml`. V3.0 permission vocabulary:

- `network_egress: [host_pattern, …]` — outbound HTTP/HTTPS
  to a glob list (`*.zendesk.com`, `api.stripe.com`).
  Default deny.
- `fs_read: [path_pattern, …]` — read access to specific
  paths under `~/.argo/` and `.skycot/`. Default deny.
- `fs_write: [path_pattern, …]` — write access. Default
  deny; commonly granted to `.skycot/outputs/<job_id>/**`.
- `executor_kinds: [string, …]` — wire ids the plugin may
  register.
- `notify_channels: [string, …]` — channel ids the plugin
  may register.
- `vertical_packs: [string, …]` — pack ids it may
  contribute.
- `secrets: [secret_id, …]` — references resolved by the
  host's secrets layer; the plugin sees opaque tokens, not
  the secret material at rest.

Capabilities map directly onto Argo's existing
`argo-capability` crate, so RBAC enforcement is reused.

### 4.2 Tenant isolation

V3.0 baseline: **one host, one plugin process per plugin
binary, plugin sees `tenant_id` on every call.** The plugin
is responsible for keeping per-tenant state separate;
Argo does not partition the subprocess.

V3.x stretch: **per-tenant subprocess** spawned on demand
for plugins flagged `isolation = "per-tenant"` in the
manifest. Cost is one extra process per active tenant per
plugin; benefit is hard isolation for plugins that talk to
tenant-specific cloud accounts.

The host always plumbs `tenant_id` through the call
context (Invariant 2). A plugin that ignores it is a
plugin bug, not a host bug; capability checks happen
**before** the RPC is dispatched.

### 4.3 Code-signing

V3.0: **none required.** Plugins are operator-installed,
operator-trusted. The marketplace records SHA-256 of every
published artifact and shows it in `argo plugin show`.
Tampering detection is the operator's responsibility.

V3.x stretch: **optional Sigstore signatures.** The
marketplace records a Sigstore bundle per release;
operators can opt into `require_signatures = true` in
`argo.toml`, after which unsigned plugins refuse to load.

### 4.4 Failure isolation

- A plugin panic / OOM / crash is detected by the host's
  `PluginHost` (already implemented), the in-flight RPC
  errors with `PluginCrashed`, and the worker treats the
  call as a transient failure (retried per the executor's
  retry policy).
- The host enforces a per-call timeout
  (`DEFAULT_CALL_TIMEOUT`). Slow plugins are killed and
  replaced.
- Plugin stdout outside JSON framing is logged as a
  warning and dropped; stderr is captured into the audit
  log.
- A plugin that crashes more than N times in a window is
  quarantined with a `PluginQuarantined` audit event;
  operator must `argo plugin reset <id>` to re-enable.

### 4.5 V3.0 minimum bar vs V3.x stretch bar

| Concern | V3.0 minimum | V3.x stretch |
|---|---|---|
| Process isolation | ✅ subprocess | per-tenant subprocess |
| OS sandbox | ❌ documented risk | `sandbox-exec` / `seccomp` profile per plugin |
| Capability enforcement | ✅ at host RPC boundary | extended to fs/network at OS level |
| Signing | hash recorded; not enforced | Sigstore opt-in |
| Quarantine | ✅ on crash burst | rate-limit per (tenant, plugin) |

V3.0 ships with the documented risk that a malicious
plugin can do anything the gateway process can do, modulo
host-enforced capability checks. The marketplace and
operator docs name this risk explicitly.

---

## 5. Discovery + distribution

### 5.1 Registry shape

Plugins are described by a JSON manifest, modeled on the
skill marketplace:

```json
{
  "id": "skycot.plugin.teams-notify",
  "version": "0.3.1",
  "kind": "notify_channel",
  "summary": "Microsoft Teams approval routing",
  "binary": "https://.../teams-notify-0.3.1-aarch64-darwin.tar.gz",
  "sha256": "…",
  "protocol_version": "0.1.0",
  "permissions": {
    "network_egress": ["*.office.com", "graph.microsoft.com"],
    "secrets": ["teams_webhook_url"]
  },
  "channels": ["teams"]
}
```

The registry is a static JSON file served from
`registry.skycot.com/plugins/index.json`. CDN-cached, no
auth needed for read. Publish is a PR against the registry
repo for V3.0; an authenticated publish API is V3.x.

### 5.2 Install mechanism

```
argo plugin search teams        # query the registry
argo plugin install skycot.plugin.teams-notify@0.3.1
argo plugin list                # JSON output (per CLAUDE.md)
argo plugin show <id>
argo plugin remove <id>
argo plugin doctor              # ABI compat, permissions, crash counts
```

`install` downloads the binary, verifies the SHA-256,
extracts to `~/.argo/plugins/<id>/<version>/`, and adds an
entry to `~/.argo/plugins.toml`. The plugin is **not
loaded** until the operator grants its declared
permissions in `argo.toml`. This is intentional friction:
running with default-deny + explicit grant is the V3.0
security posture.

### 5.3 Sandboxing posture

V3.0 ships **no host-enforced sandbox** beyond the OS
process boundary. The doc (and `argo plugin install`) name
this risk so operators understand the trust model. V3.x
adds opt-in `sandbox-exec` / `seccomp` profiles per plugin
binary. WASM is the V4 stretch.

---

## 6. Migration path

### 6.1 Dogfood candidate: Slack, Stripe, Gmail

Recommended: keep the in-tree implementations as the
NotifyChannel / MCP-connector references and **publish
parallel out-of-tree plugins** that re-implement the same
contracts over the plugin ABI. One per extension point:
Slack as NotifyChannel, Stripe as executor-shaped MCP
connector, Gmail as executor.

Each is high-traffic in the v1 verticals (sales-sdr →
Gmail, gtm-engineer → Stripe events, receptionist →
Slack), so real load surfaces real bugs; each has clear
in-tree behavior to diff against in a reference test.

The in-tree versions stay shipped during V3.x as the
backward-compat path. After two minor releases without
plugin-side regressions, the in-tree copies are removed
in V4.0.

### 6.2 Backward compatibility for in-tree code paths

In-tree executors and channels keep their existing trait
impls. The plugin host becomes one more consumer of those
traits via a thin adapter. Code paths that branch on
`kind` (e.g. action dispatch in `argo-orchestrator`) pick
in-tree first, fall through to the plugin host. No change
to manifests, no change to vertical configs.

### 6.3 What `argo-plugin-api` extraction looks like

The next brief, `claude-plugin-api-crate`, derives crate
boundaries from this doc: move shared types (`Action`,
`Notification`, `VerticalPackManifest`, …) out of
`argo-types` / `argo-channels` / `argo-genesis` into a new
`argo-plugin-api` crate; re-export from the original
crates for source-compat through V3.x; generate JSON
schemas via `schemars` and ship them in
`crates/argo-plugin-api/schemas/`; keep `argo-plugin` (the
host) where it is and have it depend on `argo-plugin-api`
for plugin-facing types.

---

## 7. Prior art

Pulled from comparable systems; design choices below
borrow from each.

- **Cargo.** Ships in-tree. Plugins (`cargo-x`) are
  literally separate binaries; cargo finds them on
  `$PATH`. Discovery is "convention over registry."
  Argo's `argo plugin install` is more curated, but the
  binary-on-disk model is the same shape.
- **Helix.** Steno-script plugin model in 2026 — WASM
  components for editor extensions. Validates WASM as a
  V4-grade target; also confirms it's still rough enough
  that a 2026 ship-now system should not bet on it.
- **Neovim.** Plugins are Lua running in a shared
  interpreter. Trade-off: zero-overhead calls, near-zero
  isolation. Argo's reliability budget rules this out.
- **Obsidian.** TypeScript plugins running in the
  renderer process. Strong ecosystem; weak security
  model. Argo's approval-ledger threat model demands
  process isolation Obsidian does not provide.

The recommendation (subprocess + JSON-RPC) maps most
cleanly to Cargo's "binary on `$PATH`" approach with the
addition of a versioned wire protocol and a manifest-based
permissions layer.

---

## 8. What lands when

| Brief | Output |
|---|---|
| `claude-plugin-system-design` (this doc) | `docs/skycot/PLUGIN_SYSTEM.md` |
| `claude-plugin-api-crate` (next) | `crates/argo-plugin-api/` extracted; JSON schemas; semver baseline at `0.1.0` |
| `claude-plugin-host-permissions` | Permission grant/check at the host RPC boundary |
| `claude-plugin-marketplace-mvp` | Static registry + `argo plugin install/list/show` |
| `claude-plugin-dogfood-slack` | First out-of-tree plugin (Slack NotifyChannel) |
| `skycot-plugin-sandbox-exec` (V3.x stretch) | `sandbox-exec` profile applied on macOS |

Reviewers approving this doc are committing to the **shape**
(extension points, loading model, permission vocabulary,
SemVer policy, dogfood targets) — not the line-by-line
implementation. Implementation lands in the briefs above.
