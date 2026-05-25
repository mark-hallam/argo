# Round 63 — PR #2297 audit list/detail tenant-scope PRE-MERGE PASS + 🔴 PR #2300 skills/verticals `/api/v1/` prefix BREAKS LIVE SKILLS + VERTICALS PAGES

- **Date**: 2026-05-25 (UTC); Melbourne local 2026-05-25 23:08 AEST
- **Round**: 63 (scheduled-task hourly fire; ~93min after r62 ship 11:35Z; cadence holding)
- **Claims exercised**: Claim 6 (Multi-tenant by construction) via #2297 + frontend↔gateway contract via #2300 + process drift
- **Scenarios**: 3 — 1🟢 + 1🔴 (P0 demo-blocker, primitives empirically wrong) + 1🟢 process-drift confirmed clean
- **Cost / wall**: ~50K tokens, ~55min wall (within 30-80K / 30-60min budgets)

## Scenario 1 — 🟢 PR #2297 `Fix audit list/detail tenant scope` PRE-MERGE PASS

### Setup
- PR head `8548b16987f8843833e886949fd6d9b744b7649d`, MERGEABLE per `gh pr view 2297` (gh reports mergeable=UNKNOWN because the OR-poll hasn't fired yet; combined-checks all green; merge-tree clean), 4 files +132/-4
- `git worktree add /tmp/argo-r63-pr2297-wt pr-2297` (after `git fetch origin pull/2297/head:pr-2297`)
- `CARGO_TARGET_DIR=/tmp/argo-r63-pr2297-target RUSTC_WRAPPER="" CARGO_INCREMENTAL=0 cargo test -p argo-cli --bin argo list_detail_alignment` (cold; sccache disabled to ensure deterministic cold build)
- Cited by PR body: `docs/skycot/ux-audits/2026-05-25-1100-scene-6-claim-6-trust-and-approvals.md` (Scene-6 demo-gate)

### Observed
- **Build**: cold 6m13s, test run 0.32s, **3/3 PASS, 0 fail, 0 ignored** (`list_detail_alignment_tests::{default_feed_legacy_id_fetches_detail, explicit_tenant_feed_id_fetches_detail_but_not_other_tenant, explicit_tenant_feed_does_not_inherit_legacy_rows}`)
- **Production wire is real**. Diff against `origin/main`:
  1. `routes_audit_detail.rs:74-87` rewrites `fetch_row` SQL from `WHERE id = ? AND tenant_id = ?` (strict) to `WHERE id = ? AND (tenant_id = ? OR (? = ? AND tenant_id = ?))` (default-only legacy inheritance). Bind order: `id_bytes, tenant_id, tenant_id, DEFAULT_AUDIT_TENANT_ID="ws-default", LEGACY_AUDIT_TENANT_ID="__legacy__"`. Semantics: detail returns the row when either (a) `row.tenant_id == req_tenant_id` OR (b) `req_tenant_id == "ws-default" AND row.tenant_id == "__legacy__"`. Other tenants never inherit legacy rows.
  2. `routes_audit_feed_scope.rs:23,38` flips `include_legacy` from unconditional `true` to `!explicit_workspace_tenant`. List endpoint now mirrors detail's tenant-scoping.
- **List/detail alignment proven**. Pre-fix asymmetry was: list (`/api/v1/audit?range=7d`) returned legacy rows by default, but detail (`/api/v1/audit/<id>`) only matched `tenant_id=ws-default` rows → drill-in from a legacy row returned 404 (Scene-6 demo-gate). Post-fix both endpoints accept the same row-set for default workspace.
- **Multi-tenant boundary verified**. Constant `DEFAULT_AUDIT_TENANT_ID` exported from `crates/argo-audit/src/sink_tenant.rs:11` = `"ws-default"`, identical to local `DEFAULT_WORKSPACE_TENANT_FALLBACK` in `routes_audit_feed_scope.rs:8`. Minor coupling note: if the audit-side constant ever changes value, the local fallback would silently diverge; both should re-export from `argo-audit`. Not a merge blocker.

### Anti-theater check
Three tests, two of which are strict regression-pins (would FAIL on revert of the fix), one is a contract-pin:
1. `default_feed_legacy_id_fetches_detail` — **regression-pin**. Seeds a row with no `tenant_id` column (column defaults to `__legacy__`); asserts list status 200, list contains the id, AND detail returns 200. Pre-fix detail SQL `tenant_id = ?` with bind `"ws-default"` would not match `__legacy__` → detail returns 404 → test fails. Post-fix OR-branch matches → 200 → passes.
2. `explicit_tenant_feed_does_not_inherit_legacy_rows` — **regression-pin**. Seeds legacy row, lists with `X-Argo-Tenant-Id: tenant-a`, asserts NO item in response carries the legacy id. Pre-fix `include_legacy: true` unconditionally would have surfaced the legacy id into tenant-a's view → tenant-a sees rows it shouldn't. Post-fix `!explicit_workspace_tenant` flips include_legacy=false for tenant-a → legacy excluded → passes.
3. `explicit_tenant_feed_id_fetches_detail_but_not_other_tenant` — **contract-pin**. Same-tenant returns 200, cross-tenant returns 404. Passes on both pre and post-fix code paths (pre-fix had the same strict tenant_id equality for non-legacy rows). Useful: proves the fix didn't break cross-tenant scoping while widening the default-workspace gate.
- LOC budget all under ceiling: largest touched file is `routes_audit_feed.rs` at 488 LOC post-merge (+4 lines for `#[cfg(test)] mod` declaration, no functional change). New test file `routes_audit_list_detail_alignment_tests.rs` = 114 LOC.

### Verdict
🟢 PRE-MERGE PASS — closes Scene-6 demo-gate finding (`docs/skycot/ux-audits/2026-05-25-1100-scene-6-claim-6-trust-and-approvals.md`). Recommend MERGE on next conductor tick via `conductor-pre-merge-gate.sh --skip-clippy` (operator-locked authorization for evaluator-PRE-MERGE-PASS'd PRs within ~6h). **47th anti-theater-honest verification this week.**

---

## Scenario 2 — 🔴 PR #2300 `fix(web): prefix skills gateway API calls` — DO NOT MERGE — empirical 404 + envelope-shape mismatch

### Setup
- PR head `b072326a4049eeb13a8e3ab6bb23849c8d6529f6`, MERGEABLE per gh, 3 files +53/-11
- `git fetch origin pull/2300/head:pr-2300` (no worktree needed — pure path-string changes in `web/lib/api/skills.ts` + `web/lib/api/verticals.ts` plus new lint script `scripts/check-api-prefix.sh`)
- Verification axis: empirical curl against the live gateway at `:7400` (schema v81 FRESH; uptime ~5h since 09:44Z restart)

### What the PR does
Changes 6 path strings in `web/lib/api/*.ts` from `/skills*` and `/verticals*` to `/api/v1/skills*` and `/api/v1/verticals*` to satisfy the new `scripts/check-api-prefix.sh` lint (regex enforces every `apiGet/apiPost/...` literal under `web/lib/api/` to start with `/api/v1/`). PR title declares `fix(web):` only — gateway is untouched.

### Observed (empirical against live gateway, fresh curls this round)

| Path | Status | Body |
|---|---|---|
| `GET /skills` | **200** | `{"count":0,"risk":"medium","risk_defaulted":true,"skills":[]}` ← matches `SkillsEnvelope` |
| `GET /api/v1/skills` | **404** | `{"error":"not found"}` ← gateway catch-all |
| `GET /skills/learned-this-month` | **200** | real envelope |
| `GET /api/v1/skills/learned-this-month` | **404** | `{"error":"not found"}` |
| `GET /verticals` | **200** | `{"errors":[],"loaded":[33 verticals…],"ok":true}` ← matches `VerticalsEnvelope.loaded` |
| `GET /api/v1/verticals` | **200** | `{"verticals":[]}` ← **different envelope shape**, different handler |

### Static analysis confirming the empirical finding
- `crates/argo-cli/src/handlers/gateway/routes_skills.rs:11-46` (`dispatch_route`) matches **only** paths starting with `/skills/...` — not `/api/v1/skills/...`. Mounted via `routes_dispatch_get_catalog_groups::dispatch_skills_family` at line 64-120 of that file, which also matches only `/skills*`.
- `routes_skill_ir::dispatch` matches `GET /api/v1/skills/<name>/ir...` (the IR sub-path only); does not handle bare `/api/v1/skills`, `/api/v1/skills/<name>/usage`, or `/api/v1/skills/learned-this-month`.
- `routes_dispatch_get_api_groups.rs:35-37` routes `GET /api/v1/verticals` to `routes_verticals_list_get::list_response`, a DIFFERENT handler that returns the `{"verticals": [...]}` shape. `dispatch_verticals` at `routes_dispatch_get_catalog_groups.rs:16` routes `GET /verticals` to `routes_verticals::list_response`, which returns `{"loaded": [...], "errors": [], "ok": true}`.
- Web type `VerticalsEnvelope` at `web/lib/api/verticals.ts:42-50` declares optional `loaded?: VerticalRow[]` (the legacy shape); the new path's `{verticals: [...]}` shape is NOT read by the existing client code (`envelope.loaded ?? []` evaluates to empty array on the new payload).

### What breaks on merge
1. **Skills marketplace page (`/skills`)**: `fetchSkills` calls `apiGet<SkillsEnvelope>("/api/v1/skills")` → 404 → `apiGet` throws `ApiError` (not `TypeError`; see `web/lib/api/client.ts:122-148, 187`). The catch block at `web/lib/api/skills.ts:151-156` checks `isMissingOrUnreachable(err)` which returns true ONLY for `TypeError` instances (see `verticals.ts:189-194` for the shared helper shape). So the `ApiError` propagates. The page either error-states (strict prod) OR throws to the React boundary (non-prod). **Demo-blocker**: workers/skills/learned-this-month all 404. Scene-3 (skills marketplace install flow) and Scene-4 (worker uses skill, marketplace shows learned-this-month chip) of the v1.0 demo runbook depend on this page rendering.
2. **Skill detail/usage page (`/skills/<name>/usage`)**: `fetchSkillUsage` calls `/api/v1/skills/<name>/usage` → 404 → ApiError propagates → page errors.
3. **Learned-this-month filter chip**: `fetchLearnedThisMonth` calls `/api/v1/skills/learned-this-month` → 404 → ApiError propagates → chip errors or shows 0.
4. **Verticals list (`/verticals` page, builder dropdown, etc.)**: `fetchVerticals` calls `/api/v1/verticals` → 200 with `{verticals: [...]}` → `envelope.loaded ?? []` evaluates to `[]` → `verticals.length === 0` → in non-prod returns `SEED_VERTICALS` (the 8 hardcoded seed entries), in strict prod returns empty `[]`. **Camera-visible regression**: 33 real loaded verticals collapse to 8 seeds (non-prod) or zero (prod). The 30-second Scene-2 hire-flow demo and the workflow builder both stop seeing real bookkeeping-assistant / customer-onboarding / etc.
5. **Vertical detail (`/verticals/<id>`)**: `fetchVertical` calls `/api/v1/verticals/<id>` — not tested empirically, but same routing asymmetry expected.

### Anti-theater check
The web client's TypeScript compiles fine (the lint passes after the change), and the unit tests (if any exist for these clients in PR head) would not call the real gateway — so this defect is **invisible to type-checking and to the new lint script**. It only surfaces at runtime against the live gateway. Proof I'm not making this up:
- Re-ran each curl twice. Same results.
- The gateway's "not found" envelope (`{"error":"not found"}`) is the literal output of the gateway's catch-all 404 handler, distinguishable from a real handler's 404 (which returns a structured ApiError envelope).
- For `/api/v1/verticals` the 200 response IS real (handler `routes_verticals_list_get::list_response` runs) but returns a different shape; confirmed by static grep showing two distinct registration sites for verticals listings.

### Gap / follow-up brief candidates
- **CHOICE A — Close #2300, reauthor as a paired brief.** Recommended. Add a sibling brief that mounts `/api/v1/skills*` and the existing `/api/v1/verticals` routes to return the `loaded:` shape (or update `VerticalsEnvelope.verticals` reader in web). Web-side prefix renaming MUST land in the same merge wave as the gateway-side route registration, otherwise either direction merging first breaks production.
- **CHOICE B — Author follow-up brief** `gateway-mount-api-v1-skills-routes-+-align-verticals-list-envelope` (P0, ops, ~1h). Concretely:
  1. In `routes_dispatch_get_catalog_groups::dispatch_skills_family` and `routes_skills::dispatch_route`, accept BOTH `/skills*` and `/api/v1/skills*` prefixes (strip-prefix branch). Same for `routes_lists::skills_response` if it parses path directly.
  2. In `routes_verticals_list_get::list_response`, return `{loaded: [...], errors: [], ok: true}` to match the legacy `routes_verticals::list_response` shape — OR delete the new handler and have `dispatch_auth_and_meta` forward `/api/v1/verticals` to the legacy handler.
  3. Add a gateway-level integration smoke test that asserts both `/skills` and `/api/v1/skills` return identical non-empty envelopes (regression-pin for future prefix work).
- **Stop-gap, only if PR #2300 MUST merge now**: gate the prefix lint behind a `// codex-allow-legacy-prefix` comment on the four currently-failing call sites until the gateway lands the routes. Less clean than CHOICE B but unblocks the strangler-pattern strategy.
- **Recommendation to next conductor tick**: 🔴 BLOCK PR #2300 with the diagnosis above (paste the curl matrix into a PR comment). Do NOT --skip-clippy merge it. The current `conductor-pre-merge-gate.sh` does not exercise the live gateway, so it would not catch this on its own.

### Verdict
🔴 PRE-MERGE FAIL · P0 demo-blocker · DO NOT MERGE without paired gateway-side change. Severity rationale: directly breaks the v1.0 demo MP4 path (skills marketplace + verticals list both camera-visible in Scenes 1-4 of `docs/skycot/demos/v1-mission-mvp-runbook.md`).

---

## Scenario 3 — 🟢 Process drift sweep CLEAN; surface zombie-claim loops to next conductor

### Setup
- `bash scripts/check-gateway-freshness.sh` + `bash scripts/check-findings-only-fix-by.sh` + `bash scripts/check-unread-handoffs.sh`
- `git log --oneline origin/main -40` to inspect recent commit cadence
- Cross-checked the **T13-11 conductor heartbeat** (`docs/skycot/agents/heartbeats/conductor.json`), which fired 3 minutes before this evaluator round started

### Observed
- **Gateway**: 🟢 FRESH v81=v81 drift=0, uptime 25556s (~7h since 09:44Z restart).
- **Findings-only**: 4/4 within budget (all 2026-05-30, 5d remaining); 0 due today, 0 overdue.
- **Unread handoffs**: above conductor cutoff `2026-05-25T09:15:00Z` — composition (informational): the same 12 entries r62 reported (6 old conductor scheduled-tick ticks from yesterday + 2 supervisor-watchdog R5-kill T11-07 entries + 4 evaluator r58/r59/r60/r61 already actioned), PLUS the T12-07Z supervisor-watchdog ALERT and CONTEXT pair documenting the now-resolved slot-1 TRAP on `fix-replay-page-fabricated-forensic-report-for-unknown-action-ids` (the brief maps to PR #2296 which is the round-62 PRE-MERGE-PASS'd theater-fix and is still un-merged). Bookkeeping lag, no action required.
- **Conductor T13-11 tick already deferred merges** for concurrent-interactive-session safety. Its heartbeat explicitly logs MERGE-READY queue: #2274 (r60 PASS pack-lint), #2290 (r61 PASS kpi-rollup take 2), #2296 (r62 PASS replay theater fix). Conductor will retry at T14-11 if interactive session has wound down.
- **🚨 Zombie-claim loops** (also flagged in conductor T13-11 notes — my evaluator confirmation):
  - `fix-skills-detail-install-button-and-flow`: 12 claim/release cycles across the last ~12 hours per `git log` grep (literal "claim fix-skills-detail-install-button-and-flow" appears in commit subject lines), all marked `(zombie; no PR opened)`. Brief is being claimed but never produces a PR.
  - `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`: ≥3 zombie-releases since the conductor's latest BLOCK pass.
  - `fix-approvals-cross-tenant-read-path-verification`: conductor reports 4 zombie-releases.
- Number of consecutive unbroken process-loop health rounds: **8** (r55→r63 — `r55 BLOCK-in-0min · r56 MERGE-WITH-FOLLOWUP · r57 5-carry-closures · r58 carry-now-tracked · r59 KPI-rollup-intent-sound · r60 #2274 + #2268 LOC-fix · r61 #2295 LOC-fix + #2290 rebase · r62 #2296 theater-fix · r63 #2297 audit-tenant-scope + #2300 P0-block`).

### Verdict
🟢 process drift CLEAN. **Three PRE-MERGE-PASS'd PRs (#2274, #2290, #2296) have been waiting 90+ minutes** because T13-11 conductor deferred for concurrent-interactive-session safety. They should fire at T14-11 if interactive winds down, or the interactive operator can merge them now (the `--skip-clippy` authorization is already in place for evaluator-PRE-MERGE-PASS'd PRs).

---

## Recommendations to next conductor tick (T14-11Z or earlier interactive)

1. 🟢 **P0 MERGE #2297** — Scene-6 demo-gate audit list/detail tenant-scope fix. PRE-MERGE PASS confirmed this round. CLEAN MERGEABLE. Closes `docs/skycot/ux-audits/2026-05-25-1100-scene-6-claim-6-trust-and-approvals.md`. **NEW recommendation this round.**
2. 🟢 **P0 MERGE #2296** — Scene-6 demo-gate replay theater fix. PRE-MERGE PASS round 62. CLEAN MERGEABLE.
3. 🟢 **P0 MERGE #2290** — claim-2 KPI rollup take 2 (supersedes #2279). PRE-MERGE PASS round 61. CLEAN MERGEABLE.
4. 🟢 **P2 MERGE #2274** — pack-lint memory schema. PRE-MERGE PASS round 60. CLEAN MERGEABLE.
5. 🔴 **P0 BLOCK #2300** — `fix(web): prefix skills gateway API calls`. Empirical 404 on `/api/v1/skills*` + envelope-shape mismatch on `/api/v1/verticals`. Either close the PR or pair with gateway-side route mount. Recipe inline in Scenario 2 above. **NEW recommendation this round.**
6. 🔴 **P1 Author** `fix-pr-2295-+-pr-2268-split-event-tests-rs-loc-budget-breach` (batch brief, ~10min mechanical) — round-60 + round-61 finding, 7th-round carry for #2268.
7. 🔴 **P1 BLOCK / diagnose** the three zombie-claim loops. Recommend BLOCK with a diagnostic comment ("brief impossible / hidden dependency / file already updated upstream") — current behavior is wasting 6+ workers per brief without progress signal.
8. housekeeping **Close #2279** as superseded by #2290 per PR body follow-up checklist.

## Carry status
- (h) `mission-slack-inbound-tenant-mismatch-audit-emission` PR #2268 OPEN 9th-round (still CONFLICTING since 09:53Z; no fix-pr-2268 brief authored yet).
- (i) PR #2274 MERGEABLE — closes on merge.
- (m) `conductor-claim-cycle-detector` downgrade held.
- (n-r55) #2290 CLEAN — closes on merge.
- (o-r56) `fix-demo-seed-emit-worker-hired-audit-rows` tracked TODO row 214 — #2294 may bundle.
- (p-r57) `mission-demo-seed-real-llm-driver` PR #2294 OPEN pending PRE-MERGE eval (deferred again this round to keep budget on the load-bearing #2297/#2300 findings).
- (q-r60)+(r-r61) split `event_tests.rs` OUTSTANDING — batch as single brief.
- **(NEW r63, P0)** PR #2300 BLOCK recommendation — paired-PR strategy required before any web-side `/api/v1/...` prefix lint can merge.
- **(NEW r63, P1)** Zombie-claim loop diagnosis for `fix-skills-detail-install-button-and-flow`, `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`, and `fix-approvals-cross-tenant-read-path-verification`.
