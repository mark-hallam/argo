# eval-argo-mcp-core-surface

**Priority**: P1 (substrate audit; top of next-session pickup list per `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` §Crates NOT YET TOUCHED row #3)
**Owner**: claude
**Type**: Read-only audit — public surface + cross-tenant pin tests + FINDINGS-ONLY ignored tests naming follow-up fix-briefs
**Size**: ~200-400 LOC (audit report under `docs/skycot/evals/scheduled/2026-05-24/` + 1 pin test file under `crates/argo-mcp/tests/`)
**Authored against**: origin/main `576e48f5c` on 2026-05-24
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-mcp-core-surface`

## Why

`argo-mcp` is the in-core MCP (Model Context Protocol — the
JSON-RPC wire protocol Claude uses to talk to external tool
servers) crate. It hosts the connection pool, client, server,
rate limiter, and protocol types that every MCP-backed
verticalo uses for capability-bound tool calls. **It is the only
P1 crate in the gap-manifest §"NOT YET TOUCHED by any eval
session" list that hasn't been audited yet — the manifest
explicitly marks it as "now top of next-session list".**

Mission claim 5 ("Any task a human can do on a PC — MCP +
computer-use") rests on this crate. Mission claim 1 ("Teammate,
not tool — per-tenant scoping") and claim 4 ("Skills shared +
learned — credential scoping") both rest on this crate
operationally too — a tenant-scoping bug here is a cross-tenant
credential leak in any vertical that uses MCP.

Sibling crates `argo-mcp-connectors/*` (29 of them) were audited
2026-05-23 (`eval-mcp-connector-credential-scoping-sweep` +
`eval-mcp-connector-real-vs-stub-inventory`) — they are 29/29
real, with 3 cwd-cache leaks already fixed via #2103. The
core crate has NOT been audited.

Surface (from `crates/argo-mcp/src/lib.rs` re-exports):
17 source files. Production-relevant modules: `pool` (the
connection pool used by every MCP-backed worker turn), `client`
(the JSON-RPC client), `server` (the in-process server when
argo acts as the MCP host), `protocol` (wire types),
`rate_limit` (per-tenant request budget), `error`.

## What

Mirror the shape of `eval-argo-routing-crate-surface` (#2115)
and `eval-argo-replay-after-1992-fix` (#2130):

1. **Audit report** at
   `docs/skycot/evals/scheduled/2026-05-24/eval-argo-mcp-core-surface.md`
   enumerating:
   - Every `pub` re-export from `lib.rs` + every `pub fn` / `pub
     struct` / `pub enum` in `pool.rs` / `client.rs` /
     `server.rs` / `protocol.rs` / `rate_limit.rs` / `error.rs`.
   - For each public function: which production callers exist
     (enumerate via `rg 'argo_mcp::FunctionName' crates/`,
     skipping `crates/argo-mcp/`'s own tests).
   - For each surface, a verdict: 🟢 (tenant-scoped + audited)
     / 🟡 (substrate present but no cross-tenant pin) / 🔴
     (cross-tenant leak / unscoped global state / surface
     without production caller).
2. **Pin test file** at
   `crates/argo-mcp/tests/mcp_core_surface_test.rs` (~150-300
   LOC) covering the 🟢 surfaces — minimum:
   - `McpClientPool::get` round-trips per tenant.
   - Pool-level isolation: two tenants requesting the same
     connector get separate `McpClient` instances + can't see
     each other's request state.
   - Rate-limit budget exhaustion is per-tenant (one tenant
     hitting cap doesn't starve another).
   - At least one negative test: cross-tenant `get` with a
     `TenantId` mismatched to the connector's credential
     binding refuses with a structured error (NOT a panic, NOT
     a silent fallback to another tenant's credentials).
3. **FINDINGS-ONLY `#[ignore]`'d tests** (zero, one, or more —
   shipped only if the audit surfaces a real broken-wire) each
   naming a fix-brief in its `#[ignore = "FINDINGS-ONLY: …,
   fix in <brief-id>"]` attribute.

## Acceptance

- `cargo test -p argo-mcp --test mcp_core_surface_test` → all
  non-`#[ignore]`'d tests pass.
- `cargo clippy -p argo-mcp --all-targets -- -D warnings` clean.
- Workspace clippy fast-fail unchanged.
- Audit report cites the live `origin/main` SHA + per-section
  line:file pointers.
- If any FINDINGS surface, each one names a fix-brief that does
  NOT yet exist in `streams/` (verify pre-PR with `ls streams/
  | grep <brief-id>`).
- PR body includes the verdict tally (🟢/🟡/🔴 count) + a one-
  line cite back to the gap manifest row this closes.
- Update `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-
  gap-manifest.md` §"Crates NOT YET TOUCHED" row #3 from 🟡 P1
  → 🟢 CLEARED (if no FINDINGS) or 🟡 AUDITED (if FINDINGS shipped
  with fix-briefs queued).

## May-touch

- `crates/argo-mcp/tests/mcp_core_surface_test.rs` (new).
- `docs/skycot/evals/scheduled/2026-05-24/eval-argo-mcp-core-
  surface.md` (new).
- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-
  manifest.md` (single-line edit to the row).
- **NOT** any production source — this is an audit. If the audit
  finds a bug, the fix lands in a separate brief authored as a
  follow-up.

## Source

- Gap manifest §"Crates NOT YET TOUCHED" row #3 explicitly
  marks `argo-mcp` (core, not connectors) as "🟡 P1 — **now top
  of next-session list**".
- Mission claims 1 + 4 + 5 (cross-tenant + credential scoping +
  computer-use wiring).
- Sibling audit-format established by #2115
  (`eval-argo-routing-crate-surface`) and #2130
  (`eval-argo-replay-after-1992-fix`).

This is an evaluator-style audit, **not** a fix brief. Findings
are shipped as `#[ignore]`'d tests with named fix-briefs. Do
NOT inline-fix anything the audit turns up.
