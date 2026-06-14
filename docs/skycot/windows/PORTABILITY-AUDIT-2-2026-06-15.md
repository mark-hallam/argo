# Windows-portability audit #2 — 2026-06-15

**Second proactive sweep by the Windows fleet (Latitude5440), on the axes the
first audit (`PORTABILITY-AUDIT-2026-06-15.md`) did NOT cover.** The first audit
swept shell spawns + HOME/USERPROFILE (9 findings, all fixed/merged: #2944–#2948
+ #2950). This one targets: **unix file-permission hardening dropped on Windows,
hardcoded POSIX absolute/tilde paths, non-shell external-program spawns, and
CRLF-fragile / unix-signal control paths** — deduped against everything already
fixed.

## Method / provenance

Workflow fan-out (run `wf_fac53193-3e6`, **14 agents**): 4 axis-scoped sweeps →
production-filter → per-candidate adversarial verify (real-bug / production /
already-handled checks). **Raw 10 candidates → 7 confirmed NEW un-briefed
production Windows bugs.** All 7 recommended brief-ids verified absent from
`streams/` + `BUILD_STATUS_WINDOWS.md`.

**NO THEATER.** Static reasoning + git archaeology, not live-reproduced. I
(Latitude5440) **spot-re-verified the 2 security findings (1, 5) against
`origin/main b7135a81d`** by reading the cited lines:
- ✅ `argo-skill/src/sign.rs:354` — `#[cfg(not(unix))] write_secret_file` silently
  delegates to plain `fs::write` (no 0600, no warning); unix arm uses `mode(0o600)`.
- ✅ `argo-cli/.../cred_set.rs:49` — `#[cfg(not(unix))] disable_echo()` returns
  `false` (no-op); unix arm spawns `stty -echo`.

The other 5 carry the verifier's reasoning and should be re-confirmed at
brief-claim time per the freshness gate.

## Summary table

| # | Sev | Crate | File:line | Class | Recommended brief id |
|---|-----|-------|-----------|-------|----------------------|
| 1 | **P2 🔒** | argo-skill | `src/sign.rs:354` | private-key 0600 dropped **silently** on Windows | `win-portability-skill-sign-private-key-unix-perms` |
| 2 | P3 🔒 | argo-mcp-connector-gmail | `src/auth.rs:108` | OAuth token-cache 0600 dropped silently | `win-gmail-connector-token-cache-perm-gap-not-unix-warn` |
| 3 | P2 | argo-cli | `src/handlers/worker_actions_executor_code.rs:423` | POSIX `ulimit`/`2>/dev/null`/`;` in code-exec cmd | `win-cli-code-executor-build-command-posix-syntax` |
| 4 | P2 | argo-executor-document-csv | `src/sandbox.rs:28`+`:121` | `~/` default + HOME-only tilde (missed by #2904) | `win-csv-sandbox-tilde-userprofile-fallback` |
| 5 | **P2 🔒** | argo-cli | `src/handlers/cred_set.rs:49` | secret **echoed in plaintext** (`disable_echo` no-op) | `win-cli-cred-set-echo-suppression` |
| 6 | **P2 🔒** | argo-cli | `src/handlers/connector_authorize_input.rs:59` | OAuth token echoed in plaintext (same pattern) | `win-cli-hidden-input-echo-mask-console-api` |
| 7 | P2 | argo-executor-document | `src/pdf.rs:68` | hardcoded macOS Chrome path, no Windows discovery | `win-doc-executor-chrome-path-windows-discovery` |

🔒 = a real **security / confidentiality** gap (not just a feature break).

**The Windows fleet is taking findings 1, 2, 4, 7 directly** (clean, no new dep /
no `unsafe` — fast per-crate fixes; see their ledger rows + PRs). **Findings 3,
5, 6 are flagged for conductor triage** — 5+6 (console echo masking) need either
a new `rpassword` workspace dependency **or** a marked Win32 Console-API `unsafe`
FFI shim (an architectural/dep call above a single worker's remit), and 3 needs
the slow argo-cli build plus a cmd-aware `shell_escape`.

---

## Finding 1 — P2 🔒 — `argo-skill` skill-signing private key written without 0600 (silently) on Windows

**File:** `crates/argo-skill/src/sign.rs:354`

```rust
#[cfg(not(unix))]
fn write_secret_file(path: &Path, bytes: &[u8]) -> ArgoResult<()> {
    write_file(path, bytes)   // plain fs::write — no 0600, NO warning
}
```

**Why it matters:** `write_secret_file` persists the cosign-style **ed25519
private signing key** (`write_keypair_to_dir`, sign.rs:159; reached from the
production `argo skill keygen` handler `skill_keygen.rs:13`). The unix arm opens
with `mode(0o600)`; the non-unix arm silently falls back to plain `fs::write`, so
on Windows the private key inherits the parent dir's (typically Users-readable)
ACLs. **It is the lone secret-writer with NO NO-THEATER warning** — every sibling
(`argo-credentials/src/lib.rs:319`, `argo-config/src/cred/io.rs:281`,
`gateway/bootstrap_token.rs:131`) emits a one-shot warning. P2 (private signing
key at rest).

**Recommended fix:** mirror the sibling pattern — a one-shot (AtomicBool-gated)
`tracing::warn!` in the `#[cfg(not(unix))]` arm naming the path + that 0600 is
unix-only, before `write_file`; + a `#[cfg(not(unix))]` test asserting the note.
(Best-effort Windows ACL tightening is a stronger follow-up.)

## Finding 2 — P3 🔒 — `argo-mcp-connector-gmail` OAuth token cache 0600 dropped silently

**File:** `crates/argo-mcp-connector-gmail/src/auth.rs:108` (and `:119`)

Both the open-time `opts.mode(0o600)` and the post-write `set_permissions(0o600)`
on the cached Gmail OAuth **access token** (`<cwd>/.argo/connectors/<tenant>/gmail/
access_token.json`, written by `refresh_and_cache`, auth.rs:54) are inside
`#[cfg(unix)]` with **no `#[cfg(not(unix))]` arm** → silent on Windows. P3 (NTFS
parent ACLs are usually already user-restricted, so weaker than the unix 0644
case; the real defect is the silent, undocumented loss vs the project's NO-THEATER
convention). **Recommended fix:** add the one-shot non-unix warning (reuse
`argo-config`'s `warn_perm_hardening_unix_only` or a shared helper). NB Salesforce
+ gcal connectors make no 0600 attempt at all — a shared helper would cover all.

## Finding 3 — P2 — `argo-cli` code-executor bakes POSIX shell syntax into the command

**File:** `crates/argo-cli/src/handlers/worker_actions_executor_code.rs:423`

```rust
Ok(format!("ulimit -v {mem_kib} 2>/dev/null; {runner} {}", shell_escape(...)))
```

Unconditional POSIX syntax (no cfg arm). When `ARGO_E2B_TOKEN` is unset the
executor falls back to `argo-sandbox` `LocalSandbox`, which on Windows runs the
string via `cmd /C` (the #2948 shell-binary fix selects a shell that exists but
explicitly leaves *syntax* to the caller). Under cmd: `ulimit` isn't a builtin,
`2>/dev/null` creates a literal file named `null`, `;` isn't a separator. P2
(only the LocalSandbox code-exec fallback on native Windows; fails loudly).
**Recommended fix:** cfg-gate `build_command` — on Windows emit `{runner}
{script}` (drop ulimit/redirect/`;`) and make `shell_escape` cmd-aware
(double-quote, not POSIX single-quote). The identical `2>/dev/null` axis was
already cfg-gated in `doctor_network.rs` (#2950). *(Flagged for conductor — argo-cli build + shell_escape change.)*

## Finding 4 — P2 — `argo-executor-document-csv` `~/` default + HOME-only tilde (missed by #2904)

**File:** `crates/argo-executor-document-csv/src/sandbox.rs:28` + `:121`

`DEFAULT_OUTPUT_ROOT = "~/.argo/exports/csv/"` is the runtime default when the
CSV manifest omits `output_root` (`parse_request`, worker_actions_executor_document_csv.rs:128).
It's expanded by a **local `expand_tilde`** (sandbox.rs:121) that reads only
`HOME` — **no USERPROFILE fallback**. #2904 added HOME→USERPROFILE to every other
tilde resolver but missed this copy. On native Windows (HOME unset) `expand_tilde`
returns the path verbatim → a literal `~` directory under the process CWD instead
of the user profile. P2. **Recommended fix:** HOME→USERPROFILE fallback (the
#2904 `home_dir_from` pattern) + a pure env-pair test; ideally call the canonical
argo-config helper to kill the duplicate resolver.

## Finding 5 — P2 🔒 — `argo cred set` echoes the secret in plaintext on Windows

**File:** `crates/argo-cli/src/handlers/cred_set.rs:49`

```rust
#[cfg(not(unix))]
fn disable_echo() -> bool { false }   // no-op → echo stays ON
```

The interactive `argo cred set` (no value arg) calls `disable_echo()` before
reading a **secret credential** from stdin. Unix spawns `stty -echo`; the Windows
arm is a silent no-op, so `echo_disabled=false` and the secret is **echoed to the
terminal in plaintext as the user types it**. P2 (confidentiality leak; Windows
interactive only; non-interactive arg workaround exists). **Recommended fix:**
`rpassword` (cross-platform masked read) **or** a marked Win32 Console-API FFI
shim (`GetConsoleMode`/`SetConsoleMode` clearing `ENABLE_ECHO_INPUT`). *(Flagged
for conductor — needs a dep or `unsafe` FFI decision; pairs with finding 6.)*

## Finding 6 — P2 🔒 — `argo connector authorize` echoes the OAuth token in plaintext (same pattern)

**File:** `crates/argo-cli/src/handlers/connector_authorize_input.rs:59`

Identical `disable_echo()` no-op on the connector/OAuth **token** prompt
(`read_hidden_value`, called from `connector_authorize.rs:105`). Same P2
confidentiality leak. **Fix once, shared:** factor the masked-read into one
module and fix findings 5 + 6 together (rpassword or Win32 FFI). *(Flagged for
conductor with finding 5.)*

## Finding 7 — P2 — `argo-executor-document` PDF export can't find Chrome on Windows

**File:** `crates/argo-executor-document/src/pdf.rs:68`

`resolve_chrome_path()` returns `ARGO_DOC_CHROME` or a **hardcoded macOS path**
(`/Applications/Google Chrome.app/...`, pdf.rs:25). No Windows arm, no
auto-discovery. On Windows the macOS path `.exists()` is false → `render_pdf`
returns `ChromeMissing` even when Chrome IS installed at the standard Windows
location (`%ProgramFiles%\Google\Chrome\Application\chrome.exe`). PDF generation
silently non-functional on Windows. P2 (clean error, not a crash). **Recommended
fix:** cfg(target_os)-selected default + Windows probe of the standard install
locations (+ Edge `msedge.exe` as a Chromium fallback); update the `ChromeMissing`
hint + module doc.

---

## Recommended brief-wave ordering for the conductor

1. **Security cluster (1, 5, 6)** — private-key perms + secret-echo are the
   highest-risk. Finding 1 is cheap (warn pattern); 5+6 need the rpassword-vs-FFI
   call (do them as one shared masked-read fix).
2. **Quick wins (2, 4)** — perm-warning + HOME→USERPROFILE, both the established
   #2904/perm patterns.
3. **Feature parity (3, 7)** — code-exec cmd syntax + Chrome discovery (real
   Windows features currently broken/degraded).

This doc is **advisory** — no scope claim. The Windows fleet is shipping 1/2/4/7
directly; 3/5/6 await conductor triage.
