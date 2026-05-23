# Mission claim dashboards — MVP + end-state scoring

**Authored**: 2026-05-23 (evaluator round 19) with direct operator sign-off.
**Status**: 🔴 LIVING DOCUMENT — each eval round updates the cells.
**Replaces**: the TBD rows in MISSION.md's "What 'LIVE' means per claim" table for claims 4, 5, 6.

## How this works

Per the operator framing (round 19): each claim has **two dashboards** running in parallel.

- **MVP dashboard** = the shippable bar. When MVP = 100% across every row, that claim is **MVP-LIVE** and v1 can launch on it.
- **End-state dashboard** = the full mission ambition. Every platform / channel / capability tracked individually with a tick (✅) or cross (❌) or in-flight marker (⏳). End-state may stay <100% through v1 / v1.x without lying about progress.

Cells use this legend:

| Symbol | Meaning |
|---|---|
| 🟢 ✅ | Verified working e2e + has regression test |
| 🟡 | Substrate exists, e2e not yet verified |
| ⏳ | In flight — brief CLAIMED, fix-by date logged |
| 🔴 ❌ | Not done; gap logged in mission-vs-code-gap-manifest.md |
| 🚫 | DEFERRED-v1.x per operator-locked strategic decision |

Cells should cite (a) the file/route/test that backs the state, or (b) the brief id that will land it.

---

## Claim 4 — Skills shared AND learned

**MVP requires BOTH the shared half AND the learned half.** Operator-locked round 19 — *"For claim 4 we got to achieve the learned skills."*

### Claim 4 — MVP dashboard

| # | Item | Cite | State |
|---|---|---|---|
| 1 | `argo skill install <pack>` writes a marketplace skill to the workspace's skill library | `argo-skill::library` | 🟡 substrate present, install path not e2e-verified |
| 2 | Worker invokes the installed skill through a workflow `skill_invoke` step | `wire-worker-session-runner` | ⏳ CLAIMED, fix-by 2026-05-30 |
| 3 | `SkillInvoked` audit row written with provenance (pack + version + author + tenant_id) | `argo-audit::AuditKind::SkillInvoked` | 🟡 audit kind exists, payload shape not verified |
| 4 | Novel-sequence detection — worker's 3× successful composed sequence surfaces as skill-candidate | `argo-skill::ingest_learned`, `argo-skill::feedback` | 🔴 unknown — needs round of audit work, may or may not exist |
| 5 | Skill-candidate review queue at `/skills` | round-6 `/skills` screenshot | 🔴 marketplace tab verified, "candidates" tab not surfaced — likely missing |
| 6 | Operator-approve flow for learned skill candidate | argo-skill approval path | 🔴 need to trace |
| 7 | Trust grade ratchets up after 3 successful invocations of the (now-learned) skill | `argo-trust-engine` | 🔴 ratchet signal-sources unaudited (gap-manifest row `eval-trust-engine-ratchet-from-zero-to-grade-A` open) |

**Claim 4 MVP score: ~30% (≈ 2 of 7 substrate-present, 1 in flight, 4 unknown-or-missing).**

When 7/7 turn 🟢 with regression tests, **MVP-LIVE**.

### Claim 4 — End-state dashboard

Adds on top of MVP. All MVP rows must already be 🟢 plus:

| # | Item | Cite | State |
|---|---|---|---|
| A | Cross-tenant marketplace — tenant_b installs tenant_a's learned skill via marketplace OCI distribution | `argo-marketplace::marketplace_oci` | 🟡 OCI substrate present, end-to-end install not verified |
| B | Provenance fully preserved across cross-tenant install (author + lineage chain visible to tenant_b) | argo-skill provenance fields | 🟡 fields exist, cross-tenant render not verified |
| C | Skill canary deployment — promoted skill rolls out to N% of traffic, falls back on regression | `argo-skill::canary`, migration 47 (`skill_canary`) | 🟡 schema + canary module exist, runtime rollout not verified |
| D | Skill A/B test infrastructure operational | `argo-skill::ab_test`, migration 20 | 🟡 schema + module exist |
| E | Skill rejection flow — operator rejects a learned skill candidate, candidate removed from queue, `SkillRejected` audit row written | argo-skill rejection path | 🔴 need to trace |
| F | Per-tenant trust grades (tenant_a's invocation of skill X doesn't affect tenant_b's grade for skill X) | `argo-trust-engine` | 🔴 need to verify scoping shape |
| G | Skill SDK supports Rust + at least one other language for authoring | `argo-skill-sdk` | 🔴 unaudited (gap-manifest row `eval-argo-skill-sdk-author-flow` open) |

**Claim 4 end-state score: ~25% (≈ 4 of 14 total items substrate-present, none e2e-verified yet).**

---

## Claim 5 — Any task a human can do on a PC

**Operator-locked MVP** (round 19): *"if computer use works e2e on Mac that is a score of 100% on MVP."*

### Claim 5 — MVP dashboard

| # | Item | Cite | State |
|---|---|---|---|
| 1 | macOS computer-use executor working e2e (Notes round-trip — create, type, read back) | `crates/argo-executor-computer-use-macos/tests/notes_smoke.rs` (env-gated) | 🟡 37/37 unit tests green; e2e smoke gated on `ARGO_NATIVE_MACOS_INTEGRATION=1`, not run regularly |
| 2 | Capability engine gates every cu action against the worker's vertical action-allowlist | `argo-capability` | 🟢 cap engine verified round 9 |
| 3 | Audit row written for every cu action (action_type + duration_ms + screenshot artifact id) | `argo-audit::AuditKind::ToolCall` | 🟢 dispatch path verified round 9 |
| 4 | Cross-tenant isolation — tenant_a's cu session can't see tenant_b's screen / files / Notes | `argo-agent-runner::dispatch_tenant_isolation_tests::mcp_pool_isolates_same_server_by_tenant` | 🟢 negative test passes round 9 |
| 5 | The cu loop is invoked from a real workflow step (not just unit tested) | `wire-worker-session-runner` | ⏳ CLAIMED, fix-by 2026-05-30 |

**Claim 5 MVP score: ~60% (3 of 5 🟢, 1 substrate-only, 1 in flight).**

When `wire-worker-session-runner` lands + notes_smoke runs green with the env flag in a documented eval cadence, **MVP-LIVE**.

### Claim 5 — End-state dashboard

| Platform | Computer-use | MCP | Audit | Cross-tenant | Workflow-integrated |
|---|---|---|---|---|---|
| **macOS** | 🟡 (notes_smoke gated) | 🟢 (round-10 mcp pool audited) | 🟢 | 🟢 | ⏳ wire-worker-session-runner |
| **Linux** | 🚫 DEFERRED-v1.x | 🟢 | 🟢 | 🟢 | ⏳ |
| **Windows** | 🚫 DEFERRED-v1.x | 🟢 | 🟢 | 🟢 | ⏳ |
| **E2B microVM** | 🚫 DEFERRED-v1.x | 🟢 | 🟢 | 🟢 | ⏳ |

**Claim 5 end-state score: cu ≈ 12.5% (1 of 4 platforms, and that one substrate-only) × MCP 100% × audit 100% × xt 100% × workflow ≈ 0%.**

(The deferred-v1.x cells are honest — operator locked Linux/Windows/E2B cu out of v1 explicitly per `project_v1_strategic_decisions_2026_05_14.md`.)

---

## Claim 6 — Multi-channel approval + multi-tenant by construction

**Operator-locked MVP** (round 19): *"one channel say slack working would be a 100% for that channel and the MVP."* MVP target channel: **Slack**.

### Claim 6 — MVP dashboard (Slack-only)

| # | Item | Cite | State |
|---|---|---|---|
| 1 | Worker triggers approval-gated action → Slack outbound message fires | `argo-channels::slack`, `worker_actions_notify_chat.rs` | 🟡 substrate present, e2e not verified |
| 2 | Slack inbound (operator approves via Slack interactive button) → action proceeds | `argo-channels::slack_inbound` | 🟡 substrate present (round 12), e2e not verified |
| 3 | HMAC-signed approval token in the Slack message body | `argo-channels::token` | 🟢 token signing verified (round 12 import trace) |
| 4 | `ApprovalGranted` audit row on approve; `ApprovalDeclined` on decline | `argo-audit::AuditKind::Approval*` | 🟢 substrate verified rounds 3/4 |
| 5 | Cross-tenant isolation — tenant_b can't see/decide tenant_a's pending Slack-approval queue | `argo-cli` gateway `routes_approvals_json`, audit-export #2051 | 🟢 substrate scoped + 1992 fix verified rounds 3/4 |
| 6 | Audit chain unbroken across approval + decision (round-15 strengthener — `argo audit verify` returns OK) | `argo audit verify` subcommand | 🟡 substrate verified at audit-record layer, full-chain verify not run |

**Claim 6 MVP score: ~55% (3 of 6 🟢, 3 substrate-only).**

When 6/6 turn 🟢 with regression tests covering an e2e Slack approval round-trip + audit-verify pass, **MVP-LIVE**.

### Claim 6 — End-state dashboard (channel × axis matrix)

Per round-6 audit, `argo-channels` ships 13 channel implementations. Mission text names 6 (Email, Slack, SMS, push, voice, web). End-state = full matrix green.

| Channel | Outbound | Inbound | Audit row | Cross-tenant | Signed token | Channel-level cite |
|---|---|---|---|---|---|---|
| **Email** | 🟡 | 🟡 | 🟢 | 🟢 | 🟢 | `argo-channels::email`, `email_inbound` |
| **Slack** | 🟡 | 🟡 | 🟢 | 🟢 | 🟢 | `argo-channels::slack`, `slack_inbound` |
| **iMessage** | 🟡 | 🔴 | 🟢 | 🟢 | 🔴 | `argo-channels::imessage` (no inbound module visible) |
| **Teams** | 🟡 | 🔴 | 🟢 | 🟢 | 🟢 | `argo-channels::teams`, `teams_payload` |
| **Discord** | 🟡 | 🔴 | 🟢 | 🟢 | 🔴 | `argo-channels::discord` |
| **SMS** | 🟡 | 🔴 | 🟢 | 🟢 | 🟢 | `argo-channels::sms` |
| **WhatsApp** | 🟡 | 🔴 | 🟢 | 🟢 | 🔴 | `argo-channels::whatsapp` |
| **Voice** | 🟡 | 🔴 | 🟢 | 🟢 | 🔴 | `argo-channels::voice`, `voice_format` |
| **Matrix** | 🟡 | 🔴 | 🟢 | 🟢 | 🔴 | `argo-channels::matrix` |
| **Push** | 🟡 | 🔴 | 🟢 | 🟢 | 🟢 | `argo-channels::push`, `push_format` |
| **Web inbox** | 🟡 | 🟡 | 🟢 | 🟢 | 🟢 | `argo-channels::webinbox` |
| **Audit chain (`argo audit verify`)** | n/a | n/a | n/a | n/a | n/a | 🟡 strengthener — separate row |

**Claim 6 end-state score: outbound ≈ 100% substrate × inbound ≈ 18% (only email + Slack + webinbox have real inbound) × audit 100% × xt 100% × token ≈ 45% (5 of 11 channels with signed-token visible).**

The 6 mission-named channels (Email, Slack, SMS, push, voice, web) are the operator's stated ambition. WhatsApp, Discord, Teams, Matrix, iMessage are beyond the mission text — operator can choose to keep or defer.

---

## How this doc updates

Every eval round that touches a claim 4/5/6 surface updates the corresponding row's cell. Each cell change adds a one-line audit-trail entry at the bottom of this doc (the changelog section below).

The `argo audit verify` mechanism + the channel inbound-path audit are likely the next two eval-round targets — they unlock the most green cells in claim 6's end-state dashboard.

## Changelog

| Date | Round | Change |
|---|---|---|
| 2026-05-23 | 19 | Initial authoring — operator-locked MVPs for claims 4/5/6; row-by-row state seeded from rounds 1-18 audits |
