# SKYCOT v1.0 — Definition + Ship Checklist + Mission-Claim Coverage Map

**Authored**: 2026-05-14 by conductor as the consolidated source-of-truth for the v1.0 ship gate. Drawn from `docs/skycot/MISSION.md`, `docs/build-plan/BUILD_PLAN_V11.md`, `docs/build-plan/PRODUCT_ROADMAP.md`, `docs/build-plan/MISSION_BACKLOG.md`, and the Tracks K/L/M overview briefs.

**Operator scope decision (2026-05-14)**: v1.0 = **all 5 MISSION.md success scenarios + ALL of Tracks L + M shipped + demo recording**. Comprehensive ship gate; no Phase-1-only shortcuts.

**Additional operator-locked scope decisions (2026-05-14 evening):**
- **macOS-first for computer-use.** `argo-executor-computer-use-macos` is the v1 primary execution path. E2B (cloud sandbox), Linux, Windows executors are DEFERRED-v1.x (still shipped, just not v1-polished).
- **Enterprise focus over consumer.** 4 consumer/redundant MCP connector stubs deleted (signal, telegram, mailgun, resend). 3 enterprise stubs kept as DEFERRED-v1.x placeholders (apollo, zoominfo, zoom).
- **🔴 v1 SHIP-BLOCKER discovered**: Phase A.5 audit (2026-05-14) found that workers do NOT load marketplace skills at runtime. `argo-agent-runner/src/turn.rs` never calls `LayeredLookup::with_library`. MISSION.md success scenario #2 silently fails. Fix: brief `claude-skill-library-worker-load-wire`, ~150 LOC. **This must ship before v1.**
- **Type canonicalization in progress.** 11+ duplicate `pub struct/enum` definitions across crates. 4 canonicalization briefs + 1 CI-lint brief authored.

Audit reports backing these decisions:
- `docs/skycot/audit/2026-05-14-phase-A-crate-inventory.md` — 88 crates × 6 claims
- `docs/skycot/audit/2026-05-14-phase-A.5-skills-loop.md` — claim-4 edge trace

> If this document disagrees with `docs/skycot/MISSION.md`, MISSION.md's mission statement + 6 claims + 5 success scenarios win (they are operator-authored ground truth). This document consolidates everything else.

---

## 1. The mission (verbatim)

> AI workers that humans can bring into their team as a real teammate, that can be delegated a responsibility / task / work / role; the platform can build an AI worker for it, draw upon a shared skills library, or even learn and build its own new skills; it has memory; it works autonomously and alongside humans; AI workers can fully autonomously do real-world work; that work is any task a human can do on a PC/Mac; approval requests can flow via many different communication methods.

## 2. The 6 load-bearing claims (verbatim from MISSION.md)

1. **Teammate, not tool.** A worker has persistent identity, voice, memory, role-membership over time. You don't run it; you delegate to it.
2. **Delegation is the unit.** "Be my marketing intern" or "handle inbound recruiter messages" is the contract — not "run this one job." A worker has duties, KPIs, and a charter.
3. **The platform builds workers.** Operator describes a role in plain English; SKYCOT generates the vertical (system prompt, action vocabulary, memory schema, eval seeds). No hand-authoring TOML.
4. **Skills are shared and learned.** A library workers draw from, plus a learning loop where successful novel sequences become reusable skills. Cross-tenant sharing with provenance + trust.
5. **Real-world execution = any task a human can do on a PC.** That requires two execution backends: **MCP** (well-known APIs / tools) AND **computer use** (Anthropic-style desktop control). Workers must have BOTH.
6. **Multi-channel approval gates.** Email, Slack, SMS, push, voice, web. The current CLI-only surface is a v0 placeholder, not the product.

## 3. The 5 v1.0 success scenarios (verbatim from MISSION.md)

We hit v1 launch when each of these works end-to-end:

1. **A generated worker runs autonomously over a non-CLI channel.** Operator says "build me a marketing intern named Maya"; Genesis emits the vertical; Maya runs duties on a schedule, asks for approval via Slack, gets it via Slack reply, posts a draft to Marketplace via computer-use, and logs the outcome — all without the operator opening a terminal.
2. **A learned skill propagates across workers.** Maya learns a "post-listing follow-up template" through repetition with operator approval; a different worker (e.g., a sales follow-up worker) discovers and reuses that skill from the shared library.
3. **Multi-worker handoff works end-to-end.** Worker A completes its duty, hands off to Worker B with shared memory; B continues without re-asking the operator the same questions A asked.
4. **Auto-approval under a budget works.** Operator sets "auto-approve up to $5 per low-risk action, max $50/day, only during weekday business hours"; worker observes the policy; no money spent outside the policy.
5. **Audit trail is complete.** Every action has a screenshot (computer-use), an evidence chain (MCP + research), and an approver (human or auto-approval policy). No silent steps.

## 4. Track inventory — what we must ship

Every track + phase + which claim(s) it advances + current state + ship-gate-criticality.

### Substrate tracks (claim 1, 2 — Teammate + Delegation primitives)

| Item | Where | Claim | State | v1.0-required |
|---|---|---|---|---|
| Worker / Persona / Role / Vertical primitives | `argo-identity`, `argo-vertical` | 1, 2 | ~85% (per MISSION.md 2026-05-07 snapshot; higher now) | ✅ Yes |
| Capability engine + RBAC | `argo-types::capability` | 1, 2, 5 | Shipped | ✅ Yes |
| Multi-tenant isolation | `argo-store` + every primitive | All | Shipped | ✅ Yes |
| Audit chain (hash-verified) | `argo-audit` | 1, 5 | Shipped | ✅ Yes |
| Duty scheduler | `argo-orchestrator` | 2 | Shipped (#939) | ✅ Yes |
| Layered memory composition | `argo-memory` | 1, 3 | Workspace-shared shipped; per-worker queued | ⚠️ per-worker scope needed |

### Track A — UX research

| Phase | State | v1.0-required |
|---|---|---|
| Persona docs + journey maps | DONE | ✅ |

### Track B — capability fill-ins

| Phase | State | v1.0-required |
|---|---|---|
| Backend capability mvps | DONE 2026-05-12 | ✅ |

### Track C — design language

| Phase | State | v1.0-required |
|---|---|---|
| ADR + Next.js scaffold + primitives + ApprovalCard + multi-tenant theming | DONE | ✅ |
| Storybook stories per primitive | TODO | ⚠️ Dev-experience, not blocking demo |
| Playwright e2e smoke | TODO | ⚠️ CI gate; not strictly demo-blocking |
| Real gateway wire-up against /inbox | TODO | ✅ Yes — required for live demo |

### Track D — wireframes

| Phase | State | v1.0-required |
|---|---|---|
| Wireframes | DONE 2026-05-11 | ✅ |

### Track E — frontend implementation (SPA)

| Phase | Sub-briefs | State | v1.0-required |
|---|---|---|---|
| E0 frontend foundation | 1 | MERGED | ✅ |
| E1 Approvals inbox | 6 | All MERGED | ✅ |
| E2 Hire flow | 9 | 7 MERGED, **2 CLAIMED** (E2-f, E2-i) | ✅ |
| E3 Team page | 5 | All MERGED | ✅ |
| E4 Worker profile | 9 | All MERGED | ✅ |
| E5 Audit timeline | 8 | All MERGED | ✅ |

Plus 5 mission-X-UI briefs MERGED: trust-grade-ui, skill-cohort-ui, retro-review-ui, approval-policy-ui, approval-bundle-ui.

### Track F — backend fill-ins for claim 4 (Skills)

| Phase | State | v1.0-required |
|---|---|---|
| Skill library + propagation + AB-test substrate | substantively MERGED 2026-05-12 | ✅ |

### Track G — claim 5 finish (PC execution)

| Phase | State | v1.0-required |
|---|---|---|
| G1/G2/G3 MERGED 2026-05-12 | DONE | ✅ |

### Track K — deployment

| Phase | State | v1.0-required |
|---|---|---|
| K1 docker-compose stack | MERGED | ✅ |
| K2 k8s manifests | MERGED | ✅ |
| K3 single-VM systemd | MERGED | ✅ |
| K4 first-run setup wizard | BLOCKED (v1+v2) | ⚠️ Nice-to-have; not strictly blocking |
| K5 deploy-smoke e2e | MERGED (workflow disabled per 0/71 success rate) | ⚠️ Workflow needs fix post-v1 |

### Track L — Skill Canvas (NEW, pulled into v1.0 by operator 2026-05-14)

**Operator-scoped: ALL 17 L-track briefs required for v1.0.**

| Phase | Briefs | State | Notes |
|---|---|---|---|
| Phase 1 — foundation + viewer + Ghostwriter | L1-L6 + L4-gw | **L1, L2, L4 MERGED**; L3, L4-gw, L5, L6 TODO/CLAIMED | First canvas surface live |
| Phase 2 — editor + diff | L7-L10 | BLOCKED on Phase 1 chain | |
| Phase 3 — observation extraction | L11-L13 | BLOCKED on Phase 1 chain | |
| Phase 4 — composition + library + e2e | L14-L17 | BLOCKED on Phase 3 chain | |

**Mission-claim alignment**: Track L primarily advances claim 4 (Skills shared+learned); L11 also advances claim 5 (CU recording); L13 advances claim 1 (teammate-refinement loop).

### Track M — Enterprise Parity (NEW, pulled into v1.0 by operator 2026-05-14)

**Operator-scoped: ALL 20 M-track briefs required for v1.0 (M2 PaddleOCR DEFERRED until first airgapped customer asks).**

| Sub-track | Briefs | State | Claim |
|---|---|---|---|
| M-Doc — Document Understanding | M1, M2 (DEFERRED), M3, M4 | M1 BLOCKED → unblocked; others TODO | 5 |
| M-Orch — Orchestration maturity | M5-M8 | M5 BLOCKED → unblocked; others TODO; **SLA tracking #1618 already MERGED** | 2, 6 |
| M-Eval — Regression eval | M9-M11 | M9 BLOCKED → unblocked; M10/M11 TODO | 4 |
| M-Code — Pro-code skill mode | M12-M14 | **M12 MERGED**; M13/M14 TODO | 3 |
| M-Tmpl — Vertical template packs | M15-M20 | M15-M20 TODO (operator pre-req: skycot-packs repo done) | 1, 3 |

## 5. v1.0 ship checklist (operator + swarm)

**Operator-only artifacts** (cannot be produced by the swarm):

- [x] `github.com/mark-hallam/skycot-packs` repo created (operator confirmed 2026-05-14)
- [ ] `docs/skycot/demos/v1-mission-mvp.mp4` recorded
- [ ] Demo script refreshed to walk through the SPA flow (current `demo_script.md` is CLI-based, predates SPA)

**Swarm-deliverable gates**:

- [ ] **🔴 SHIP-BLOCKER: `claude-skill-library-worker-load-wire` MERGED** (workers load marketplace skills at runtime; success scenario #2 works)
- [ ] All 5 MISSION.md success scenarios pass smoke test
- [ ] `scripts/smoke-skycot.sh` passes
- [ ] All Track L briefs MERGED (L1-L17 + L4-gw) — 18 briefs total; **3 done, 15 to go**
- [ ] All Track M briefs MERGED except M2 (DEFERRED) — 19 briefs; **2 done, 17 to go**
- [ ] All E2 hire-flow polish briefs MERGED (E2-f, E2-i — currently CLAIMED)
- [ ] Track C polish: real gateway wire-up at `/inbox` (Storybook + Playwright are post-v1)
- [ ] Production-hardening items from MISSION_BACKLOG marked as v1.0-relevant:
  - mission-vertical-action-executor-completion
  - mission-per-vertical-demo-script
  - mission-marketplace-seller-e2b-demo
  - mission-cross-tenant-trajectory-aggregation
  - mission-rtbf-dreams-propagation
  - mission-forge-spend-polling-race-fix
  - mission-canonical-exponential-backoff

## 6. Mission-claim coverage map

For each of the 6 claims: what code/tracks deliver it + gaps.

### Claim 1 — Teammate, not tool

**Delivering**:
- Substrate: Worker / Persona / Role / Vertical primitives in `argo-identity` + `argo-vertical`
- SPA surfaces: E2 (hire flow) + E3 (team page) + E4 (worker profile with persona/charter/duties/KPI/memory tabs)
- Track L13: suggested-edits banner — refinement loop is the teammate relationship
- Track M16-M20: 5 named-role vertical packs (Recruiter / AP Specialist / Vendor Manager / Support Specialist / Deal Desk Analyst)

**Gaps**: Per-worker memory scope (workspace-shared shipped, per-worker queued — `mission-per-worker-memory-scope` or equivalent brief).

### Claim 2 — Delegation is the unit

**Delivering**:
- Duty scheduler in `argo-orchestrator` (gateway boot wire-up shipped #939)
- KPI primitives + charter editor in E4
- M5 SLA tracking (already MERGED #1618)
- M6 retry policy (TODO)
- M7 queue management UI (TODO)
- M8 multi-channel approval routing composition (TODO)

**Gaps**: M6 + M7 + M8 not yet shipped; without them the "duties + KPIs + escalation" promise is partial.

### Claim 3 — The platform builds workers

**Delivering**:
- Genesis vertical generator (`argo-genesis`) — operator-described role → emitted vertical
- M12 TypeScript SDK (already MERGED #1629) — pro-code authoring tier
- M13 Rust SDK (TODO)
- M14 canvas↔code round-trip (TODO)
- M15 vertical-pack-loader (TODO; needs operator's skycot-packs repo — ✅ exists)

**Gaps**: M13 + M14 + M15 unshipped. Pack distribution untested end-to-end.

### Claim 4 — Skills are shared and learned

**Delivering**:
- Skill library substrate (`argo-skill`)
- Track F backend fill-ins (substantively MERGED)
- Track L (full Skill Canvas — Skill IR, storage, viewer, editor, diff, Ghostwriter, CU recording, suggested edits, composition, marketplace templates, e2e)
- M9-M11 regression eval + self-healing
- mission-cross-tenant-trajectory-aggregation (TODO)
- mission-rtbf-dreams-propagation (TODO)

**Coverage**: HIGHEST. This claim has the most code paths delivering it. Verify no parallel-system drift in the audit (e.g., L-track's Skill IR vs existing argo-skill `SKILL.md` parsing).

### Claim 5 — Real-world execution on PC

**Delivering**:
- MCP executor (`argo-mcp` + connectors)
- Computer-use executor (`argo-executor-computer-use` + e2b sandbox)
- M1 Claude vision extraction (TODO)
- M3 ExtractDocument canvas node (TODO)
- M4 table extraction (TODO)
- L11 CU-recording → IR (TODO)
- mission-vertical-action-executor-completion (TODO)
- mission-marketplace-seller-e2b-demo (TODO)

**Gaps**: M-Doc sub-track all TODO; without M1 the "extract data from invoice/receipt/contract" demo doesn't work. mission-vertical-action-executor-completion needed so the per-vertical action-executor isn't fallback-stubbed.

### Claim 6 — Multi-channel approval gates

**Delivering**:
- NotifyChannel trait (`argo-channels`) + 8 channel implementations
- E1 approvals SPA (6 sub-briefs all MERGED)
- M8 multi-channel approval routing composition (TODO)

**Gaps**: M8 not yet shipped — without it, the "Slack→SMS→email fallback" composition isn't operator-editable. SAML signature verification (mission-sso-saml-xml-dsig-verify) BLOCKED on SSO gateway dependency.

## 7. Documented gaps (for the audit to verify)

These are the categories the codebase audit must check against this document:

1. **Crate inventory**: every crate in `crates/` should serve at least one claim. Any crate that doesn't → flag as candidate for delete or rationale-document.
2. **Parallel-system drift**: L1's Skill IR types should not duplicate existing `argo-skill` types. SessionId / JudgeVerdict / Role across crates should be unique (audit-3 from 2026-05-13 found 5+ duplicates; recheck).
3. **Dead code**: pub items declared but never called. `cargo +nightly udeps` or manual rg.
4. **Test coverage gaps**: crates without `[[test]]` or without `_tests.rs` siblings.
5. **Mission-claim under-coverage**: any claim with <2 code paths delivering it = single point of failure.
6. **Mission-claim over-coverage / parallel impls**: any claim with 3+ overlapping impls = drift pattern #2 candidate.
7. **Operator pre-reqs**: are all operator-side artifacts documented + tracked? (demo MP4, skycot-packs, smoke script env)

## 8. Reference docs (current as of 2026-05-14)

- `docs/skycot/MISSION.md` — ground truth mission + 6 claims + 5 success scenarios. **NOTE: status snapshot is 2026-05-07 stale**; refresh next time a headline reconciliation brief lands.
- `docs/build-plan/PRODUCT_ROADMAP.md` — Tracks A-G + L + M (current)
- `docs/build-plan/BUILD_PLAN_V11.md` — backend phased plan v1.1-v2.2 (mostly current)
- `docs/build-plan/BUILD_PLAN_2WK.md` — original 14-day plan (referenced in MISSION.md)
- `docs/build-plan/MISSION_BACKLOG.md` — 280-item backlog with [NEW]/[BRIEF]/[SHIPPED]/[PARTIAL]/[DEFER] tags
- `docs/build-plan/BUILD_STATUS.md` — live ledger (~600 KB)
- `docs/build-plan/streams/L0-skill-canvas-track-overview.md` — Track L authoritative spec
- `docs/build-plan/streams/M0-enterprise-parity-track-overview.md` — Track M authoritative spec
- `docs/skycot/DRIFT_PREVENTION.md` — 8 drift patterns + brief pre-flight + audit cadence
- `scripts/lint-brief.sh` — pre-commit brief gate (drift-prevention layer 2)
- `scripts/find-stale-claim-drift.py` — drift sweep run every conductor tick

---

## 9. Refresh discipline

This document is the source-of-truth audit target. Refresh triggers:

- New track added → add to §4 inventory + §6 claim coverage map
- New mission claim (operator-authored only) → add to §2 + §3
- Ship-gate criterion changes → update §5
- Track phase completes → update state column in §4 + claim coverage in §6
- Audit finds drift → log in §7 + open brief

Next planned refresh: after the 2026-05-14 audit lands; expected to update §4 state columns + §6 claim coverage as L+M briefs ship.
