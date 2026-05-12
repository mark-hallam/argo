# SKYCOT Operator Runbook — Incident Response

Step-by-step playbook for the operator in the middle of a live
incident: declare → triage → contain → diagnose → mitigate →
communicate → post-mortem. The static failure-mode catalogue lives
in [failure_modes.md](../failure_modes.md); this runbook tells you
what to *do* when one fires.

Siblings: [observability](./observability.md), [security-audit](./security-audit.md), [production-deploy](./production-deploy.md), [workers](./workers.md), [actions](./actions.md), [conductor-troubleshooting](./conductor-troubleshooting.md).

---

## SEV taxonomy

Industry-standard four-tier ladder (Google SRE / PagerDuty). Pick
from observed impact, not from the suspected defect.

| SEV | Impact                                                          | Page             | Update cadence |
|-----|-----------------------------------------------------------------|------------------|----------------|
| 1   | data corruption, customer-data exposure, total outage           | page on-call now | every 30 min   |
| 2   | >30% error rate or latency >10× p50, partial outage             | business hours   | every 2 h      |
| 3   | single tenant or single feature degraded                        | log + queue      | daily          |
| 4   | minor / cosmetic                                                | routine queue    | as fixed       |

When in doubt take the higher tier — downgrading is cheap.

---

## 1. Declaring an incident

Any operator may declare. Declaration is no-cost; false alarms that
downgrade to SEV-4 within 10 min are fine.

1. Post in the operations Slack channel using the template under
   [Communicate](#6-communicate). Include UTC wall-clock time.
2. The Slack declaration routes through `/incidents/declare` and
   writes an `incident_declared` audit row automatically.
3. **Gateway-down fallback.** If `/incidents/declare` is itself
   the outage:

   ```bash
   echo "$(date -u +%FT%TZ) SEV-1 declared: <one-line>" \
     >> docs/skycot/incidents/_live-incident.log
   ```

   The conductor reconciles this into the audit chain once the
   gateway is back up.
4. SEV-1 only: update the public status page within 15 min of
   declaration ([template](#6-communicate)).

**Decision tree.** Data lost or exposed → SEV-1. All customers
affected → SEV-1. >30% error rate → SEV-2. One tenant or feature
→ SEV-3. Otherwise SEV-4.

---

## 2. Triage (first 5 min)

### 2a. `argo doctor`

```bash
argo doctor --json | jq '.summary'
```

A red `status: critical` row tells you the *what*; detail rows
identify *which subsystem* (manifest, queue, provider, audit-chain
continuity). Row catalogue:
[observability runbook §9.2](./observability.md).

### 2b. Gateway / endpoint health

```bash
argo gateway routes-print --json | jq '.[] | select(.status != "ok")'
curl -fsS http://localhost:7777/livez
curl -fsS http://localhost:7777/readyz
```

`/livez` 200 + `/readyz` 503 = binary up but a dependency (DB,
provider, MCP) is failing.

### 2c. Worker crash-loop signal

```bash
argo worker restarts --since 1h --json
```

`restarts_per_worker[*] > 5` in the last hour = crash-loop. Match
the failure mode in
[failure_modes.md §Worker-crash-mid-step](../failure_modes.md) before
containing.

---

## 3. Contain (next 10 min)

Four containment primitives in increasing severity.

### 3a. Pause one tenant

```bash
argo tenant quota --tenant <id> --pause
```

Workers complete the in-flight step and idle. Resume with
`--unpause`. Cross-ref: [workers runbook](./workers.md).

### 3b. Disable one connector

```bash
argo connector disable <name>
```

Connector tools return a clean "disabled" error so calling skills
can fall back. Re-enable with `argo connector enable <name>`.

### 3c. Fail-open vs fail-closed

Default is fail-closed. Fail-open is only correct when (a) the
failing service is non-load-bearing AND (b) skipping cannot cause
incorrect or unsafe actions. Trust-graph and approval gates are
NEVER fail-open candidates. Override is in the manifest, not a
runtime flag — there is no runtime fail-open env var.

### 3d. Restart the whole binary

```bash
argo gateway stop --grace 30s && argo gateway start   # graceful
pkill -TERM argo                                       # emergency
```

Use graceful unless restart-latency is itself the SLA breach.
OS-managed restart paths:
[production-deploy](./production-deploy.md).

---

## 4. Diagnose (next 30 min)

### 4a. Snapshot diagnostic state

```bash
argo incident bundle --since 1h --output incident-$(date -u +%Y%m%dT%H%M%SZ).tar.gz
```

The bundle captures, in one file:

- `argo doctor --json`
- audit-chain tail since `--since`
- `argo worker restarts --since <duration>`
- `argo gateway metrics-json`
- `argo audit list-archives` (if archival job configured)
- `git rev-parse HEAD` + version banner
- `.argo/argo.toml` with credentials scrubbed
- last 1000 lines of process stderr (if accessible)

`manifest.json` at the top enumerates every section with byte size
and success/failure status. **The bundle is read-only on system
state** — it does not pause, restart, or mutate config.

Bundles may contain operational PII even after redaction. **Do not
commit a bundle to the repo.** Send via support portal / ticket
per the tenant's data-handling policy.

### 4b. Read the audit-chain tail

```bash
argo audit query --since-minutes 60 --json | jq '.[] | {at, kind, worker_id}'
```

Look for: chain gaps (crash / hash mismatch), bursts of one `kind`
(single defect amplified), unexpected `kind` values (injection or
auth issue — escalate to
[security-audit](./security-audit.md) immediately).

### 4c. Check upstream

Confirm upstream is healthy before assuming the bug is yours.
Provider status pages: [failure_modes.md
§Provider-failures](../failure_modes.md). If a provider is down the
recipe is wait + fallback provider, not redeploy.

### 4d. Security incidents

For credential leak, suspected intrusion, or audit-chain tampering,
switch immediately to [security-audit](./security-audit.md).
Chain-of-custody, evidence preservation, and regulator-notification
timelines have different operational requirements not covered here.

---

## 5. Mitigate

Six rollback recipes. Run only after
[Diagnose](#4-diagnose-next-30-min) has confirmed the matching
symptom.

### 5a. Bad config deploy

```bash
git -C ~/.skycot/config log --oneline -5
git -C ~/.skycot/config revert <bad-sha>
argo gateway reload
argo doctor --json | jq '.summary.status'
```

### 5b. Leaking credential

```bash
argo cred clear --name <slot>
argo connector revoke --connector <name>
# rotate upstream out-of-band, then:
argo cred set --name <slot> --value-stdin
argo connector authorize --connector <name>
```

File the leak with [security-audit](./security-audit.md).

### 5c. Bad migration

```bash
argo backup list --json | jq '.[0].path'
argo gateway stop --grace 5s
argo restore <pre-migration-backup> --into /tmp/restored
# operator verifies /tmp/restored manually, then promotes
```

Migrations are additive by core invariant, so true rollback is
rare — usually "stop, restore, restart". Full procedure:
[workers runbook §Backup](./workers.md).

### 5d. Broken vertical manifest

```bash
argo vertical lint --vertical <id>
argo vertical pull --vertical <id> --revision <last-good-sha>
argo doctor --json | jq '.verticals'
```

### 5e. Worker OOM crash-loop

```bash
argo worker restarts --since 1h --json
argo tenant quota --tenant <id> --pause
ulimit -v unlimited
argo tenant quota --tenant <id> --unpause
```

If OOM repeats after resume, root cause is in the model call (token
explosion) or action-output capture (unbounded buffer). Both are
escalations, not in-incident fixes.

### 5f. Gateway 5xx spike

```bash
argo gateway metrics-json | jq '.http_5xx_by_route'
argo gateway routes-print | grep -v ok
argo gateway stop --grace 30s && argo gateway start
```

If 5xx persists post-restart the cause is downstream — re-run
[Triage](#2-triage-first-5-min) and contain the downstream
subsystem, not the gateway.

---

## 6. Communicate

Copy/paste these under stress.

### Internal Slack — declaration

```text
:rotating_light: SEV-<N> declared <UTC-time> by <operator>.
Symptom: <one line>.
Suspected scope: <tenant / region / global>.
First action: <containment step you are taking>.
Next update: <wall-clock time, per cadence>.
```

### Internal Slack — update

```text
SEV-<N> update <UTC-time>. Status: <investigating / contained / mitigated>.
Change since last update: <one line>.
Next update: <wall-clock time>.
```

### External status page — SEV-1 ack (≤15 min after declare)

```text
We are investigating a service disruption affecting <scope>.
Customers may see <observable symptom>. Engineering is engaged.
Next update by <UTC time, +30 min>.
```

### External customer email — SEV-1 resolution

```text
Subject: Service disruption resolved — <UTC date>

Between <start-UTC> and <end-UTC>, customers on <scope> saw
<observable symptom>. The disruption was caused by <root cause,
plain English>. Service is fully restored.

We will share a blameless post-mortem within 5 business days.
```

Cadence: SEV-1 every 30 min, SEV-2 every 2 h, SEV-3 daily.

---

## 7. Post-mortem (within 5 business days)

```bash
argo incident new --from <yaml-fixture>      # CI / scripted
argo incident new                            # interactive stdin
```

The walker writes `docs/skycot/incidents/<YYYY-MM-DD>-<slug>.md`
from `docs/skycot/incidents/_template.md`. Required sections, all
blameless and factual:

- **What happened.** Plain-English summary, no role-attribution.
- **Timeline.** UTC timestamps from declaration to resolution.
- **Root cause.** No "operator error" without the missing
  safeguard that allowed the error.
- **Contributing factors.** Latent issues that delayed
  detection / containment.
- **Detection.** Time-to-alarm; human-noticed vs automated.
- **Resolution.** The mitigation that worked + the revert diff.
- **Action items.** Each one filed as `[NEW]` in
  `docs/build-plan/BUILD_STATUS.md` (track-fit) or
  `docs/build-plan/MISSION_BACKLOG.md` (mission-level). No owner
  + deadline = a wish, not an action item.
- **Lessons learned.** Forward-looking, generalizable.

Audit-chain forensics link from the post-mortem via row IDs —
[observability §9.1](./observability.md) covers worker trail
extraction.

---

## 8. Specific playbooks

Six failure-mode playbooks. Each:
**symptom → confirm → contain → diagnose → fix → verify**.

### 8a. Migration apply failure

- **Symptom.** Binary refuses to start; `argo doctor` reports
  `migrations_pending > 0` or `schema_drift`.
- **Confirm.** `argo db migrate --dry-run` returns non-zero with
  the failing migration named.
- **Contain.** `argo gateway stop --grace 5s`. Do NOT re-attempt
  the migration without a backup.
- **Diagnose.** `argo backup list --json` for last good
  pre-migration backup. Inspect the failing SQL by hand.
- **Fix.** [5c](#5c-bad-migration).
- **Verify.** `argo doctor` reports `status: ok` and
  `migrations_pending: 0`.

### 8b. Audit-chain hash mismatch

- **Symptom.** `argo audit verify` returns non-zero; row N+1
  `prev_hash` does not match row N.
- **Confirm.** `argo audit query --row <N> --row <N+1>` shows two
  consecutive rows with the wrong link.
- **Contain.** Stop the binary immediately
  (`argo gateway stop --grace 0`). A corrupt chain that keeps
  accepting writes corrupts further. Escalate to
  [security-audit](./security-audit.md) — tamper signature.
- **Diagnose.** Pull audit DB from last good backup; compare row
  hashes side-by-side; identify divergence point.
- **Fix.** No in-incident fix without security review. Freeze,
  capture forensics, escalate. Chain repair happens out-of-band.
- **Verify.** `argo audit verify --json | jq '.ok'` returns true
  after the approved repair.

### 8c. Provider hard-quota exhaustion

- **Symptom.** Workers stall with `[STOP: NEED_INFO transport
  error]` and provider status page shows quota / billing notice.
- **Confirm.** `argo doctor --json | jq '.provider'` shows
  `quota_exhausted`.
- **Contain.** Switch the vertical to its configured fallback
  provider; if none configured, pause affected tenants
  [(3a)](#3a-pause-one-tenant).
- **Diagnose.** Read
  [failure_modes.md §Provider-failures](../failure_modes.md).
  Confirm quota exhaustion vs credential rotation.
- **Fix.** Restore quota out-of-band or fail over. Add fallback
  provider to manifest if not present.
- **Verify.** `argo doctor` reports provider `ok`; a test job
  completes without `need_info`.

### 8d. MCP connector auth-rotate cascade

- **Symptom.** Multiple connectors fail authorization
  simultaneously after credential rotation.
- **Confirm.** `argo connector list --json | jq '.[] |
  select(.status != "ok")'` shows >1 connector with the same
  `last_error` shape.
- **Contain.** `argo connector disable <name>` per cascading
  connector.
- **Diagnose.** Shared OAuth client rotated? Shared cloud
  credential? Cross-ref with `argo cred list` for recent `set`
  activity.
- **Fix.** Re-authorize one connector at a time:
  `argo connector authorize --connector <name> &&
   argo connector test --connector <name>`.
- **Verify.** All affected connectors `status: ok`; test
  invocation through each succeeds.

### 8e. Worker OOM crash-loop

- **Symptom.** `argo worker restarts --since 1h` shows a worker
  restarting on sub-minute cadence; kernel `dmesg` shows
  `Out of memory: Killed process <pid> argo`.
- **Confirm.** Reproduce with one job:
  `argo work serve --vertical <id> --once 2>&1 | tail -40`;
  process exits with SIGKILL after RSS growth.
- **Contain.** [5e](#5e-worker-oom-crash-loop). Pause first;
  ulimit alone may push OOM later.
- **Diagnose.** Unbounded prompt? Tool returning unbounded
  payload (50MB blob into model context)? Per-step output budget:
  [workers runbook §Memory](./workers.md).
- **Fix.** Cap the offending tool's output budget in the
  vertical manifest; or lower `--max-tool-rounds` for the
  vertical.
- **Verify.** Re-run completes under the configured memory
  budget; `argo worker restarts --since 30m` is zero for the
  affected worker.

### 8f. Gateway 5xx spike

- **Symptom.** `argo gateway metrics-json` shows
  `http_5xx_total` rising; user reports say "everything is
  broken".
- **Confirm.** `argo gateway routes-print --json | jq '.[] |
  select(.status != "ok")'` isolates failing routes.
- **Contain.** [5f](#5f-gateway-5xx-spike). If 5xx is one-route,
  most callers continue degraded.
- **Diagnose.** Recent stderr tail from the incident bundle. The
  first 5xx usually has the root-cause trace; subsequent ones are
  downstream retries.
- **Fix.** Usually [5a](#5a-bad-config-deploy) or
  [5c](#5c-bad-migration).
- **Verify.** `http_5xx_rate` returns to baseline (<0.1%);
  `/livez` and `/readyz` both 200.
