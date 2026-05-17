# 2026-05-18 Claude Conductor Audit — v1.0 Ship-Readiness

**HEAD**: `2fd3a9c5`
**Auditor**: claude conductor (4 parallel sub-agents + direct CI checks)
**Date**: 2026-05-18
**Prior audit**: `2026-05-17-claude-conductor-audit.md` (HEAD `fa849bbb`)
**Delta since prior**: ~25 merges, including #1819–#1825 + #1810/#1811/#1812/#1820/#1821/#1822/#1823/#1824

---

## TL;DR

🟢 **Demo recording GREEN**. Magic-link signup+verify is live on main via #1823; smoke-skycot.sh covers it (scenarios #34/#35); BrowserExecutor (macOS) builds cleanly; Slack-approval has two distinct E2E tests; demo script aligns with current code.

⚠️ **2 of 6 mission claims are PAPER, not PROVEN at the agent loop layer.**

- **Claim 3 (provider routing via TOML manifests with cost/tier/fallback)** — `argo-routing` crate exists with `TieredRouter` and `FixedRouter`, but ZERO consumers in agent-runner / agent-core / runtime / orchestrator. The only workspace consumer is `argo route inspect|bench` CLI. **Adaptive/cost-tier routing is paper-only at the agent loop layer.**
- **Claim 5 (hash-chained audit + replay)** — chain emission, verify, and replay CLI all exist and individually pass tests, but no single E2E test traces a session through `verify_session` + `Replayer` end-to-end. Marketing-claimable but not regression-locked.

These are the two highest-leverage findings. Everything else is polish.

---

## Audit Coverage Plan

What I would check as the operator, in priority order for a v1.0 ship-tonight:

| # | Category | Owner Agent |
|---|---|---|
| 1 | Build/CI health (compile, clippy, tests, LOC, migrations, unwrap/unsafe) | Agent A |
| 2 | Demo readiness (smoke, magic-link wire, BrowserExecutor, Slack, single-binary boot) | Agent B |
| 3 | Security posture (token hygiene, JWT cookie headers, tenant_id propagation, audit) | Agent B |
| 4 | Mission claims 1–6 runtime-PROVEN vs PAPER | Agent C |
| 5 | Architectural invariants (polyglot, multi-tenant, RBAC, manifests, MCP-first) | Agent C |
| 6 | Audit-closure check vs 2026-05-17 (13 findings) | Agent D |
| 7 | Swarm/ledger health (stale rows, brief quality, worker output) | Agent D |
| 8 | Product surface (SPA + marketing typecheck, README) | Agent D |

---

## Severity-Sorted Findings

### P0 — Must fix before recording

**None.** Demo recording is GREEN to proceed.

### P1 — Significant; operator should know

| # | Finding | Evidence | Impact |
|---|---|---|---|
| 1 | **Claim 3 (provider routing) not wired into agent loop** | `argo-routing` has 0 consumers in `argo-agent-runner/agent-core/runtime/orchestrator`; only `crates/argo-cli/src/handlers/route.rs` uses it | Launch claim "cost-tier/adaptive routing" is half-implemented; PR pitch / launch deck text claims it works |
| 2 | **Claim 5 missing full E2E (session → chain → replay)** | Components individually tested; no test ties them | Marketing-claimable but no regression lock; first prod-customer regression would slip silently |
| 3 | **Magic-link verify binds OLDEST tenant, not newest** | `crates/argo-cli/src/handlers/gateway/auth_magic_link.rs:149` — `tenants.pop()` against `ORDER BY created_at DESC` returns last element = oldest | Multi-tenant users get bound to wrong tenant; 1-line fix (`.into_iter().next()`). Sidesteps tonight's single-tenant demo |
| 4 | **Magic-link verify endpoint has no rate limit** | `auth_magic_link.rs:121-169` — no IP throttling; signup gets 5/hour, verify gets none | Brute-force surface (256-bit token makes it practically infeasible, but DoS risk remains) |
| 5 | **Magic-link verify emits no audit row** | `grep audit auth_magic_link.rs` → zero hits; only `tracing::warn!` on success | Demo script promises "every action is on this page"; auth flow itself isn't on the page |
| 6 | **`worker_actions_auto_approval.rs` at 504 LOC** | HARD RULE 6 violation (>500 ceiling); drifted +14 LOC from #1819 ApprovalAsked/Replied add | Anything new touching this file currently fails LOC gate; trivial sibling-split |
| 7 | **220 functions >80 LOC** | `argo-agent-runner/src/turn_helpers.rs:139 run_turn_with_tools` = 334 LOC; `routes_dispatch_post.rs:136 dispatch_post_inner` = 354 LOC; `boot.rs:67 start` = 278 LOC | Gate is advisory not blocking; tech debt accumulating |
| 8 | **15 unsafe blocks in Windows executor lack SAFETY comments** | `crates/argo-executor-computer-use-windows/{screenshot,a11y,preflight/probes,executor,events}.rs` | CLAUDE.md invariant violation. Per operator decision Windows is DEFERRED-v1.x — not ship-blocking but should be tracked |
| 9 | **web/ typecheck broken on main** | `pnpm typecheck` fails: `@xyflow/react` + `sonner` declared in `package.json` but missing from `node_modules`. Lockfile mentions exist (`pnpm-lock.yaml`) — needs `pnpm install` | Production deploy would fail; CI didn't catch it because typecheck-web isn't in the active CI lane while GHA is disabled |
| 10 | **`mission-sqlcipher-default-v1x` brief structurally infeasible** | Worker proved (BLOCKED note 2026-05-18) that Path A is incompatible with sqlx-based store; rusqlite swap = ~1430 callsite touches | Brief stays BLOCKED until re-authored as a 4-PR sequence |

### P2 — Polish; schedule before launch

| # | Finding | Evidence |
|---|---|---|
| 11 | `manual_is_multiple_of` clippy lint on chain-perf bench | `crates/argo-audit/benches/fixtures/seed_chain.rs:84`; already TODO via `codex-audit-bench-manual-is-multiple-of-fix` |
| 12 | 10 files in 450-499 LOC at-risk band | Already covered by `codex-split-files-450-500-wave` (CLAIMED) |
| 13 | README snapshot date stale (`2026-05-17`) | `README.md:7`; today is 2026-05-18 |
| 14 | CLAUDE.md still flags `#1796 magic-link rebase` as P0 SHIP-GATE | CLAUDE.md L122; #1823 already landed it |
| 15 | `DISABLED_WORKFLOWS_POLICY.md` missing `load-soak.yml.disabled` entry | Added by #1811 but not classified |
| 16 | `claude-rebase-pr-1796.md` brief STALE-FIXED | Recovery target #1823 merged; brief should be closed |
| 17 | Local smoke does not exercise magic-link in-process | `scripts/smoke-skycot.sh:486-517` only against `--target-url` |
| 18 | X-Forwarded-For trust without proxy allowlist | `auth_magic_link.rs:283-286`; spoofable behind generic L4 LB |
| 19 | IP rate-limit DashMap has no eviction | `auth_magic_link.rs:295`; memory leak over time |
| 20 | Cookie `secure` flag tied to `NODE_ENV=production` | `web/app/api/auth/verify/route.ts:64`; acceptable for v1, but staging deploys must have `NODE_ENV=production` |
| 21 | Provider wire/stream code in Rust, not TOML | `argo-provider/src/{anthropic,openai_compat,openai_responses}/{wire,stream}.rs` = ~1200 LOC; invariant 4 line is fuzzier than "manifests not adapters" suggests |
| 22 | `argo-plugin` ABI has no in-tree consumer | Only `tests/host.rs` exercises it (consistent with polyglot intent — real plugins are out-of-tree subprocesses — but unproven) |
| 23 | `pub` rustdoc not gate-enforced workspace-wide | No `#![deny(missing_docs)]`; coverage is by convention, not lint |

### CLEAN — Verified green

- Workspace `cargo check` (0 errors, 0 warnings)
- 5 of 5 architectural invariants ENFORCED (with 2 P2 caveats)
- 4 of 6 mission claims PROVEN (1, 2, 4, 6)
- Migration slot collisions (0)
- Parallel-pub-types canonicalization gate (passes)
- Single-binary CLI boot
- BrowserExecutor (macOS) builds
- 2 Slack-approval E2E tests
- Token-at-rest hygiene (32-byte CSPRNG, SHA-256 hash, 15-min TTL, single-use enforcement, unit test)
- SPA cookie hardening (HttpOnly + SameSite=Lax + path + maxAge matching JWT)
- Demo script alignment with code surface
- Ledger sanity (`find-stale-claim-drift.py` → 0 drift)
- BLOCKED row accuracy (5 of 5 reasons accurate)
- Worker output quality across last 10 PRs

---

## Root-Cause Analysis

Five themes explain the P1 cluster:

### RC1 — Crate-without-consumer drift (the headline)

**Symptom**: `argo-routing` ships as a complete crate (`Router` trait + `TieredRouter` + `FixedRouter` + fallback flag) but is not consumed by the agent loop.

**Root cause**: CLAUDE.md DRIFT_PREVENTION rule "wire or defer" was violated when `argo-routing` landed. The brief author declared the new public surface; no follow-up brief wired it into `argo-agent-runner::dispatch`.

**This is the exact failure mode** CLAUDE.md L121–123 warns about. Today's audit catches it; it has likely been latent for weeks.

### RC2 — Brief-authoring vs runtime-reality drift

**Symptom**: My own brief `mission-skill-md-capabilities-serde-fix` (authored last tick) said the fix was `Capability::Cu`. That variant doesn't exist. Worker correctly substituted `Capability::ToolDispatch { tool_name_glob: "computer_use" }`.

**Root cause**: Same family — "grep before write" wasn't applied tightly. CLAUDE.md DRIFT_PREVENTION rule violated by the conductor (me).

**Mitigation**: every brief that names a `pub` type must `rg 'pub (struct|enum|fn) <Name>'` before publishing.

### RC3 — LOC-ceiling drift from feature additions

**Symptom**: `worker_actions_auto_approval.rs` was at 490 LOC yesterday, 504 today. The push came from #1819 (`ApprovalAsked`/`ApprovalReplied` variants), which had to thread the new variants through the auto-approval policy logic.

**Root cause**: brief author didn't size the LOC delta against the call-site's current ceiling state. The PR's may-touch list named the file but didn't predict the +14 LOC.

**Mitigation**: brief-lint hook should refuse a may-touch entry pointing at a file ≥480 LOC unless the brief includes a sibling-split as its first step.

### RC4 — Tight rebase windows leave observability/edge-case gaps

**Symptom**: Magic-link landed via a tight conflict-resolution rebase. The result is correct on the happy path but missing:
- Audit emission (no `AuthSucceeded` row)
- Verify-endpoint rate limit
- Multi-tenant `tenants.pop()` correctness

**Root cause**: the recovery brief `claude-rebase-pr-1796` explicitly said "do not expand scope." Worker correctly resisted scope-creep; the gaps surface in a focused audit.

**Mitigation**: every SHIP-GATE rebase brief should ship a paired "post-merge hardening" brief that lands the observability + edge-case fixes the rebase deliberately skipped.

### RC5 — GHA disabled hides hard gates

**Symptom**: `manual_is_multiple_of` clippy lint from #1812 was on main for ~16h before #1823/#1824 noticed via `codex-pre-push.sh`. The CI-as-merge-gate flow that would have caught it (`workspace-clippy`) is in `.github/workflows/ci.yml` but blocked by operator-level GHA billing.

**Root cause**: operator-decisional (GHA disabled to control cost) trades a class of clippy/test drift for cost predictability. Worker-side `codex-pre-push.sh` is the partial substitute, but it only fires when a worker pushes — not on conductor admin-merges.

**Mitigation**: run `bash scripts/workspace-clippy-fast-fail.sh` as a conductor pre-merge gate. Add a column to BUILD_STATUS.md tick log: "pre-merge clippy: OK / FAIL on commit X".

---

## Fix Plan

Tiered by urgency. Each item is sized for a worker pick-up (200–600 LOC, single brief).

### Tier 1 — Demo-night (do before recording)

| # | Action | Who | Effort |
|---|---|---|---|
| T1.1 | Land `codex-audit-bench-manual-is-multiple-of-fix` (already TODO) | codex worker | 1 LOC |
| T1.2 | `pnpm install` in `web/` to fix typecheck | operator (or conductor commits lockfile) | 1 command |
| T1.3 | Run `bash scripts/demo-dry-run.sh` immediately before recording | operator | 5 min |

### Tier 2 — Pre-launch polish (this week)

Author the following briefs:

| # | Brief | Owner | Sizing | Closes |
|---|---|---|---|---|
| T2.1 | `mission-claim-3-runtime-wiring` — wire `argo-routing::TieredRouter` into `argo-agent-runner::dispatch` for the per-call provider pick; add cross-provider fallback E2E test | claude | ~600 LOC | **Claim 3 PAPER → PROVEN** |
| T2.2 | `mission-claim-5-runtime-closure-trace` — full E2E test: simulated session → `audit::verify_session` → `Replayer::replay` → assert trajectory equivalence | claude | ~300 LOC | **Claim 5 PARTIAL → PROVEN** |
| T2.3 | `mission-magic-link-hardening-followup` — fix `tenants.pop()` to `.into_iter().next()`; add 60/min/IP guard on `/verify`; emit `AuthEvent::MagicLinkVerified` audit row | claude | ~200 LOC | P1 #3, #4, #5 |
| T2.4 | `claude-split-worker-actions-auto-approval` — extract policy helper into sibling, drop to ≤420 LOC | claude | ~150 LOC | P1 #6 |
| T2.5 | `claude-launch-doc-refresh-2026-05-18` — bump README/CLAUDE.md/DISABLED_WORKFLOWS_POLICY; close `claude-rebase-pr-1796` brief as STALE-FIXED | claude | ~80 LOC | P2 #13, #14, #15, #16 |
| T2.6 | `codex-magic-link-trusted-proxy-and-eviction` — add `SKYCOT_TRUSTED_PROXIES` env-var allowlist; periodic eviction sweep on rate-limit DashMap | codex | ~120 LOC | P2 #18, #19 |
| T2.7 | `mission-local-smoke-magic-link-coverage` — add scenario #34a/#35a to `scripts/smoke-skycot.sh` exercising magic-link in-process (not just `--target-url`) | claude | ~80 LOC | P2 #17 |

### Tier 3 — Post-launch backlog

| # | Brief | Owner | Sizing |
|---|---|---|---|
| T3.1 | `codex-fn-loc-wave-2-top-20` — split the 20 largest >80 LOC functions (after current wave 1 lands) | codex | ~800 LOC across 5 PRs |
| T3.2 | `codex-windows-executor-safety-comments` — add `// SAFETY:` comments to the 15 unsafe blocks in `argo-executor-computer-use-windows/` | codex | ~30 LOC |
| T3.3 | `mission-sqlcipher-default-v1x-rewrite` — author as 4-PR sequence (workspace `libsqlite3-sys` patch → keyring wiring → migrate-CLI → default-flip) | operator decides + claude | ~2000 LOC across 4 PRs |
| T3.4 | `mission-provider-wire-tomlification` — fold per-provider `wire.rs/stream.rs` knowledge into manifest TOML (response-shape schemas) | claude | ~600 LOC |
| T3.5 | `claude-clippy-unwrap-expect-deny` — add `clippy::unwrap_used` and `clippy::expect_used` to workspace lints; clean up any production-path violations surfaced | claude | ~200 LOC |
| T3.6 | `claude-plugin-in-tree-smoke` — write a minimal in-tree plugin to exercise the `argo-plugin` JSON-RPC ABI for regression-lock | claude | ~300 LOC |
| T3.7 | `mission-incident-response-runbook-magic-link` — operator runbook for "what to check when magic-link breaks in prod" | claude | ~100 LOC |

### Tier 4 — Process changes

| # | Action | Trigger |
|---|---|---|
| T4.1 | Conductor pre-merge gate: run `bash scripts/workspace-clippy-fast-fail.sh` before every admin-merge while GHA is disabled | RC5 |
| T4.2 | Brief-lint hook upgrade: refuse a may-touch entry pointing at a file ≥480 LOC unless the brief includes a sibling-split as step 1 | RC3 |
| T4.3 | Brief-lint hook upgrade: refuse a brief that names a new `pub` type without a `rg pub (struct\|enum\|fn) <Name>` proof of non-collision | RC1, RC2 |
| T4.4 | Pair every SHIP-GATE rebase brief with a `<brief>-followup-hardening` companion brief that lands observability + edge-case fixes | RC4 |

---

## Operator Decisions Needed

1. **Is Claim 3 (cost-tier routing) load-bearing for the launch pitch?** If yes, T2.1 is Tier-1, not Tier-2. If no, accept it as v1.x scope and update the pitch / launch deck text to remove the claim.
2. **Is Claim 5 (full audit-replay E2E) needed before public launch?** Currently marketing-claimable; if a regression slipped post-launch we'd lose it silently.
3. **`pnpm install` in `web/` from this tab vs operator-side?** Lockfile edit would be a small commit; conductor mutation argument is borderline (deps were declared but not installed — drift between package.json and lockfile state).
4. **`mission-sqlcipher-default-v1x` re-author scope** — the worker's BLOCKED note proposes 4 PRs (~2000 LOC); is that v1.0 scope or v1.x?
5. **GHA reactivation** — RC5 says GHA-as-merge-gate would have caught `manual_is_multiple_of` in ~30s. Cost-vs-drift trade is live.

---

## Demo-Night Verdict

🟢 **GREEN to record tonight.** All ship-affecting findings are post-recording follow-ups. The single-user signup→verify→dashboard path works end-to-end on main; smoke covers it; Slack-approval has redundant E2E tests; BrowserExecutor (macOS) builds.

Recommended pre-record sequence:
1. `cd web && pnpm install` (fixes typecheck for prod deploy)
2. `bash scripts/demo-dry-run.sh` (smoke before recording)
3. Record per `docs/skycot/demo_script.md`
