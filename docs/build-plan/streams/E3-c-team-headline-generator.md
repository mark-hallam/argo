# Stream E3-c — Team page: Today-block headline generator (backend, vertical-aware)

**Owner**: claude.
**Depends on**: E3-a MERGED (consumes the `Worker.headline` field defined there); existing audit-record schema + vertical KPI schema (`mission-vertical-kpi-declarative` MERGED).
**Authored against**: origin/main `ccfae5a2` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E3-c-team-headline-generator`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E3 brief 3 of 5. **Backend brief — Rust, not TypeScript.**

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| KPI selection per vertical | Hardcoded mapping (gtm-engineer → revenue; recruiter → interviews; etc.) | Each vertical's manifest declares its `[team_headline]` KPIs; generator reads from manifest, not code | `mission-enterprise-team-headline-manifest` |
| Headline language | English-only | i18n-ready: headlines emitted as `{ template_id, params }` so the frontend can localize | `mission-enterprise-team-headline-i18n` |
| Privacy filtering | Includes raw counts only (no customer names, no monetary values for masked PII) | Server-side filtering against the viewer's RBAC: e.g. ops-only headlines hide revenue from operator-view | `mission-enterprise-team-headline-rbac-filter` |
| Cost | Pure SQL aggregation; no LLM call | Same — keep this fast. LLM is overkill for fixed counts | n/a |
| Caching | Recomputed per `GET /api/v1/team` request | Per-worker cached for 60s; invalidated on action commit | `mission-enterprise-team-headline-cache` |

v1 is acceptable: the generator is read-only,
deterministic, and feeds the team-page UI. Hardcoded
KPI mapping is OK for v1 since vertical KPI schema
already exists; refactoring to manifest-driven is a
data-only migration.

## Why

The Team page renders 2-3 "today" bullets per worker
(see [wireframe-team-page.md](../../skycot/ux-research/wireframes/wireframe-team-page.md)):
"8 replies sent (2 waiting your OK)", "$340 in pending
offers", etc. These bullets must be vertical-aware —
gtm-engineer shows revenue, recruiter shows interview
counts, customer-support shows resolution rate.

E3-a defined the `Worker.headline` field with
`bullets[]` and `tomorrow?` strings; this brief is the
server-side generator that fills it. Without this, the
team API returns empty headlines + the UI shows
"No activity yet" everywhere.

## Scope

NEW backend library function `generate_team_headline`
in `argo-team-headline` crate (NEW crate). The
function:
1. Reads the worker's audit events for the last 24h.
2. Reads the worker's vertical id.
3. Looks up the vertical's KPI table (hardcoded in
   v1 inside the crate).
4. Computes 2-3 KPI bullets per the table.
5. Computes a Tomorrow line from the worker's duty
   schedule (next-due duty in next 24h).
6. Returns `{ bullets: Vec<String>, tomorrow:
   Option<String> }`.

The team API endpoint
(`mission-gateway-team-list-endpoint`, sibling backend
brief from E3-a's hardening list) calls this generator
per-worker as part of building the response.

## KPI mapping table (v1, hardcoded)

| Vertical | Headline KPIs (order) |
|---|---|
| gtm-engineer / marketplace-seller | replies sent, pending counter-offers, revenue this week |
| recruiter | interviews booked this week, candidates screened today, pending rejection drafts |
| customer-support | tickets resolved today, avg response time, pending escalations |
| ai-receptionist | calls handled today, scheduled meetings today, pending callbacks |
| sales-sdr | outreach emails sent today, meetings booked, pending follow-ups |
| sales-follow-up | follow-ups sent today, deals advanced, pending approvals |
| product-manager | feedback items triaged, specs drafted, pending stakeholder updates |
| tax-prep | documents extracted, returns drafted, pending operator review |
| finance-ar | invoices sent, payments received, pending collections |
| healthcare-intake | patients scheduled, forms triaged, pending operator approvals |
| legal-paralegal | documents reviewed, contracts drafted, pending operator review |
| event-coordinator | events booked, RSVPs handled, pending coordination items |
| real-estate-agent | listings replied, viewings scheduled, pending offers |
| pr-checker | PRs reviewed today, issues flagged, pending re-reviews |
| receptionist | inquiries handled, appointments booked, pending handoffs |
| (unknown vertical) | fallback: actions taken today, pending approvals, last activity |

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: NEW `crates/argo-team-headline`
  crate; narrow extension to the team-list endpoint
  (when sibling brief lands).

## Files you may touch

- `crates/argo-team-headline/Cargo.toml` — NEW.
  Standard crate manifest. Deps: `argo-types`,
  `argo-vertical`, `argo-audit`, `sqlx`. ≤ 30 LOC.
- `crates/argo-team-headline/src/lib.rs` — NEW. Crate
  root + main `generate_team_headline()` function.
  ≤ 80 LOC.
- `crates/argo-team-headline/src/kpi_table.rs` — NEW.
  The hardcoded KPI mapping table per vertical.
  ≤ 250 LOC.
- `crates/argo-team-headline/src/compute.rs` — NEW.
  SQL aggregation helpers + bullet formatting. ≤ 300
  LOC.
- `crates/argo-team-headline/src/tomorrow.rs` — NEW.
  Tomorrow-line computation from duty schedule. ≤ 150
  LOC.
- `crates/argo-team-headline/src/lib_tests.rs` — NEW.
  ≥ 12 unit tests covering each vertical's KPIs +
  edge cases. ≤ 400 LOC.
- `Cargo.toml` (workspace root) — narrowly scoped:
  1-line addition to `members =`. No other changes.

## Files you must NOT touch

- `crates/argo-vertical/**` — KPI schema locked from
  `mission-vertical-kpi-declarative`. v1 reads from
  hardcoded table; v1.5 reads from this schema (sibling
  Track-H brief).
- `crates/argo-audit/**` — read-only consumer.
- `crates/argo-cli/**` — sibling team-list endpoint
  brief calls this generator; not in this PR.
- All E1/E2/E3-a/b files — frontend; out of scope.

## Acceptance criteria

- [ ] NEW crate `argo-team-headline` compiles standalone.
- [ ] `generate_team_headline(worker_id, vertical_id,
      db_conn) -> Result<TeamHeadline>` callable from
      any axum/tonic handler.
- [ ] `TeamHeadline` struct:
      ```rust
      pub struct TeamHeadline {
          pub bullets: Vec<String>,
          pub tomorrow: Option<String>,
      }
      ```
- [ ] For each vertical in the table above, the
      generator returns the correct 2-3 bullets given
      seeded test audit data.
- [ ] Unknown vertical: returns the fallback 3-bullet
      structure.
- [ ] Worker with zero activity in 24h: bullets is
      `vec!["No activity yet."]` (single-element
      list).
- [ ] Tomorrow line: computed from duty schedule;
      returns the next-due duty's plain-English
      summary if within 24h, otherwise `None`.
- [ ] No PII in bullets — customer names, addresses,
      message contents not echoed. Only counts +
      monetary aggregates + KPI labels.
- [ ] Bullets ≤ 60 chars each (for readability in
      the team card layout).
- [ ] `cargo build -p argo-team-headline` clean.
- [ ] `cargo clippy -p argo-team-headline --all-targets
      -- -D warnings` clean.
- [ ] `cargo test -p argo-team-headline` passes (12
      tests).
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E3-c-team-headline-generator`
- PR title: `team-headline: vertical-aware Today-block bullets generator (E3-c)`

## Notes for the implementer

- **SQL aggregation strategy.** Single CTE per vertical
  computing all the KPI counts in one query:
  ```sql
  WITH today AS (
    SELECT * FROM audit_record
    WHERE worker_id = $1
      AND created_at > NOW() - INTERVAL '24 hours'
  )
  SELECT
    COUNT(*) FILTER (WHERE action_type = 'message_sent') AS replies_sent,
    COUNT(*) FILTER (WHERE status = 'pending') AS pending_approvals,
    COALESCE(SUM(amount) FILTER (WHERE action_type = 'offer_made'), 0) AS pending_revenue,
    ...
  FROM today;
  ```
  Per-vertical CTE shape lives in `compute.rs`.

- **Bullet formatting.** Each vertical's bullets emit
  via a small builder fn that handles plural / zero /
  null cases:
  ```rust
  fn fmt_replies(n: i64) -> Option<String> {
      match n {
          0 => None,                                 // skip
          1 => Some("1 reply sent".into()),
          n => Some(format!("{n} replies sent")),
      }
  }
  ```
  Each KPI gets its own formatter. Bullets with
  `None` are dropped before the final return —
  workers with no activity in a KPI dimension don't
  show that line.

- **Tomorrow line.** Read from
  `worker_duty_schedule`:
  ```sql
  SELECT id, name, next_fire_at, cron_expr
  FROM worker_duty
  WHERE worker_id = $1
    AND next_fire_at > NOW()
    AND next_fire_at < NOW() + INTERVAL '24 hours'
  ORDER BY next_fire_at
  LIMIT 1;
  ```
  Format: `"morning routine at 06:30"` (combine duty
  name + local time of next fire).

- **Time zone for "today" vs "tomorrow".** Use the
  workspace timezone, not server UTC. Read from the
  workspace row.

- **Per-vertical kpi_table.rs structure.**
  ```rust
  pub struct VerticalKpiTable {
      pub vertical_id: &'static str,
      pub bullets: &'static [BulletKind],
  }

  pub enum BulletKind {
      RepliesSent,
      PendingApprovals,
      RevenuePending,
      InterviewsBooked,
      ...
  }
  ```
  One static table per vertical. Generator picks the
  table by id, computes each `BulletKind` via its
  formatter, drops the Nones.

- **PII filtering.** Counts + monetary values are
  fine; raw text from messages is NOT. The audit
  record schema already separates structured fields
  from message-body content; the generator only
  reads structured fields.

- **Caching is out of scope.** v1 recomputes per
  request; performance is fine for the team list
  (≤ 50 workers per typical workspace). v1.5 cache
  brief handles it.

- **No new schema migrations.** This brief reads
  existing tables only.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-team-headline-manifest` — read
  KPIs from vertical manifests, not hardcoded table.
- `mission-enterprise-team-headline-i18n` — emit
  bullets as `{ template_id, params }` for
  localization.
- `mission-enterprise-team-headline-rbac-filter` —
  per-viewer RBAC filter on visible KPIs.
- `mission-enterprise-team-headline-cache` — per-
  worker cached for 60s with action-commit
  invalidation.
