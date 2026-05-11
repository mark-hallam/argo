# Soak test — operator runbook

The soak test (`scripts/soak-skycot.sh`) drives a steady prompt
stream against a fresh SKYCOT workspace for ~4 hours, then asserts
five resource invariants. It catches gradual-creep failure modes
that `scripts/smoke-skycot.sh` (first-30-seconds) and production
monitoring (week-1 cadence) both miss.

## What it catches

| Assertion         | Threshold      | Failure class                                  |
| ----------------- | -------------- | ---------------------------------------------- |
| `no_crash`        | 4/4 workers up | panic / OOM / abort in a worker or the gateway |
| `memory_growth_ok`| ≤ 1.5× over 4h | leak via never-dropped Arc, unbounded cache    |
| `fh_growth_ok`    | ≤ 1.5× over 4h | unreaped subprocess, never-closed sqlite/fd    |
| `audit_chain_intact` | `verify` ok | chain-hash bug under concurrent appenders      |
| `gateway_healthy` | `/health` 200  | death-spiral via blocked tokio / stuck mutex   |

Each assertion is a 1:1 map from a specific bug class to a specific
operator response. When an alert fires, the runbook entry for that
assertion (below) names the subsystem to chase first.

## What it does NOT catch

- **Live-provider failure modes.** The default config uses the
  `stub-echo` fakeprovider; real Anthropic/OpenAI/Gemini quirks
  (rate-limit storms, model rollback, cost spikes) are not in
  scope. Set `ARGO_SOAK_PROVIDER=anthropic` locally to opt-in, but
  budget for ~$1–5 USD per 4-hour run.
- **Multi-tenant noise.** The soak runs single-tenant. Tenant-
  isolation bugs need their own brief.
- **Real-network jitter.** 127.0.0.1-only.
- **Cost-bounded LLM-call paths.** `mock` / `stub-echo` short-circuit
  the provider transport entirely.
- **Kernel-level resource tracking** (cgroup memory, ulimit walls).
  The soak uses userspace `ps` / `/proc` only.

## Modes

| Mode      | Trigger                    | Duration | Cadence  |
| --------- | -------------------------- | -------- | -------- |
| default   | `bash scripts/soak-skycot.sh` | 240 min | snapshot every 5 min; baseline at 30 min |
| fast      | `ARGO_SOAK_FAST=1`         | 20 min  | snapshot every 60 s; baseline at 120 s |
| override  | `ARGO_SOAK_DURATION_OVERRIDE_MIN=N` | N min | snapshot every 10 s; baseline at 20 s |

Fast-mode is a **smoke-of-the-soak**, not a substitute. The 20-min
window is too tight to surface the slow-creep memory leaks the 4-hour
mode catches, but it exercises all five assertions and the JSON
summary shape — fine for dev-loop validation and CI sanity checks.
The override mode is for `scripts/soak-skycot-tests.sh`; do not use
it as a production gate.

## How to run locally

```bash
# Fast-mode (≈ 20 min, no provider key needed):
ARGO_SOAK_FAST=1 bash scripts/soak-skycot.sh

# Full 4-hour run against the in-tree fakeprovider:
bash scripts/soak-skycot.sh

# Full 4-hour run against Anthropic (costs real $; ~$1–5):
ANTHROPIC_API_KEY=... ARGO_SOAK_PROVIDER=anthropic \
  bash scripts/soak-skycot.sh

# JSON summary lands here:
ls -la target/soak-report-*.json
```

## Reading the JSON summary

```json
{
  "start": "2026-05-12T04:00:00Z",
  "end":   "2026-05-12T08:00:00Z",
  "duration_min": 240,
  "workers": 4,
  "provider": "stub-echo",
  "vertical": "gtm-engineer",
  "prompts_sent": 4800,
  "prompts_succeeded": 4798,
  "baseline_rss_kb": 312000,
  "final_rss_kb": 351000,
  "memory_growth_pct": 12.50,
  "baseline_fh": 84,
  "final_fh": 96,
  "fh_growth_pct": 14.28,
  "audit_chain_intact": true,
  "assertions": {
    "no_crash":             true,
    "memory_growth_ok":     true,
    "fh_growth_ok":         true,
    "audit_chain_intact":   true,
    "gateway_healthy":      true,
    "all_pass":             true
  }
}
```

The baseline is captured at the 30-min mark (or 2-min in fast-mode);
growth is `(final / baseline) − 1`. The script exits non-zero if any
assertion is false.

## How to interpret a failure

When an assertion fires, the per-assertion subsystem ownership is:

| Assertion         | First place to look                                         |
| ----------------- | ----------------------------------------------------------- |
| `no_crash` false  | `target/soak-skycot-*/logs/worker-N.log` for the dead pid; grep for `panicked at`, `SIGABRT`, `out of memory`. Likely culprits: `argo-agent-runner`, `argo-cli/work_loop`, recent `Arc<Mutex<...>>` changes. |
| `memory_growth_ok` false | Run the soak with `ARGO_SOAK_FAST=1` repeatedly under `heaptrack` on a single worker pid. Likely culprits: never-evicted `lru` caches, growing `Vec<Audit>` buffers in `argo-store`, never-dropped session handles. |
| `fh_growth_ok` false | `lsof -p <worker_pid>` mid-run; look for accumulating sqlite cursors or socket fds. Likely culprits: missing `drop` on `SessionStore::transaction()`, MCP subprocess that doesn't reap. |
| `audit_chain_intact` false | `argo audit verify --json` standalone, then `argo audit show --kind ...`. Likely culprit: a concurrent-appender path in `argo-audit::append` that races on `prev_hash`. |
| `gateway_healthy` false | `target/soak-skycot-*/logs/gateway.log`; look for tokio-runtime blocked-poll warnings or recent `Mutex::lock` adds in `argo-gateway`. |

## Operating the nightly CI

`.github/workflows/soak.yml.disabled` ships disabled while billing is
locked out. To activate:

```bash
git mv .github/workflows/soak.yml.disabled .github/workflows/soak.yml
git commit -m "ci(soak): enable nightly soak (billing restored)"
```

The workflow runs `0 4 * * *` (4am UTC daily), uploads the JSON
report as a workflow artifact, and opens an issue tagged
`soak-failure` if any assertion trips. Manual dispatch is available
year-round via `workflow_dispatch`. Same pattern as
`audit.yml.disabled` and `oci-smoke.yml.disabled`.
