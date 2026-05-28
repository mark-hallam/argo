# Gateway freshness — the running binary vs `origin/main`

The argo gateway is a long-lived dev process. Once `argo gateway start`
launches it, the binary stays in memory until the operator explicitly
restarts it — even after the source code on `origin/main` moves on. The
running process can therefore be silently stale: serving pre-merge
route handlers while the rest of the swarm thinks the merged PR is
live.

This document defines the freshness contract, the two drift signals
the platform surfaces, and the operator's responsibility.

## The two drift signals

### Schema-version drift (canonical)

Migrations live under `crates/argo-store/src/migrations/v*.rs` and bump
a monotonic `schema_version` integer. The running gateway reports its
applied `schema_version` on `GET /health`. The freshness script
compares that against the highest `v*.rs` on disk. Drift > 2 → STALE.

Schema-version is the safer of the two signals because the rest of the
stack will hard-fail on a mismatch (a v76 binary cannot talk to a v77
database without migration). When schema is fresh you cannot be
running a binary that is dangerously out of date for storage purposes.

### Build-commit drift (the foot-gun)

Schema-fresh does not mean route-handler-fresh. Most PRs change route
code without touching migrations. The audit on `2026-05-26` caught
exactly this: the live gateway was 5.5 hours behind PR #2400 (Genesis
wire) and PR #9ea693db1 (`/api/v1/workers` collection alias), but
`schema_version` was unchanged the whole time. The demo recording
would have hit the pre-merge code paths without any visible warning.

`crates/argo-cli/build.rs` embeds the 7-character git SHA into the
binary at compile time as `crate::build_info::GIT_SHA`. The gateway
includes it in the `/health` response body as `build_commit`. The
freshness script compares it to `origin/main` and prints both signals:

```
gateway_schema=v83 codebase_latest=v83 drift=0
gateway_commit=abc1234 commits_behind=12 gateway_code_changes=3
STATUS: STALE-COMMITS  (gateway abc1234 is 12 commits behind origin/main def5678; 3 touch crates/argo-cli/src/handlers/gateway/; restart: `make restart-live-gateway`)
```

Escalation rule (exit code 1):

- `commits_behind > 50`, OR
- any of the missed commits touched
  `crates/argo-cli/src/handlers/gateway/` (one file is enough).

Below those thresholds the script stays quiet (exit 0). Most commits
don't change gateway code; the operator shouldn't be nagged for every
unrelated PR.

## The one-command check

```
make gateway-warn-if-stale
```

Reads the running gateway on `http://127.0.0.1:7400/health` by
default. Pass `PORT=NNNN` for a non-default gateway. Pass
`COMMIT_THRESHOLD=N` to override the 50-commit escalation cap (lower
in CI, higher in long-lived dev envs).

Exit codes are a stable contract for the conductor tick and CI:

- `0` — FRESH. Schema drift ≤ 2 AND no escalating commit drift.
- `1` — STALE. Restart with `make restart-live-gateway`.
- `2` — UNREACHABLE. `/health` did not respond. Likely the gateway is
  intentionally down; distinct exit code so the conductor doesn't
  keep nagging.
- `3` — PARSE_ERROR. `/health` returned an unexpected shape.
  Investigate, don't restart.

## Operator responsibility

Schema-version drift is automated (CI rejects skipped migration slots,
the restart script refuses to run a binary whose schema differs from
disk). Commit drift is on the operator: there is no auto-restart, and
hot-reloading a Rust binary isn't possible without losing the cassette
in-memory state.

Practical habits:

1. Run `make gateway-warn-if-stale` at the top of every dev session.
   Five-second probe.
2. After merging any PR that touched `crates/argo-cli/src/handlers/`,
   run `make restart-live-gateway`.
3. Before recording the demo MP4 (operator-only artifact), always
   restart: `bash scripts/restart-live-gateway.sh`. The check ran
   green the moment before the merge train fires is not the same as
   the check running green now.

## Why no auto-restart

Long-running gateway work (recording a demo, hosting a customer
session, replaying a cassette) would be silently nuked by a background
restart. The freshness signal is the platform's job; deciding *when*
to act on it is the operator's. This is a deliberate seam, not a gap.

## Related

- `scripts/check-gateway-freshness.sh` — the implementation.
- `scripts/check-gateway-freshness-tests.sh` — 9-scenario smoke suite
  (5 schema scenarios + 4 commit scenarios).
- `scripts/restart-live-gateway.sh` — the safe-restart script the
  STALE hint points at.
- `crates/argo-cli/build.rs` — embeds `GIT_SHA` at compile time.
- `docs/skycot/operator/gateway-restart-policy.md` — restart-script
  policy + first-run bootstrap.
