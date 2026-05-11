# Mission load-bearing wire — bench perf-regression detection (criterion delta + CI fail)

**Owner**: claude. **Depends on**: just-shipped `mission-criterion-load-tests` (#1365, MERGED 2026-05-12) — 5 criterion benches across `argo-runtime`, `argo-audit`, `argo-cli`, `argo-skill`, `argo-store`; `.github/workflows/bench.yml.disabled`; `scripts/run-bench.sh`; runbook `docs/skycot/runbook/perf-benches.md`.
**Authored against**: origin/main `d1b6501b` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-bench-perf-regression-detection`.

> SOURCE: MISSION_BACKLOG.md Section 6 Self-improvement infra #205 ("Build benchmark suite (perf regression detection over skill versions)"). **Tier M (perf-regression-detection).** The criterion benches just landed (#1365) — they MEASURE p50/p95/p99 per bench. What's MISSING: a way to detect REGRESSIONS between runs. Today an operator runs `bash scripts/run-bench.sh` and reads numbers; if last week's run was 12% faster, no signal fires. This brief lands: (1) baseline-recording, (2) delta-comparison, (3) regression-detection w/ tunable threshold (default 5% — matching the runbook's "noise floor" guidance), (4) optional CI fail when CI runs the bench workflow.

## Scope

### NEW `scripts/bench-record.sh` (≤ 100 LOC, mode 755)

Runs `bash scripts/run-bench.sh --quiet`, parses `target/criterion/*/new/estimates.json` for every bench, and writes a single `target/bench-baseline-<git-sha>.json` w/ shape:

```json
{
  "git_sha": "...",
  "recorded_at": "2026-05-12T03:00:00Z",
  "host_fingerprint": "darwin-arm64-host-id-...",
  "results": {
    "agent_turn::p99":     { "ns": 1234567, "stderr_ns": 12345 },
    "audit_write::seq":    { "ns":  234567, "stderr_ns":  2345 },
    "audit_write::8par":   { "ns": 1234567, "stderr_ns": 12345 },
    "...": ...
  }
}
```

The host_fingerprint guards against comparing M1-recorded baseline to x86_64-recorded baseline (warns + refuses; operator passes `--allow-cross-host` to override).

### NEW `scripts/bench-compare.sh` (≤ 150 LOC, mode 755)

Compares the current run against a baseline:

```
bash scripts/bench-compare.sh \
  --baseline target/bench-baseline-<sha>.json \
  --threshold-pct 5 \
  [--allow-cross-host] \
  [--json]
```

For each bench:
- Computes `delta_pct = (current_ns - baseline_ns) / baseline_ns * 100`.
- Classifies: `OK` (|delta| ≤ threshold), `REGRESS` (delta > threshold), `IMPROVE` (delta < -threshold).
- Aggregates per-bench results.
- Exits non-zero (1) iff any bench was REGRESS classified.

Output shape (human):
```
agent_turn::p99   1.23ms → 1.28ms  +4.1%  OK
audit_write::seq  234µs  → 251µs   +7.3%  REGRESS  ⚠️
...
2 regressions, 1 improvement, 3 ok.
Exit: 1
```

JSON output: structured envelope with `results: [{name, baseline_ns, current_ns, delta_pct, classification}]` + `summary: {regress, improve, ok, threshold_pct}`.

### Workflow integration

EXTEND `.github/workflows/bench.yml.disabled` (the just-shipped weekly workflow) to:

- After `cargo bench`, run `bash scripts/bench-compare.sh --baseline scripts/bench-baselines/main.json --threshold-pct 5 --json` against the committed baseline at `scripts/bench-baselines/main.json`.
- On REGRESS exit-code, the workflow step fails — operator sees the regression in the GitHub UI.
- On IMPROVE, the workflow step succeeds but PRINTS the improvements (operator may want to refresh the baseline).

### Baseline refresh procedure (operator)

NEW `scripts/bench-baseline-refresh.sh` (≤ 60 LOC, mode 755) — wraps `bench-record.sh` + copies output to `scripts/bench-baselines/main.json`. Operator runs after a deliberate perf improvement lands, commits the new baseline alongside the perf-changing PR. Document in the runbook.

### Runbook extension

EXTEND `docs/skycot/runbook/perf-benches.md` (the just-shipped runbook) w/ a new section "Regression detection" (≤ 80 LOC delta):

- How to record baselines locally
- How the comparison classifies results
- "5% noise floor" rationale (matches the existing runbook section on per-run variability)
- How to refresh baselines + when (every deliberate perf-touching merge)
- How to interpret a CI regression failure: read the JSON, identify which bench, run locally w/ `--quiet` to confirm, propose fix or refresh baseline w/ justification
- Cross-host caveat (M1 vs x86; the host_fingerprint guard prevents accidental cross-host comparison)
- Per-bench thresholds: future-extension (different benches may warrant different thresholds — gateway_request more variable than agent_turn) — v1 uses one threshold globally; per-bench override is a v2 follow-up

### Tests

NEW `scripts/bench-compare-tests.sh` (≤ 100 LOC, mode 755) — meta-test:
- Generates 2 synthetic baseline/current JSONs
- Asserts classification correctness (regress at +6%, ok at +4%, improve at -6%)
- Asserts threshold-tuning works
- Asserts cross-host guard fires correctly
- Asserts JSON output schema valid

NO Rust changes. NO migration. NO crate touches. This is a script + workflow + runbook brief.

This is NOT in scope: per-bench / per-percentile threshold tuning (v1 = one threshold globally; v2 if needed); historical-trend visualization (separate operator-tooling brief); cost-per-bench attribution (the `mission-forge-cost-attribution` brief covers Forge-run cost; bench-cost is a follow-up); auto-baseline-refresh on improvement (intentionally manual — operator commit-locks the baseline); CI matrix runs across multiple hosts (the workflow is single-host in v1; multi-host matrix is a v2 to validate cross-arch perf).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `scripts/bench-record.sh` (NEW, ≤ 100 LOC, mode 755)
- `scripts/bench-compare.sh` (NEW, ≤ 150 LOC, mode 755)
- `scripts/bench-baseline-refresh.sh` (NEW, ≤ 60 LOC, mode 755)
- `scripts/bench-compare-tests.sh` (NEW, ≤ 100 LOC, mode 755)
- `scripts/bench-baselines/main.json` (NEW, ≤ 60 LOC) — committed baseline; placeholder values acceptable for v1 (operator refreshes after first real run)
- `scripts/bench-baselines/README.md` (NEW, ≤ 40 LOC) — explain what these files are + how to refresh
- `.github/workflows/bench.yml.disabled` (EXTEND, ≤ 40 LOC delta) — add comparison step
- `docs/skycot/runbook/perf-benches.md` (EXTEND, ≤ 80 LOC delta) — new "Regression detection" section

## Files you must NOT touch

- Any `crates/**` Rust source (no Rust changes in this brief)
- Any `benches/*.rs` file (benches stay as shipped via #1365)
- `Cargo.toml` / `Cargo.lock`

## Acceptance criteria

- [ ] `bash scripts/bench-record.sh --help` prints usage
- [ ] `bash scripts/bench-compare.sh --help` prints usage including all flags
- [ ] `bash scripts/bench-compare-tests.sh` exits 0 on a fresh checkout (synthesizes its own test JSONs)
- [ ] Classification correctness verified across REGRESS (+6%), OK (+4%), IMPROVE (-6%) cases
- [ ] Default threshold = 5%; tunable via `--threshold-pct`
- [ ] Cross-host guard refuses comparison + suggests `--allow-cross-host`
- [ ] JSON output validates against documented schema
- [ ] Workflow `.disabled` file is syntactically valid YAML (operator-renamed-to-.yml post-billing-restore)
- [ ] Runbook covers all 7 sections (record / compare / threshold / refresh / cross-host / per-bench-future / CI-failure-interpretation)
- [ ] `shellcheck scripts/bench-*.sh` (if installed) emits no `error:`-level findings
- [ ] `bash -n scripts/bench-*.sh` (syntax check) clean
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-bench-perf-regression-detection`
- PR title: `tools(bench): perf-regression detection — baseline record/compare + workflow integration + runbook (mission-bench-perf-regression-detection)`

## Notes for the implementer

- The `target/criterion/*/new/estimates.json` schema is criterion's own — it has `mean.point_estimate` and `mean.standard_error` fields. Use those. Document the parse expectation in `bench-record.sh`.
- `host_fingerprint` should be cheap: `uname -sm` + a hash of `sysctl -n hw.model` (macOS) / `lscpu | head -5` (linux). Don't fingerprint by hardware-serial — operators copy baselines between identical machines.
- The 5% threshold matches the criterion-suite runbook's "noise floor" guidance. Tunable per CI run via `--threshold-pct 10` if a bench is known-noisy.
- The classification THREE-state (REGRESS / OK / IMPROVE) is deliberate — operators want to see improvements too; they may want to refresh the baseline to lock in the gain. v1 doesn't auto-refresh; the manual baseline-refresh keeps the operator in the loop.
- `--allow-cross-host` is the safety override for cross-arch comparison. CI runs same-host always (single-machine matrix in `.disabled` workflow); manual operator-side compare may legitimately span hosts during a migration.
- The shipped baseline at `scripts/bench-baselines/main.json` may be placeholder for v1 — operators refresh once they have a first real CI run. Document this in the README so the placeholder doesn't get treated as authoritative.
- DO NOT implement a Rust-side bench-comparison tool. Shell + jq is sufficient for v1 — the data is plain JSON.
- DO NOT auto-update the baseline on green CI runs. Manual baseline-refresh + commit is the invariant — drift detection requires the baseline be stable across merges.
- The workflow uses `jq` heavily; document `jq` as a CI-runner dependency in the workflow file's header comment.
- If `target/criterion/` doesn't exist (operator hasn't run `cargo bench` yet), `bench-record.sh` emits a clear "Run cargo bench first" message + exits non-zero. Don't silently produce an empty baseline.
- The runbook's "Per-bench thresholds: future-extension" section flags the v2 evolution: gateway_request is more variable than memory_store, so per-bench thresholds (gateway 10% / memory 5%) would reduce false-positives. Note as a follow-up; do NOT build in v1.
