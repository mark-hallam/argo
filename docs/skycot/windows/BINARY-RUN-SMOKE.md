# Windows binary-run smoke — `argo.exe`

**Brief**: `win-binary-runs-smoke-doctor-and-cli` (P2). Compiling/testing ≠
running — this confirms the built **release** `argo.exe` actually RUNS on a real
Windows host.

| Field | Value |
|---|---|
| Machine | `Latitude5440` — Windows 11 Pro, `x86_64-pc-windows-msvc` |
| Binary | `…\argo-target\release\argo.exe` (81 MB), `cargo build --release -p argo-cli` (exit 0, default features) |
| Build commit | `41fc3d7` (2026-06-14), schema v94 |

## Results — PASS/GAP per command

### 1. `argo --version` — ✅ PASS
```
argo argo 0.0.1 (41fc3d7 2026-06-14)
  target:   x86_64-pc-windows-msvc
  rustc:    rustc 1.96.0 (ac68faa20 2026-05-25)
  profile:  release
  features: default
```
Runs, exit 0. **Minor cosmetic NIT**: the name renders doubled — "argo argo 0.0.1"
(the binary name is prepended to a version string that already starts with "argo").
Not Windows-specific; low-priority cleanup.

### 2. `argo doctor` — ✅ PASS (exit 2 = uninitialized workspace, accurate)
Runs end-to-end and produces the full health report. Exit 2 reflects a
not-yet-initialized workspace (this cwd has no `argo.toml`), not a crash. Windows
specifics all correct:
```
target:    x86_64-pc-windows-msvc
binary:    C:\Users\operator\AppData\Local\argo-target\release\argo.exe
ARGO_HOME: C:\Users\operator\.argo (OK)        ← Windows home resolves correctly
argo.toml: ...\argo (MISSING)               ← accurate (no workspace init here)
db:        schema v94  workspace=registered ← SQLite opens on Windows
encryption: OFF (plaintext)
routes:    275 endpoints (157 GET, 104 POST)
Network checks: DNS OK api.anthropic.com / api.openai.com / skycot.com
```
**No unix-path failures.** Home resolved to `C:\Users\operator\.argo` (validates the
HOME→USERPROFILE work). Pre-existing `WARN … tenant-unscoped credential read` lines
are cred-loader noise, not Windows-specific.

### 3. `argo session list --json` — 🟡 GAP (JSON-array contract)
Exit 0, but **emits 0 bytes** on stdout for an empty session list. The CLI JSON
contract (CLAUDE.md: "JSON output for CLI list/show/status commands") expects a
valid JSON array — `[]` — not an empty string (`json.load` fails: "Expecting
value"). May not be Windows-specific (could be an empty-list bug on all platforms);
the conductor should compare on unix. **Recommend a follow-up brief**:
`win-session-list-json-empty-array` (emit `[]` for zero sessions).

### 4. `argo gateway start --port N` + `/health` + stop — ✅ PASS (boots + serves)
Two runs:

**(a) No provider key (default env)** → refuses to bind, **correct guard**:
```
WARN … bootstrap token written without unix 0600 perm-hardening on this platform;
     restrict access via filesystem ACLs  path=C:\Users\operator\.argo\.bootstrap-token
argo gateway: STARTUP CHECK FAILED (no provider API key set
     (need ANTHROPIC_API_KEY or OPENAI_API_KEY)); refusing to bind   [exit 78]
```
This is a platform-agnostic config guard (same on unix), **not** a Windows
failure. Note the bootstrap-token line is the *honest documented-gap warning*
(unix 0600 unavailable → use ACLs) working as designed, with the correct Windows
home path.

**(b) With a dummy `ANTHROPIC_API_KEY` (presence-only startup check)** → boots +
serves:
```
argo gateway: startup_check ok in 27 ms
argo gateway: listening on 127.0.0.1:7412 (pid 16180)
  (duty scheduler / kpi rollup / approval reminder / cu_screenshot sweep /
   audit chain watchdog / worker supervisor [cage + oom-restart] all wired)

$ curl http://127.0.0.1:7412/health
{"status":"ok","version":"0.0.1","schema_version":94,"build_commit":"41fc3d7","uptime_seconds":11}
HTTP_CODE=200
```
**The gateway boots and serves the API on Windows.** Stopped cleanly
(`taskkill /F /PID 16180` → port released, subsequent curl refused).

## Verdict

The `argo` binary **runs on Windows**: `--version` ✓, `doctor` ✓ (correct Windows
home + DB + network), and the **gateway boots and serves `/health` (HTTP 200)** ✓.
The only real gap is `session list --json` emitting an empty string instead of
`[]`. The gateway's no-key refusal and the bootstrap-token ACL warning are correct,
documented behaviors — not Windows bugs.

## Honest gaps / follow-ups
1. **`session list --json` → `[]`** for empty lists (recommend a brief; verify
   unix first).
2. **`argo argo` doubled version name** — cosmetic, low-pri.
3. Gateway `/health` was exercised with a **dummy** API key (presence-only startup
   check); no real provider traffic was made.
