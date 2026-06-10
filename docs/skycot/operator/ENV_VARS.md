# Argo / Skycot environment-variable reference

This is the operator-facing reference for `ARGO_*` environment variables
that the runtime reads but that are **not** otherwise set in `scripts/`,
the `Makefile`, `web/`, or a dedicated runbook. Each row names the reading
crate so a knob is discoverable from one place.

It is **not** an exhaustive list of every env var Argo reads. Variables that
are already wired in scripts, the Makefile, deployment runbooks, or
`provider-routing.md` (for example `ARGO_HOME`, `ARGO_PITCH_LLM_PROVIDER`,
`SKYCOT_DB_KEY_BASE64`, the Stripe / SES plumbing) are documented at their
existing call sites and are intentionally not duplicated here. This file
exists specifically to close the "read in code, written nowhere" gap found in
the 2026-06-10 architecture audit (P3-7).

> **Closing the gap going forward.** Before adding a new `std::env::var("ARGO_…")`
> read to non-test crate code, add a one-line row here (or wire the var into a
> script/runbook). That keeps the "no read-but-undocumented flag remains"
> invariant from acceptance check 2 true.

## Provider / LLM routing

| Variable | What it does | Default | Read at |
| --- | --- | --- | --- |
| `ARGO_MARKETING_LLM_PROVIDER` | Provider override for the marketing draft endpoint. Falls back to `ARGO_PITCH_LLM_PROVIDER`, then `anthropic`. | `anthropic` (via fallback chain) | `argo-cli` · `handlers/gateway/routes_marketing_draft_response.rs` |
| `ARGO_GTM_LLM_PROVIDER` | Provider override for the GTM (go-to-market) follow-up draft endpoint. Falls back to `ARGO_PITCH_LLM_PROVIDER`, then `anthropic`. | `anthropic` (via fallback chain) | `argo-cli` · `handlers/gateway/routes_gtm_draft_followup.rs` |
| `ARGO_PROVIDER_DEDUP_TTL_SECS` | When set to a positive integer, wraps the provider transport in a dedup cache with this time-to-live (in seconds) so identical in-flight provider calls are coalesced. Any non-positive / unset value disables the cache. | unset (no dedup cache) | `argo-agent-runner` · `transport.rs` |

## Pitch / facts

| Variable | What it does | Default | Read at |
| --- | --- | --- | --- |
| `ARGO_PITCH_FACTS_PATH` | Absolute path to the pitch-agent `facts.toml` company-facts file. Blank or unset uses the bundled pack. | `<workspace-root>/packs/pitch-agent/facts.toml` | `argo-cli` · `handlers/gateway/routes_pitch_ask_llm_facts.rs` |

## Channels (SMS / push)

| Variable | What it does | Default | Read at |
| --- | --- | --- | --- |
| `ARGO_TWILIO_SMS_DECISION_URL` | Public URL base used to recompute the Twilio request signature for the inbound SMS decision webhook (needed when the gateway sits behind a proxy and the externally-visible URL differs from the local one). Blank or unset derives the URL from the `X-Forwarded-Proto` / `X-Forwarded-Host` / `Host` headers. | derived from forwarded headers | `argo-cli` · `handlers/gateway/routes_channels_sms.rs` |
| `ARGO_PUSH_DEVICES_PATH` | Path to the JSONL registry of push-notification device tokens. | `<cwd>/.skycot/push_devices.jsonl` | `argo-cli` · `handlers/gateway/routes_channels_push.rs`; `argo-channels` · `push_format.rs` |

## Identity / compliance

| Variable | What it does | Default | Read at |
| --- | --- | --- | --- |
| `ARGO_PRINCIPAL` | Label recorded as the acting local principal in compliance / ROPA (Record of Processing Activities) output. Falls back to `USER`, then `USERNAME`. | `operator` (via fallback chain) | `argo-cli` · `handlers/compliance_ropa.rs` |
| `ARGO_SAML_CLOCK_SKEW_SECS` | Allowed clock-skew tolerance, in seconds, when validating SAML assertion timestamps at the ACS (Assertion Consumer Service) endpoint. Negative values are ignored. | `60` | `argo-cli` · `handlers/gateway/sso_saml_acs.rs` |

## Operations (maintenance / eval / doctor)

| Variable | What it does | Default | Read at |
| --- | --- | --- | --- |
| `ARGO_MAINTENANCE_MODE` | Set to `1` to put the gateway in planned-downtime mode: all routes return `503 Service Unavailable` except liveness paths (`/health[/...]`), which still answer so external probes don't trip. | unset (normal serving) | `argo-cli` · `handlers/gateway/maintenance_middleware.rs` |
| `ARGO_MAINTENANCE_ETA` | Optional human-readable ETA string echoed in the `503` JSON body so the SPA can render an operator ETA. Only consulted when `ARGO_MAINTENANCE_MODE=1`. | none | `argo-cli` · `handlers/gateway/maintenance_middleware.rs` |
| `ARGO_EVAL_RUNS_DIR` | Directory where `argo eval` run artifacts are read from / listed. Falls back to `$ARGO_HOME/eval-runs`, then `<cwd>/.argo/eval-runs`. | `<cwd>/.argo/eval-runs` (via fallback chain) | `argo-cli` · `handlers/eval_list.rs` |
| `ARGO_DOCTOR_SKIP_NETWORK` | Set to `1` / `true` / `yes` to make `argo doctor` skip DNS / TLS / auth network probes (useful offline or in CI). Equivalent to the `--skip-network` flag. | unset (probes run) | `argo-cli` · `handlers/doctor_network.rs` |

## Skill judging

| Variable | What it does | Default | Read at |
| --- | --- | --- | --- |
| `ARGO_JUDGE_SKILL_TIMEOUT_MS` | Timeout, in milliseconds, for a single skill-adjudication LLM judge call before it is treated as unavailable. | `30000` (30s) | `argo-skill` · `store_vertical_judge.rs` |
| `ARGO_JUDGE_SKILL_FLOOR` | Confidence floor below which the judge vetoes a skill candidate. `< 0.5` enables lenient mode (a `Partial` verdict passes); otherwise `Partial` fails. | `0.6` | `argo-skill` · `store_vertical_judge.rs` |

## Storage connection pool

These tune the sqlx (SQLite) connection pool at `SessionStore::open`. With none
set, behaviour is byte-equivalent to the historical defaults. Invalid values
warn (`eprintln!`) and fall back to the default — they never panic. Recommended
swarm-scale tuning: `ARGO_STORE_POOL_MAX=16` and `ARGO_STORE_POOL_IDLE_TIMEOUT_S=60`.

| Variable | What it does | Default | Read at |
| --- | --- | --- | --- |
| `ARGO_STORE_POOL_MAX` | Hard ceiling on simultaneously-checked-out connections. | `8` | `argo-store` · `pool_config.rs` |
| `ARGO_STORE_POOL_MIN` | Lower bound — sqlx keeps at least this many connections live. | `0` | `argo-store` · `pool_config.rs` |
| `ARGO_STORE_POOL_ACQUIRE_TIMEOUT_MS` | How long a `pool.acquire()` waits, in milliseconds, before erroring out. | `30000` (30s) | `argo-store` · `pool_config.rs` |
| `ARGO_STORE_POOL_IDLE_TIMEOUT_S` | Idle-reap window in seconds. Unset keeps connections forever. | unset | `argo-store` · `pool_config.rs` |

## CLI / local tooling

| Variable | What it does | Default | Read at |
| --- | --- | --- | --- |
| `ARGO_EDITOR` | First entry in the editor-resolution chain for `argo persona edit` (`$ARGO_EDITOR` > `$VISUAL` > `$EDITOR`). | falls through to `$VISUAL` / `$EDITOR` | `argo-cli` · `handlers/persona_edit.rs` |

## Test-only knobs

These are read only from `#[cfg(test)]` code and have no effect on a running
binary. They are listed so the discoverability checker treats them as
accounted-for rather than dead.

| Variable | What it does | Read at |
| --- | --- | --- |
| `ARGO_CALIBRATION_STRICT` | Set to `1` to make the vertical-eval judge-calibration test fail (rather than skip) on a ranking inversion. Requires `ANTHROPIC_API_KEY`. | `argo-cli` · `tests/vertical_eval_judge_calibration.rs` |
| `ARGO_XDOTOOL_LOG` | Log-file path injected into the fake `xdotool` wrapper used by the Linux computer-use input tests, so the test can assert on the emitted command line. | `argo-executor-computer-use-linux` · `src/x11/input.rs` (test module) |
