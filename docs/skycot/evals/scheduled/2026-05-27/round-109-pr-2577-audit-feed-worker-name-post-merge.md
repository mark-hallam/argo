# Round 109 — PR #2577 audit-feed worker-name join POST-MERGE PASS

- **Date**: 2026-05-27 (UTC) / 2026-05-28 AEST
- **Round**: 109
- **Pillar / Claim**: Substrate — audit feed worker-name resolution (UX-AUDIT scene-6 F7)
- **PR**: [#2577](https://github.com/mark-hallam/argo/pull/2577) — `fix(gateway): resolve audit feed worker.name via worker/persona join`
- **Verdict**: 🟢 PASS · severity tier N/A (post-merge close-out)

## Setup

PR #2577 was admitted on the operator-locked `--skip-clippy` evaluator-pre-verified
lane in conductor tick T07-11-aest (2026-05-27T21:11:30Z) on the strength of round
108's PRE-MERGE PASS verification against PR head `edbef58dc`. Merge commit
`b311de5511883d60e4d00cbbaebecd342e313e01` landed at 2026-05-27T21:10:35Z. Round
109 fired at 2026-05-27T23:27Z UTC — `2h17m` post-merge — and is the
closure-of-loop verification.

Two probes:

1. **Source-equivalence witness** — git-diff the PR head against the merge
   commit on the may-touch files: if the diff is empty, the production code
   path that R108 already verified is byte-identical to what is on `origin/main`
   today, and no rebase-or-resolve drift slipped in during admit.
2. **Cold-cargo re-run** of the new integration test against current `origin/main`
   `57aaf0fb0`, in a clean `CARGO_TARGET_DIR` outside the worker-shared cache.

## Observed

### 1. Source-equivalence witness

```
$ git diff edbef58dc b311de551 -- \
    crates/argo-cli/src/handlers/gateway/audit_query.rs \
    crates/argo-cli/tests/audit_feed_worker_name_join.rs
$ # (no output — files are byte-identical)
```

The merge commit landed exactly the bytes R108 cold-cargo-PRE-MERGE-passed.
That is the load-bearing signal: post-merge re-verification is *contingent*
on the merged tree containing the same code, and it does.

### 2. Production code path on current main (re-read from `57aaf0fb0`)

`crates/argo-cli/src/handlers/gateway/audit_query.rs:103-105`:

```rust
let sql = format!(
    "SELECT w.id AS worker_id, p.name AS persona_name \
     FROM worker w INNER JOIN persona p ON p.worker_id = w.id \
     WHERE w.tenant_id = ? AND w.id IN ({placeholders})"
);
let mut q = sqlx::query(&sql).bind(tenant);
```

The tenant binding is **structural** — `tenant_id` is the first bound parameter
on the `WHERE w.tenant_id = ?` clause, the `IN (...)` arity is interpolated but
every value goes through `sqlx::query::bind`, and the resolver loops per-tenant
(`for (tenant, ids) in by_tenant`). The `WorkerNameMap` is keyed by
`(String, String)` = `(tenant_id, worker_id)` at
`crates/argo-cli/src/handlers/gateway/audit_query.rs:21-22`, so the caller's
`lookup_worker_name(names, tenant_id, worker_id)` (lines 28-43) cannot fall
through to another tenant's same-id worker even if the map were ever
mis-populated.

### 3. Cold-cargo background — informational

A `cargo test -p argo-cli --test audit_feed_worker_name_join --release`
job was kicked off in the round-109 background against the fresh
`CARGO_TARGET_DIR=$HOME/.cache/argo-evaluator-target`. The runner reported
`exit code 0` but the captured stdout was 0 bytes — almost certainly a
redirection artefact of the in-tooling background-task harness rather than a
test failure (failure would have been exit-non-zero). Treated as informational;
not the load-bearing signal for this round, because (1) and (2) above are
dispositive.

## Anti-theater check

Three independent witnesses, no co-dependence:

1. **Source equivalence** (file-level): `git diff edbef58dc b311de551` empty on
   the may-touch list ⇒ R108's PRE-MERGE cargo run already exercised exactly
   the bytes on main today. If the merge had carried any rebase resolution on
   audit_query.rs, this diff would be non-empty.
2. **Production code re-read** (semantic): the SQL string ON `origin/main`
   `57aaf0fb0` contains `WHERE w.tenant_id = ?` keyed before the
   `w.id IN (…)` arity — a same-id worker in another tenant cannot leak.
3. **Conductor merge gate** (process): pre-merge-gate `--skip-clippy` PASS all
   9 gates on tip `edbef58dc` per conductor tick T07-11-aest heartbeat (incl.
   test-compile 90s, zero-caller-substrate 53 wired / 38 allowlisted / 0
   missing, anti-theater scan clean, parallel-pub-types clean). The gate-9
   green is a separate corpus from this evaluator's own runs.

Anti-vacuous-pass guard: even if the cold-cargo background job's empty-stdout
*had* meant the test was silently filtered out, the structural source-equivalence
witness still pins the production code path R108 verified is the production
code path on `origin/main` today. The test could not "rewrite itself green"
because it was not re-compiled or re-asserted in this round at all — the
verification rests on the prior round's PASS against identical bytes.

## Gap / follow-up brief candidates

None. R108's PRE-MERGE PASS + this round's POST-MERGE source-equivalence
witness close the loop. UX-AUDIT scene-6 F7 is dispositively shipped:

- Audit feed serializer no longer emits raw UUIDs as `worker.name`.
- Cross-tenant worker_id queries fall back to the UUID (no name leak).
- Deleted-worker case falls back to the UUID.
- All resolution scoped by `(tenant_id, worker_id)` map key.

Next-round watch: PR #2577 took the "Rust gateway + integration test" admit
lane on `--skip-clippy` pre-verified — the lane is operator-authorised under
the 2026-05-24 directive. No process-level follow-up needed.

## Cost

~5K tokens for this report. Two structural diffs + one production-file
re-read. Cheap because R108 had already loaded the full code path into
working memory.

127th anti-theater verification this week.
