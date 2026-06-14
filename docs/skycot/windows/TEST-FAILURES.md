# Windows test-suite — definitive failure inventory

**Brief**: `win-full-workspace-test-suite-now-all-compiles` (P1 — the FIRST
all-crates Windows test pass, now that argo-cli + the CU-executor compile).
Supersedes the `win-test-suite-run-and-failure-inventory` (#2911) draft, which
**excluded argo-cli** (its test target wouldn't compile).
**Machine**: `Latitude5440` — Windows 11 Pro, `x86_64-pc-windows-msvc`,
cargo/rustc 1.96.0. Run against `origin/main` `306d2405` (+ the one-line
argo-cli test-compile fix in this PR). Deliverable is the categorized inventory,
not green.

## Headline

**argo-cli is Windows-test-compilable for the first time.** #2911 hit
`E0599: no method named dispatch found for ComputerUseMacOSExecutor` and had to
`--exclude argo-cli`. Root cause: `worker_actions_executor_cu_macos_tests.rs`
calls `exec.dispatch()` (an `Executor` trait method) in its
`#[cfg(not(target_os = "macos"))]` test but never imported the `Executor`
trait. **This PR adds that import** (used on every platform — the macOS-gated
test calls `.dispatch()` too), so `cargo test -p argo-cli` now runs. That is the
one fix landing here; the failures below are inventory → follow-up briefs.

## Definitive all-crates result (current main)

| Surface | Pass | Fail | How verified |
|---|---:|---:|---|
| **argo-cli** (newly unblocked, biggest crate) | **3684** | **209** | `cargo test -p argo-cli --no-fail-fast` this PR |
| Previously-failing crates (capability, identity, store, tool, memory) | **1360** | **1** | `cargo test -p argo-capability -p argo-identity -p argo-memory -p argo-store -p argo-tool` this PR |
| `argo-executor-computer-use-windows` (CU crown jewel) | **52** | **0** | `cargo test -p ...-windows` (#2925) |
| Rest of workspace (excl. above) | **~397** | **0\*** | #2911 baseline (those crates unchanged on the Windows axis) |

\* #2911's 6 failing groups in capability/identity/store/tool were **fixed** by
the merged fleet PRs #2913 / #2914 / #2915 / #2917 — confirmed green by the
re-test above. The lone survivor is the argo-memory timestamp test (below).

## Non-argo-cli: the one survivor

### argo-memory — `shared_team::tests::author_stamp_survives_round_trip`

- **Cat (a) — lossy timestamp round-trip, latent on unix too.** Original has
  **nanosecond** precision (`…300` ns); serialization truncates to
  **microseconds**, so the round-trip isn't equal. The Windows high-res clock
  surfaces it (`…300` ns); macOS values are often µs-aligned (`…000`), hiding it.
- **Still unaddressed** — #2911 recommended `win-shared-team-timestamp-precision`;
  no fix has merged. **Recommend authoring that brief** (decide ns-serialize vs
  truncate-before-compare; it's a real precision bug, not Windows-only). Not
  fixed here to keep this PR's blast radius to the argo-cli compile unblock.

## argo-cli: the 209 failures, root-caused

84 integration-test binaries + part of the `--bin argo` unit target. They
collapse into a small number of **root causes**, not 209 independent bugs:

### 1. Gateway-spawn integration harness on Windows — **~166 failures (DOMINANT)**

- Signature: `gateway crashed before listener bound (stderr closed without
  announce)` (125) · `gateway listener signal: GatewayCrashed` · `listener did
  not announce in Ns` · then cascading `401 Unauthorized` (41).
- These are the `gateway_smoke*`, `audit_*`, `dashboard_*`, `mcp_*_smoke`,
  `setup_*`, `session_*_vertical`, `genesis_*`, `gdpr_*`, … binaries — each
  spawns a real `argo gateway` child and curls it.
- **Root cause is ONE thing, and it's already documented**: the gateway's boot
  gate refuses to bind without a provider key + a SQLCipher DB key (see
  [`GATEWAY-E2E.md`](GATEWAY-E2E.md) — exit 78 `no provider API key set`). The
  integration harness boots a test gateway in a way that satisfies this on macOS
  but not on Windows (env/spawn/keyring path). The gateway itself **does** serve
  on Windows once given those (proven in GATEWAY-E2E.md) — so this is a
  **test-harness** gap, not a product defect.
- **Cat (a/b) — single large class.** **Recommend ONE brief:
  `win-gateway-integration-test-harness-spawn`** — make the argo-cli gateway-spawn
  test helper provide the provider/DB key (or `--skip-startup-check`) on Windows.
  Fixing the harness should clear ~166 of the 209 in one stroke. NOT attempted
  here (it's a shared test-harness change spanning 80+ test binaries — needs its
  own focused PR + conductor unix re-verify).

### 2. Unix-subprocess spawn assumptions — **~15 failures**

- `rotate-keys subprocess spawn: The system cannot find the path specified`
  (security_rotate_keys_subprocess ×2); MCP `echo` server spawned as `python3`
  (Windows ships `python`, not `python3`) → `MCP spawn failed: cannot find the
  path specified` (worker_actions_executor_mcp / browser / promotion).
- **Cat (a/b).** **Recommend brief: `win-subprocess-spawn-portability`** (audit
  every `Command::new`/shell-spawn in argo-cli handlers + tests for unix-only
  binaries; this is the same class as the merged process-registry #2915). Some
  are production spawns (rotate-keys) → check production first.

### 3. TOML manifest fixtures embed un-escaped Windows paths — **~10 failures**

- Test fixtures build a manifest string like `command = "C:\Users\…\python"` and
  feed it to the TOML parser, which reads `\U` / `\m` as escape sequences →
  `invalid unicode N-digit hex code`.
- **Cat (b) — test fixture bug.** **Recommend brief:
  `win-toml-fixture-path-escaping`** (normalize fixture paths to `/`, or use TOML
  literal strings `'…'`, in the affected test helpers).

### 4. Instant overflow — **2 failures**

- `overflow when adding duration to instant` (std `time.rs:746`) — a test adds a
  large duration to `Instant::now()`; Windows' `Instant` range/epoch differs from
  macOS, so the add overflows. **Cat (a/c).** Small — **recommend folding into a
  `win-instant-duration-overflow` fix** (saturating add, or a smaller test
  duration).

### 5. Remainder (~16) — path-sep + error-string unit assertions

- `eval\known_good_actuals.jsonl` vs `eval/…` (path-sep in assertions);
  provider-manifest-missing error text (`The system cannot find the path
  specified. (os error 3)` vs unix `os error 2`) in the
  `vertical_eval_judge_*::build_*_backend_errors_clearly_when_manifest_missing`
  tests; `backup_create` default-path, `handoff_policy` error file-path, etc.
- **Cat (b) — test-assumes-unix literals**, the same class #2908 cleared for
  argo-tool. **Recommend brief: `win-argo-cli-path-sep-and-errstr-test-audit`.**

## Recommended follow-up briefs (for the conductor to author)

| Brief | Clears | Cat |
|---|---:|---|
| `win-gateway-integration-test-harness-spawn` | ~166 | (a/b) test-harness gateway boot |
| `win-subprocess-spawn-portability` | ~15 | (a/b) unix-spawn (rotate-keys, python3 MCP) |
| `win-toml-fixture-path-escaping` | ~10 | (b) `\`-in-TOML fixture paths |
| `win-argo-cli-path-sep-and-errstr-test-audit` | ~16 | (b) path-sep + os-error-string literals |
| `win-instant-duration-overflow` | 2 | (a/c) Instant add overflow |
| `win-shared-team-timestamp-precision` | 1 | (a) lossy ns→µs round-trip (latent unix) |

## Honest gaps (NO THEATER)

1. **The full `cargo test --workspace` was NOT run as one invocation.** With
   argo-cli's test target now compiling, it *could* be — but it would re-run the
   ~397 #2911-baseline groups + argo-cli's slow gateway-spawn timeouts for little
   new signal. Instead this inventory is assembled from: argo-cli (full run, this
   PR) + the 5 previously-failing crates (full run, this PR) + CU-executor
   (#2925) + the #2911 baseline for the untouched remainder. Every number above
   is a real run; the only *carried* (not re-run this PR) figure is the ~397
   baseline, flagged with `*`.
2. **The ~166 gateway-spawn failures share one suspected root cause** (harness
   boot gate) but that's an inference from the crash signature + GATEWAY-E2E.md,
   confirmed only to the point of "the gateway needs keys to bind." The owning
   brief should confirm the exact harness gap before fixing.
3. **Single run** — the (c)-suspects (instant overflow) seen once; owning briefs
   re-run in isolation.

_Verified by the Windows fleet (Latitude5440). The argo-cli compile unblock in
this PR is unix-safe (adds a trait import used on every platform); conductor
re-verifies argo-cli on macOS._
