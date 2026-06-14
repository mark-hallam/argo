# `argo gateway stop` on Windows (`win-gateway-stop-portability`)

**Question:** can `argo gateway stop` actually terminate the gateway on native
Windows? (Before this fix it could not — the `#[cfg(not(unix))]` `signal_term`
arm returned `Err("SIGTERM not supported on this platform")`, so the operator had
to `taskkill` by hand. This is the root cause of the leaked-gateway processes the
journey [`E2E-FULL-JOURNEY.md`] and soak [`E2E-SOAK.md`] both had to `taskkill`
around.)

**Verdict: 🟢 `argo gateway stop` now stops the gateway on Windows** — exit 0,
the process is gone (`tasklist` confirms), the PID file is removed, and the
gateway stops serving. Unix SIGTERM path unchanged.

NO THEATER: every value below is real captured output (`gwstop-results.json`).

---

## The bug (before)

`crates/argo-cli/src/handlers/gateway/lifecycle.rs`:
- `signal_term` `#[cfg(not(unix))]` → `Err("SIGTERM not supported on this platform")`
  → `boot.rs::stop` returned `kill pid <N>: SIGTERM not supported…` and the
  gateway **kept listening**.
- `pid_is_alive` `#[cfg(not(unix))]` → always `true`, so the stale-PID cleanup
  branch could never fire on Windows.
- `boot.rs` printed `sent SIGTERM to pid <N>` even though nothing was sent.

## The fix

- **`signal_term` (Windows)** — no POSIX signals, so honour the SIGTERM intent
  via the built-in `taskkill`: try a graceful tree-stop (`taskkill /PID <pid>
  /T`); a console daemon with no window handler can only be terminated
  forcefully, so fall back to `/F`. `/T` includes any child processes.
- **`pid_is_alive` (Windows)** — real liveness via `tasklist /FI "PID eq <pid>"`
  (live process → its quoted pid in stdout), so the stop path's stale-PID check
  and stop→confirm are correct.
- **`boot.rs::stop`** — platform-accurate message; on Windows it also removes the
  PID file (the forceful `taskkill` bypasses the daemon's `PidGuard`, which on
  unix removes the file during graceful shutdown). **Unix path byte-identical.**

## Unit tests (cross-platform — run on Windows AND the swarm)

`cargo test --release -p argo-cli --bin argo lifecycle::tests` → **2/2 PASS**:
- `current_process_reads_as_alive` — the running test process reads alive via
  `tasklist`.
- `improbable_pid_reads_as_dead` — pid `2_147_483_646` reads dead (so the
  stale-cleanup branch can actually fire).

## LIVE end-to-end (real Windows)

| Field | Value |
|---|---|
| Host / binary | `LATITUDE5440` · `argo 0.0.1 (2fffaba)` release, **built from the branch working tree with the fix** (the version string embeds the base commit; the behaviour below proves the fix is linked) |
| Gateway | started on `127.0.0.1:7434`, fresh `ARGO_HOME`, pid **21164** |
| `/health` before stop | **200** `{"status":"ok","schema_version":94,"build_commit":"2fffaba"}` |
| PID file before | `…\home\gateway.pid` exists, contents `21164` (matches the live pid) |

```
$ argo gateway stop
argo gateway: stopped pid 21164 (taskkill)      ← exit 0
```

| Assertion (after stop) | Result |
|---|---|
| `stop` exit code | **0** |
| stop message | `argo gateway: stopped pid 21164 (taskkill)` (platform-accurate — no false "sent SIGTERM") |
| process alive? | **false** — `tasklist /FI "PID eq 21164"` → `INFO: No tasks are running which match the specified criteria.` |
| PID file removed? | **yes** (`gateway.pid` gone) |
| `/health` still reachable? | **no** (gateway stopped serving) |
| **Verdict** | ✅ **PASS** |

## Honest notes (NO THEATER)

- The Windows stop is **forceful** (`taskkill /F` after the graceful `/T`
  attempt, which a windowless console daemon does not honour). This matches the
  operator intent ("stop it") but is not a graceful in-process shutdown; the
  unix SIGTERM path remains the graceful one and is unchanged. A truly graceful
  Windows stop (a cross-process `CTRL_BREAK` to the gateway's console group)
  needs `windows-sys` + `CREATE_NEW_PROCESS_GROUP` at start time and is out of
  scope here.
- Verified on the **release** binary; the unix path is byte-identical and is
  re-verified by the conductor on the unix side per fleet rules.

_Verified by the Windows fleet (Latitude5440) on `x86_64-pc-windows-msvc`.
Gateway pid 21164 started → `/health` 200 → `argo gateway stop` exit 0 →
`tasklist` confirms gone → PID file removed → `/health` unreachable. Raw:
`gwstop-results.json`._
