# Mission load-bearing wire — Nightly 4-hour soak test

**Owner**: claude. **Depends on**: existing `scripts/smoke-skycot.sh` smoke runner; `mission-criterion-load-tests` for the in-process throughput numbers (separate brief, may queue parallel).
**Authored against**: origin/main `47749a1b` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-soak-test-nightly`.

> SOURCE: MISSION_BACKLOG.md Section 1 #16 ("Add soak test (4-hour multi-worker run) as nightly"). **Tier M (reliability).** Smoke tests catch first-30-seconds bugs. Production catches week-1 bugs. The gap in between (memory leak over hours, file-handle exhaustion, audit-chain hash collision under sustained load) is invisible to both. This brief lands a soak runner that exercises the runtime for 4 hours and asserts resource ceilings hold.

## Scope

NEW `scripts/soak-skycot.sh` (≤ 280 LOC) — orchestrates a 4-hour multi-worker run against the local gateway + a configurable provider (`ARGO_SOAK_PROVIDER=anthropic|openai|gemini|stub-echo`):

- Spin up 1 gateway + 4 worker processes (configurable via `ARGO_SOAK_WORKERS`)
- Submit a continuous stream of 12-token "what is 2+2" prompts at 5 prompts/min/worker (total 20 prompts/min)
- Every 5 minutes: snapshot {RSS memory per pid, open file handles per pid, sqlite-store size on disk, audit-chain tail-seq, gateway requests-total counter}
- At 4 hours: assert
  - **No worker crashed** (all 4 pids still alive)
  - **No memory growth > 50% over 4h** (RSS_4h / RSS_30min ≤ 1.5)
  - **No file-handle growth > 50% over 4h**
  - **Audit chain is intact** (`argo audit verify --full` exits 0)
  - **Gateway is still serving** (`/health` returns 200)
- Emit machine-readable summary JSON to `target/soak-report-<timestamp>.json` + human-readable markdown to stderr

Optional **fast-mode** (`ARGO_SOAK_FAST=1`) shrinks to 20 minutes for CI / dev-loop validation — same assertions, smaller windows. Document in the runbook.

NEW `.github/workflows/soak.yml.disabled` (≤ 80 LOC) — schedule `0 4 * * *` (daily 4am UTC). Mirrors the `.disabled` pattern from #1325. Runs `bash scripts/soak-skycot.sh` w/ `ARGO_SOAK_PROVIDER=stub-echo` (no provider key needed) under a Linux runner. Posts the summary JSON as an artifact + comments on `main` if any assertion fails.

NEW runbook `docs/skycot/runbook/soak-test.md` (≤ 240 LOC):
- What it catches (memory leaks, FH leaks, audit-chain bugs, gateway death-spirals)
- What it doesn't (cost-bounded LLM-call paths, multi-tenant noise, real-network jitter)
- How to interpret a failure (which assertion fired → which subsystem to chase)
- How to run locally w/ `ARGO_SOAK_FAST=1`

NEW stub-echo provider manifest `manifests/providers/stub-echo.toml` (≤ 50 LOC) — fakeprovider that returns the canned response `{"role": "assistant", "content": "4"}` after a `sleep_ms` (default 30ms) so the soak's 20-prompts/min is realistic without burning real provider $. (If `manifests/providers/stub-echo.toml` or equivalent already exists — verify via `ls manifests/providers/` — reuse; otherwise create.)

This is NOT in scope: longer-than-4h tests (separate brief once 4h is stable); soak against real Anthropic/OpenAI/Gemini (cost concern; operator can opt-in by setting `ARGO_SOAK_PROVIDER=anthropic` locally); per-tenant load distribution (separate brief); kernel-level resource tracking (separate); soak against the actual `argo doctor` health output (separate observability brief).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `scripts/soak-skycot.sh` (NEW, ≤ 280 LOC, mode 755) — main soak runner
- `scripts/soak-skycot-tests.sh` (NEW, ≤ 120 LOC) — runs `ARGO_SOAK_FAST=1` in CI-friendly mode, asserts the script exit-codes are correct shape
- `.github/workflows/soak.yml.disabled` (NEW, ≤ 80 LOC)
- `docs/skycot/runbook/soak-test.md` (NEW, ≤ 240 LOC)
- `manifests/providers/stub-echo.toml` (NEW IF NOT EXISTS, ≤ 50 LOC) — fake provider for cost-bounded soak

## Files you must NOT touch

- Existing scripts (`smoke-skycot.sh`, `smoke-skycot-real-llm.sh`) — sibling stable
- `crates/argo-store/migrations/**`
- Vertical content / channels / frontend

## Acceptance criteria

- [ ] `bash scripts/soak-skycot.sh --help` prints usage + env-var documentation
- [ ] `ARGO_SOAK_FAST=1 bash scripts/soak-skycot.sh` runs in ~20min and emits valid JSON summary
- [ ] Without `ARGO_SOAK_FAST`, script runs ~4h (verify timing via `scripts/soak-skycot-tests.sh` w/ shortened `ARGO_SOAK_DURATION_OVERRIDE_MIN=2` for CI sanity-check)
- [ ] Summary JSON has the documented shape: `{ start, end, workers, prompts_sent, prompts_succeeded, memory_growth_pct, fh_growth_pct, audit_chain_intact, assertions }`
- [ ] All 5 assertions are checked (no-crash, memory ≤ 1.5x, FH ≤ 1.5x, audit-intact, gateway-200) and exit code is non-zero on ANY failure
- [ ] `manifests/providers/stub-echo.toml` (NEW or extended) is parsed by `argo provider list`
- [ ] `.github/workflows/soak.yml.disabled` is valid YAML; daily-4am-UTC cron; renames-to-`.yml` per CI-billing-restored playbook
- [ ] Runbook explicitly names which assertion fires for which class of bug
- [ ] `bash -n scripts/soak-skycot.sh` (syntax check) clean
- [ ] `shellcheck scripts/soak-skycot.sh` (if installed) emits no `error:`-level findings
- [ ] No Rust changes; no cargo clippy/test relevant
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-soak-test-nightly`
- PR title: `tools(soak): nightly 4-hour multi-worker soak runner + assertions (mission-soak-test-nightly)`

## Notes for the implementer

- The 4h duration is deliberate — memory leaks under steady load typically show within 2-3h; FH leaks similarly. Going shorter misses the gradual-creep cases.
- The 5 assertions are the LOAD-BEARING contract. Each maps to a specific failure class:
  - no-crash → panic / OOM / abort in a worker or gateway
  - memory ≤ 1.5x → leak via `Arc::new(...).clone()` never-dropped, or growing-cache without eviction
  - FH ≤ 1.5x → spawning subprocess that doesn't reap, or sqlite/socket fd never closed
  - audit-intact → chain-hash bug under concurrent appenders (most-likely failure mode)
  - gateway-200 → death-spiral via blocked tokio runtime / stuck mutex
- Use `ps -o pid,rss,nfds -p <pid>` for the per-process snapshots; `lsof -p <pid> | wc -l` is the FH count fallback if `nfds` isn't available on the host.
- The stub-echo provider should be a real provider manifest that the existing argo-provider crate parses — don't try to monkey-patch a mock at the runtime layer. Verify the manifest shape first via `ls manifests/providers/` + read one of the existing ones.
- `ARGO_SOAK_FAST=1` shortens to 20min for dev-loop validation. Document that fast-mode DOES exercise the 5 assertions but in a tighter window where memory creep is less detectable; explicitly state in the runbook that fast-mode is a smoke-of-the-soak, not a substitute.
- The CI-billing-locked-out `.disabled` extension follows the `mission-cargo-audit-disabled-workflow` (#1325) pattern. The workflow file ships, operators rename to `.yml` once billing restored, no CI cost until then.
- DO NOT add per-host-OS branches — the soak runs on Linux only. macOS/Windows local-dev runs are a follow-up brief (different ps/lsof semantics).
