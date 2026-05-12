# Mission load-bearing wire — incident-response runbook

**Owner**: claude. **Depends on**: existing `docs/skycot/runbook/*.md` (sibling runbooks: actions / conductor-troubleshooting / observability / production-deploy / security-audit / soak-test / workers); `docs/skycot/failure_modes.md` (existing failure-mode catalogue — verify file path at claim time).
**Authored against**: origin/main `20240927` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-incident-response-runbook`.

> SOURCE: MISSION_BACKLOG.md Section 7 Claim 7 #226 ("Incident-response runbook beyond failure_modes.md"). **Tier S (compliance + ops).** Today: `docs/skycot/failure_modes.md` catalogues known failure modes statically — but operators in the middle of a live incident need a step-by-step playbook: triage → contain → diagnose → mitigate → post-mortem. This brief writes that playbook as a runbook, cross-referenced with the existing observability / security-audit / production-deploy runbooks, and adds an `argo incident bundle` CLI that snapshots the diagnostic state (audit-chain tail, recent metrics, worker restart history, queue depth) into a single tar.gz for sharing with the on-call team or upstream support.

## Scope

NEW `docs/skycot/runbook/incident-response.md` (NEW, ≤ 450 LOC):

The runbook follows the operational SEV taxonomy:

- **SEV 1** — data-corruption, customer-data-exposure, total-outage. Page the on-call. Snapshot diagnostic state. Halt ingress.
- **SEV 2** — degraded service (>30% error rate, latency >10× p50, partial outage). Page during business hours; off-hours wait until morning. Reduce blast radius via tenant pause.
- **SEV 3** — single-tenant or single-feature degradation. Track in incident log; fix in next business-day work block.
- **SEV 4** — minor / cosmetic. Track in routine queue.

Sections required (≤ 30 LOC each):

1. **Declaring an incident** — who, how, where (Slack channel + audit-chain row + status page). Decision tree for SEV-1 vs SEV-2 vs SEV-3.
2. **Triage (first 5 min)** — three checks: (a) `argo doctor`, (b) `argo gateway routes-print` for endpoint health, (c) `argo worker restarts --since 1h` for crash-loop signal. Cross-refs to `observability.md` for metrics dashboards.
3. **Contain (next 10 min)** — when to pause a tenant via `argo tenant quota --tenant <id> --pause`; when to fail open vs fail closed; when to restart the whole binary vs hotpatch a single connector via `argo connector disable <name>`.
4. **Diagnose (next 30 min)** — running `argo incident bundle` to snapshot state; reading the audit-chain tail; checking E2B / connector quotas; verifying upstream provider status pages. Cross-ref to `security-audit.md` for security-incident-specific diagnostic.
5. **Mitigate (within SLA)** — rollback recipes for: a bad config deploy, a leaking credential, a bad migration, a broken vertical manifest. Each recipe is 3–5 specific commands.
6. **Communicate** — internal vs external comms templates; cadence for SEV-1 (every 30 min) vs SEV-2 (every 2h) vs SEV-3 (daily).
7. **Post-mortem (within 5 business days)** — blameless template, link to audit-chain forensics, action-items into BUILD_STATUS.md / MISSION_BACKLOG.md as `[NEW]` items.
8. **Specific playbooks** — short sections (≤ 20 LOC each) for the 6 most common incidents drawn from `failure_modes.md`:
   - Migration apply failure
   - Audit-chain hash mismatch (verify failure)
   - Provider hard-quota exhaustion mid-run
   - MCP connector auth-rotate cascade
   - Worker OOM crash-loop
   - Gateway 5xx spike

Each playbook follows the same shape: **symptom → confirm → contain → diagnose → fix recipe → verify**.

### `argo incident bundle` CLI

EXTEND `crates/argo-cli/src/cli_command_enum.rs` (≤ 15 LOC delta) — new top-level `IncidentCmd::Bundle`:
```
argo incident bundle [--since 1h] [--output <path>]
```

NEW `crates/argo-cli/src/handlers/incident_bundle.rs` (NEW, ≤ 220 LOC) — captures:
- `argo doctor` JSON output
- audit-chain tail (last N rows or since timestamp)
- `argo worker restarts --since <duration>` JSON
- `argo gateway metrics-json` snapshot
- `argo audit list-archives` (if `mission-evidence-archival-audit` shipped)
- `git rev-parse HEAD` (binary build identity) + version banner
- redacted `.argo/argo.toml` (credentials scrubbed)
- recent log tail (last 1000 lines of process stderr if accessible)

Writes everything into `<output>` (default `incident-<ts>.tar.gz`) with a top-level `manifest.json`. Each captured section is one file in the tarball.

Secret redaction is load-bearing: scan all captured fields for `(?i)(api_key|secret|password|token|bearer|cred)`-named keys and replace values with `<REDACTED>`. Match the existing pattern from `mission-vcr-cassette-fixtures` (just shipped #1377).

NEW `crates/argo-cli/tests/incident_bundle_smoke.rs` (NEW, ≤ 130 LOC) — synthesizes a mock state + asserts: bundle is tar.gz; manifest lists all expected sections; credential fields are redacted; bundle is openable without Argo installed.

NO migration. NO Rust changes in core crates (only argo-cli additions).

This is NOT in scope: automated incident detection (separate concern — metric alerting is `mission-prometheus-alerting` brief if/when authored); automatic SEV classification (operator judgment is load-bearing in v1); paging integration (PagerDuty / OpsGenie / Slack-alert — separate operator-integration brief); incident database / Jira sync (out-of-scope); post-mortem-publishing automation (operator copies blameless template into wiki / Notion manually); SOC2 evidence-collection ties (the existing `argo compliance export` covers SOC2 evidence on demand); status-page integration (operator updates statuspage.io manually following the runbook).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `docs/skycot/runbook/incident-response.md` (NEW, ≤ 450 LOC)
- `crates/argo-cli/src/cli_command_enum.rs` (EXTEND, ≤ 15 LOC)
- `crates/argo-cli/src/handlers/incident_bundle.rs` (NEW, ≤ 220 LOC)
- `crates/argo-cli/src/handlers/mod.rs` (EXTEND, ≤ 3 LOC)
- `crates/argo-cli/tests/incident_bundle_smoke.rs` (NEW, ≤ 130 LOC)

## Files you must NOT touch

- `docs/skycot/failure_modes.md` (sibling — runbook cross-references but does NOT rewrite the catalogue)
- Other `docs/skycot/runbook/*.md` files (cross-reference only; don't rewrite siblings)
- `crates/argo-runtime/**` (no runtime changes; runbook is documentation-first)
- `crates/argo-store/migrations/**` (no migration)

## Acceptance criteria

- [ ] `docs/skycot/runbook/incident-response.md` exists with all 8 sections + 6 playbooks
- [ ] Runbook cross-references existing siblings (observability / security-audit / production-deploy) without duplicating their content
- [ ] SEV taxonomy clearly defined with paging cadence per tier
- [ ] Each of the 6 specific playbooks follows the symptom → confirm → contain → diagnose → fix recipe → verify shape
- [ ] `argo incident bundle` produces a `.tar.gz` with manifest.json + sections per spec
- [ ] Credential fields are redacted (smoke verifies)
- [ ] Bundle is openable via `tar -tzf` without Argo installed
- [ ] `cargo test -p argo-cli --test incident_bundle_smoke` passes
- [ ] No migration; no core Rust changes
- [ ] All new files < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-incident-response-runbook`
- PR title: `runbook: incident-response playbook (SEV taxonomy + 6 specific playbooks) + argo incident bundle CLI (mission-incident-response-runbook)`

## Notes for the implementer

- The runbook is the deliverable; the CLI is a force multiplier on top of it. Write the runbook FIRST and reference the CLI from inside it, not the reverse.
- SEV definitions should match common industry practice (Google SRE / PagerDuty's standard). Don't invent a 5-tier model — operators expect SEV 1-4.
- Cross-references to sibling runbooks: use relative-path links (`[observability runbook](./observability.md)`), not absolute URLs. The runbook lives alongside its siblings.
- The 6 specific playbooks are drawn from `failure_modes.md` — read that file first and pick the 6 highest-priority failure modes for v1. If you find a failure mode in `failure_modes.md` that's not covered in a sibling runbook (e.g. `actions.md` covers action-executor failures), include it in the playbook section.
- Bundle redaction is load-bearing for sharing with upstream support. Match the existing redaction pattern from `mission-vcr-cassette-fixtures` (just shipped #1377). Don't reinvent the regex.
- DO NOT write an actual incident — write the runbook for handling future incidents. The blameless post-mortem template is one section, not a worked example.
- Bundle output default is `incident-<utc-ts>.tar.gz` in cwd. Override via `--output <path>`. Document that operators should NOT commit incident bundles to the repo (they may contain operational PII even after redaction).
- The CLI is read-only on the system state. NO mutation flags. NO restart, NO pause, NO retry. Operators who want to act execute the recipe from the runbook manually — this preserves the audit-chain trail.
- DO NOT auto-upload bundles anywhere. The operator decides where the bundle goes (slack channel, ticket, S3, support portal).
- The runbook section on "Communicate" should include both internal templates (Slack channel posts) and external templates (statuspage updates / customer email shape). Keep them short — operators need to copy/paste under stress.
- Post-mortem section: blameless template is the industry standard ("what happened / what we did / what we'd do differently"). Include the link-back to BUILD_STATUS.md / MISSION_BACKLOG.md so action items don't vanish.
- DO NOT include security-incident-specific playbooks beyond cross-referencing `security-audit.md`. That domain has its own runbook for a reason (chain-of-custody, evidence preservation, regulator-notification timeline). Keep this brief's scope to operational incidents.
- Coordinate w/ `mission-evidence-archival-audit` parallel brief: the bundle should include `argo audit list-archives` if the archival brief shipped first; if not, that line is a no-op (just logs "audit archival not configured").
