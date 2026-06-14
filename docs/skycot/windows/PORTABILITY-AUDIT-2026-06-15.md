# Windows-portability audit — 2026-06-15

**Proactive sweep by the Windows fleet (Latitude5440). No ledger row — the
Windows queue was drained (only `win-e2e-soak-multi-run` TODO, dep-blocked on
#2940 IN_REVIEW), so this is fan-out discovery work for the conductor to triage
into the next brief wave.**

## What this is

A workflow fan-out (15 agents: sweep → dedup → adversarial verify) over SKYCOT
**production** code for un-briefed Windows-portability breaks that the existing
Windows hardening waves (#2904 HOME/tmp paths, #2915 process-registry spawn,
DEC-054 executor) did **not** cover. Raw 11 → 9 deduped → **9 adversarially
confirmed** (each verifier checked: is it a real Windows bug? is it production
[not test/bench]? is it already cfg-gated / already briefed / already fixed?).

**NO THEATER.** These are *candidate* bugs surfaced by static reasoning + git
archaeology, not live-reproduced crashes. The conductor should treat each as a
brief candidate, not a confirmed regression. Confidence notes below. I
(Latitude5440) **spot-re-verified 3 of the 9 against current `origin/main`
`6298e118d`** by reading the cited lines directly:

- ✅ `argo-tool/src/tools/terminal_streaming.rs:39` — `Command::new("bash")` confirmed ungated in `pub(crate) async fn run_streaming` (P1).
- ✅ `argo-sandbox/src/local.rs:30` — `Command::new("bash")` confirmed ungated in production `impl Sandbox for LocalSandbox`; only the *tests* are `#[cfg(all(test, unix))]`-gated (line 69) (P2).
- ✅ `argo-config/src/loader.rs:101` — `std::env::var_os("HOME")?` confirmed, no `USERPROFILE` fallback (P2).

The other 6 carry the verifier's reasoning verbatim and were **not** hand-checked
this turn — the brief author should re-confirm at claim time per the freshness gate.

## Summary table

| # | Sev | Crate | File:line | Class | Recommended brief id |
|---|-----|-------|-----------|-------|----------------------|
| 1 | **P1** | argo-tool | `src/tools/terminal_streaming.rs:39` | unix-only shell spawn | `win-terminal-streaming-bash-portability` |
| 2 | P2 | argo-sandbox | `src/local.rs:30` | unix-only shell spawn | `win-sandbox-localsandbox-bash-cfg-gate` |
| 3 | P2 | argo-cli | `src/handlers/doctor_network.rs:209` | unix-only shell spawn | `win-cli-doctor-cert-summary-sh-portability` |
| 4 | P2 | argo-vertical-packs | `src/loader.rs:51` | HOME w/o USERPROFILE | `win-vertical-packs-home-userprofile-fallback` |
| 5 | P2 | argo-marketplace | `src/oci_auth.rs:119` | HOME w/o USERPROFILE | `win-marketplace-docker-config-home-userprofile` |
| 6 | P2 | argo-config | `src/loader.rs:101` | HOME w/o USERPROFILE | `win-portability-loader-user-config-path` |
| 7 | P2 | argo-config | `src/source_tracking.rs:160` | HOME w/o USERPROFILE | `win-config-user-config-path-home-userprofile-fallback` |
| 8 | P2 | argo-cli | `src/handlers/doctor_gateway.rs:22` | unix-only `kill -0` | `win-cli-doctor-gateway-kill-portability` |
| 9 | P3 | argo-cli | `src/handlers/doctor_storage.rs:266` | unix-only `df` | `win-doctor-disk-usage-df-portability` |

All 9 brief-ids verified **absent** from `streams/`, `BUILD_STATUS_WINDOWS.md`,
and `BUILD_STATUS.md` at audit time — no collision, no drift.

Two failure shapes dominate:
- **Hard runtime failure** (the program isn't on Windows at all): `bash`, `sh`,
  `kill`, `df`. Finding 1 is the only one that's a *complete feature break*
  (no graceful fallback) → P1. Findings 3/8/9 degrade gracefully (`.ok()?`
  swallows the spawn error) so they're silent-loss, not crashes → P2/P3.
- **Silent config miss** (`HOME` unset on native Windows, no `USERPROFILE`
  fallback): findings 4–7. #2904 fixed this exact pattern in `expand_tilde` +
  `argo-credentials` but missed five more call sites.

---

## Finding 1 — P1 — `argo-tool` terminal streaming hardcodes `bash`

**File:** `crates/argo-tool/src/tools/terminal_streaming.rs:39`

```rust
let mut child = tokio::process::Command::new("bash")
    .arg("-c")
    .arg(command)
```

**Why it breaks Windows:** Windows has no `/bin/bash`; bash ships only with
Git/WSL/MSYS2 (optional). `Command::new("bash")` returns `NotFound` at runtime
on `x86_64-pc-windows-msvc`. `run_streaming` is production (`pub(crate) async fn`,
called from `terminal.rs` dispatch ~line 286 when a progress callback is
attached). No cfg-gate, no Windows fallback.

This is **the same bug #2915 fixed in `ProcessRegistry::spawn`** — but
`terminal_streaming` was not touched (`git show 6181a69bd -- .../terminal_streaming.rs`
returns nothing). The streaming path fails unconditionally on Windows whenever an
agent uses `terminal` with a progress callback → feature break + loss of
real-time output. **P1: complete runtime failure.**

**Recommended fix:** extract a `shell_command()` helper selecting the platform
shell (the pattern at `process_registry.rs:141-154`): `cmd /C` on Windows,
`bash -c` on Unix. Unix path stays byte-identical via
`#[cfg(windows)] / #[cfg(not(windows))]`. Document that command *syntax* is
caller-responsibility per platform (as `SpawnRequest` does, `process_registry.rs:97-99`).

**Brief:** `win-terminal-streaming-bash-portability`

---

## Finding 2 — P2 — `argo-sandbox` LocalSandbox hardcodes `bash` (prod impl ungated)

**File:** `crates/argo-sandbox/src/local.rs:30`

```rust
let mut cmd = tokio::process::Command::new("bash");
cmd.arg("-c")
```

**Why it breaks Windows:** `impl Sandbox for LocalSandbox` (lines 22–61) is
**not** cfg-gated; only the tests are `#[cfg(all(test, unix))]` (line 69).
`LocalSandbox` is instantiated from production dispatch in
`argo-cli/src/handlers/worker_actions_executor_code.rs:258` as the **E2B
fallback** when `ARGO_E2B_TOKEN` is unset. On Windows `Command::new("bash")`
fails `NotFound`, and bash-isms won't run under cmd.exe. The gateway is known to
run on Windows (GATEWAY-E2E.md).

**Recommended fix:** add `#[cfg(unix)]` to the `impl Sandbox for LocalSandbox`
block, and on Windows either (a) a PowerShell/cmd implementation, or — safest —
(b) make `new()` return `Err` on `cfg(windows)` with a clear "code-execution on
Windows requires E2B (no local fallback)" message. Update the docstring.

**Brief:** `win-sandbox-localsandbox-bash-cfg-gate`

---

## Finding 3 — P2 — `argo doctor` cert summary shells out to `sh`

**File:** `crates/argo-cli/src/handlers/doctor_network.rs:209`

```rust
tokio::process::Command::new("sh")
    .arg("-c")
    .arg(cmd)
```

**Why it breaks Windows:** `cert_summary()` (lines 200–238) spawns `/bin/sh` to
run an `openssl` pipeline with `2>/dev/null` redirection. No `/bin/sh` on native
Windows → `NotFound`. The error is swallowed by `.ok()?` (line 215) so the TLS
probe **silently** downgrades to "certificate details unavailable" — graceful,
but the expiry + SHA256 fingerprint are never retrieved. The shell redirection
also won't parse under cmd.exe. Production (`argo doctor`), shipped binary
(#2931 smoke), no cfg-gate.

**Recommended fix:** `#[cfg(unix)]` around `cert_summary()` + a Windows stub that
returns `None`/"not available on this platform". Better long-term: port the cert
fetch to native Rust TLS (rustls/native-tls) and drop the shell-out entirely.

**Brief:** `win-cli-doctor-cert-summary-sh-portability`

---

## Finding 4 — P2 — `argo-vertical-packs` loader reads HOME only

**File:** `crates/argo-vertical-packs/src/loader.rs:51`

```rust
let home = std::env::var("HOME")
    .map(PathBuf::from)
    .map_err(|_| ArgoError::Io("HOME is not set; cannot resolve ~/.argo/packs".into()))?;
```

**Why it breaks Windows:** `default_from_env()` reads `HOME` only and **errors**
("HOME is not set") on native Windows, where `USERPROFILE` is the home var.
Production: `argo-cli/src/handlers/packs.rs:58` calls `PackLoader::default_loader()`
→ `default_from_env()`. `argo packs list` (any packs subcommand) fails at runtime.
The gateway route `routes_packs.rs` already uses
`argo_credentials::dirs_home_dir()` (with the #2904 USERPROFILE fallback) — the
loader was missed.

**Recommended fix:** reuse the #2904 pattern — a pure
`home_dir_from(HOME, USERPROFILE)` helper (HOME wins, USERPROFILE fallback, empty
ignored). Add 4–5 unit tests (home wins / USERPROFILE fallback / empty HOME
ignored / neither set). Verify `cargo test -p argo-vertical-packs` + clippy on Windows.

**Brief:** `win-vertical-packs-home-userprofile-fallback`

---

## Finding 5 — P2 — `argo-marketplace` docker config path reads HOME only

**File:** `crates/argo-marketplace/src/oci_auth.rs:119`

```rust
std::env::var_os("HOME")
    .filter(|v| !v.is_empty())
    .map(PathBuf::from)
    .map(|home| home.join(".docker").join("config.json"))
```

**Why it breaks Windows:** `docker_config_path()` (public, called from
`argo-cli` `vertical_publish.rs:70` + `vertical_pull.rs:56`) returns `None` on
native Windows (HOME unset) → Docker registry auth silently skips the config
fallback, forcing explicit `--token` every time. #2904 fixed this class in
argo-config/argo-credentials but not argo-marketplace.

**Recommended fix:** one-line `.or_else(|| std::env::var_os("USERPROFILE"))`
before the `.filter`, preserving HOME-first on unix:

```rust
std::env::var_os("HOME")
    .or_else(|| std::env::var_os("USERPROFILE"))
    .filter(|v| !v.is_empty())
    .map(PathBuf::from)
    .map(|home| home.join(".docker").join("config.json"))
```

**Brief:** `win-marketplace-docker-config-home-userprofile`

---

## Finding 6 — P2 — `argo-config` loader user_config_path reads HOME only

**File:** `crates/argo-config/src/loader.rs:101`

```rust
let home = std::env::var_os("HOME")?;
Some(PathBuf::from(home).join(".config").join(defaults::USER_CONFIG_RELATIVE))
```

**Why it breaks Windows:** `user_config_path()` returns `None` on native Windows
(HOME unset, after the `XDG_CONFIG_HOME` check), so the user-level config layer
(`load_from` lines 59–64) is skipped entirely — breaks the documented precedence
(env > project-local > user-level > defaults). No cfg-gate. #2904 explicitly
fixed only `sections_core.rs` + `argo-credentials`; this is the third victim.

**Recommended fix:** after the XDG check, add USERPROFILE fallback:
`let home = std::env::var_os("HOME").filter(|h| !h.is_empty()).or_else(|| std::env::var_os("USERPROFILE").filter(|h| !h.is_empty()))?;`
Preferred: factor the shared `home_dir_from` helper (see finding 7 — fix 6 & 7 together).

**Brief:** `win-portability-loader-user-config-path`

---

## Finding 7 — P2 — `argo-config` source_tracking user_config_path reads HOME only

**File:** `crates/argo-config/src/source_tracking.rs:160`

```rust
let home = std::env::var_os("HOME")?;
Some(PathBuf::from(home).join(".config").join(defaults::USER_CONFIG_RELATIVE))
```

**Why it breaks Windows:** Same HOME-only bug as finding 6, in the source-tracking
twin. On Windows `argo config show --source` omits user-config annotations
(incomplete source labels — degrades, no crash). #2904 did not touch it.

**Recommended fix:** **fix findings 6 and 7 in one brief** — extract a pure
`home_config_dir_from(home, userprofile) -> Option<PathBuf>` helper in argo-config
and call it from both `loader.rs:97` and `source_tracking.rs:156`. HOME wins
(unix byte-identical), USERPROFILE fallback on Windows, empty ignored. Cross-platform
equivalence test. The two brief-ids are kept separate in the table for
traceability but a single combined PR is the right shape (one helper, two call sites).

**Brief:** `win-config-user-config-path-home-userprofile-fallback`

---

## Finding 8 — P2 — `argo doctor` gateway status spawns `kill -0`

**File:** `crates/argo-cli/src/handlers/doctor_gateway.rs:22`

```rust
std::process::Command::new("kill")
    .args(["-0", &pid.to_string()])
```

**Why it breaks Windows:** `gateway_status()` (lines 12–31) uses `kill -0` as a
PID-liveness probe. No `kill` on Windows → spawn fails. **Latent**: #2931's
smoke passed only because no `gateway.pid` existed, so the fn returned early
(line 15) before reaching line 22. Any run with a live PID file triggers the
failure → broken gateway-status reporting. Comment even says "works on every Unix".

**Recommended fix:** `#[cfg(unix)]` around the `kill -0` spawn + a Windows
fallback — `tasklist /FI "PID eq <pid>" /FO CSV`, the `sysinfo` crate, or
conservatively return `{"running": false}` (PID-liveness is nice-to-have on
Windows; the pid-file lock gives partial validation). Mirror #2915 / #2904.

**Brief:** `win-cli-doctor-gateway-kill-portability`

---

## Finding 9 — P3 — `argo doctor` disk usage spawns `df`

**File:** `crates/argo-cli/src/handlers/doctor_storage.rs:266`

```rust
Command::new("df").arg("-Pk").arg(path)
```

**Why it breaks Windows:** `disk_usage()` calls `df` unconditionally; no `df` on
Windows → spawn fails, `.ok()?` → `None`, so `capacity_status()` reports
`WARN`/blank instead of real disk diagnostics. Graceful (no crash) but the
diagnostic is silently incomplete. Lowest severity — diagnostic-only, degrades
cleanly. **P3.**

**Recommended fix:** `#[cfg(unix)]` the `df` path + a Windows equivalent
(`fsutil`, Win32 `GetDiskFreeSpaceEx`, or a cross-platform crate like `sysinfo`),
keeping the `Option<(u64, u64, u64)>` signature; return `None` only on genuine
platform gaps.

**Brief:** `win-doctor-disk-usage-df-portability`

---

## Recommended brief-wave ordering for the conductor

1. **`win-terminal-streaming-bash-portability` (P1)** — only complete feature
   break; ship first. *(The Windows fleet is taking this one directly — see the
   ledger row + PR; it does not need a conductor-authored brief.)*
2. `win-sandbox-localsandbox-bash-cfg-gate` (P2) — code-exec fallback on Windows.
3. **Combined HOME→USERPROFILE brief** covering findings 4, 5, 6+7 — same pattern,
   same proven #2904 helper; could be one PR per crate (argo-vertical-packs /
   argo-marketplace / argo-config×2) or one sweep. These are the cheapest, lowest-risk.
4. `win-cli-doctor-cert-summary-sh-portability` + `win-cli-doctor-gateway-kill-portability`
   + `win-doctor-disk-usage-df-portability` (P2/P2/P3) — the three `argo doctor`
   shell-outs; could be one "doctor cross-platform" sweep brief.

## Method / provenance

- Generated by a `Workflow` fan-out (run `wf_7a2467b1-7e8`): parallel sweep of
  production crates → dedup → per-finding adversarial verification (real-bug /
  production / cfg-gated / already-briefed checks). Raw 11 → 9 deduped → 9 confirmed.
- Audited against `origin/main` at the time of the run; re-spot-checked 3/9
  against `6298e118d` (this turn). Brief-id collision check: clean.
- This doc is **advisory** — no ledger row, no scope claim. The conductor decides
  what becomes a brief.
