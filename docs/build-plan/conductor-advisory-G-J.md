# Conductor advisory — Tracks G + J (21 briefs combined)

Authored 2026-05-12 against origin/main `fb0ddeb2` after the
G+J author phase completed. Paste the fenced block below into
the running conductor at the next tick. The conductor inserts
21 new TODO rows (3 from G, 18 from J).

## Why both in one advisory

Track G (mission claim 5 finish — 3 briefs) and Track J
(gateway endpoint siblings to Track E — 18 briefs) are
independent but small enough to ship together. The conductor
handles the row inserts in one tick.

## Paste this to the conductor

```
MID-TICK ADVISORY — Tracks G + J briefs queued.
Twenty-one new TODO rows for the ledger. Dependencies noted
per row.

Documents changed:
  - PRODUCT_ROADMAP.md — Tracks G + J already IN PROGRESS
    (flipped earlier in /loop).
  - 21 new files under docs/build-plan/streams/.

Action required from you this tick:
  Insert twenty-one new TODO rows in BUILD_STATUS.md. Commit
  ONE row per commit per existing convention.

────────────────────────────────────────────────────────────
TRACK G — MISSION CLAIM 5 FINISH (3 briefs)
────────────────────────────────────────────────────────────

Row 1
  brief id:  G1-cu-native-windows-executor-dispatch
  owner:     claude  status: TODO  PR: —
  depends:   mission-cu-native-windows-poc MERGED (Track B)
  note:      "**Track G mission claim 5, 1/3.** Mirrors macOS
              #955 dispatch wiring for the new Windows POC
              crate. ExecutorKind::ComputerUseWindows variant
              + adapter file + runtime probe. Brief:
              streams/G1-cu-native-windows-executor-dispatch.md."

Row 2
  brief id:  G2-cu-native-linux-wayland
  owner:     claude  status: TODO  PR: —
  depends:   claude-cu-native-linux-poc MERGED
  note:      "**Track G, 2/3.** Wayland support for the Linux
              CU executor — xdg-desktop-portal D-Bus via zbus,
              wtype for typing, grim fallback for sway/Hyprland.
              Brief: streams/G2-cu-native-linux-wayland.md."

Row 3
  brief id:  G3-real-llm-journey-ci-integration
  owner:     claude  status: TODO  PR: —
  depends:   mission-real-llm-operator-journey-test MERGED
              (Track B)
  note:      "**Track G, 3/3 (LAST).** NEW GitHub Actions
              workflow real-llm-smoke.yml — nightly cron +
              workflow_dispatch + Slack failure alert. Brief:
              streams/G3-real-llm-journey-ci-integration.md."

────────────────────────────────────────────────────────────
TRACK J — GATEWAY ENDPOINTS SIBLING TO TRACK E (18 briefs)
────────────────────────────────────────────────────────────

All Track J briefs are RUST backend work. Most depend on E0
MERGED (auth/AppShell contract) AND specific frontend E*
briefs that consume them (mock fallback covers the gap until
the J brief merges).

J briefs that establish primitives — claim FIRST:
  J1 (approvals JSON API)
  J2 (server-side PII fence)
  J7 (worker detail)
  J14 (audit feed)

Row 4
  brief id:  J1-gateway-approvals-json-api
  owner:     claude  status: TODO  PR: —
  note:      "**Track J 1/18.** /api/v1/approvals JSON wrapper
              over existing webinbox state machine. Unblocks E1-a.
              Brief: streams/J1-gateway-approvals-json-api.md."

Row 5
  brief id:  J2-gateway-approvals-pii-redaction-server-side
  owner:     claude  status: TODO  PR: —  depends: J1 MERGED
  note:      "**Track J 2/18.** NEW argo-redact crate; server-
              side fence over J1's responses. Sentinel format
              byte-identical to E1-e client. Brief:
              streams/J2-gateway-approvals-pii-redaction-server-side.md."

Row 6
  brief id:  J3-gateway-inbox-ws-structured-events
  owner:     claude  status: TODO  PR: —  depends: J1 MERGED
  note:      "**Track J 3/18.** Structured event types
              (approval.created/decided/undone/expired) on
              /inbox/ws + query-param JWT auth. Brief:
              streams/J3-gateway-inbox-ws-structured-events.md."

Row 7
  brief id:  J4-gateway-genesis-classify-endpoint
  owner:     claude  status: TODO  PR: —
  note:      "**Track J 4/18.** HTTP wrappers around existing
              argo-genesis pipeline (classify + draft-worker).
              Unblocks E2-c + E2-d. Brief:
              streams/J4-gateway-genesis-classify-endpoint.md."

Row 8
  brief id:  J5-gateway-workers-hire-endpoint
  owner:     claude  status: TODO  PR: —  depends: J4 MERGED
  note:      "**Track J 5/18.** POST /api/v1/workers — atomic
              create (worker + duties + memory namespace);
              Idempotency-Key support; full hire-context audit.
              Brief: streams/J5-gateway-workers-hire-endpoint.md."

Row 9
  brief id:  J6-gateway-team-list-endpoint
  owner:     claude  status: TODO  PR: —  depends: J5 + E3-c MERGED
  note:      "**Track J 6/18.** GET /api/v1/team — tenant-scoped
              WorkerSummary[] with E3-c headlines. Brief:
              streams/J6-gateway-team-list-endpoint.md."

Row 10
  brief id:  J7-gateway-worker-detail-endpoint
  owner:     claude  status: TODO  PR: —  depends: J5/J6 MERGED
  note:      "**Track J 7/18.** GET /api/v1/workers/<id> — full
              WorkerDetail + permissions resolver shared with
              other endpoints. Brief:
              streams/J7-gateway-worker-detail-endpoint.md."

Row 11
  brief id:  J8-gateway-worker-overview-endpoint
  owner:     claude  status: TODO  PR: —  depends: J7 MERGED
  note:      "**Track J 8/18.** GET /workers/<id>/overview —
              charter + duties + 4 KPIs (with 7-day sparklines)
              + last-5 activity. Single SQL query to avoid N+1.
              Brief: streams/J8-gateway-worker-overview-endpoint.md."

Row 12
  brief id:  J9-gateway-worker-activity-endpoint
  owner:     claude  status: TODO  PR: —  depends: J7 MERGED
  note:      "**Track J 9/18.** GET /workers/<id>/activity —
              cursor-paginated audit feed scoped to one worker.
              Brief: streams/J9-gateway-worker-activity-endpoint.md."

Row 13
  brief id:  J10-gateway-worker-memory-endpoints
  owner:     claude  status: TODO  PR: —  depends: J7 MERGED
  note:      "**Track J 10/18.** 4 endpoints: GET /memory + facts
              CRUD (POST/PUT/DELETE soft-delete). Voice profile
              read-only (edits via J12). Brief:
              streams/J10-gateway-worker-memory-endpoints.md."

Row 14
  brief id:  J11-gateway-worker-settings-endpoints
  owner:     claude  status: TODO  PR: —  depends: J7 MERGED
  note:      "**Track J 11/18.** 9 endpoints: GET /settings,
              PATCH channels/quiet-hours/budget/autonomy, POST
              pause/resume/archive, DELETE (operator-only with
              type-name-to-confirm header). Brief:
              streams/J11-gateway-worker-settings-endpoints.md."

Row 15
  brief id:  J12-gateway-retrain-endpoints
  owner:     claude  status: TODO  PR: —  depends: J7+J10+J4 MERGED
  note:      "**Track J 12/18.** 4 endpoints: classify / commit
              (with 14-day undo snapshot) / undo / reverse-test
              (side-effect-free, capped 10 LLM calls). Brief:
              streams/J12-gateway-retrain-endpoints.md."

Row 16
  brief id:  J13-gateway-duty-endpoints
  owner:     claude  status: TODO  PR: —  depends: J7+J4 MERGED
  note:      "**Track J 13/18.** 5 endpoints: classify /
              commit / preview / PATCH / DELETE. Cross-cron
              overlap conflict detection at worker level. Brief:
              streams/J13-gateway-duty-endpoints.md."

Row 17
  brief id:  J14-gateway-audit-feed-endpoint
  owner:     claude  status: TODO  PR: —  depends: J2 MERGED
  note:      "**Track J 14/18.** GET /api/v1/audit — cursor-
              paginated feed with 7 filter dims + ?q= substring
              search. Index migration for tenant+created_at+
              worker_id composite if needed. Brief:
              streams/J14-gateway-audit-feed-endpoint.md."

Row 18
  brief id:  J15-gateway-audit-detail-endpoint
  owner:     claude  status: TODO  PR: —  depends: J2+J14 MERGED
  note:      "**Track J 15/18.** GET /api/v1/audit/<id> — full
              AuditDetail (summary + 8-step decision chain +
              provenance + output + external_refs pre-scan).
              Companion screenshot endpoint via HMAC-signed URLs.
              Brief: streams/J15-gateway-audit-detail-endpoint.md."

Row 19
  brief id:  J16-gateway-elevated-view-request-endpoint
  owner:     claude  status: TODO  PR: —  depends: J2 MERGED
  note:      "**Track J 16/18.** NEW elevated_view_grant table +
              3 endpoints (request → auto-grant in v1; list active;
              revoke). Brief:
              streams/J16-gateway-elevated-view-request-endpoint.md."

Row 20
  brief id:  J17-compliance-hipaa-disclosure-log
  owner:     claude  status: TODO  PR: —
  note:      "**Track J 17/18.** NEW HIPAA disclosure-log
              generator aligned to 45 CFR § 164.528. Reuses
              existing compliance-export Ed25519 signing.
              Migration adds phi_accessed column. Brief:
              streams/J17-compliance-hipaa-disclosure-log.md."

Row 21
  brief id:  J18-compliance-generic-export
  owner:     claude  status: TODO  PR: —  depends: J17 MERGED
  note:      "**Track J 18/18 (LAST).** Generic JSON + PDF export
              format. Pure-Rust printpdf — no headless browser.
              Closes E5-e's 4-format menu and Track J author phase.
              Brief: streams/J18-compliance-generic-export.md."

Commit each row insertion per existing convention, one commit
per row. Commit messages follow:
  status: new G1-cu-native-windows-executor-dispatch TODO
  status: new G2-cu-native-linux-wayland TODO
  status: new G3-real-llm-journey-ci-integration TODO
  status: new J1-gateway-approvals-json-api TODO
  ... etc through J18.

Worker pickup guidance:

CONTRACT briefs — claim ASAP for dependency unblocking:
  - J1 (approvals JSON API) — unblocks J2/J3
  - J2 (PII fence) — unblocks J14/J15
  - J4 (Genesis HTTP) — unblocks J5/J12/J13
  - J5 (workers hire) — unblocks J6/J7
  - J7 (worker detail) — unblocks J8-J11 + J12
  - J14 (audit feed) — unblocks J15

Parallel-safe pairs once contracts merge:
  - J3 + J14 (independent of J5 chain)
  - J8 + J9 + J10 + J11 (all wait on J7, then parallel)
  - J12 + J13 (independent once J4 + J7 merge)
  - J17 + J18 (sequential — J18 depends on J17 pattern)

Track G briefs are independent of each other and of Track J.
G1/G2 are Rust executor work; G3 is workflow YAML.

Per the enterprise-grade-defaults memory, ALL briefs in this
batch state v1 posture explicitly and name Track-H hardening
briefs per shortcut.

Resume your normal tick after the twenty-one row inserts +
commits. Acknowledge by including the new row IDs in your
next status note.
```

## Mission claim coverage after G + J land

| Mission claim | Status |
|---|---|
| 1. Teammate, not tool | E + J endpoints fully back the SPA |
| 2. Delegation is the unit | E2 hire flow + J5 hire-commit + J13 duties |
| 3. Platform builds workers | J4 Genesis HTTP + J5 atomic hire |
| 4. Skills shared and learned | Substantially MERGED (Track F) |
| 5. MCP + computer-use | G1 + G2 close native CU on all 3 OSes |
| 6. Multi-channel approval | E1 + J1 + J3 land the full WS+JSON stack |

Net: when E0–E5 + G1–G3 + J1–J18 all MERGE, **all six mission
claims have working production backends + frontends**.

## What this advisory replaces

Nothing. This is the third Track-related advisory. Companion
to:
- `conductor-advisory-E0-E1.md` (7 rows)
- `conductor-advisory-E2-E5.md` (31 rows)
- `conductor-advisory-F-reconciliation.md` (2 reconciliations)

Total rows queued via advisories across this /loop run:
**7 + 31 + 21 = 59 new ledger rows**.
