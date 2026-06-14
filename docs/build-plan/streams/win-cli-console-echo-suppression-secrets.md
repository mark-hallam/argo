# win-cli-console-echo-suppression-secrets

**Owner**: windows-fleet — **🔒 P2 SECURITY — secrets are echoed in PLAINTEXT on the Windows console (echo-suppression is a no-op).**
**Depends on**: none.
**Authored against**: origin/main `ce99d4e23` on `2026-06-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh win-cli-console-echo-suppression-secrets`
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify on Windows; the FIX is cross-platform (`#[cfg(unix)]` `stty` path stays byte-identical) so the conductor ALSO re-verifies on unix before merge. Push branch, IN_REVIEW; conductor reviews+merges. **Do NOT self-merge.**
Source: docs/skycot/windows/PORTABILITY-AUDIT-2-2026-06-15.md — findings **5 + 6 of 7** (the two echo-suppression security gaps; same pattern, one brief). Findings 1/2/4/7 are taken by the fleet directly; finding 3 is `win-cli-code-executor-build-command-posix-syntax`.

## Problem (VERIFIED on origin/main `ce99d4e23`)

Two interactive-secret prompts disable terminal echo on unix (`stty -echo`) but
have a **no-op `#[cfg(not(unix))]` arm**, so on native Windows the secret is
typed and **rendered in plaintext** on the console (shoulder-surf / screen-share
/ terminal-scrollback exposure):

1. **Finding 5 — `crates/argo-cli/src/handlers/cred_set.rs:49-52`** —
   `#[cfg(not(unix))] fn disable_echo() -> bool { false }`. The `argo cred set`
   provider-secret/API-key prompt echoes the key as typed on Windows.
2. **Finding 6 — `crates/argo-cli/src/handlers/connector_authorize_input.rs:~50`**
   — the same `disable_echo()`/`enable_echo()` pattern (its own copy) for the
   **OAuth token** prompt → token echoed in plaintext on Windows.

The unix arms shell out to `stty`; the Windows arms do nothing.

## Fix (one cohesive PR — same class, no parallel abstraction)

Implement real Windows console echo suppression for BOTH call sites. Pick the
lowest-risk option and use ONE shared helper (do not leave two diverging copies):
- **Preferred:** a small portable hidden-input helper. If a maintained
  cross-platform crate is already in the workspace tree (check `rpassword` /
  `dialoguer` first — `cargo tree` / grep `Cargo.toml`s), use it for both
  prompts and delete the hand-rolled `stty`/no-op pair. Justify any NEW dep.
- **Otherwise:** a `#[cfg(windows)]` arm using the Console API
  (`SetConsoleMode` clearing `ENABLE_ECHO_INPUT` on the stdin handle, restored
  after) — only if `windows`/`windows-sys` is already an argo-cli dep
  (`argo-executor-computer-use-windows` is now a cross-platform dep of argo-cli
  per #2942; check whether its windows-sys is reachable) — else prefer the crate.
- Factor a single `read_secret_no_echo()` (or `with_echo_disabled`) used by both
  `cred_set` and `connector_authorize_input`; the unix path stays byte-identical
  (`stty -echo` … `stty echo`).

NO unsafe outside a marked FFI shim if you use the raw Console API. Keep each
touched file < 500 LOC.

## Verify (RUN it — NO THEATER)
- On Windows: `cargo build -p argo-cli` clean; `cargo test -p argo-cli` for the
  touched handlers green; clippy `-D warnings` clean on the touched files.
  **Manual/scripted check (the point of this fleet):** run `argo cred set` (and
  the connector-authorize prompt) on Windows and confirm typed input is **masked
  / not echoed** (capture the honest before/after into a results note). If a
  chosen mechanism can't fully suppress echo, record it honestly + recommend a
  follow-up — never a fabricated pass.
- Conductor re-verifies on unix: `cargo test -p argo-cli` (the `#[cfg(unix)]`
  `stty` path is unchanged → byte-identical) + a local `argo cred set` still
  hides input on macOS.

## Out of scope
- Changing what the prompts read/store; only the echo-suppression on Windows.

**LOC budget:** ~40–120 (one shared helper + 2 call-site swaps + the Windows arm).
