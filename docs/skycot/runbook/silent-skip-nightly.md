# Silent-skip nightly — operator runbook

The silent-skip nightly (`scripts/silent-skip-nightly.sh` driven by
`.github/workflows/silent-skip-nightly.yml.disabled`) runs the
workspace's silent-skip integration tests against real provider test
accounts once per day at 05:00 UTC. It exists because most provider-
backed integration tests today silently `return` early when the
relevant `*_API_KEY` env var is absent — so `cargo test` in CI passes
without ever exercising a live provider, and breakages on the real
API surface are invisible until a customer hits them.

This runbook covers: the pattern, adding new silent-skip tests,
configuring repo secrets, interpreting nightly failures, and the
local-debug flow.

## What the silent-skip pattern is

A "silent-skip" test is one shaped like this:

```rust
#[tokio::test]
async fn smoke_hubspot_create_contact() {
    if std::env::var("HUBSPOT_ACCESS_TOKEN").is_err() {
        eprintln!("HUBSPOT_ACCESS_TOKEN not set; skipping");
        return;
    }
    // … the real test body that hits api.hubapi.com …
}
```

Without the env var the test prints a message and exits success.
With it, the body runs against the live API. This keeps `cargo test`
green for contributors without provider accounts, while letting an
operator with credentials opt in by exporting the env var locally.

The cost is observability: in CI we have no signal that the real-
provider path even compiles, much less works. The nightly job closes
that gap by injecting test-account secrets from GitHub repo secrets
and running the full discovered set.

Future: rewrite silent-skip → VCR/cassette fixtures (separate brief
#12); this nightly is the interim coverage gate.

## Discovery heuristic

The runner discovers test targets via file naming:

```
crates/<crate>/tests/*.rs
   where filename matches *_smoke* | smoke_* | *_integration* |
   integration_* | *_live* | live_*
```

Each matching file is one cargo `--test` target; the runner invokes
`cargo test -p <crate> --test <target>` per file so that one hung
test cannot block the rest. The discovery does NOT inspect function
bodies — silent-skip tests that don't follow the file-naming
convention are missed. Workarounds:

- Add the suffix when authoring a new silent-skip test target.
- If a target legitimately needs a different name, list it in this
  runbook's "Known discovery gaps" section below and add a follow-up
  brief to bring it into convention.

`bash scripts/silent-skip-nightly.sh --discover-only` prints the
discovered list and warns if the count drops below 40 (the brief's
sanity-check lower bound). Today's count is ~170; healthy range is
40–250.

## Adding a new silent-skip test

1. Place the test in `crates/<crate>/tests/<name>_smoke.rs` (or
   `_integration.rs` / `_live.rs`). The suffix is what makes
   discovery find it.
2. Use the env-var contract:
   - Primary credential: `ARGO_TEST_<PROVIDER>_KEY`. (Or the
     pre-existing convention, e.g. `ANTHROPIC_API_KEY`.)
   - Secondary keys: `ARGO_TEST_<PROVIDER>_<KIND>` (portal id,
     domain, region, …).
3. Begin the test body with:
   ```rust
   if std::env::var("ARGO_TEST_FOO_KEY").is_err() {
       eprintln!("ARGO_TEST_FOO_KEY not set; skipping");
       return;
   }
   ```
4. Run `bash scripts/silent-skip-nightly.sh --discover-only` and
   confirm the new target appears.
5. If the env-var name is new, add the corresponding `env:` line to
   `.github/workflows/silent-skip-nightly.yml.disabled` and create
   the matching repo secret (next section).

## Adding a new repo secret

GitHub repo secrets are the canonical source for test-account
credentials. To add one:

1. Provision the test account with the provider. Note the credential
   value(s). For OAuth-based providers, generate a long-lived token
   scoped to the test account only.
2. In the GitHub UI: **Settings → Secrets and variables → Actions →
   New repository secret**. Name follows the
   `ARGO_TEST_<PROVIDER>_KEY` (or `_<KIND>`) convention.
3. Add the matching `env:` line under the `Run silent-skip` step of
   `.github/workflows/silent-skip-nightly.yml.disabled`. Map repo-
   secret → test-side env-var name (often the existing
   `<PROVIDER>_API_KEY` convention used by crate-side
   `env::var(...)` calls):
   ```yaml
   ANTHROPIC_API_KEY: ${{ secrets.ARGO_TEST_ANTHROPIC_KEY }}
   ```
4. Commit + push. The next scheduled run picks up the change. To
   test immediately, trigger via the **Actions → Silent-skip nightly
   → Run workflow** button (workflow_dispatch).

Secrets visible to PR runs are scoped to the workflow file —
forks/external PRs do not have access. The nightly schedule runs in
the upstream repo context only.

## Interpreting failures

A nightly run that produces `failed > 0` or `timeout > 0` opens an
issue automatically (tagged `silent-skip-failure` + `nightly`). The
issue body lists up to 20 failures with crate, test target, exit
code, and `outcome` (`failed` vs `timeout`).

Failures fall into three classification modes:

### 1. Real API breakage

Provider shipped a breaking change (renamed field, removed endpoint,
new auth requirement). Indicators:

- Multiple tests across the same provider fail simultaneously.
- Failure tails reference `404 Not Found`, `400 Bad Request`,
  `Unauthorized`, or schema-deserialisation panics.
- Recent provider changelog/blog confirms a deprecation.

Response: open a fix-brief targeting the affected crate(s). The
nightly continues to surface the failure until the fix lands.

### 2. Test-account quota

The test account hit a rate limit, monthly quota, or balance-zero
state. Indicators:

- A subset of tests for one provider fail; others on the same
  provider pass.
- Failure tails reference `429 Too Many Requests`, `quota
  exceeded`, `insufficient balance`.
- Failure pattern aligns with monthly-cycle boundaries.

Response: top up the test account (or wait for quota reset). No
code change needed. If quota-burn is recurring, file an operator-
side budget brief.

### 3. Network / flake

Transient infrastructure issue (DNS hiccup, TLS handshake jitter,
provider 5xx). Indicators:

- A small number of tests fail; re-running the same workflow
  passes.
- Failure tails reference `connection reset`, `timeout`, `5xx`.
- Same target's recent runs (visible in the workflow history)
  alternate pass/fail.

Response: tag the failing test in
`docs/build-plan/KNOWN_FLAKES.md` under `[Silent-Skip Nightly]`. If
flake frequency exceeds ~20%, file a brief to add retry+backoff to
that specific test (do NOT add blanket retries to the runner —
those mask real breakages).

## Running locally

`bash scripts/silent-skip-nightly.sh --local` runs discovery + per-
target invocation against the operator's local env. Without GH
secrets set, most tests silently skip — but the run completes, the
report is emitted, and the runner shape is verified. Useful for:

- Validating a new silent-skip test before merge.
- Debugging a discovery gap (does my new file show up?).
- Sanity-checking the runner after editing the script.

Knobs:

- `ARGO_SILENT_SKIP_TIMEOUT_SECS=600` — per-target timeout (default
  300). Bump for slow-network providers.
- `ARGO_SILENT_SKIP_REPORT_DIR=/tmp/foo` — alternate output dir.

The 5-min per-target timeout requires `timeout` (GNU coreutils) on
Linux or `gtimeout` on macOS (`brew install coreutils`). Without
either, the runner warns and proceeds without timeout enforcement —
acceptable for local debug, NOT for CI.

## Report shape

`target/silent-skip-report-<ts>.json`:

```json
{
  "started":      "2026-05-12T05:00:00Z",
  "completed_at": "2026-05-12T05:42:17Z",
  "mode":         "nightly",
  "total":        174,
  "completed":    174,
  "passed":       170,
  "failed":       3,
  "timeout":      1,
  "skipped":      0,
  "failures": [
    {
      "test": "live_hubspot",
      "crate": "argo-mcp-connector-hubspot",
      "exit_code": 101,
      "outcome": "failed",
      "stderr_tail": "thread 'tokio-runtime-worker' panicked at …"
    }
  ]
}
```

Caveat: `skipped` is always `0` today. Cargo exits 0 for silently-
skipped tests, so the runner can't distinguish "ran and passed" from
"ran and silently skipped" without test-internal cooperation. The
VCR-cassette migration (brief #12) is the planned fix; an interim
option is a `#[ignore]`-attribute convention plus
`cargo test -- --include-ignored`, which would let the runner count
skips explicitly.

## Workflow lifecycle

Ships with `.disabled` suffix while CI billing is frozen. Activate
by renaming `silent-skip-nightly.yml.disabled` →
`silent-skip-nightly.yml` and committing — same pattern as
`audit.yml.disabled` and `soak.yml.disabled`. Disable by renaming
back; only the `.yml` form is picked up by Actions.
