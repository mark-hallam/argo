# Windows backend session — 2026-05-31

A dedicated **backend / Rust** session running on a **Windows** machine, in
parallel with the Mac session that owns Genesis + the live gateway.
Coordination is git-only (origin/main). This log is the in-band progress
record (appended ~every 90 min), truth-bounded per
[DEC-029](decisions/DEC-029-truth-bounded-status-reporting.md): each item is
marked **VERIFIED** (with evidence shown) or **UNCONFIRMED** (needs the Mac's
live gateway).

## Machine reality

- Verification here = `cargo build` / `cargo test` / `cargo clippy` native on
  Windows. Cannot reach the live gateway (`:7400`), `env.local` secrets, or the
  demo tenant/seed — those live on the Mac. "Done" here = `cargo test` green +
  committed; the Mac live-verifies.
- Toolchain installed this session: rustc/cargo **1.96.0** (stable-x86_64-pc-
  windows-msvc) + clippy + rustfmt; MSVC C++ build tools 14.44 (linker verified
  by compiling + running a throwaway binary).
- Cargo `target/` redirected to `%LOCALAPPDATA%\argo-target` (OFF OneDrive) —
  the repo lives under OneDrive and syncing `target/` causes file-lock errors +
  cripples build speed. `CARGO_INCREMENTAL=0`.
- Lane: backend Rust **excluding** the Genesis lane (`argo-genesis/`,
  `routes_genesis*` / `routes_workers_hire*` / `routes_onboarding*`) — those are
  the Mac's. Also read-only: CLAUDE.md, docs/skycot/decisions/.

## Operating rules I am following (from CLAUDE.md + this session's prompt)

- <500 LOC/file, <80 LOC/fn, no `unwrap`/`expect` outside `#[cfg(test)]`,
  additive migrations only, a test for every change.
- In-band ledger: any shipped capability updates its `FEATURE_MATRIX.md` row +
  regenerates `docs/skycot/feature-matrix.html` (`python scripts/render-feature-matrix.py`)
  in the SAME commit.
- Truth-bounded reporting (DEC-029): VERIFIED with evidence vs UNCONFIRMED. No
  "it works" without a check this turn.
- NO THEATER (DEC-007): real impls + real tests, nothing faked.
- Git: verify green before commit; small commits; `fetch && rebase origin/main
  && push`; never force-push. Branch/PR-style, re-fetch before push (two
  machines on one main).

---

## Phase 0 — Toolchain + compat smoke — COMPLETE (VERIFIED)

A cold `cargo build --workspace --keep-going` (run twice; once via PowerShell,
once via a clean `cmd /c` byte-level redirect) plus a targeted core-lane build
establish the Windows-usable surface.

### Compat map

**Builds on Windows ✅ (VERIFIED):** ~85 of the 89 workspace crates. Targeted
build `cargo build -p argo-workflow -p argo-runtime -p argo-store -p argo-memory
-p argo-orchestrator` → **exit 0, "Finished dev profile in 4m 32s"**, which also
pulled + compiled argo-security, argo-mcp, argo-telemetry, argo-audit,
argo-identity, argo-vertical, argo-mcp-connector-shared. The full `--keep-going`
workspace build compiled 91 `Compiling argo-*` lines with only the failures
below. **My entire assigned lane builds.**

**Fails to compile ❌ (2 crates) — VERIFIED from build logs:**

| Crate | Errors | Cause | Mine to fix? |
|---|---|---|---|
| `kryptering` v0.1.0 (3rd-party, crates.io) | 2 × `Ulong: From<u64>` at `pkcs11/mod.rs:147,610` | PKCS#11 `CK_ULONG` = C `unsigned long` = 32-bit on Win64 (LLP64) but 64-bit on Unix (LP64); crate does `(x as u64).into()` assuming `Ulong: From<u64>`, only true on Unix. Default feature `pkcs11`→`cryptoki`. | **No** — third-party source; feature-gate lives in third-party `bergshamra-crypto`; flipping it risks SAML-SSO correctness + touches argo-cli's shared manifest. Documented limitation. |
| `argo-executor-computer-use-windows` | 10 (windows-crate 0.58 API drift: `is_invalid`/`as_bool` gone, unresolved imports, ptr mismatch) | Native Windows CU executor; **DEFERRED-v1.x** per DEC-002 (macOS is v1 CU path). | **No** (out of v1 priority; separate decision). |

**Cannot build — cascade-blocked ❌ (2 crates) — VERIFIED via `cargo tree -i kryptering`:**

```
kryptering (fails on Win) → bergshamra-crypto → bergshamra-{dsig,keys,transforms} → argo-cli → argo-voice-tui
```

`argo-cli` deps `bergshamra-dsig 0.4.0` + `bergshamra-keys 0.4.0` (XML
dig-signature for SAML SSO, `argo-cli/Cargo.toml:157-158`). Under `--keep-going`
these are *skipped* (a dep failed), so they don't show as "could not compile" —
they were never attempted.

### Consequence for the plan

- **`argo-cli` is unbuildable on Windows.** It holds ALL gateway handlers,
  including the Phase 2 swap point `routes_workflow_execute.rs:209`. I can
  *write* gateway-handler code but **cannot compile/test it here** — only the
  Mac can. This reshapes Phase 2 (below).
- **Phase 1 surface is large** — every priority crate (argo-store,
  argo-workflow, argo-runtime, argo-memory, argo-orchestrator, argo-security,
  argo-siem, argo-telemetry, argo-marketplace, argo-eval[non-genesis],
  argo-capability[non-genesis], argo-acp, argo-forge, the connectors) builds
  and is fully testable here.

### Phase 2 re-scope (consequence of the argo-cli constraint)

`WorkerSessionRunner` ALREADY exists + is fully built in
`argo-workflow/src/runner.rs`; `argo-workflow/tests/execution_runs_real_step_test.rs`
already drives it end-to-end vs stub adapters (happy-path / cross-tenant-refuse
/ MCP-500). Remaining work = 3 **production** adapters wrapping `McpClientPool`
(argo-mcp) / `AuditSink` (argo-audit) / `CapabilityEngine` (argo-capability).
**Plan:** house the adapters in **`argo-runtime`** (builds + tests on Windows;
all three deps available there, no dep-cycle) so they are fully
Windows-verifiable. Only the final ~1-line `MockRunner::echo()` →
`WorkerSessionRunner` swap in `routes_workflow_execute.rs` lands in argo-cli and
is **Mac-verified (UNCONFIRMED from Windows by construction).**

---

## Phase 1 — Backend test-suite health — IN PROGRESS

### Discovery: the inventory is gated by Windows test-COMPILE breaks, not test failures

First `cargo test --workspace …` run returned **0 test summaries** and exit 101.
Root cause (VERIFIED from the log): `cargo test` compiles **every** test binary
before running **any** test, and `argo-agent-runner`'s test binary failed to
compile on Windows:

```
error[E0433]: cannot find `unix` in `os`
  --> crates\argo-agent-runner\src\dispatch_tests.rs:5
  --> crates\argo-agent-runner\src\dispatch_tenant_isolation_tests.rs:5
     use std::os::unix::fs::PermissionsExt;
error[E0599]: no method named `set_mode` found for struct `Permissions`
     perms.set_mode(0o755);
```

`--no-fail-fast` skips past test *failures*, not *compile* failures — so one
Windows-incompatible test file aborts the whole workspace inventory. There are
**35 files** workspace-wide referencing `std::os::unix` / `PermissionsExt` /
`set_mode` / `/bin/sh` (grep); most are production `src/` with proper
`#[cfg(unix)]` fallbacks or live in already-excluded crates, but any **test
file lacking the gate** breaks its crate's test binary on Windows. A
`cargo build --tests --workspace --keep-going` pass is running to name every
one precisely (build accepts `--keep-going`; test does not — that was a
flag error on my first attempt, now corrected).

### Fix #1 (applied, pending verify) — gate 2 Unix-only agent-runner tests

`crates/argo-agent-runner/src/dispatch_tests.rs` and
`dispatch_tenant_isolation_tests.rs` each hold exactly ONE test that writes a
`#!/bin/sh` script, `chmod 0o755`s it, and spawns it as a real **stdio MCP
subprocess**. These are **genuinely Unix-only** (no `/bin/sh` on Windows;
`.sh` not executable; `set_mode` doesn't exist) — not a Windows bug to fix but
a test that only makes sense on Unix. Fix = add `#![cfg(unix)]` at the top of
each test module. **No coverage lost:** CI (Linux) + the Mac still compile +
run both. Edits confined to the two `*_tests.rs` files — `dispatch.rs` (the
Mac's active Genesis-fence file) and `lib.rs` are untouched (collision-safe).
This matches the codebase's existing platform-gating idiom (cfg-gated macOS CU,
`[target.'cfg(unix)'.dependencies]` in argo-cli).

Classification of this item: **Windows-only test-compat** (not a real product
bug, not a flake). Status: **edit applied; green-build + test-run UNVERIFIED
until the `--tests` build + per-crate `cargo test` complete this turn.**

Inventory + categorization of the remaining crates follows once the test
binaries build.
