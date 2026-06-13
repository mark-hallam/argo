# Windows gateway end-to-end smoke (`win-gateway-e2e-on-windows`)

**Question the brief asks:** does the *server* half of SKYCOT actually SERVE
on Windows? (The CLI/`doctor` smoke covers the binary; this covers the served
HTTP API.)

**Verdict: 🟢 YES — the gateway boots and serves a real HTTP API on Windows.**
`/health` + `/readyz` + liveness + `/version` return 200; a real authenticated
round-trip (audit feed + workspace row + audit count) reads back live data from
the on-disk SQLite store; the bearer-auth gate correctly 401s protected routes
when bypass is off. Two honest GAPs recorded below (neither blocks "does it
serve"): the boot requires a provider-key *presence* check, and the **debug
binary under test was built WITHOUT `--features bundled-sqlcipher`, so the
at-rest DB is plaintext** (the `PRAGMA key` path runs but no-ops on stock
SQLite).

NO THEATER: every row below is real `curl` output captured on this Windows host.

---

## Environment

| Field | Value |
|---|---|
| Machine | `Latitude5440` (Windows 11 Pro 10.0.26200, x86_64) |
| Binary | `argo argo 0.0.1 (2434e59 2026-06-13)` — `target/debug/argo.exe` |
| Target triple | `x86_64-pc-windows-msvc` |
| rustc | `1.96.0 (ac68faa20 2026-05-25)` |
| Build profile | `debug` (no `--features bundled-sqlcipher` — see GAP-2) |
| Schema version (live) | `94` |
| `build_commit` (from `/health`) | `2434e59` |

**Binary-vs-main disclosure (NO THEATER):** the binary tested is the most
recent real Windows build present in the shared `CARGO_TARGET_DIR`
(`%LOCALAPPDATA%\argo-target\debug\argo.exe`), at commit `2434e59` — **42
commits behind `origin/main`** at smoke time. A fresh-from-main rebuild was
*deliberately deferred*: the shared target dir was actively contended by a
sibling Windows worker (14 live `rustc`/`cargo` processes + `link.exe`), and a
competing `cargo build` would thrash its incremental cache. The 42-commit gap
does **not** touch the gateway-serve runtime path — `git log 2434e59..origin/main
-- crates/argo-cli/src/handlers/gateway/ crates/argo-store/` returns only
`#2917` (test-only cascade seed), `#2912` (non-unix secret-perm WARN —
observability), and `#2909` (approvals double-submit idempotency, an unrelated
route). **Recommend the conductor re-run this smoke against a fresh
main-built binary** once the shared target dir is free.

---

## How it was booted

Isolated, non-interfering run (dedicated `ARGO_HOME` + cwd under `%TEMP%`, so no
shared `~/.argo` state is touched):

```bash
ARGO_HOME=<temp>/argo-gw-e2e \
SKYCOT_DB_KEY_BASE64=<base64 of 32 random bytes> \
ANTHROPIC_API_KEY=sk-ant-placeholder-...        # placeholder, see GAP-1 \
argo.exe gateway start --bind 127.0.0.1 --port 7401 --quiet
```

- **`127.0.0.1` bind = dev mode** — the production bearer-token + enterprise
  at-rest-encryption boot gates are loopback-bypassed by design.
- **`SKYCOT_DB_KEY_BASE64`** is *required* to boot — without it the store-open
  step fails deriving a SQLCipher key (no OS-keyring entry in this temp env).
- **`ANTHROPIC_API_KEY`** is a **placeholder**, supplied only to satisfy the
  boot-time provider-key *presence* check (GAP-1). The endpoints exercised here
  make **no** provider calls, so the key's validity is irrelevant.
- The authenticated-read rows below used **`ARGO_GATEWAY_AUTH_BYPASS=1`** (a
  non-production-only dev escape hatch) to drive the bearer-gated GETs; the
  unauthenticated run (gate ON) is recorded separately to prove the gate works.

Real boot sequence (`--quiet`, abridged to the load-bearing lines):

```
argo gateway: startup_check ok in 185 ms
argo gateway: duty scheduler enabled (interval=60s, gate=closed, runner=wired, sink=wired)
argo gateway: cu_screenshot sweep enabled (interval=3600s, retention=14 days)
argo gateway: audit chain watchdog enabled
argo gateway: worker supervisor constructed (cage + oom-restart wired)
argo gateway: skill learning cron enabled (workers=0, interval=3600s)
argo gateway: listening on 127.0.0.1:7401 (pid 10004)
argo gateway: workspace = 'argo-gw-cwd'
```

`netstat` confirmed the listener: `TCP 127.0.0.1:7401 ... LISTENING`. The
SQLite store materialised on disk (`sessions.db` 1.5 MB + `-wal` + `-shm`).

---

## Per-endpoint results

### Public surface (auth gate ON — no bypass)

| Method · Path | HTTP | Body | Result |
|---|---|---|---|
| `GET /health` | **200** | `{"status":"ok","version":"0.0.1","schema_version":94,"build_commit":"2434e59","uptime_seconds":30}` | ✅ PASS |
| `GET /readyz` | **200** | `{"status":"ok",...,"schema_version":94,...}` | ✅ PASS |
| `GET /livez` | **200** | `{"status":"alive","epoch_micros":1781367477221585}` | ✅ PASS |
| `GET /health/live` | **200** | `{"status":"alive",...}` | ✅ PASS |
| `GET /health/ready` | **200** | `{"status":"ready",...}` | ✅ PASS |
| `GET /version` | **200** | `{"version":"0.0.1","profile":"debug"}` | ✅ PASS |
| `POST /api/v1/auth/signup` `{"email":...}` | **200** | `{"status":"not_configured"}` | ✅ PASS (route reachable; magic-link delivery unconfigured — GAP-3) |
| `POST /api/v1/demo/mint-principal` | **401** | `{"error":{"code":"unauthorized","message":"authentication required"}}` | ✅ PASS (auth gate correctly rejects) |
| `GET /audit` | **401** | `{"error":{"code":"unauthorized",...}}` | ✅ PASS (gate active) |
| `GET /workspace/info` | **401** | `{"error":{"code":"unauthorized",...}}` | ✅ PASS (gate active) |

The structured 401s (not a crash / not a 500) prove the bearer-auth middleware
runs correctly on Windows.

### Authenticated round-trip (`ARGO_GATEWAY_AUTH_BYPASS=1`, dev)

| Method · Path | HTTP | Body | Result |
|---|---|---|---|
| `GET /audit` | **200** | `{"breaks":0,"kinds":{"hipaa_compliance_check":3,"http.request":8},"ok":true,"rows":11}` | ✅ PASS — live hash-chained audit feed, clean chain (`breaks:0`) |
| `GET /audit/count` | **200** | `{"count":12,"kind":null,"session":null}` | ✅ PASS |
| `GET /workspace/info` | **200** | `{"created_at_micros":...,"name":"argo-gw-cwd","root_path":"C:\\Users\\operator\\AppData\\Local\\Temp\\argo-gw-cwd","workspace_id":"019ec1c6-17b1-71e0-a189-205a7e444c49"}` | ✅ PASS — workspace row persisted to & read from the **Windows** SQLite store (note the native `C:\…` `root_path`) |

This is a genuine read-write-read round-trip on Windows: gateway boot wrote the
workspace row + audit rows into `sessions.db`; the GETs read them back; the
audit chain (`http.request` rows = the gateway auditing its own requests)
verified clean.

---

## Findings / GAPs (honest)

- **GAP-1 — boot requires a provider-key *presence*.** Without
  `ANTHROPIC_API_KEY`/`OPENAI_API_KEY` the gateway refuses to bind (exit 78:
  `STARTUP CHECK FAILED ... no provider API key set`). This is the documented
  startup health check, not a Windows-specific issue — but it means a bare
  `argo gateway start` on a fresh Windows box won't serve until a provider key
  (or `--skip-startup-check`) is supplied. Not a portability defect; recorded
  for operator awareness.
- **GAP-2 — at-rest encryption is NOT active in this build.** `sessions.db`
  begins with the plaintext `SQLite format 3\0` magic, so the debug binary was
  built **without `--features bundled-sqlcipher`**: the `SKYCOT_DB_KEY_BASE64`
  → `PRAGMA key` path runs but no-ops on stock SQLite. The *key requirement* is
  still enforced (boot fails with no key source), but real SQLCipher
  encryption-at-rest on Windows/MSVC is **unverified by this smoke**. A
  production Windows build must use `--features bundled-sqlcipher`; verifying
  that SQLCipher links + encrypts on `x86_64-pc-windows-msvc` is a distinct
  follow-up (matches the known deploy caveat for the prod image).
- **GAP-3 — magic-link signup returns `not_configured`.** The route is
  reachable and returns a well-formed 200, but no email/SMTP delivery is
  configured in this minimal env, so no link is actually sent. Expected for a
  bare boot; a full signup→verify round-trip needs mail config.
- **Cosmetic — Windows perm WARN at boot.** `.bootstrap-token` writes with a
  one-shot WARN: "written without unix 0600 perm-hardening on this platform;
  restrict access via filesystem ACLs". This is the expected non-unix
  perm-hardening observability warn (the `#2901`/`#2912` pattern), not a
  failure.

---

## Conclusion

The SKYCOT gateway **serves a real HTTP API on Windows**: it boots, opens its
SQLite store, runs its full background-task suite, binds a listener, passes
health/liveness/version, enforces bearer auth, and round-trips authenticated
audit + workspace reads against the on-disk store — all verified with real
`curl` output on `x86_64-pc-windows-msvc`. The server half is **live on
Windows**, with the two substantive caveats above (provider-key boot gate;
SQLCipher-at-rest unverified because the test binary lacks the feature) recorded
honestly rather than papered over.

_Verified by the Windows fleet (Latitude5440) on the `2434e59` debug binary.
Conductor: re-run against a fresh main build when the shared target dir frees._
